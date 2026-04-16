use crate::error::ParseError;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Timezone info from ISO-8601 parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsoTz {
    /// UTC timezone (from 'Z' or zero offset with `zero_as_utc`).
    Utc,
    /// Fixed offset in total seconds from UTC.
    Offset(i32),
}

/// Result of a full ISO-8601 datetime parse.
#[derive(Debug)]
pub struct IsoParsed {
    pub datetime: NaiveDateTime,
    pub tz: Option<IsoTz>,
}

/// Result of an ISO-8601 time parse.
#[derive(Debug)]
pub struct IsoTimeParsed {
    pub time: NaiveTime,
    pub tz: Option<IsoTz>,
}

// ---------------------------------------------------------------------------
// IsoParser
// ---------------------------------------------------------------------------

/// ISO-8601 parser with configurable date/time separator.
pub struct IsoParser {
    sep: Option<u8>,
}

impl Default for IsoParser {
    fn default() -> Self {
        Self { sep: None }
    }
}

impl IsoParser {
    /// Create a new parser.  `sep` must be a non-digit ASCII byte.
    pub fn new(sep: Option<u8>) -> Result<Self, ParseError> {
        if let Some(s) = sep {
            if !s.is_ascii() || s.is_ascii_digit() {
                return Err(verr(
                    "Separator must be a single, non-numeric ASCII character",
                ));
            }
        }
        Ok(Self { sep })
    }

    // -----------------------------------------------------------------------
    // Public methods
    // -----------------------------------------------------------------------

    /// Parse a full ISO-8601 datetime string.
    pub fn isoparse(&self, s: &str) -> Result<IsoParsed, ParseError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseError::NoDate("".into()));
        }
        let bytes = s.as_bytes();

        let (date, pos) = self.internal_parse_isodate(bytes)?;

        if pos >= bytes.len() {
            return Ok(IsoParsed {
                datetime: date.and_hms_opt(0, 0, 0).unwrap(),
                tz: None,
            });
        }

        // Validate separator
        let sep_byte = bytes[pos];
        if let Some(expected) = self.sep {
            if sep_byte != expected {
                return Err(verr("String contains unknown ISO components"));
            }
        }

        let time_str = &bytes[pos + 1..];
        let (time, tz, hour24) = self.internal_parse_isotime(time_str)?;

        let datetime = if hour24 {
            date.and_hms_opt(0, 0, 0).unwrap() + chrono::Duration::days(1)
        } else {
            NaiveDateTime::new(date, time)
        };

        Ok(IsoParsed { datetime, tz })
    }

    /// Parse just the date portion of an ISO string.
    pub fn parse_isodate(&self, s: &str) -> Result<NaiveDate, ParseError> {
        let bytes = s.as_bytes();
        let (date, pos) = self.internal_parse_isodate(bytes)?;
        if pos < bytes.len() {
            return Err(verr(&format!(
                "String contains unknown ISO components: '{s}'"
            )));
        }
        Ok(date)
    }

    /// Parse just the time portion of an ISO string.
    pub fn parse_isotime(&self, s: &str) -> Result<IsoTimeParsed, ParseError> {
        let bytes = s.as_bytes();
        let (time, tz, hour24) = self.internal_parse_isotime(bytes)?;
        let time = if hour24 {
            NaiveTime::from_hms_opt(0, 0, 0).unwrap()
        } else {
            time
        };
        Ok(IsoTimeParsed { time, tz })
    }

    /// Parse a timezone offset string.
    pub fn parse_tzstr(&self, s: &str, zero_as_utc: bool) -> Result<IsoTz, ParseError> {
        self.internal_parse_tzstr(s.as_bytes(), zero_as_utc)
    }

    // -----------------------------------------------------------------------
    // Internal: date parsing
    // -----------------------------------------------------------------------

    fn internal_parse_isodate(&self, bytes: &[u8]) -> Result<(NaiveDate, usize), ParseError> {
        self.parse_isodate_common(bytes)
            .or_else(|_| self.parse_isodate_uncommon(bytes))
    }

    fn parse_isodate_common(&self, bytes: &[u8]) -> Result<(NaiveDate, usize), ParseError> {
        let len = bytes.len();
        if len < 4 {
            return Err(verr("ISO string too short"));
        }

        let year = parse_int_range(bytes, 0, 4)?;
        let mut pos = 4;

        if pos >= len {
            // YYYY only
            return Ok((make_date(year, 1, 1)?, pos));
        }

        let has_sep = bytes[pos] == b'-';
        if has_sep {
            pos += 1;
        }

        if pos >= len {
            // YYYY- (trailing dash)
            return Err(verr("Incomplete date"));
        }

        // Non-digit after year → delegate non-common formats (W, ordinal)
        // or return YYYY-only when no separator (e.g. "2014T12:00")
        if !bytes[pos].is_ascii_digit() {
            if has_sep || bytes[pos] == b'W' {
                // YYYY-W... or YYYYW... → delegate to uncommon (week/ordinal)
                return Err(verr("not a common date format"));
            }
            return Ok((make_date(year, 1, 1)?, 4));
        }

        // Need at least 2 digits for month
        if pos + 2 > len || !bytes[pos + 1].is_ascii_digit() {
            return Err(verr("Incomplete month"));
        }

        let month = parse_int_range(bytes, pos, pos + 2)? as u32;
        pos += 2;

        if !has_sep {
            // Compact: must have 2 more digits for day (YYYYMMDD)
            // YYYYMM (6 chars without day) is not a valid ISO format
            if pos + 2 > len || !bytes[pos].is_ascii_digit() {
                return Err(verr("Invalid format"));
            }
            let day = parse_int_range(bytes, pos, pos + 2)? as u32;
            pos += 2;
            return Ok((make_date(year, month, day)?, pos));
        }

        // Extended: YYYY-MM, or YYYY-MM-DD if another '-' follows
        if pos >= len || bytes[pos] != b'-' {
            // If a digit follows, this isn't YYYY-MM — likely ordinal (YYYY-DDD)
            if pos < len && bytes[pos].is_ascii_digit() {
                return Err(verr("not a common date format"));
            }
            return Ok((make_date(year, month, 1)?, pos));
        }

        pos += 1; // skip second '-'
        if pos + 2 > len {
            return Err(verr("Incomplete date"));
        }
        let day = parse_int_range(bytes, pos, pos + 2)? as u32;
        pos += 2;

        Ok((make_date(year, month, day)?, pos))
    }

    fn parse_isodate_uncommon(&self, bytes: &[u8]) -> Result<(NaiveDate, usize), ParseError> {
        if bytes.len() < 4 {
            return Err(verr("ISO string too short"));
        }

        let year = parse_int_range(bytes, 0, 4)?;
        let has_sep = bytes.get(4) == Some(&b'-');
        let mut pos = 4 + has_sep as usize;

        if pos >= bytes.len() {
            return Err(verr("Incomplete date"));
        }

        if bytes[pos] == b'W' {
            // Week date: YYYY-Www[-D] or YYYYWww[D]
            pos += 1;
            if pos + 2 > bytes.len() {
                return Err(verr("Invalid week date"));
            }
            let weekno = parse_int_range(bytes, pos, pos + 2)?;
            pos += 2;

            let mut dayno = 1;
            if pos < bytes.len() && (bytes[pos].is_ascii_digit() || bytes[pos] == b'-') {
                let day_has_sep = bytes[pos] == b'-';
                if day_has_sep != has_sep {
                    return Err(verr("Inconsistent use of dash separator"));
                }
                if day_has_sep {
                    pos += 1;
                }
                if pos < bytes.len() && bytes[pos].is_ascii_digit() {
                    dayno = (bytes[pos] - b'0') as i32;
                    pos += 1;
                }
            }

            let date = calculate_weekdate(year, weekno, dayno)?;
            Ok((date, pos))
        } else {
            // Ordinal date: YYYY-DDD or YYYYDDD
            if pos + 3 > bytes.len() {
                return Err(verr("Invalid ordinal day"));
            }
            let ordinal = parse_int_range(bytes, pos, pos + 3)?;
            pos += 3;

            let is_leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
            let max_days = if is_leap { 366 } else { 365 };
            if ordinal < 1 || ordinal > max_days {
                return Err(verr(&format!(
                    "Invalid ordinal day {ordinal} for year {year}"
                )));
            }

            let base = make_date(year, 1, 1)?;
            let date = base + chrono::Duration::days((ordinal - 1) as i64);
            Ok((date, pos))
        }
    }

    // -----------------------------------------------------------------------
    // Internal: time parsing
    // -----------------------------------------------------------------------

    /// Returns (time, tz, is_hour_24).
    fn internal_parse_isotime(
        &self,
        bytes: &[u8],
    ) -> Result<(NaiveTime, Option<IsoTz>, bool), ParseError> {
        let len = bytes.len();
        if len < 2 {
            return Err(verr("ISO time too short"));
        }

        let hour = parse_int_range(bytes, 0, 2)?;
        let mut pos = 2;
        let mut minute = 0i32;
        let mut second = 0i32;
        let mut microsecond = 0u32;
        let mut tz: Option<IsoTz> = None;

        if pos < len && is_tz_char(bytes[pos]) {
            tz = Some(self.internal_parse_tzstr(&bytes[pos..], true)?);
            pos = len;
        } else if pos < len && (bytes[pos].is_ascii_digit() || bytes[pos] == b':') {
            let has_sep = bytes[pos] == b':';
            if has_sep {
                pos += 1;
            }
            if pos + 2 > len {
                return Err(verr("Incomplete time"));
            }
            minute = parse_int_range(bytes, pos, pos + 2)?;
            pos += 2;

            // Seconds
            if pos < len && !is_tz_char(bytes[pos]) {
                if bytes[pos] == b':' {
                    if !has_sep {
                        return Err(verr("Inconsistent separator use"));
                    }
                    pos += 1;
                } else if has_sep && bytes[pos].is_ascii_digit() {
                    return Err(verr("Inconsistent separator use"));
                }

                if pos + 2 <= len && bytes[pos].is_ascii_digit() {
                    second = parse_int_range(bytes, pos, pos + 2)?;
                    pos += 2;

                    // Fractional seconds (. or ,)
                    if pos < len && (bytes[pos] == b'.' || bytes[pos] == b',') {
                        pos += 1;
                        let frac_start = pos;
                        while pos < len && bytes[pos].is_ascii_digit() {
                            pos += 1;
                        }
                        let frac_len = pos - frac_start;
                        if frac_len > 0 {
                            microsecond = parse_microseconds(&bytes[frac_start..pos], frac_len)?;
                        }
                    }
                }
            }

            // Timezone after time components
            if pos < len && is_tz_char(bytes[pos]) {
                tz = Some(self.internal_parse_tzstr(&bytes[pos..], true)?);
                pos = len;
            }
        }

        if pos < len {
            return Err(verr("Unused components in ISO string"));
        }

        // Hour 24 handling
        let hour24 = hour == 24;
        let actual_hour = if hour24 {
            if minute != 0 || second != 0 || microsecond != 0 {
                return Err(verr("Hour may only be 24 at 24:00:00.000"));
            }
            0
        } else {
            if hour > 23 {
                return Err(verr("Invalid hours"));
            }
            hour
        };

        if minute > 59 {
            return Err(verr("Invalid minutes"));
        }
        if second > 59 {
            return Err(verr("Invalid seconds"));
        }

        let time = NaiveTime::from_hms_micro_opt(
            actual_hour as u32,
            minute as u32,
            second as u32,
            microsecond,
        )
        .ok_or_else(|| verr("invalid time"))?;

        Ok((time, tz, hour24))
    }

    // -----------------------------------------------------------------------
    // Internal: timezone parsing
    // -----------------------------------------------------------------------

    fn internal_parse_tzstr(&self, bytes: &[u8], zero_as_utc: bool) -> Result<IsoTz, ParseError> {
        if bytes.is_empty() {
            return Err(verr("Empty timezone string"));
        }

        // Exact Z/z match (single byte only)
        if bytes.len() == 1 && (bytes[0] == b'Z' || bytes[0] == b'z') {
            return Ok(IsoTz::Utc);
        }

        let len = bytes.len();
        if len != 3 && len != 5 && len != 6 {
            return Err(verr("Time zone offset must be 1, 3, 5 or 6 characters"));
        }

        let mult: i32 = match bytes[0] {
            b'+' => 1,
            b'-' => -1,
            _ => return Err(verr("Time zone offset requires sign")),
        };

        let hours = parse_int_range(bytes, 1, 3)?;
        let minutes = if len == 3 {
            0
        } else if bytes[3] == b':' {
            parse_int_range(bytes, 4, 6)?
        } else {
            parse_int_range(bytes, 3, 5)?
        };

        if hours > 23 {
            return Err(verr("Invalid hours in time zone offset"));
        }
        if minutes > 59 {
            return Err(verr("Invalid minutes in time zone offset"));
        }

        if zero_as_utc && hours == 0 && minutes == 0 {
            return Ok(IsoTz::Utc);
        }

        Ok(IsoTz::Offset(mult * (hours * 60 + minutes) * 60))
    }
}

// ---------------------------------------------------------------------------
// Convenience functions
// ---------------------------------------------------------------------------

/// Parse an ISO-8601 datetime string with the default parser.
pub fn isoparse(s: &str) -> Result<IsoParsed, ParseError> {
    IsoParser::default().isoparse(s)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[inline]
fn verr(msg: &str) -> ParseError {
    ParseError::ValueError(msg.to_string().into_boxed_str())
}

#[inline]
fn make_date(year: i32, month: u32, day: u32) -> Result<NaiveDate, ParseError> {
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| verr(&format!("invalid date: {year}-{month}-{day}")))
}

#[inline]
fn is_tz_char(b: u8) -> bool {
    matches!(b, b'+' | b'-' | b'Z' | b'z')
}

#[inline]
fn parse_int_range(bytes: &[u8], start: usize, end: usize) -> Result<i32, ParseError> {
    if end > bytes.len() {
        return Err(verr("unexpected end of string"));
    }
    let mut n: i32 = 0;
    for &b in &bytes[start..end] {
        if !b.is_ascii_digit() {
            return Err(verr("expected digit"));
        }
        n = n * 10 + (b - b'0') as i32;
    }
    Ok(n)
}

#[inline]
fn parse_microseconds(bytes: &[u8], frac_len: usize) -> Result<u32, ParseError> {
    if frac_len >= 6 {
        let mut n: u32 = 0;
        for &b in &bytes[..6] {
            if !b.is_ascii_digit() {
                return Err(verr("expected digit in fractional seconds"));
            }
            n = n * 10 + (b - b'0') as u32;
        }
        Ok(n)
    } else {
        let mut raw: u32 = 0;
        for &b in &bytes[..frac_len] {
            if !b.is_ascii_digit() {
                return Err(verr("expected digit in fractional seconds"));
            }
            raw = raw * 10 + (b - b'0') as u32;
        }
        const SCALE: [u32; 5] = [100_000, 10_000, 1_000, 100, 10];
        Ok(raw * SCALE[frac_len - 1])
    }
}

fn calculate_weekdate(year: i32, week: i32, day: i32) -> Result<NaiveDate, ParseError> {
    if week < 1 || week > 53 {
        return Err(verr(&format!("Invalid week: {week}")));
    }
    if day < 1 || day > 7 {
        return Err(verr(&format!("Invalid weekday: {day}")));
    }

    // Jan 4 is always in ISO week 1
    let jan_4 = NaiveDate::from_ymd_opt(year, 1, 4).ok_or_else(|| verr("invalid year"))?;
    let iso_wd = jan_4.weekday().num_days_from_monday() as i64;
    let week_1_monday = jan_4 - chrono::Duration::days(iso_wd);

    let offset = ((week - 1) * 7 + (day - 1)) as i64;
    Ok(week_1_monday + chrono::Duration::days(offset))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn iso_date_only() {
        let r = isoparse("2024-01-15").unwrap();
        assert_eq!(
            r.datetime.date(),
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()
        );
        assert!(r.tz.is_none());
    }

    #[test]
    fn iso_datetime() {
        let r = isoparse("2024-01-15T10:30:45").unwrap();
        assert_eq!(r.datetime.hour(), 10);
        assert_eq!(r.datetime.minute(), 30);
        assert_eq!(r.datetime.second(), 45);
    }

    #[test]
    fn iso_compact() {
        let r = isoparse("20240115T103045").unwrap();
        assert_eq!(
            r.datetime.date(),
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()
        );
        assert_eq!(r.datetime.hour(), 10);
    }

    #[test]
    fn iso_with_tz_z() {
        let r = isoparse("2024-01-15T10:30:45Z").unwrap();
        assert_eq!(r.tz, Some(IsoTz::Utc));
    }

    #[test]
    fn iso_with_tz_offset() {
        let r = isoparse("2024-01-15T10:30:45+05:30").unwrap();
        assert_eq!(r.tz, Some(IsoTz::Offset(19800)));
    }

    #[test]
    fn iso_week_date() {
        let r = isoparse("2017-W10").unwrap();
        assert_eq!(
            r.datetime.date(),
            NaiveDate::from_ymd_opt(2017, 3, 6).unwrap()
        );
    }

    #[test]
    fn iso_week_date_with_day() {
        let r = isoparse("2016-W13-7").unwrap();
        assert_eq!(
            r.datetime.date(),
            NaiveDate::from_ymd_opt(2016, 4, 3).unwrap()
        );
    }

    #[test]
    fn iso_ordinal() {
        let r = isoparse("2016-060").unwrap();
        assert_eq!(
            r.datetime.date(),
            NaiveDate::from_ymd_opt(2016, 2, 29).unwrap()
        );
    }

    #[test]
    fn iso_hour24_midnight() {
        let r = isoparse("2014-04-10T24:00:00").unwrap();
        assert_eq!(
            r.datetime,
            NaiveDate::from_ymd_opt(2014, 4, 11)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        );
    }

    #[test]
    fn iso_comma_decimal() {
        let r = isoparse("2024-01-15T10:30:45,123456").unwrap();
        assert_eq!(r.datetime.nanosecond() / 1000, 123456);
    }

    #[test]
    fn iso_empty() {
        assert!(isoparse("").is_err());
    }

    #[test]
    fn iso_trailing_t() {
        assert!(isoparse("2024-01-15T").is_err());
    }

    #[test]
    fn iso_yyyymm_invalid() {
        assert!(IsoParser::default().parse_isodate("201202").is_err());
    }

    #[test]
    fn iso_sep_validation() {
        assert!(IsoParser::new(Some(b'9')).is_err());
        assert!(IsoParser::new(Some(b'T')).is_ok());
    }

    #[test]
    fn parse_tzstr_utc() {
        let p = IsoParser::default();
        assert_eq!(p.parse_tzstr("Z", true).unwrap(), IsoTz::Utc);
        assert_eq!(p.parse_tzstr("+00:00", true).unwrap(), IsoTz::Utc);
        assert_eq!(p.parse_tzstr("+00:00", false).unwrap(), IsoTz::Offset(0));
    }

    #[test]
    fn parse_tzstr_offset() {
        let p = IsoParser::default();
        assert_eq!(p.parse_tzstr("+05:30", true).unwrap(), IsoTz::Offset(19800));
        assert_eq!(
            p.parse_tzstr("-08:00", true).unwrap(),
            IsoTz::Offset(-28800)
        );
    }

    #[test]
    fn parse_isodate_common() {
        let p = IsoParser::default();
        assert_eq!(
            p.parse_isodate("2024-01-15").unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()
        );
        assert_eq!(
            p.parse_isodate("20240115").unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()
        );
        assert_eq!(
            p.parse_isodate("2024-01").unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );
        assert_eq!(
            p.parse_isodate("2024").unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );
    }

    #[test]
    fn parse_isotime_basic() {
        let p = IsoParser::default();
        let r = p.parse_isotime("10:30:45.123456").unwrap();
        assert_eq!(
            r.time,
            NaiveTime::from_hms_micro_opt(10, 30, 45, 123456).unwrap()
        );
        assert!(r.tz.is_none());
    }

    #[test]
    fn parse_isotime_with_tz() {
        let p = IsoParser::default();
        let r = p.parse_isotime("10:30:45Z").unwrap();
        assert_eq!(r.tz, Some(IsoTz::Utc));
    }

    #[test]
    fn parse_isotime_midnight_24() {
        let p = IsoParser::default();
        let r = p.parse_isotime("24:00:00").unwrap();
        assert_eq!(r.time, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }

    #[test]
    fn week_date_compact() {
        let r = isoparse("2017W10").unwrap();
        assert_eq!(
            r.datetime.date(),
            NaiveDate::from_ymd_opt(2017, 3, 6).unwrap()
        );
    }

    #[test]
    fn ordinal_compact() {
        let r = isoparse("2016060").unwrap();
        assert_eq!(
            r.datetime.date(),
            NaiveDate::from_ymd_opt(2016, 2, 29).unwrap()
        );
    }

    #[test]
    fn invalid_week_zero() {
        assert!(isoparse("2012-W00").is_err());
    }

    #[test]
    fn invalid_weekday_zero() {
        assert!(isoparse("2012-W01-0").is_err());
    }

    #[test]
    fn invalid_ordinal_zero() {
        assert!(isoparse("2013-000").is_err());
    }

    #[test]
    fn invalid_ordinal_overflow() {
        assert!(isoparse("2013-366").is_err());
    }

    #[test]
    fn microseconds_frac_3() {
        let r = isoparse("2024-01-15T10:30:45.123").unwrap();
        assert_eq!(r.datetime.nanosecond() / 1000, 123000);
    }

    #[test]
    fn microseconds_frac_6() {
        let r = isoparse("2024-01-15T10:30:45.123456").unwrap();
        assert_eq!(r.datetime.nanosecond() / 1000, 123456);
    }

    #[test]
    fn custom_sep() {
        let p = IsoParser::new(Some(b'C')).unwrap();
        let r = p.isoparse("2012-04-25C01:25:00").unwrap();
        assert_eq!(r.datetime.hour(), 1);
        assert_eq!(r.datetime.minute(), 25);
    }

    #[test]
    fn custom_sep_mismatch() {
        let p = IsoParser::new(Some(b'C')).unwrap();
        assert!(p.isoparse("2012-04-25T01:25:00").is_err());
    }
}
