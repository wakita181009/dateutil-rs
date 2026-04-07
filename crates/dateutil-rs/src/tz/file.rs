use std::io::Read;
use std::path::Path;

use chrono::{Duration, NaiveDateTime};

use super::range::TzStr;

// ============================================================================
// Data structures
// ============================================================================

/// Transition type info (from TZif ttinfo structure).
#[derive(Debug, Clone)]
pub struct TtInfo {
    /// UTC offset in seconds.
    pub offset: i32,
    /// Whether this is DST.
    pub is_dst: bool,
    /// Timezone abbreviation.
    pub abbr: String,
    /// The total UTC offset as a Duration.
    pub utoff: Duration,
}

/// A single transition: UTC timestamp and the associated ttinfo index.
#[derive(Debug, Clone)]
struct Transition {
    /// UTC timestamp of the transition.
    utc_time: i64,
    /// Wall-clock timestamp of the transition.
    wall_time: i64,
    /// Index into the ttinfo_list.
    ttinfo_idx: usize,
    /// Per-transition DST offset: the DST portion of the total offset,
    /// computed by pairing each DST transition with its preceding standard
    /// transition (matching python-dateutil's algorithm).
    dst_offset: Duration,
}

/// Timezone loaded from a TZif (RFC 8536) binary file.
#[derive(Debug, Clone)]
pub struct TzFile {
    /// All transitions (sorted by UTC time).
    transitions: Vec<Transition>,
    /// All ttinfo entries.
    ttinfo_list: Vec<TtInfo>,
    /// ttinfo for times before the first transition.
    ttinfo_before: usize,
    /// Default standard ttinfo index (reserved for future use).
    _ttinfo_std: Option<usize>,
    /// Default DST ttinfo index (reserved for future use).
    _ttinfo_dst: Option<usize>,
    /// Optional POSIX TZ string for times beyond the last transition (v2/v3).
    posix_tz: Option<TzStr>,
    /// Source filename (for display).
    filename: Option<String>,
}

// ============================================================================
// TZif binary parser
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum TzFileError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid TZif magic")]
    InvalidMagic,
    #[error("invalid TZif data: {0}")]
    InvalidData(String),
    #[error("POSIX TZ string parse error: {0}")]
    PosixTz(#[from] super::range::TzStrError),
}

impl TzFile {
    /// Load a timezone from a file path.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, TzFileError> {
        let path = path.as_ref();
        let data = std::fs::read(path)?;
        let filename = path.to_string_lossy().to_string();
        Self::from_bytes(&data, Some(filename))
    }

    /// Load a timezone from raw bytes.
    pub fn from_bytes(data: &[u8], filename: Option<String>) -> Result<Self, TzFileError> {
        let mut cursor = data;

        // Check magic
        let mut magic = [0u8; 4];
        cursor.read_exact(&mut magic)?;
        if &magic != b"TZif" {
            return Err(TzFileError::InvalidMagic);
        }

        // Version byte
        let mut version_byte = [0u8; 1];
        cursor.read_exact(&mut version_byte)?;
        let version = match version_byte[0] {
            b' ' | b'\0' => 1,
            b'2' => 2,
            b'3' => 3,
            _ => 1,
        };

        // Skip 15 reserved bytes
        let mut reserved = [0u8; 15];
        cursor.read_exact(&mut reserved)?;

        // Read v1 header counts
        let isutcnt = read_be_i32(&mut cursor)? as usize;
        let isstdcnt = read_be_i32(&mut cursor)? as usize;
        let leapcnt = read_be_i32(&mut cursor)? as usize;
        let timecnt = read_be_i32(&mut cursor)? as usize;
        let typecnt = read_be_i32(&mut cursor)? as usize;
        let charcnt = read_be_i32(&mut cursor)? as usize;

        if version == 1 {
            // Parse v1 data (32-bit timestamps)
            return Self::parse_data(
                &mut cursor, timecnt, typecnt, charcnt, leapcnt, isstdcnt, isutcnt, false, None,
                filename,
            );
        }

        // v2/v3: skip the v1 data block entirely
        let v1_data_size = timecnt * 4           // transition times (32-bit)
            + timecnt                            // transition type indices
            + typecnt * 6                        // ttinfo entries
            + charcnt                            // abbreviation strings
            + leapcnt * 8                        // leap second records (v1: 4+4)
            + isstdcnt                           // standard/wall indicators
            + isutcnt;                           // UT/local indicators
        skip_bytes(&mut cursor, v1_data_size)?;

        // Now read the v2/v3 header
        let mut magic2 = [0u8; 4];
        cursor.read_exact(&mut magic2)?;
        if &magic2 != b"TZif" {
            return Err(TzFileError::InvalidMagic);
        }
        // Skip version byte + 15 reserved
        skip_bytes(&mut cursor, 16)?;

        // v2/v3 counts
        let isutcnt2 = read_be_i32(&mut cursor)? as usize;
        let isstdcnt2 = read_be_i32(&mut cursor)? as usize;
        let leapcnt2 = read_be_i32(&mut cursor)? as usize;
        let timecnt2 = read_be_i32(&mut cursor)? as usize;
        let typecnt2 = read_be_i32(&mut cursor)? as usize;
        let charcnt2 = read_be_i32(&mut cursor)? as usize;

        // Read the POSIX TZ string after the v2/v3 data block
        // First we need to know the size of the v2/v3 data block
        // We need to parse the v2/v3 data AND then read the POSIX string
        // Save position in remaining data
        let remaining_before = cursor.len();
        let result = Self::parse_data(
            &mut cursor,
            timecnt2,
            typecnt2,
            charcnt2,
            leapcnt2,
            isstdcnt2,
            isutcnt2,
            true,
            None,
            filename.clone(),
        )?;

        // Read POSIX TZ string (after newline)
        let _consumed = remaining_before - cursor.len();
        // Now read the POSIX TZ string: \n<posix_tz>\n
        let posix_tz = if !cursor.is_empty() {
            // Skip leading newline
            if cursor.first() == Some(&b'\n') {
                cursor = &cursor[1..];
            }
            // Read until next newline or end
            let end = cursor.iter().position(|&b| b == b'\n').unwrap_or(cursor.len());
            let tz_str = std::str::from_utf8(&cursor[..end]).unwrap_or("");
            if !tz_str.is_empty() {
                TzStr::parse(tz_str, false).ok()
            } else {
                None
            }
        } else {
            None
        };

        Ok(TzFile {
            posix_tz,
            ..result
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn parse_data(
        cursor: &mut &[u8],
        timecnt: usize,
        typecnt: usize,
        charcnt: usize,
        leapcnt: usize,
        isstdcnt: usize,
        isutcnt: usize,
        is_64bit: bool,
        posix_tz: Option<TzStr>,
        filename: Option<String>,
    ) -> Result<Self, TzFileError> {
        // Read transition times
        let mut trans_times = Vec::with_capacity(timecnt);
        for _ in 0..timecnt {
            let t = if is_64bit {
                read_be_i64(cursor)?
            } else {
                read_be_i32(cursor)? as i64
            };
            trans_times.push(t);
        }

        // Read transition type indices
        let mut trans_type_indices = Vec::with_capacity(timecnt);
        for _ in 0..timecnt {
            let mut buf = [0u8; 1];
            cursor.read_exact(&mut buf)?;
            trans_type_indices.push(buf[0] as usize);
        }

        // Read ttinfo entries
        #[derive(Debug)]
        struct RawTtInfo {
            utoff: i32,
            is_dst: bool,
            abbr_idx: usize,
        }

        let mut raw_ttinfos = Vec::with_capacity(typecnt);
        for _ in 0..typecnt {
            let utoff = read_be_i32(cursor)?;
            let mut dst_buf = [0u8; 1];
            cursor.read_exact(&mut dst_buf)?;
            let is_dst = dst_buf[0] != 0;
            let mut idx_buf = [0u8; 1];
            cursor.read_exact(&mut idx_buf)?;
            let abbr_idx = idx_buf[0] as usize;
            raw_ttinfos.push(RawTtInfo {
                utoff,
                is_dst,
                abbr_idx,
            });
        }

        // Read abbreviation strings
        let mut abbr_data = vec![0u8; charcnt];
        cursor.read_exact(&mut abbr_data)?;

        // Skip leap second records
        let leap_size = if is_64bit { 12 } else { 8 };
        skip_bytes(cursor, leapcnt * leap_size)?;

        // Skip standard/wall indicators
        skip_bytes(cursor, isstdcnt)?;

        // Skip UT/local indicators
        skip_bytes(cursor, isutcnt)?;

        // Build ttinfo list with proper abbreviations
        let ttinfo_list: Vec<TtInfo> = raw_ttinfos
            .iter()
            .map(|raw| {
                let abbr = extract_abbr(&abbr_data, raw.abbr_idx);
                TtInfo {
                    offset: raw.utoff,
                    is_dst: raw.is_dst,
                    abbr,
                    utoff: Duration::seconds(raw.utoff as i64),
                }
            })
            .collect();

        if ttinfo_list.is_empty() {
            return Err(TzFileError::InvalidData(
                "no ttinfo entries in timezone file".into(),
            ));
        }

        // Find standard and DST ttinfo indices
        let ttinfo_std = ttinfo_list.iter().position(|t| !t.is_dst);
        let ttinfo_dst = ttinfo_list.iter().position(|t| t.is_dst);

        // Determine ttinfo_before (for times before first transition)
        let ttinfo_before = if !trans_type_indices.is_empty() {
            // Use the first non-DST ttinfo, or the first ttinfo
            ttinfo_std.unwrap_or(0)
        } else {
            0
        };

        // Build transitions with per-transition DST offsets.
        // Matches python-dateutil's algorithm: walk transitions in chronological
        // order, and when a DST entry follows a standard entry, compute
        // dst_offset = current_offset - previous_offset.
        let mut transitions = Vec::with_capacity(timecnt);
        let mut last_dst: Option<bool> = None;
        let mut last_offset: i32 = 0;
        let mut last_dst_offset: i32 = 0;

        for i in 0..timecnt {
            let utc_time = trans_times[i];
            let idx = trans_type_indices[i];
            let tt = &ttinfo_list[idx];
            let offset = tt.offset;
            let mut dstoffset: i32 = 0;

            if let Some(prev_was_dst) = last_dst {
                if tt.is_dst {
                    // Transitioning from standard to DST: compute the delta
                    if !prev_was_dst {
                        dstoffset = offset - last_offset;
                    }
                    // Carry forward previous DST offset if we couldn't compute one
                    // (e.g., consecutive DST entries)
                    if dstoffset == 0 && last_dst_offset != 0 {
                        dstoffset = last_dst_offset;
                    }
                    last_dst_offset = dstoffset;
                }
            }

            // Compute wall time: UTC time + base offset (total offset minus DST portion)
            let baseoffset = offset - dstoffset;
            let wall_time = utc_time + baseoffset as i64;

            transitions.push(Transition {
                utc_time,
                wall_time,
                ttinfo_idx: idx,
                dst_offset: Duration::seconds(dstoffset as i64),
            });

            last_dst = Some(tt.is_dst);
            last_offset = offset;
        }

        Ok(TzFile {
            transitions,
            ttinfo_list,
            ttinfo_before,
            _ttinfo_std: ttinfo_std,
            _ttinfo_dst: ttinfo_dst,
            posix_tz,
            filename,
        })
    }

    /// Find the index of the last transition before the given time.
    /// Returns None if the time is before all transitions.
    fn find_transition(&self, timestamp: i64, use_utc: bool) -> Option<usize> {
        if self.transitions.is_empty() {
            return None;
        }

        // Zero-allocation binary search via partition_point
        let idx = self.transitions.partition_point(|t| {
            let time = if use_utc { t.utc_time } else { t.wall_time };
            time <= timestamp
        });

        if idx == 0 {
            None
        } else {
            Some(idx - 1)
        }
    }

    /// Get the ttinfo for a given transition index.
    fn get_ttinfo(&self, idx: Option<usize>) -> &TtInfo {
        match idx {
            Some(i) => &self.ttinfo_list[self.transitions[i].ttinfo_idx],
            None => &self.ttinfo_list[self.ttinfo_before],
        }
    }

    /// Check if a wall datetime is beyond the last transition.
    fn is_beyond_last_transition(&self, dt: NaiveDateTime) -> bool {
        if let Some(last) = self.transitions.last() {
            dt.and_utc().timestamp() > last.wall_time
        } else {
            false
        }
    }

    /// Get the ttinfo for a wall datetime, considering fold for ambiguity.
    fn find_ttinfo(&self, dt: NaiveDateTime, fold: bool) -> &TtInfo {
        let timestamp = dt.and_utc().timestamp();
        let idx = self.find_transition(timestamp, false);

        if let Some(i) = idx {
            if i > 0 {
                let prev_tt = &self.ttinfo_list[self.transitions[i - 1].ttinfo_idx];
                let curr_tt = &self.ttinfo_list[self.transitions[i].ttinfo_idx];

                if !fold {
                    // fold=0: if ambiguous (fall-back overlap), use previous ttinfo
                    if prev_tt.offset > curr_tt.offset {
                        let curr_wall = self.transitions[i].wall_time;
                        let overlap = (prev_tt.offset - curr_tt.offset) as i64;
                        if timestamp >= curr_wall && timestamp < curr_wall + overlap {
                            return prev_tt;
                        }
                    }
                } else {
                    // fold=1: if in a gap (spring-forward), use previous ttinfo
                    // so that resolve_imaginary gets a different offset to try
                    if prev_tt.offset < curr_tt.offset {
                        let curr_wall = self.transitions[i].wall_time;
                        let gap = (curr_tt.offset - prev_tt.offset) as i64;
                        if timestamp >= curr_wall && timestamp < curr_wall + gap {
                            return prev_tt;
                        }
                    }
                }
            }
        }

        self.get_ttinfo(idx)
    }

    pub fn utcoffset(&self, dt: Option<NaiveDateTime>, fold: bool) -> Option<Duration> {
        let dt = dt?;
        if let Some(ref posix_tz) = self.posix_tz {
            if self.is_beyond_last_transition(dt) {
                return posix_tz.utcoffset(Some(dt), fold);
            }
        }
        let tt = self.find_ttinfo(dt, fold);
        Some(tt.utoff)
    }

    pub fn dst(&self, dt: Option<NaiveDateTime>, fold: bool) -> Option<Duration> {
        let dt = dt?;
        if let Some(ref posix_tz) = self.posix_tz {
            if self.is_beyond_last_transition(dt) {
                return posix_tz.dst(Some(dt), fold);
            }
        }
        Some(self.find_dst_offset(dt, fold))
    }

    /// Find the per-transition DST offset for a wall datetime, considering fold.
    fn find_dst_offset(&self, dt: NaiveDateTime, fold: bool) -> Duration {
        let timestamp = dt.and_utc().timestamp();
        let idx = self.find_transition(timestamp, false);

        if !fold {
            // fold=0: if ambiguous, use the *previous* transition's dst_offset
            // (first occurrence = DST time)
            if let Some(i) = idx {
                if i > 0 {
                    let prev_tt = &self.ttinfo_list[self.transitions[i - 1].ttinfo_idx];
                    let curr_tt = &self.ttinfo_list[self.transitions[i].ttinfo_idx];
                    if prev_tt.offset > curr_tt.offset {
                        let curr_wall = self.transitions[i].wall_time;
                        let overlap = (prev_tt.offset - curr_tt.offset) as i64;
                        if timestamp >= curr_wall && timestamp < curr_wall + overlap {
                            return self.transitions[i - 1].dst_offset;
                        }
                    }
                }
            }
        }

        match idx {
            Some(i) => self.transitions[i].dst_offset,
            None => Duration::zero(), // Before first transition: standard time
        }
    }

    pub fn tzname(&self, dt: Option<NaiveDateTime>, fold: bool) -> Option<String> {
        let dt = dt?;
        if let Some(ref posix_tz) = self.posix_tz {
            if self.is_beyond_last_transition(dt) {
                return posix_tz.tzname(Some(dt), fold);
            }
        }
        let tt = self.find_ttinfo(dt, fold);
        Some(tt.abbr.clone())
    }

    pub fn is_ambiguous(&self, dt: NaiveDateTime) -> bool {
        let timestamp = dt.and_utc().timestamp();

        // Check POSIX TZ string for times beyond last transition
        if let Some(ref posix_tz) = self.posix_tz {
            if let Some(last) = self.transitions.last() {
                if timestamp > last.wall_time {
                    return posix_tz.is_ambiguous(dt);
                }
            }
        }

        let idx = match self.find_transition(timestamp, false) {
            Some(i) => i,
            None => return false,
        };

        // Check if the current transition was a fall-back (offset decreased).
        // The ambiguous period starts at the current transition's wall_time
        // and lasts for the difference in offsets.
        if idx > 0 {
            let prev_tt = &self.ttinfo_list[self.transitions[idx - 1].ttinfo_idx];
            let curr_tt = &self.ttinfo_list[self.transitions[idx].ttinfo_idx];
            // Fall-back: previous offset (DST) > current offset (standard)
            if prev_tt.offset > curr_tt.offset {
                let curr_wall = self.transitions[idx].wall_time;
                let overlap = (prev_tt.offset - curr_tt.offset) as i64;
                return timestamp >= curr_wall && timestamp < curr_wall + overlap;
            }
        }

        false
    }

    pub fn fromutc(&self, dt: NaiveDateTime) -> (NaiveDateTime, bool) {
        let utc_ts = dt.and_utc().timestamp();

        // Check POSIX TZ string
        if let Some(ref posix_tz) = self.posix_tz {
            if let Some(last) = self.transitions.last() {
                if utc_ts > last.utc_time {
                    return posix_tz.fromutc(dt);
                }
            }
        }

        let idx = self.find_transition(utc_ts, true);
        let tt = self.get_ttinfo(idx);
        let wall = dt + tt.utoff;
        let fold = self.is_ambiguous(wall);
        (wall, fold)
    }

    /// Get the filename this timezone was loaded from.
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Get the POSIX TZ string (if v2/v3 file).
    pub fn posix_tz_str(&self) -> Option<&TzStr> {
        self.posix_tz.as_ref()
    }
}

impl std::fmt::Display for TzFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.filename {
            Some(name) => write!(f, "tzfile('{}')", name),
            None => write!(f, "tzfile(...)"),
        }
    }
}

// ============================================================================
// Binary reading helpers
// ============================================================================

fn read_be_i32(cursor: &mut &[u8]) -> Result<i32, TzFileError> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf)?;
    Ok(i32::from_be_bytes(buf))
}

fn read_be_i64(cursor: &mut &[u8]) -> Result<i64, TzFileError> {
    let mut buf = [0u8; 8];
    cursor.read_exact(&mut buf)?;
    Ok(i64::from_be_bytes(buf))
}

fn skip_bytes(cursor: &mut &[u8], n: usize) -> Result<(), TzFileError> {
    if cursor.len() < n {
        return Err(TzFileError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "not enough data",
        )));
    }
    *cursor = &cursor[n..];
    Ok(())
}

fn extract_abbr(abbr_data: &[u8], idx: usize) -> String {
    if idx >= abbr_data.len() {
        return String::new();
    }
    let end = abbr_data[idx..]
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(abbr_data.len() - idx);
    String::from_utf8_lossy(&abbr_data[idx..idx + end]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_system_utc() {
        // Try to read UTC timezone file (common path on Unix)
        let paths = [
            "/usr/share/zoneinfo/UTC",
            "/usr/share/zoneinfo/Etc/UTC",
        ];
        let tz = paths
            .iter()
            .find_map(|p| TzFile::from_path(p).ok());
        if let Some(tz) = tz {
            let dt = NaiveDateTime::parse_from_str("2020-06-15 12:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap();
            assert_eq!(tz.utcoffset(Some(dt), false), Some(Duration::zero()));
            assert_eq!(tz.dst(Some(dt), false), Some(Duration::zero()));
        }
        // Skip test if no timezone files available
    }

    #[test]
    fn test_read_new_york() {
        let paths = [
            "/usr/share/zoneinfo/America/New_York",
            "/usr/share/zoneinfo/US/Eastern",
        ];
        let tz = paths.iter().find_map(|p| TzFile::from_path(p).ok());
        if let Some(tz) = tz {
            // January: EST (UTC-5)
            let winter = NaiveDateTime::parse_from_str(
                "2020-01-15 12:00:00",
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap();
            assert_eq!(
                tz.utcoffset(Some(winter), false),
                Some(Duration::seconds(-18000))
            );

            // July: EDT (UTC-4)
            let summer = NaiveDateTime::parse_from_str(
                "2020-07-15 12:00:00",
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap();
            assert_eq!(
                tz.utcoffset(Some(summer), false),
                Some(Duration::seconds(-14400))
            );
        }
    }

    #[test]
    fn test_read_tokyo() {
        let paths = ["/usr/share/zoneinfo/Asia/Tokyo"];
        let tz = paths.iter().find_map(|p| TzFile::from_path(p).ok());
        if let Some(tz) = tz {
            let dt = NaiveDateTime::parse_from_str("2020-06-15 12:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap();
            // JST is UTC+9, no DST
            assert_eq!(
                tz.utcoffset(Some(dt), false),
                Some(Duration::seconds(32400))
            );
            assert_eq!(tz.dst(Some(dt), false), Some(Duration::zero()));
        }
    }

    #[test]
    fn test_fromutc() {
        let paths = ["/usr/share/zoneinfo/America/New_York"];
        let tz = paths.iter().find_map(|p| TzFile::from_path(p).ok());
        if let Some(tz) = tz {
            // UTC 2020-01-15 17:00:00 → EST 2020-01-15 12:00:00
            let utc = NaiveDateTime::parse_from_str(
                "2020-01-15 17:00:00",
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap();
            let (wall, _fold) = tz.fromutc(utc);
            let expected = NaiveDateTime::parse_from_str(
                "2020-01-15 12:00:00",
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap();
            assert_eq!(wall, expected);
        }
    }

    #[test]
    fn test_invalid_magic() {
        let data = b"NOT_TZif_data";
        let result = TzFile::from_bytes(data, None);
        assert!(result.is_err());
    }

    // --- tzname ---

    #[test]
    fn test_tzname_new_york() {
        let paths = ["/usr/share/zoneinfo/America/New_York"];
        let tz = paths.iter().find_map(|p| TzFile::from_path(p).ok());
        if let Some(tz) = tz {
            let winter = NaiveDateTime::parse_from_str("2020-01-15 12:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap();
            assert_eq!(tz.tzname(Some(winter), false), Some("EST".into()));

            let summer = NaiveDateTime::parse_from_str("2020-07-15 12:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap();
            assert_eq!(tz.tzname(Some(summer), false), Some("EDT".into()));
        }
    }

    // --- is_ambiguous ---

    #[test]
    fn test_is_ambiguous_normal() {
        let paths = ["/usr/share/zoneinfo/America/New_York"];
        let tz = paths.iter().find_map(|p| TzFile::from_path(p).ok());
        if let Some(tz) = tz {
            let normal = NaiveDateTime::parse_from_str("2020-06-15 12:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap();
            assert!(!tz.is_ambiguous(normal));
        }
    }

    // --- utcoffset / dst with None ---

    #[test]
    fn test_utcoffset_none_dt() {
        let paths = ["/usr/share/zoneinfo/UTC", "/usr/share/zoneinfo/Etc/UTC"];
        let tz = paths.iter().find_map(|p| TzFile::from_path(p).ok());
        if let Some(tz) = tz {
            assert_eq!(tz.utcoffset(None, false), None);
            assert_eq!(tz.dst(None, false), None);
            assert_eq!(tz.tzname(None, false), None);
        }
    }

    // --- Display ---

    #[test]
    fn test_display() {
        let paths = ["/usr/share/zoneinfo/America/New_York"];
        let tz = paths.iter().find_map(|p| TzFile::from_path(p).ok());
        if let Some(tz) = tz {
            let s = format!("{}", tz);
            assert!(s.contains("tzfile"));
        }
    }

    #[test]
    fn test_display_no_filename() {
        let paths = ["/usr/share/zoneinfo/UTC", "/usr/share/zoneinfo/Etc/UTC"];
        if let Some(path) = paths.iter().find(|p| std::path::Path::new(p).exists()) {
            let data = std::fs::read(path).unwrap();
            let tz = TzFile::from_bytes(&data, None).unwrap();
            let s = format!("{}", tz);
            assert!(s.contains("tzfile"));
        }
    }

    // --- filename / posix_tz_str accessors ---

    #[test]
    fn test_filename_accessor() {
        let paths = ["/usr/share/zoneinfo/America/New_York"];
        let tz = paths.iter().find_map(|p| TzFile::from_path(p).ok());
        if let Some(tz) = tz {
            assert!(tz.filename().is_some());
        }
    }

    // --- fold behavior ---

    #[test]
    fn test_fold_winter_summer() {
        let paths = ["/usr/share/zoneinfo/America/New_York"];
        let tz = paths.iter().find_map(|p| TzFile::from_path(p).ok());
        if let Some(tz) = tz {
            // fold=0 and fold=1 should return same offset for unambiguous time
            let dt = NaiveDateTime::parse_from_str("2020-06-15 12:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap();
            assert_eq!(tz.utcoffset(Some(dt), false), tz.utcoffset(Some(dt), true));
        }
    }

    // --- dst for Tokyo (no DST) ---

    #[test]
    fn test_dst_tokyo() {
        let paths = ["/usr/share/zoneinfo/Asia/Tokyo"];
        let tz = paths.iter().find_map(|p| TzFile::from_path(p).ok());
        if let Some(tz) = tz {
            let dt = NaiveDateTime::parse_from_str("2020-06-15 12:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap();
            assert_eq!(tz.dst(Some(dt), false), Some(Duration::zero()));
            assert!(!tz.is_ambiguous(dt));
        }
    }

    // --- New York DST in summer ---

    #[test]
    fn test_dst_new_york_summer() {
        let paths = ["/usr/share/zoneinfo/America/New_York"];
        let tz = paths.iter().find_map(|p| TzFile::from_path(p).ok());
        if let Some(tz) = tz {
            let summer = NaiveDateTime::parse_from_str("2020-07-15 12:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap();
            // DST should be non-zero in summer
            let dst = tz.dst(Some(summer), false);
            assert!(dst.is_some());
            assert_ne!(dst.unwrap(), Duration::zero());
        }
    }
}
