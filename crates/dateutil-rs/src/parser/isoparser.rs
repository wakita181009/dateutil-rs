use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta};

// ---------------------------------------------------------------------------
// Public result type
// ---------------------------------------------------------------------------

/// Parsed ISO-8601 date/time components.
#[derive(Debug, Clone)]
pub struct IsoDateTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub microsecond: u32,
    /// `None` = naive (no timezone info).
    /// `Some(seconds)` = fixed UTC offset.
    pub tz_offset_seconds: Option<i32>,
}

impl IsoDateTime {
    /// Convert to a `NaiveDateTime`, ignoring any timezone information.
    pub fn to_naive_datetime(&self) -> Option<NaiveDateTime> {
        let d = NaiveDate::from_ymd_opt(self.year, self.month, self.day)?;
        let t =
            NaiveTime::from_hms_micro_opt(self.hour, self.minute, self.second, self.microsecond)?;
        Some(NaiveDateTime::new(d, t))
    }
}

// ---------------------------------------------------------------------------
// IsoParser
// ---------------------------------------------------------------------------

/// ISO-8601 date/time parser — Rust port of `dateutil.parser.isoparser`.
#[derive(Default)]
pub struct IsoParser {
    /// If `None`, any single non-digit ASCII byte is accepted as the
    /// date-time separator.  If `Some(c)`, only that byte is accepted.
    sep: Option<u8>,
}

impl IsoParser {
    pub fn new(sep: Option<u8>) -> Result<Self, String> {
        if let Some(s) = sep {
            if !s.is_ascii() || s.is_ascii_digit() {
                return Err(
                    "Separator must be a single, non-numeric ASCII character".into(),
                );
            }
        }
        Ok(Self { sep })
    }

    /// Parse a full ISO-8601 datetime string.
    pub fn isoparse(&self, dt_str: &str) -> Result<IsoDateTime, String> {
        if !dt_str.is_ascii() {
            return Err("ISO-8601 strings should contain only ASCII characters".into());
        }
        self.isoparse_bytes(dt_str.as_bytes())
    }

    /// Parse only the date portion.
    pub fn parse_isodate_str(&self, s: &str) -> Result<(i32, u32, u32), String> {
        if !s.is_ascii() {
            return Err("ISO-8601 strings should contain only ASCII characters".into());
        }
        let b = s.as_bytes();
        let (c, pos) = self.parse_isodate(b)?;
        if pos < b.len() {
            return Err(format!(
                "String contains unknown ISO components: {:?}",
                &s[pos..]
            ));
        }
        Ok((c[0], c[1] as u32, c[2] as u32))
    }

    /// Parse only the time portion (no date, no separator).
    pub fn parse_isotime_str(&self, s: &str) -> Result<IsoDateTime, String> {
        if !s.is_ascii() {
            return Err("ISO-8601 strings should contain only ASCII characters".into());
        }
        let (h, m, sec, us, tz) = self.parse_isotime(s.as_bytes())?;
        let hour = if h == 24 { 0 } else { h };
        Ok(IsoDateTime {
            year: 1,
            month: 1,
            day: 1,
            hour,
            minute: m,
            second: sec,
            microsecond: us,
            tz_offset_seconds: tz,
        })
    }

    /// Parse a timezone string like `Z`, `+05:30`, `-03`.
    pub fn parse_tzstr_str(&self, s: &str, zero_as_utc: bool) -> Result<i32, String> {
        if !s.is_ascii() {
            return Err("ISO-8601 strings should contain only ASCII characters".into());
        }
        Self::parse_tzstr(s.as_bytes(), zero_as_utc)
    }

    // -----------------------------------------------------------------------
    // Internal — byte-level parsing
    // -----------------------------------------------------------------------

    fn isoparse_bytes(&self, dt_str: &[u8]) -> Result<IsoDateTime, String> {
        let (dc, pos) = self.parse_isodate(dt_str)?;

        let mut res = IsoDateTime {
            year: dc[0],
            month: dc[1] as u32,
            day: dc[2] as u32,
            hour: 0,
            minute: 0,
            second: 0,
            microsecond: 0,
            tz_offset_seconds: None,
        };

        if dt_str.len() > pos {
            let sep_ok = match self.sep {
                None => true,
                Some(s) => dt_str[pos] == s,
            };
            if sep_ok {
                let (h, m, s, us, tz) = self.parse_isotime(&dt_str[pos + 1..])?;
                res.hour = h;
                res.minute = m;
                res.second = s;
                res.microsecond = us;
                res.tz_offset_seconds = tz;
            } else {
                return Err("String contains unknown ISO components".into());
            }
        }

        // 24:00 → next day 00:00
        if res.hour == 24 {
            res.hour = 0;
            if let Some(d) = NaiveDate::from_ymd_opt(res.year, res.month, res.day) {
                let next = d + TimeDelta::days(1);
                res.year = next.year();
                res.month = next.month();
                res.day = next.day();
            }
        }

        Ok(res)
    }

    // ---- date ----

    fn parse_isodate(&self, dt_str: &[u8]) -> Result<([i32; 3], usize), String> {
        self.parse_isodate_common(dt_str)
            .or_else(|_| self.parse_isodate_uncommon(dt_str))
    }

    fn parse_isodate_common(&self, dt_str: &[u8]) -> Result<([i32; 3], usize), String> {
        let len = dt_str.len();
        let mut c = [1i32; 3]; // year, month, day — default 1

        if len < 4 {
            return Err("ISO string too short".into());
        }
        c[0] = parse_int(&dt_str[..4])?;
        let mut pos = 4;

        if pos >= len {
            return Ok((c, pos));
        }

        let has_sep = dt_str[pos] == b'-';
        if has_sep {
            pos += 1;
        }

        // Month
        if len - pos < 2 {
            return Err("Invalid common month".into());
        }
        c[1] = parse_int(&dt_str[pos..pos + 2])?;
        pos += 2;

        if pos >= len {
            return if has_sep {
                Ok((c, pos))
            } else {
                Err("Invalid ISO format".into())
            };
        }

        if has_sep {
            if dt_str[pos] != b'-' {
                return Err("Invalid separator in ISO string".into());
            }
            pos += 1;
        }

        // Day
        if len - pos < 2 {
            return Err("Invalid common day".into());
        }
        c[2] = parse_int(&dt_str[pos..pos + 2])?;
        Ok((c, pos + 2))
    }

    fn parse_isodate_uncommon(&self, dt_str: &[u8]) -> Result<([i32; 3], usize), String> {
        if dt_str.len() < 4 {
            return Err("ISO string too short".into());
        }

        let year = parse_int(&dt_str[..4])?;
        let has_sep = dt_str.get(4) == Some(&b'-');
        let mut pos = 4 + usize::from(has_sep);

        if dt_str.get(pos) == Some(&b'W') {
            // ISO week: YYYY-?Www-?D?
            pos += 1;
            if dt_str.len() - pos < 2 {
                return Err("Invalid week".into());
            }
            let weekno = parse_int(&dt_str[pos..pos + 2])? as u32;
            pos += 2;

            let mut dayno: u32 = 1;
            if dt_str.len() > pos {
                let sep_here = dt_str.get(pos) == Some(&b'-');
                if sep_here != has_sep {
                    return Err("Inconsistent use of dash separator".into());
                }
                if sep_here {
                    pos += 1;
                }
                if pos < dt_str.len() && dt_str[pos].is_ascii_digit() {
                    dayno = u32::from(dt_str[pos] - b'0');
                    pos += 1;
                }
            }

            let d = calculate_weekdate(year, weekno, dayno)?;
            Ok(([d.year(), d.month() as i32, d.day() as i32], pos))
        } else {
            // Ordinal: YYYY-?DDD
            if dt_str.len() - pos < 3 {
                return Err("Invalid ordinal day".into());
            }
            let ordinal = parse_int(&dt_str[pos..pos + 3])? as u32;
            pos += 3;

            let is_leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            let max = if is_leap { 366 } else { 365 };
            if ordinal < 1 || ordinal > max {
                return Err(format!(
                    "Invalid ordinal day {} for year {}",
                    ordinal, year
                ));
            }

            let base = NaiveDate::from_ymd_opt(year, 1, 1)
                .ok_or_else(|| format!("Invalid year: {}", year))?;
            let d = base + TimeDelta::days(i64::from(ordinal) - 1);
            Ok(([d.year(), d.month() as i32, d.day() as i32], pos))
        }
    }

    // ---- time ----

    fn parse_isotime(
        &self,
        timestr: &[u8],
    ) -> Result<(u32, u32, u32, u32, Option<i32>), String> {
        let len = timestr.len();
        if len < 2 {
            return Err("ISO time too short".into());
        }

        let mut vals = [0u32; 4]; // hour, minute, second, microsecond
        let mut tz: Option<i32> = None;
        let mut pos: usize = 0;
        let mut comp: i32 = -1;
        let mut has_sep = false;

        while pos < len && comp < 5 {
            comp += 1;

            // Timezone boundary
            let ch = timestr[pos];
            if matches!(ch, b'-' | b'+' | b'Z' | b'z') {
                tz = Some(Self::parse_tzstr(&timestr[pos..], true)?);
                pos = len;
                break;
            }

            // Colon separator
            if comp == 1 && timestr[pos] == b':' {
                has_sep = true;
                pos += 1;
            } else if comp == 2 && has_sep {
                if pos >= len || timestr[pos] != b':' {
                    return Err("Inconsistent use of colon separator".into());
                }
                pos += 1;
            }

            if comp < 3 {
                if pos + 2 > len {
                    break;
                }
                vals[comp as usize] = parse_int(&timestr[pos..pos + 2])? as u32;
                pos += 2;
            }

            if comp == 3 {
                // Fractional seconds — [.,][0-9]+
                if pos < len && (timestr[pos] == b'.' || timestr[pos] == b',') {
                    pos += 1;
                    let start = pos;
                    while pos < len && timestr[pos].is_ascii_digit() {
                        pos += 1;
                    }
                    if pos > start {
                        let n = (pos - start).min(6);
                        let us = parse_int(&timestr[start..start + n])? as u32;
                        vals[3] = us * 10u32.pow(6 - n as u32);
                    }
                }
            }
        }

        if pos < len {
            return Err("Unused components in ISO string".into());
        }

        // Validate 24:xx:xx
        if vals[0] == 24 && (vals[1] != 0 || vals[2] != 0 || vals[3] != 0) {
            return Err("Hour may only be 24 at 24:00:00.000".into());
        }

        Ok((vals[0], vals[1], vals[2], vals[3], tz))
    }

    // ---- timezone ----

    fn parse_tzstr(tzstr: &[u8], zero_as_utc: bool) -> Result<i32, String> {
        if tzstr == b"Z" || tzstr == b"z" {
            return Ok(0);
        }

        let n = tzstr.len();
        if !matches!(n, 3 | 5 | 6) {
            return Err("Time zone offset must be 1, 3, 5 or 6 characters".into());
        }

        let mult: i32 = match tzstr[0] {
            b'-' => -1,
            b'+' => 1,
            _ => return Err("Time zone offset requires sign".into()),
        };

        let hours = parse_int(&tzstr[1..3])? as i32;
        let minutes = if n == 3 {
            0
        } else {
            let start = if n == 6 && tzstr[3] == b':' { 4 } else { 3 };
            parse_int(&tzstr[start..])? as i32
        };

        if hours > 23 {
            return Err("Invalid hours in time zone offset".into());
        }
        if minutes > 59 {
            return Err("Invalid minutes in time zone offset".into());
        }

        let offset = mult * (hours * 3600 + minutes * 60);
        if zero_as_utc && offset == 0 {
            Ok(0)
        } else {
            Ok(offset)
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_int(bytes: &[u8]) -> Result<i32, String> {
    let s = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    s.parse::<i32>()
        .map_err(|e| format!("Invalid integer '{}': {}", s, e))
}

fn calculate_weekdate(year: i32, week: u32, day: u32) -> Result<NaiveDate, String> {
    if !(1..54).contains(&week) {
        return Err(format!("Invalid week: {}", week));
    }
    if !(1..8).contains(&day) {
        return Err(format!("Invalid weekday: {}", day));
    }
    // Jan 4 is always in ISO week 1
    let jan4 =
        NaiveDate::from_ymd_opt(year, 1, 4).ok_or_else(|| format!("Invalid year: {}", year))?;
    let iso_wd = jan4.weekday().num_days_from_monday(); // 0=Mon
    let week1 = jan4 - TimeDelta::days(i64::from(iso_wd));
    let off = i64::from((week - 1) * 7 + (day - 1));
    Ok(week1 + TimeDelta::days(off))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> IsoDateTime {
        IsoParser::default().isoparse(s).unwrap()
    }

    #[test]
    fn basic_date() {
        let r = p("2003-09-25");
        assert_eq!((r.year, r.month, r.day), (2003, 9, 25));
        assert_eq!((r.hour, r.minute, r.second, r.microsecond), (0, 0, 0, 0));
        assert_eq!(r.tz_offset_seconds, None);
    }

    #[test]
    fn compact_date() {
        let r = p("20030925");
        assert_eq!((r.year, r.month, r.day), (2003, 9, 25));
    }

    #[test]
    fn year_only() {
        let r = p("2003");
        assert_eq!((r.year, r.month, r.day), (2003, 1, 1));
    }

    #[test]
    fn year_month() {
        let r = p("2003-09");
        assert_eq!((r.year, r.month, r.day), (2003, 9, 1));
    }

    #[test]
    fn yyyymm_without_sep_invalid() {
        assert!(IsoParser::default().isoparse("200309").is_err());
    }

    #[test]
    fn datetime_t_sep() {
        let r = p("2003-09-25T10:49:41");
        assert_eq!((r.hour, r.minute, r.second), (10, 49, 41));
    }

    #[test]
    fn datetime_compact() {
        let r = p("20030925T104941");
        assert_eq!((r.year, r.month, r.day), (2003, 9, 25));
        assert_eq!((r.hour, r.minute, r.second), (10, 49, 41));
    }

    #[test]
    fn hh_only() {
        let r = p("2003-09-25T10");
        assert_eq!((r.hour, r.minute, r.second), (10, 0, 0));
    }

    #[test]
    fn hhmm() {
        let r = p("2003-09-25T10:49");
        assert_eq!((r.hour, r.minute), (10, 49));
    }

    #[test]
    fn fractional_dot() {
        let r = p("2003-09-25T10:49:41.5");
        assert_eq!((r.second, r.microsecond), (41, 500_000));
    }

    #[test]
    fn fractional_comma() {
        let r = p("2003-09-25T10:49:41,123456");
        assert_eq!((r.second, r.microsecond), (41, 123_456));
    }

    #[test]
    fn fractional_truncated() {
        let r = p("2003-09-25T10:49:41.1234567");
        assert_eq!(r.microsecond, 123_456);
    }

    #[test]
    fn utc_z() {
        let r = p("2003-09-25T10:49:41Z");
        assert_eq!(r.tz_offset_seconds, Some(0));
    }

    #[test]
    fn utc_z_lower() {
        let r = p("2003-09-25T10:49:41z");
        assert_eq!(r.tz_offset_seconds, Some(0));
    }

    #[test]
    fn positive_offset_colon() {
        let r = p("2003-09-25T10:49:41+05:30");
        assert_eq!(r.tz_offset_seconds, Some(5 * 3600 + 30 * 60));
    }

    #[test]
    fn negative_offset_compact() {
        let r = p("2003-09-25T10:49:41-0300");
        assert_eq!(r.tz_offset_seconds, Some(-3 * 3600));
    }

    #[test]
    fn offset_hours_only() {
        let r = p("2003-09-25T10:49:41+05");
        assert_eq!(r.tz_offset_seconds, Some(5 * 3600));
    }

    #[test]
    fn zero_offset_is_utc() {
        let r = p("2003-09-25T10:49:41+00:00");
        assert_eq!(r.tz_offset_seconds, Some(0));
    }

    #[test]
    fn hour_24_midnight() {
        let r = p("2003-09-25T24:00:00");
        assert_eq!((r.year, r.month, r.day, r.hour), (2003, 9, 26, 0));
    }

    #[test]
    fn week_date_no_day() {
        // 2003-W01 → week 1 Mon = 2002-12-30
        let r = p("2003W01");
        assert_eq!((r.year, r.month, r.day), (2002, 12, 30));
    }

    #[test]
    fn week_date_with_day() {
        let r = p("2003-W01-1");
        assert_eq!((r.year, r.month, r.day), (2002, 12, 30));
    }

    #[test]
    fn ordinal_date() {
        // Day 100 of 2003 = April 10
        let r = p("2003-100");
        assert_eq!((r.year, r.month, r.day), (2003, 4, 10));
    }

    #[test]
    fn ordinal_compact() {
        let r = p("2003100");
        assert_eq!((r.year, r.month, r.day), (2003, 4, 10));
    }

    #[test]
    fn custom_sep() {
        let parser = IsoParser::new(Some(b' ')).unwrap();
        let r = parser.isoparse("2003-09-25 10:49:41").unwrap();
        assert_eq!((r.hour, r.minute, r.second), (10, 49, 41));
    }

    #[test]
    fn custom_sep_rejects_t() {
        let parser = IsoParser::new(Some(b' ')).unwrap();
        assert!(parser.isoparse("2003-09-25T10:49:41").is_err());
    }

    #[test]
    fn non_ascii_rejected() {
        assert!(IsoParser::default().isoparse("２００３").is_err());
    }

    #[test]
    fn numeric_sep_rejected() {
        assert!(IsoParser::new(Some(b'1')).is_err());
    }
}
