//! TzFile — TZif binary file parser (RFC 8536, v1/v2/v3).
//!
//! Parses system timezone files (e.g. `/usr/share/zoneinfo/America/New_York`)
//! into an efficient in-memory representation for fast offset lookups.

use std::sync::Arc;

use chrono::{Datelike, NaiveDateTime, TimeDelta};
use smallvec::SmallVec;

use crate::error::TzError;

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// Transition type info — compact, no heap allocation per entry.
#[derive(Debug, Clone)]
pub(crate) struct TtInfo {
    /// Total UTC offset in seconds.
    pub utoff: i32,
    /// Whether this is a DST period.
    pub is_dst: bool,
    /// Byte offset into `abbr_data`.
    pub abbr_start: u16,
    /// DST portion of the offset in seconds (computed during parse).
    pub dst_offset: i32,
}

/// Minimal POSIX TZ rule for TZif v2/v3 footer.
/// Only handles `Mm.w.d/time` format (covers virtually all real-world zones).
#[derive(Debug, Clone)]
#[allow(dead_code)] // fields used for Debug output; abbr fields needed for future gettz lookups
pub(crate) struct PosixTzRule {
    std_abbr: Box<str>,
    std_offset: i32, // seconds, POSIX sign convention (west = positive) → we negate
    dst_abbr: Box<str>,
    dst_offset: i32,
    start: TransitionRule,
    end: TransitionRule,
}

/// `Mm.w.d/time` transition rule.
#[derive(Debug, Clone)]
pub(crate) struct TransitionRule {
    month: u8,     // 1-12
    week: u8,      // 1-5 (5 = last)
    day: u8,       // 0-6 (Sunday = 0)
    time_secs: i32, // seconds from midnight (default 7200 = 02:00)
}

// ---------------------------------------------------------------------------
// TzFileData — the shared inner data
// ---------------------------------------------------------------------------

/// Parsed TZif data, behind `Arc` for shared ownership.
#[derive(Debug)]
pub struct TzFileData {
    /// UTC transition timestamps (sorted ascending).
    trans_utc: Vec<i64>,
    /// Wall-clock transition timestamps (sorted, for wall-time lookups).
    trans_wall: Vec<i64>,
    /// Ttinfo index for each transition.
    trans_idx: Vec<u8>,
    /// Transition type info entries.
    ttinfo: SmallVec<[TtInfo; 4]>,
    /// Raw abbreviation bytes (NUL-separated).
    abbr_data: Box<[u8]>,
    /// Ttinfo index for times before the first transition.
    ttinfo_before: u8,
    /// Cached ttinfo index for the first STD entry (O(1) POSIX resolve).
    ttinfo_std: u8,
    /// Cached ttinfo index for the first DST entry (O(1) POSIX resolve).
    ttinfo_dst: Option<u8>,
    /// Optional POSIX TZ rule for times beyond the last transition.
    posix_tz: Option<PosixTzRule>,
    /// Source filename for display.
    filename: Option<Box<str>>,
}

// ---------------------------------------------------------------------------
// TzFile — newtype around Arc<TzFileData>
// ---------------------------------------------------------------------------

/// A parsed TZif timezone file.
#[derive(Debug, Clone)]
pub struct TzFile(pub(crate) Arc<TzFileData>);

impl TzFile {
    /// Parse a TZif file from raw bytes.
    pub fn from_bytes(data: &[u8], filename: Option<&str>) -> Result<Self, TzError> {
        let inner = TzFileData::parse(data, filename)?;
        Ok(TzFile(Arc::new(inner)))
    }

    /// Read and parse a TZif file from a filesystem path.
    pub fn from_path(path: &str) -> Result<Self, TzError> {
        let data = std::fs::read(path)
            .map_err(|e| TzError::Io(format!("{}: {}", path, e).into()))?;
        Self::from_bytes(&data, Some(path))
    }

    /// UTC offset in seconds for a wall-clock datetime.
    pub fn utcoffset(&self, dt: NaiveDateTime, fold: bool) -> i32 {
        let tti = self.0.find_ttinfo_wall(dt, fold);
        tti.utoff
    }

    /// DST offset component in seconds.
    pub fn dst(&self, dt: NaiveDateTime, fold: bool) -> i32 {
        let tti = self.0.find_ttinfo_wall(dt, fold);
        tti.dst_offset
    }

    /// Timezone abbreviation.
    pub fn tzname(&self, dt: NaiveDateTime, fold: bool) -> &str {
        let tti = self.0.find_ttinfo_wall(dt, fold);
        self.0.abbr(tti)
    }

    /// Whether the given wall time is ambiguous (falls in a DST overlap).
    pub fn is_ambiguous(&self, dt: NaiveDateTime) -> bool {
        self.0.is_ambiguous(dt)
    }

    /// Convert a UTC datetime to wall time.
    pub fn fromutc(&self, dt: NaiveDateTime) -> NaiveDateTime {
        self.0.fromutc(dt)
    }

    /// Source filename, if available.
    pub fn filename(&self) -> Option<&str> {
        self.0.filename.as_deref()
    }
}

// ---------------------------------------------------------------------------
// TZif binary parsing
// ---------------------------------------------------------------------------

const TZIF_MAGIC: &[u8; 4] = b"TZif";
const HEADER_LEN: usize = 44;

impl TzFileData {
    fn parse(data: &[u8], filename: Option<&str>) -> Result<Self, TzError> {
        if data.len() < HEADER_LEN {
            return Err(TzError::InvalidData("file too short".into()));
        }
        if &data[0..4] != TZIF_MAGIC {
            return Err(TzError::InvalidMagic);
        }

        let version = data[4];

        // Parse v1 header to compute v1 data block size (needed to skip to v2).
        let (timecnt, typecnt, charcnt, leapcnt, isstdcnt, isutcnt) =
            Self::parse_header_counts(&data[20..44])?;

        let v1_data_size = timecnt * 4   // transition times (i32)
            + timecnt                     // transition type indices
            + typecnt * 6                 // ttinfo entries
            + charcnt                     // abbreviation strings
            + leapcnt * 8                 // leap second records
            + isstdcnt                    // standard/wall indicators
            + isutcnt;                    // UT/local indicators

        // For v2/v3, skip v1 block and re-parse.
        if version == b'2' || version == b'3' {
            let v2_start = HEADER_LEN + v1_data_size;
            if data.len() < v2_start + HEADER_LEN {
                return Err(TzError::InvalidData("v2 header truncated".into()));
            }
            if &data[v2_start..v2_start + 4] != TZIF_MAGIC {
                return Err(TzError::InvalidData("v2 magic mismatch".into()));
            }
            return Self::parse_v2v3(&data[v2_start..], filename);
        }

        // v1 only
        Self::parse_v1(data, filename)
    }

    fn parse_header_counts(hdr: &[u8]) -> Result<(usize, usize, usize, usize, usize, usize), TzError> {
        if hdr.len() < 24 {
            return Err(TzError::InvalidData("header counts truncated".into()));
        }
        let isutcnt  = u32::from_be_bytes([hdr[0],  hdr[1],  hdr[2],  hdr[3]])  as usize;
        let isstdcnt = u32::from_be_bytes([hdr[4],  hdr[5],  hdr[6],  hdr[7]])  as usize;
        let leapcnt  = u32::from_be_bytes([hdr[8],  hdr[9],  hdr[10], hdr[11]]) as usize;
        let timecnt  = u32::from_be_bytes([hdr[12], hdr[13], hdr[14], hdr[15]]) as usize;
        let typecnt  = u32::from_be_bytes([hdr[16], hdr[17], hdr[18], hdr[19]]) as usize;
        let charcnt  = u32::from_be_bytes([hdr[20], hdr[21], hdr[22], hdr[23]]) as usize;
        Ok((timecnt, typecnt, charcnt, leapcnt, isstdcnt, isutcnt))
    }

    fn parse_v1(data: &[u8], filename: Option<&str>) -> Result<Self, TzError> {
        let (timecnt, typecnt, charcnt, leapcnt, isstdcnt, isutcnt) =
            Self::parse_header_counts(&data[20..44])?;

        let mut pos = HEADER_LEN;
        let needed = timecnt * 4 + timecnt + typecnt * 6 + charcnt + leapcnt * 8 + isstdcnt + isutcnt;
        if data.len() < HEADER_LEN + needed {
            return Err(TzError::InvalidData("v1 data truncated".into()));
        }

        // Transition times (i32)
        let mut trans_utc = Vec::with_capacity(timecnt);
        for _ in 0..timecnt {
            let ts = i32::from_be_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as i64;
            trans_utc.push(ts);
            pos += 4;
        }

        // Transition type indices
        let trans_idx: Vec<u8> = data[pos..pos + timecnt].to_vec();
        pos += timecnt;

        // TtInfo entries
        let (ttinfo, abbr_data) = Self::parse_ttinfo_and_abbr(data, pos, typecnt, charcnt)?;

        Self::build(trans_utc, trans_idx, ttinfo, abbr_data, None, filename)
    }

    fn parse_v2v3(data: &[u8], filename: Option<&str>) -> Result<Self, TzError> {
        let (timecnt, typecnt, charcnt, leapcnt, isstdcnt, isutcnt) =
            Self::parse_header_counts(&data[20..44])?;

        let mut pos = HEADER_LEN;
        let needed = timecnt * 8 + timecnt + typecnt * 6 + charcnt + leapcnt * 12 + isstdcnt + isutcnt;
        if data.len() < HEADER_LEN + needed {
            return Err(TzError::InvalidData("v2/v3 data truncated".into()));
        }

        // Transition times (i64)
        let mut trans_utc = Vec::with_capacity(timecnt);
        for _ in 0..timecnt {
            let bytes: [u8; 8] = data[pos..pos+8].try_into().unwrap();
            trans_utc.push(i64::from_be_bytes(bytes));
            pos += 8;
        }

        // Transition type indices
        let trans_idx: Vec<u8> = data[pos..pos + timecnt].to_vec();
        pos += timecnt;

        // TtInfo entries + abbreviations
        let (ttinfo, abbr_data) = Self::parse_ttinfo_and_abbr(data, pos, typecnt, charcnt)?;
        pos += typecnt * 6 + charcnt;

        // Skip leap seconds, isstd, isut
        pos += leapcnt * 12 + isstdcnt + isutcnt;

        // Parse POSIX TZ footer (after newline)
        let posix_tz = if pos < data.len() && data[pos] == b'\n' {
            pos += 1; // skip leading newline
            if let Some(end) = data[pos..].iter().position(|&b| b == b'\n') {
                let footer = std::str::from_utf8(&data[pos..pos + end])
                    .map_err(|_| TzError::InvalidData("non-UTF-8 POSIX footer".into()))?;
                if footer.is_empty() {
                    None
                } else {
                    PosixTzRule::parse(footer).ok()
                }
            } else {
                None
            }
        } else {
            None
        };

        Self::build(trans_utc, trans_idx, ttinfo, abbr_data, posix_tz, filename)
    }

    fn parse_ttinfo_and_abbr(
        data: &[u8],
        pos: usize,
        typecnt: usize,
        charcnt: usize,
    ) -> Result<(SmallVec<[TtInfo; 4]>, Box<[u8]>), TzError> {
        #![allow(clippy::type_complexity)]
        let mut ttinfo = SmallVec::with_capacity(typecnt);
        let mut p = pos;
        for _ in 0..typecnt {
            let utoff = i32::from_be_bytes([data[p], data[p+1], data[p+2], data[p+3]]);
            let is_dst = data[p + 4] != 0;
            let abbr_start = data[p + 5] as u16;
            ttinfo.push(TtInfo {
                utoff,
                is_dst,
                abbr_start,
                dst_offset: 0, // computed later
            });
            p += 6;
        }
        let abbr_data = data[p..p + charcnt].into();
        Ok((ttinfo, abbr_data))
    }

    fn build(
        trans_utc: Vec<i64>,
        trans_idx: Vec<u8>,
        mut ttinfo: SmallVec<[TtInfo; 4]>,
        abbr_data: Box<[u8]>,
        posix_tz: Option<PosixTzRule>,
        filename: Option<&str>,
    ) -> Result<Self, TzError> {
        // Compute DST offsets: when a DST ttinfo follows a STD ttinfo,
        // dst_offset = utoff(DST) - utoff(STD).
        let mut last_std_utoff: Option<i32> = None;
        // Find initial standard offset from ttinfo list
        for tti in &ttinfo {
            if !tti.is_dst {
                last_std_utoff = Some(tti.utoff);
                break;
            }
        }
        for &ti in &trans_idx {
            let idx = ti as usize;
            if idx < ttinfo.len() {
                if ttinfo[idx].is_dst {
                    if let Some(std_off) = last_std_utoff {
                        ttinfo[idx].dst_offset = ttinfo[idx].utoff - std_off;
                    }
                } else {
                    last_std_utoff = Some(ttinfo[idx].utoff);
                }
            }
        }

        // Determine ttinfo_before: the first non-DST ttinfo, or index 0.
        let ttinfo_before = ttinfo.iter()
            .position(|t| !t.is_dst)
            .unwrap_or(0) as u8;

        // Cache STD/DST ttinfo indices for O(1) POSIX resolve.
        // Use the LAST transition's entries (modern offsets), not the first
        // in the ttinfo list (which may be historical LMT).
        let mut ttinfo_std = ttinfo_before;
        let mut ttinfo_dst: Option<u8> = None;
        for &ti in trans_idx.iter().rev() {
            let idx = ti as usize;
            if idx < ttinfo.len() {
                if !ttinfo[idx].is_dst && ttinfo_std == ttinfo_before {
                    ttinfo_std = idx as u8;
                }
                if ttinfo[idx].is_dst && ttinfo_dst.is_none() {
                    ttinfo_dst = Some(idx as u8);
                }
                if ttinfo_std != ttinfo_before && ttinfo_dst.is_some() {
                    break;
                }
            }
        }

        // Compute wall-clock transition times.
        let mut trans_wall = Vec::with_capacity(trans_utc.len());
        for (i, &utc_ts) in trans_utc.iter().enumerate() {
            // Wall time of this transition = UTC time + offset that was active BEFORE this transition.
            let prev_utoff = if i == 0 {
                ttinfo[ttinfo_before as usize].utoff
            } else {
                let prev_idx = trans_idx[i - 1] as usize;
                if prev_idx < ttinfo.len() {
                    ttinfo[prev_idx].utoff
                } else {
                    0
                }
            };
            trans_wall.push(utc_ts + prev_utoff as i64);
        }

        Ok(TzFileData {
            trans_utc,
            trans_wall,
            trans_idx,
            ttinfo,
            abbr_data,
            ttinfo_before,
            ttinfo_std,
            ttinfo_dst,
            posix_tz,
            filename: filename.map(|s| s.into()),
        })
    }

    /// Get abbreviation string for a TtInfo.
    fn abbr(&self, tti: &TtInfo) -> &str {
        let start = tti.abbr_start as usize;
        if start >= self.abbr_data.len() {
            return "???";
        }
        let end = self.abbr_data[start..]
            .iter()
            .position(|&b| b == 0)
            .map(|p| start + p)
            .unwrap_or(self.abbr_data.len());
        std::str::from_utf8(&self.abbr_data[start..end]).unwrap_or("???")
    }

    // -----------------------------------------------------------------------
    // Transition lookup
    // -----------------------------------------------------------------------

    /// Find the TtInfo for a wall-clock datetime, handling fold for ambiguous times.
    fn find_ttinfo_wall(&self, dt: NaiveDateTime, fold: bool) -> &TtInfo {
        let ts = datetime_to_timestamp(dt);

        if self.trans_wall.is_empty() {
            return self.ttinfo_fallback_posix(dt, fold);
        }

        // Binary search on wall-clock transitions.
        let idx = self.trans_wall.partition_point(|&t| t <= ts);

        if idx == 0 {
            return &self.ttinfo[self.ttinfo_before as usize];
        }

        // Beyond all stored transitions → use POSIX rule.
        if idx >= self.trans_wall.len() {
            if let Some(ref posix) = self.posix_tz {
                return self.posix_resolve(posix, dt, fold);
            }
        }

        let current_idx = idx - 1;

        // Handle ambiguous times (DST overlap / fold).
        // If fold=true and the time is in the overlap zone, use the NEXT transition's ttinfo.
        if fold && idx < self.trans_wall.len() {
            let next_tti_idx = self.trans_idx[idx] as usize;
            if next_tti_idx < self.ttinfo.len() {
                let next_tti = &self.ttinfo[next_tti_idx];
                let next_wall = self.trans_wall[idx];
                let curr_tti_idx = self.trans_idx[current_idx] as usize;
                if curr_tti_idx < self.ttinfo.len() {
                    let curr_tti = &self.ttinfo[curr_tti_idx];
                    let overlap = (curr_tti.utoff - next_tti.utoff) as i64;
                    if overlap > 0 && ts >= next_wall - overlap && ts < next_wall {
                        return next_tti;
                    }
                }
            }
        }

        let tti_idx = self.trans_idx[current_idx] as usize;
        if tti_idx < self.ttinfo.len() {
            &self.ttinfo[tti_idx]
        } else {
            &self.ttinfo[self.ttinfo_before as usize]
        }
    }

    /// Fallback using POSIX rule for timestamps beyond stored transitions.
    fn ttinfo_fallback_posix(&self, dt: NaiveDateTime, fold: bool) -> &TtInfo {
        if let Some(ref posix) = self.posix_tz {
            return self.posix_resolve(posix, dt, fold);
        }
        &self.ttinfo[self.ttinfo_before as usize]
    }

    /// Resolve ttinfo using POSIX rule, handling fold for ambiguous times.
    /// Uses cached `ttinfo_std` / `ttinfo_dst` indices for O(1) lookup.
    fn posix_resolve(&self, posix: &PosixTzRule, dt: NaiveDateTime, fold: bool) -> &TtInfo {
        let in_dst = if posix.is_ambiguous(dt) {
            !fold // fold=false → first occurrence (DST), fold=true → second (STD)
        } else {
            posix.is_in_dst(dt)
        };

        let fallback = &self.ttinfo[self.ttinfo_before as usize];
        if in_dst {
            self.ttinfo_dst
                .map(|i| &self.ttinfo[i as usize])
                .unwrap_or(fallback)
        } else {
            &self.ttinfo[self.ttinfo_std as usize]
        }
    }

    /// Find TtInfo for a UTC timestamp (used by fromutc).
    fn find_ttinfo_utc(&self, utc_ts: i64) -> &TtInfo {
        if self.trans_utc.is_empty() {
            return &self.ttinfo[self.ttinfo_before as usize];
        }

        let idx = self.trans_utc.partition_point(|&t| t <= utc_ts);
        if idx == 0 {
            return &self.ttinfo[self.ttinfo_before as usize];
        }

        let tti_idx = self.trans_idx[idx - 1] as usize;
        if tti_idx < self.ttinfo.len() {
            &self.ttinfo[tti_idx]
        } else {
            &self.ttinfo[self.ttinfo_before as usize]
        }
    }

    /// Check if a wall-clock datetime is ambiguous.
    fn is_ambiguous(&self, dt: NaiveDateTime) -> bool {
        let ts = datetime_to_timestamp(dt);

        // For timestamps beyond stored transitions, use POSIX rule.
        if self.trans_wall.is_empty()
            || ts > *self.trans_wall.last().unwrap()
        {
            if let Some(ref posix) = self.posix_tz {
                return posix.is_ambiguous(dt);
            }
            return false;
        }

        if self.trans_wall.len() < 2 {
            return false;
        }

        let idx = self.trans_wall.partition_point(|&t| t <= ts);
        if idx == 0 || idx >= self.trans_wall.len() {
            return false;
        }

        // Check if we're in the overlap region after transition at `idx`.
        let next_tti_idx = self.trans_idx[idx] as usize;
        let curr_tti_idx = self.trans_idx[idx - 1] as usize;

        if next_tti_idx >= self.ttinfo.len() || curr_tti_idx >= self.ttinfo.len() {
            return false;
        }

        let curr_utoff = self.ttinfo[curr_tti_idx].utoff;
        let next_utoff = self.ttinfo[next_tti_idx].utoff;
        let overlap = (curr_utoff - next_utoff) as i64;

        if overlap <= 0 {
            return false; // gap, not overlap
        }

        // Overlap zone is BEFORE the transition wall time:
        // clocks fall back, so [trans_wall - overlap, trans_wall) is ambiguous.
        let next_wall = self.trans_wall[idx];
        ts >= next_wall - overlap && ts < next_wall
    }

    /// Convert UTC to wall time.
    fn fromutc(&self, dt: NaiveDateTime) -> NaiveDateTime {
        let utc_ts = datetime_to_timestamp(dt);
        let tti = self.find_ttinfo_utc(utc_ts);
        dt + TimeDelta::seconds(tti.utoff as i64)
    }
}

// ---------------------------------------------------------------------------
// POSIX TZ rule parser (minimal, for TZif footer)
// ---------------------------------------------------------------------------

impl PosixTzRule {
    /// Check if a wall-clock datetime is ambiguous under this POSIX rule.
    /// Ambiguous = in the overlap period when clocks fall back.
    fn is_ambiguous(&self, dt: NaiveDateTime) -> bool {
        let year = dt.year();
        // DST end transition: clocks go from dst_offset back to std_offset.
        // The overlap is |dst_offset - std_offset| seconds before the transition wall time.
        let dst_end_utc = self.end.to_timestamp(year, self.dst_offset);
        let overlap = (self.dst_offset - self.std_offset) as i64; // e.g. 3600 for 1h DST
        if overlap <= 0 {
            return false;
        }
        // The transition happens at dst_end_utc (UTC). In wall time (DST),
        // that's dst_end_utc + dst_offset. After, it's dst_end_utc + std_offset.
        // The overlap region in wall time is:
        //   [dst_end_utc + std_offset, dst_end_utc + std_offset + overlap)
        let wall_ts = datetime_to_timestamp(dt);
        let overlap_start = dst_end_utc + self.std_offset as i64;
        wall_ts >= overlap_start && wall_ts < overlap_start + overlap
    }

    /// Check if datetime is in DST period (not considering ambiguity).
    fn is_in_dst(&self, dt: NaiveDateTime) -> bool {
        let year = dt.year();
        let ts = datetime_to_timestamp(dt);
        let dst_start_wall = self.start.to_timestamp(year, self.std_offset) + self.std_offset as i64;
        let dst_end_wall = self.end.to_timestamp(year, self.dst_offset) + self.dst_offset as i64;

        if dst_start_wall < dst_end_wall {
            ts >= dst_start_wall && ts < dst_end_wall
        } else {
            ts >= dst_start_wall || ts < dst_end_wall
        }
    }

    /// Parse a POSIX TZ string like "EST5EDT,M3.2.0,M11.1.0".
    fn parse(s: &str) -> Result<Self, TzError> {
        let err = || TzError::InvalidPosixTz(s.into());

        // Split at comma to get std/dst spec and rules
        let (tz_spec, rules_str) = s.split_once(',').ok_or_else(err)?;

        // Parse standard timezone: name + offset
        let (std_abbr, rest) = parse_posix_name(tz_spec)?;
        let (std_offset_posix, rest) = parse_posix_offset(rest).ok_or_else(err)?;
        // POSIX: west is positive, we use east-is-positive → negate
        let std_offset = -std_offset_posix;

        // Parse DST timezone (optional offset)
        let (dst_abbr, rest) = parse_posix_name(rest)?;
        let (dst_offset, _) = if !rest.is_empty() && (rest.starts_with('+') || rest.starts_with('-') || rest.as_bytes()[0].is_ascii_digit()) {
            let (off, r) = parse_posix_offset(rest).ok_or_else(err)?;
            (-off, r)
        } else {
            // Default: DST offset = std_offset + 3600 (1 hour ahead)
            (std_offset + 3600, rest)
        };

        // Parse transition rules
        let (start_str, end_str) = rules_str.split_once(',').ok_or_else(err)?;
        let start = TransitionRule::parse(start_str).ok_or_else(err)?;
        let end = TransitionRule::parse(end_str).ok_or_else(err)?;

        Ok(PosixTzRule {
            std_abbr: std_abbr.into(),
            std_offset,
            dst_abbr: dst_abbr.into(),
            dst_offset,
            start,
            end,
        })
    }
}

impl TransitionRule {
    /// Parse `Mm.w.d` or `Mm.w.d/time`.
    fn parse(s: &str) -> Option<Self> {
        let (rule, time_str) = if let Some((r, t)) = s.split_once('/') {
            (r, Some(t))
        } else {
            (s, None)
        };

        if !rule.starts_with('M') {
            return None; // Only M-format supported
        }

        // Parse "m.w.d" without allocating a Vec
        let inner = &rule[1..];
        let mut iter = inner.split('.');
        let month: u8 = iter.next()?.parse().ok()?;
        let week: u8 = iter.next()?.parse().ok()?;
        let day: u8 = iter.next()?.parse().ok()?;
        if iter.next().is_some() {
            return None; // too many parts
        }

        if !(1..=12).contains(&month) || !(1..=5).contains(&week) || day > 6 {
            return None;
        }

        let time_secs = match time_str {
            Some(t) => parse_posix_time(t)?,
            None => 2 * 3600, // default 02:00
        };

        Some(TransitionRule { month, week, day, time_secs })
    }

    /// Compute the UTC timestamp for this rule in a given year.
    /// `base_offset` is the UTC offset (in seconds) active before this transition.
    fn to_timestamp(&self, year: i32, base_offset: i32) -> i64 {
        use chrono::{NaiveDate, Datelike, Weekday};

        let target_weekday = match self.day {
            0 => Weekday::Sun,
            1 => Weekday::Mon,
            2 => Weekday::Tue,
            3 => Weekday::Wed,
            4 => Weekday::Thu,
            5 => Weekday::Fri,
            _ => Weekday::Sat,
        };

        let first_of_month = NaiveDate::from_ymd_opt(year, self.month as u32, 1).unwrap();
        let first_weekday = first_of_month.weekday();
        let days_ahead = (target_weekday.num_days_from_sunday() as i32
            - first_weekday.num_days_from_sunday() as i32 + 7) % 7;

        let mut day = 1 + days_ahead + (self.week as i32 - 1) * 7;

        // Week 5 means "last occurrence" — clamp to valid days in month.
        let days_in_month = days_in_month_of(year, self.month as u32);
        if day > days_in_month as i32 {
            day -= 7;
        }

        let date = NaiveDate::from_ymd_opt(year, self.month as u32, day as u32).unwrap();
        let dt = date.and_hms_opt(0, 0, 0).unwrap();
        datetime_to_timestamp(dt) + self.time_secs as i64 - base_offset as i64
    }
}

/// Parse a POSIX TZ name (alphabetic or quoted).
fn parse_posix_name(s: &str) -> Result<(&str, &str), TzError> {
    if s.is_empty() {
        return Err(TzError::InvalidPosixTz("empty timezone name".into()));
    }
    if s.starts_with('<') {
        // Quoted name: <ABC>
        let end = s.find('>').ok_or_else(|| TzError::InvalidPosixTz(s.into()))?;
        Ok((&s[1..end], &s[end + 1..]))
    } else {
        let end = s.bytes()
            .position(|b| !b.is_ascii_alphabetic())
            .unwrap_or(s.len());
        if end < 3 {
            return Err(TzError::InvalidPosixTz(format!("name too short: {}", &s[..end]).into()));
        }
        Ok((&s[..end], &s[end..]))
    }
}

/// Parse a POSIX offset like "5", "-5", "5:30", "-5:30:15".
fn parse_posix_offset(s: &str) -> Option<(i32, &str)> {
    if s.is_empty() {
        return None;
    }

    let (sign, rest) = if let Some(stripped) = s.strip_prefix('-') {
        (-1i32, stripped)
    } else if let Some(stripped) = s.strip_prefix('+') {
        (1, stripped)
    } else {
        (1, s)
    };

    // Find end of offset (digits and colons)
    let end = rest.bytes()
        .position(|b| !b.is_ascii_digit() && b != b':')
        .unwrap_or(rest.len());

    let offset_str = &rest[..end];
    let remaining = &rest[end..];

    let secs = parse_posix_time(offset_str)?;
    Some((sign * secs, remaining))
}

/// Parse a time string "H", "H:M", "H:M:S" into seconds.
fn parse_posix_time(s: &str) -> Option<i32> {
    let mut iter = s.split(':');
    let h: i32 = iter.next()?.parse().ok()?;
    let m: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let sec: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    Some(h * 3600 + m * 60 + sec)
}

// ---------------------------------------------------------------------------
// Timestamp ↔ NaiveDateTime conversion
// ---------------------------------------------------------------------------

#[inline]
fn datetime_to_timestamp(dt: NaiveDateTime) -> i64 {
    dt.and_utc().timestamp()
}

#[inline]
#[allow(dead_code)] // used in tests; will be used by TzLocal in Phase 3
fn timestamp_to_datetime(ts: i64) -> NaiveDateTime {
    chrono::DateTime::from_timestamp(ts, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap())
        .naive_utc()
}

/// Number of days in a given month.
fn days_in_month_of(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap(year) { 29 } else { 28 },
        _ => 30,
    }
}

fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use super::*;

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, mi, s).unwrap()
    }

    // -----------------------------------------------------------------------
    // TZif parsing — real system files
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_utc() {
        let tz = TzFile::from_path("/usr/share/zoneinfo/UTC").unwrap();
        let d = dt(2024, 6, 15, 12, 0, 0);
        assert_eq!(tz.utcoffset(d, false), 0);
        assert_eq!(tz.dst(d, false), 0);
        assert_eq!(tz.tzname(d, false), "UTC");
        assert!(!tz.is_ambiguous(d));
    }

    #[test]
    fn test_parse_tokyo() {
        let tz = TzFile::from_path("/usr/share/zoneinfo/Asia/Tokyo").unwrap();
        let d = dt(2024, 6, 15, 12, 0, 0);
        assert_eq!(tz.utcoffset(d, false), 9 * 3600); // JST = UTC+9
        assert_eq!(tz.dst(d, false), 0); // No DST
        assert_eq!(tz.tzname(d, false), "JST");
        assert!(!tz.is_ambiguous(d));
    }

    #[test]
    fn test_parse_new_york_summer() {
        let tz = TzFile::from_path("/usr/share/zoneinfo/America/New_York").unwrap();
        // June: EDT (UTC-4)
        let d = dt(2024, 6, 15, 12, 0, 0);
        assert_eq!(tz.utcoffset(d, false), -4 * 3600);
        assert_eq!(tz.dst(d, false), 3600);
        assert_eq!(tz.tzname(d, false), "EDT");
    }

    #[test]
    fn test_parse_new_york_winter() {
        let tz = TzFile::from_path("/usr/share/zoneinfo/America/New_York").unwrap();
        // January: EST (UTC-5)
        let d = dt(2024, 1, 15, 12, 0, 0);
        assert_eq!(tz.utcoffset(d, false), -5 * 3600);
        assert_eq!(tz.dst(d, false), 0);
        assert_eq!(tz.tzname(d, false), "EST");
    }

    // -----------------------------------------------------------------------
    // DST gap (spring forward) — America/New_York
    // 2024-03-10 02:00 EST → 03:00 EDT (gap: 02:00-03:00 doesn't exist)
    // -----------------------------------------------------------------------

    #[test]
    fn test_spring_forward_gap() {
        let tz = TzFile::from_path("/usr/share/zoneinfo/America/New_York").unwrap();
        // 02:30 is in the gap — should report EDT offset (post-transition)
        let gap_dt = dt(2024, 3, 10, 2, 30, 0);
        // Before transition: EST (-5h), after: EDT (-4h)
        let off_fold0 = tz.utcoffset(gap_dt, false);
        let _off_fold1 = tz.utcoffset(gap_dt, true);
        // Gap time: fold=false → pre-transition (EST), fold=true → post-transition (EDT)
        assert!(off_fold0 == -5 * 3600 || off_fold0 == -4 * 3600);
        // Gap is NOT ambiguous
        assert!(!tz.is_ambiguous(gap_dt));
    }

    // -----------------------------------------------------------------------
    // DST overlap (fall back) — America/New_York
    // 2024-11-03 02:00 EDT → 01:00 EST (overlap: 01:00-02:00 happens twice)
    // -----------------------------------------------------------------------

    #[test]
    fn test_fall_back_overlap() {
        let tz = TzFile::from_path("/usr/share/zoneinfo/America/New_York").unwrap();
        let overlap_dt = dt(2024, 11, 3, 1, 30, 0);
        assert!(tz.is_ambiguous(overlap_dt));

        // fold=false → first occurrence (EDT, UTC-4)
        let off0 = tz.utcoffset(overlap_dt, false);
        // fold=true → second occurrence (EST, UTC-5)
        let off1 = tz.utcoffset(overlap_dt, true);
        assert_eq!(off0, -4 * 3600); // EDT
        assert_eq!(off1, -5 * 3600); // EST
    }

    // -----------------------------------------------------------------------
    // fromutc
    // -----------------------------------------------------------------------

    #[test]
    fn test_fromutc_tokyo() {
        let tz = TzFile::from_path("/usr/share/zoneinfo/Asia/Tokyo").unwrap();
        let utc = dt(2024, 6, 15, 0, 0, 0);
        let wall = tz.fromutc(utc);
        assert_eq!(wall, dt(2024, 6, 15, 9, 0, 0));
    }

    #[test]
    fn test_fromutc_new_york_summer() {
        let tz = TzFile::from_path("/usr/share/zoneinfo/America/New_York").unwrap();
        let utc = dt(2024, 6, 15, 16, 0, 0);
        let wall = tz.fromutc(utc);
        assert_eq!(wall, dt(2024, 6, 15, 12, 0, 0)); // EDT = UTC-4
    }

    // -----------------------------------------------------------------------
    // POSIX TZ rule parser
    // -----------------------------------------------------------------------

    #[test]
    fn test_posix_parse_est_edt() {
        let rule = PosixTzRule::parse("EST5EDT,M3.2.0,M11.1.0").unwrap();
        assert_eq!(&*rule.std_abbr, "EST");
        assert_eq!(rule.std_offset, -5 * 3600);
        assert_eq!(&*rule.dst_abbr, "EDT");
        assert_eq!(rule.dst_offset, -4 * 3600);
        assert_eq!(rule.start.month, 3);
        assert_eq!(rule.start.week, 2);
        assert_eq!(rule.start.day, 0);
        assert_eq!(rule.end.month, 11);
        assert_eq!(rule.end.week, 1);
    }

    #[test]
    fn test_posix_parse_with_time() {
        let rule = PosixTzRule::parse("CET-1CEST,M3.5.0/2,M10.5.0/3").unwrap();
        assert_eq!(&*rule.std_abbr, "CET");
        assert_eq!(rule.std_offset, 3600);
        assert_eq!(&*rule.dst_abbr, "CEST");
        assert_eq!(rule.dst_offset, 7200);
        assert_eq!(rule.start.time_secs, 7200);
        assert_eq!(rule.end.time_secs, 10800);
    }

    #[test]
    fn test_posix_parse_quoted_names() {
        let rule = PosixTzRule::parse("<+05>-5<+06>,M3.5.0,M10.5.0").unwrap();
        assert_eq!(&*rule.std_abbr, "+05");
        assert_eq!(rule.std_offset, 5 * 3600);
    }

    // -----------------------------------------------------------------------
    // Timestamp conversion
    // -----------------------------------------------------------------------

    #[test]
    fn test_timestamp_roundtrip() {
        let d = dt(2024, 6, 15, 12, 30, 45);
        let ts = datetime_to_timestamp(d);
        let back = timestamp_to_datetime(ts);
        assert_eq!(d, back);
    }

    // -----------------------------------------------------------------------
    // TzFile Clone (Arc-based, cheap)
    // -----------------------------------------------------------------------

    #[test]
    fn test_tzfile_clone_is_cheap() {
        let tz1 = TzFile::from_path("/usr/share/zoneinfo/UTC").unwrap();
        let tz2 = tz1.clone();
        // Both point to the same Arc
        assert!(Arc::ptr_eq(&tz1.0, &tz2.0));
    }

    // -----------------------------------------------------------------------
    // Error cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_invalid_magic() {
        let data = b"NotATzifFile000000000000000000000000000000000000";
        let err = TzFile::from_bytes(data, None).unwrap_err();
        assert!(matches!(err, TzError::InvalidMagic));
    }

    // -----------------------------------------------------------------------
    // filename accessor
    // -----------------------------------------------------------------------

    #[test]
    fn test_filename() {
        let tz = TzFile::from_path("/usr/share/zoneinfo/UTC").unwrap();
        assert_eq!(tz.filename(), Some("/usr/share/zoneinfo/UTC"));
    }

    #[test]
    fn test_filename_none_for_bytes() {
        let data = std::fs::read("/usr/share/zoneinfo/UTC").unwrap();
        let tz = TzFile::from_bytes(&data, None).unwrap();
        assert_eq!(tz.filename(), None);
    }

    // -----------------------------------------------------------------------
    // Far-future dates (exercises POSIX TZ rule path)
    // -----------------------------------------------------------------------

    #[test]
    fn test_far_future_posix_rule() {
        // Year 2100 is beyond stored transitions — uses POSIX footer rule
        let tz = TzFile::from_path("/usr/share/zoneinfo/America/New_York").unwrap();
        // Summer 2100: should still get EDT
        let d_summer = dt(2100, 7, 15, 12, 0, 0);
        assert_eq!(tz.utcoffset(d_summer, false), -4 * 3600);
        // Winter 2100: should still get EST
        let d_winter = dt(2100, 1, 15, 12, 0, 0);
        assert_eq!(tz.utcoffset(d_winter, false), -5 * 3600);
    }

    #[test]
    fn test_far_future_ambiguous() {
        // November 2100 fall-back overlap should still work via POSIX rule
        let tz = TzFile::from_path("/usr/share/zoneinfo/America/New_York").unwrap();
        // First Sunday in November 2100 is Nov 7
        let overlap_dt = dt(2100, 11, 7, 1, 30, 0);
        assert!(tz.is_ambiguous(overlap_dt));
    }

    #[test]
    fn test_far_future_no_dst_timezone() {
        // Tokyo has no DST — far future should still return JST
        let tz = TzFile::from_path("/usr/share/zoneinfo/Asia/Tokyo").unwrap();
        assert_eq!(tz.utcoffset(dt(2100, 6, 15, 12, 0, 0), false), 9 * 3600);
        assert!(!tz.is_ambiguous(dt(2100, 6, 15, 12, 0, 0)));
    }

    // -----------------------------------------------------------------------
    // fromutc — winter (EST)
    // -----------------------------------------------------------------------

    #[test]
    fn test_fromutc_new_york_winter() {
        let tz = TzFile::from_path("/usr/share/zoneinfo/America/New_York").unwrap();
        let utc = dt(2024, 1, 15, 17, 0, 0);
        let wall = tz.fromutc(utc);
        assert_eq!(wall, dt(2024, 1, 15, 12, 0, 0)); // EST = UTC-5
    }

    // -----------------------------------------------------------------------
    // Overlap boundary precision
    // -----------------------------------------------------------------------

    #[test]
    fn test_overlap_boundary_start() {
        let tz = TzFile::from_path("/usr/share/zoneinfo/America/New_York").unwrap();
        // Exactly 01:00 on fall-back day — start of overlap
        let boundary = dt(2024, 11, 3, 1, 0, 0);
        assert!(tz.is_ambiguous(boundary));
    }

    #[test]
    fn test_overlap_boundary_end() {
        let tz = TzFile::from_path("/usr/share/zoneinfo/America/New_York").unwrap();
        // Exactly 02:00 on fall-back day — end of overlap, no longer ambiguous
        let boundary = dt(2024, 11, 3, 2, 0, 0);
        assert!(!tz.is_ambiguous(boundary));
    }

    #[test]
    fn test_not_ambiguous_before_overlap() {
        let tz = TzFile::from_path("/usr/share/zoneinfo/America/New_York").unwrap();
        assert!(!tz.is_ambiguous(dt(2024, 11, 3, 0, 59, 59)));
    }

    // -----------------------------------------------------------------------
    // POSIX TZ rule parsing — edge cases and errors
    // -----------------------------------------------------------------------

    #[test]
    fn test_posix_parse_no_comma() {
        assert!(PosixTzRule::parse("EST5EDT").is_err());
    }

    #[test]
    fn test_posix_parse_empty() {
        assert!(PosixTzRule::parse("").is_err());
    }

    #[test]
    fn test_posix_parse_invalid_month() {
        // Month 13 is invalid
        assert!(PosixTzRule::parse("EST5EDT,M13.2.0,M11.1.0").is_err());
    }

    #[test]
    fn test_posix_parse_invalid_week() {
        // Week 0 is invalid
        assert!(PosixTzRule::parse("EST5EDT,M3.0.0,M11.1.0").is_err());
    }

    #[test]
    fn test_posix_parse_invalid_day() {
        // Day 7 is invalid (0-6 only)
        assert!(PosixTzRule::parse("EST5EDT,M3.2.7,M11.1.0").is_err());
    }

    #[test]
    fn test_posix_parse_non_m_format() {
        // Only M-format is supported
        assert!(PosixTzRule::parse("EST5EDT,J60,J300").is_err());
    }

    #[test]
    fn test_posix_parse_short_name_rejected() {
        // Names must be >= 3 characters
        assert!(PosixTzRule::parse("AB5CD,M3.2.0,M11.1.0").is_err());
    }

    #[test]
    fn test_posix_parse_explicit_dst_offset() {
        // CET-1CEST-2 means DST offset is 2 hours east (negated POSIX sign)
        let rule = PosixTzRule::parse("CET-1CEST-2,M3.5.0,M10.5.0").unwrap();
        assert_eq!(rule.std_offset, 3600);
        assert_eq!(rule.dst_offset, 7200);
    }

    #[test]
    fn test_posix_default_transition_time() {
        let rule = PosixTzRule::parse("EST5EDT,M3.2.0,M11.1.0").unwrap();
        assert_eq!(rule.start.time_secs, 2 * 3600); // default 02:00
        assert_eq!(rule.end.time_secs, 2 * 3600);
    }

    #[test]
    fn test_posix_dst_is_in_dst() {
        let rule = PosixTzRule::parse("EST5EDT,M3.2.0,M11.1.0").unwrap();
        // Mid-summer (July) should be DST
        assert!(rule.is_in_dst(dt(2024, 7, 15, 12, 0, 0)));
        // Mid-winter (January) should not be DST
        assert!(!rule.is_in_dst(dt(2024, 1, 15, 12, 0, 0)));
    }

    #[test]
    fn test_posix_is_ambiguous_fall_back() {
        let rule = PosixTzRule::parse("EST5EDT,M3.2.0,M11.1.0").unwrap();
        // Fall-back: first Sunday in November 2024 = Nov 3
        // Overlap at 01:00-02:00 EST wall time
        assert!(rule.is_ambiguous(dt(2024, 11, 3, 1, 30, 0)));
        // Before overlap
        assert!(!rule.is_ambiguous(dt(2024, 11, 3, 0, 30, 0)));
        // After overlap
        assert!(!rule.is_ambiguous(dt(2024, 11, 3, 2, 30, 0)));
    }

    #[test]
    fn test_posix_not_ambiguous_spring_forward() {
        let rule = PosixTzRule::parse("EST5EDT,M3.2.0,M11.1.0").unwrap();
        // Spring forward: no overlap
        assert!(!rule.is_ambiguous(dt(2024, 3, 10, 2, 30, 0)));
    }

    // -----------------------------------------------------------------------
    // TransitionRule::to_timestamp
    // -----------------------------------------------------------------------

    #[test]
    fn test_transition_rule_week5_last_occurrence() {
        // M3.5.0 means "last Sunday in March"
        let rule = TransitionRule::parse("M3.5.0/2").unwrap();
        // Last Sunday in March 2024 is March 31
        let ts = rule.to_timestamp(2024, -5 * 3600);
        let d = timestamp_to_datetime(ts);
        assert_eq!(d.month(), 3);
        // Should be around March 31 in UTC
        assert!(d.day() >= 30);
    }

    #[test]
    fn test_transition_rule_february_leap_year() {
        // M2.5.0 means "last Sunday in February"
        let rule = TransitionRule::parse("M2.5.0/2").unwrap();
        // 2024 is a leap year: Feb has 29 days
        let ts = rule.to_timestamp(2024, 0);
        let d = timestamp_to_datetime(ts);
        assert_eq!(d.month(), 2);
        assert!(d.day() >= 25);
    }

    // -----------------------------------------------------------------------
    // Helper functions: days_in_month_of and is_leap
    // -----------------------------------------------------------------------

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month_of(2024, 1), 31);
        assert_eq!(days_in_month_of(2024, 2), 29); // leap year
        assert_eq!(days_in_month_of(2023, 2), 28); // non-leap
        assert_eq!(days_in_month_of(2024, 4), 30);
        assert_eq!(days_in_month_of(2024, 12), 31);
    }

    #[test]
    fn test_is_leap() {
        assert!(is_leap(2024));
        assert!(!is_leap(2023));
        assert!(!is_leap(1900)); // century non-leap
        assert!(is_leap(2000));  // 400-year leap
        assert!(!is_leap(2100)); // century non-leap
    }

    // -----------------------------------------------------------------------
    // Additional error cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_truncated_file() {
        let data = b"TZif";
        let err = TzFile::from_bytes(data, None).unwrap_err();
        assert!(matches!(err, TzError::InvalidData(_)));
    }

    #[test]
    fn test_file_not_found() {
        let err = TzFile::from_path("/nonexistent/timezone").unwrap_err();
        assert!(matches!(err, TzError::Io(_)));
    }
}
