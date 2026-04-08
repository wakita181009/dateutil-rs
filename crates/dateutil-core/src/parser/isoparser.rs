use crate::error::ParseError;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

/// Parse an ISO-8601 date/time string.
///
/// Supports: YYYY-MM-DD, YYYY-MM-DDTHH:MM:SS, YYYY-MM-DDTHH:MM:SS.ffffff
/// Also supports compact forms: YYYYMMDD, YYYYMMDDTHHMMSS
pub fn isoparse(s: &str) -> Result<NaiveDateTime, ParseError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseError::NoDate(String::new()));
    }

    // Split on T or space separator
    let (date_part, time_part) = if let Some(t_pos) = s.bytes().position(|b| b == b'T' || b == b' ') {
        (&s[..t_pos], Some(&s[t_pos + 1..]))
    } else {
        (s, None)
    };

    let date = parse_iso_date(date_part)?;

    let time = if let Some(tp) = time_part {
        // Strip timezone suffix for time parsing
        let tp = strip_tz_suffix(tp);
        parse_iso_time(tp)?
    } else {
        NaiveTime::from_hms_opt(0, 0, 0).unwrap()
    };

    Ok(NaiveDateTime::new(date, time))
}

#[inline]
fn parse_iso_date(s: &str) -> Result<NaiveDate, ParseError> {
    let bytes = s.as_bytes();

    match bytes.len() {
        // YYYY-MM-DD
        10 if bytes[4] == b'-' && bytes[7] == b'-' => {
            let year = parse_int(&s[0..4])?;
            let month = parse_int(&s[5..7])? as u32;
            let day = parse_int(&s[8..10])? as u32;
            NaiveDate::from_ymd_opt(year, month, day)
                .ok_or_else(|| ParseError::ValueError(format!("invalid date: {s}")))
        }
        // YYYYMMDD
        8 if bytes.iter().all(|b| b.is_ascii_digit()) => {
            let year = parse_int(&s[0..4])?;
            let month = parse_int(&s[4..6])? as u32;
            let day = parse_int(&s[6..8])? as u32;
            NaiveDate::from_ymd_opt(year, month, day)
                .ok_or_else(|| ParseError::ValueError(format!("invalid date: {s}")))
        }
        // YYYY-MM
        7 if bytes[4] == b'-' => {
            let year = parse_int(&s[0..4])?;
            let month = parse_int(&s[5..7])? as u32;
            NaiveDate::from_ymd_opt(year, month, 1)
                .ok_or_else(|| ParseError::ValueError(format!("invalid date: {s}")))
        }
        // YYYY
        4 if bytes.iter().all(|b| b.is_ascii_digit()) => {
            let year = parse_int(&s[0..4])?;
            NaiveDate::from_ymd_opt(year, 1, 1)
                .ok_or_else(|| ParseError::ValueError(format!("invalid date: {s}")))
        }
        _ => Err(ParseError::ValueError(format!(
            "unrecognized ISO date format: {s}"
        ))),
    }
}

#[inline]
fn parse_iso_time(s: &str) -> Result<NaiveTime, ParseError> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return Ok(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }

    // HH:MM:SS.ffffff or HH:MM:SS or HH:MM or HHMMSS or HHMM
    let (h, m, s_sec, us) = if bytes.len() >= 8 && bytes[2] == b':' && bytes[5] == b':' {
        // HH:MM:SS[.ffffff]
        let h = parse_int(&s[0..2])?;
        let m = parse_int(&s[3..5])? as u32;
        let sec_str = &s[6..];
        let (sec, us) = parse_seconds_with_frac(sec_str)?;
        (h, m, sec, us)
    } else if bytes.len() >= 5 && bytes[2] == b':' {
        // HH:MM
        let h = parse_int(&s[0..2])?;
        let m = parse_int(&s[3..5])? as u32;
        (h, m, 0u32, 0u32)
    } else if bytes.len() >= 6 && bytes.iter().take(6).all(|b| b.is_ascii_digit()) {
        // HHMMSS[.ffffff]
        let h = parse_int(&s[0..2])?;
        let m = parse_int(&s[2..4])? as u32;
        let sec_str = &s[4..];
        let (sec, us) = parse_seconds_with_frac(sec_str)?;
        (h, m, sec, us)
    } else if bytes.len() >= 4 && bytes.iter().take(4).all(|b| b.is_ascii_digit()) {
        // HHMM
        let h = parse_int(&s[0..2])?;
        let m = parse_int(&s[2..4])? as u32;
        (h, m, 0u32, 0u32)
    } else if bytes.len() >= 2 && bytes[0].is_ascii_digit() && bytes[1].is_ascii_digit() {
        // HH
        let h = parse_int(&s[0..2])?;
        (h, 0u32, 0u32, 0u32)
    } else {
        return Err(ParseError::ValueError(format!(
            "unrecognized ISO time format: {s}"
        )));
    };

    NaiveTime::from_hms_micro_opt(h as u32, m, s_sec, us)
        .ok_or_else(|| ParseError::ValueError(format!("invalid time: {s}")))
}

#[inline]
fn parse_seconds_with_frac(s: &str) -> Result<(u32, u32), ParseError> {
    if let Some(dot_pos) = s.find('.') {
        let sec = parse_int(&s[..dot_pos])? as u32;
        let frac_str = &s[dot_pos + 1..];
        // Convert fractional digits to microseconds via arithmetic (no String allocation)
        let us = if frac_str.is_empty() {
            0u32
        } else if frac_str.len() >= 6 {
            parse_int(&frac_str[..6])? as u32
        } else {
            let raw = parse_int(frac_str)? as u32;
            // Multiply by 10^(6-len) to left-align: "123" → 123_000
            const SCALE: [u32; 5] = [100_000, 10_000, 1_000, 100, 10];
            raw * SCALE[frac_str.len() - 1]
        };
        Ok((sec, us))
    } else {
        // Just digits, take first 2 as seconds
        let sec_end = s.len().min(2);
        let sec = parse_int(&s[..sec_end])? as u32;
        Ok((sec, 0))
    }
}

#[inline]
fn strip_tz_suffix(s: &str) -> &str {
    // Remove trailing Z, +HH:MM, -HH:MM, +HHMM, -HHMM
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return s;
    }
    if bytes[bytes.len() - 1] == b'Z' {
        return &s[..s.len() - 1];
    }
    // Find + or - for timezone offset
    for i in (1..bytes.len()).rev() {
        if (bytes[i] == b'+' || bytes[i] == b'-') && bytes[i - 1].is_ascii_digit() {
            return &s[..i];
        }
    }
    s
}

/// Fast integer parse optimized for short ASCII digit strings (ISO date/time fields).
#[inline]
fn parse_int(s: &str) -> Result<i32, ParseError> {
    let bytes = s.as_bytes();
    if !bytes.is_empty() && bytes.len() <= 6 {
        let mut n: i32 = 0;
        for &b in bytes {
            if !b.is_ascii_digit() {
                return Err(ParseError::ValueError(format!("expected integer: {s}")));
            }
            n = n * 10 + (b - b'0') as i32;
        }
        return Ok(n);
    }
    s.parse::<i32>()
        .map_err(|_| ParseError::ValueError(format!("expected integer: {s}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_iso_date_only() {
        let dt = isoparse("2024-01-15").unwrap();
        assert_eq!(dt.date(), NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        assert_eq!(dt.hour(), 0);
    }

    #[test]
    fn test_iso_datetime() {
        let dt = isoparse("2024-01-15T10:30:45").unwrap();
        assert_eq!(dt.date(), NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }

    #[test]
    fn test_iso_compact() {
        let dt = isoparse("20240115T103045").unwrap();
        assert_eq!(dt.date(), NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }

    #[test]
    fn test_iso_microseconds() {
        let dt = isoparse("2024-01-15T10:30:45.123456").unwrap();
        assert_eq!(dt.nanosecond() / 1000, 123456);
    }

    #[test]
    fn test_iso_with_tz_z() {
        let dt = isoparse("2024-01-15T10:30:45Z").unwrap();
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.second(), 45);
    }

    #[test]
    fn test_iso_with_tz_offset() {
        let dt = isoparse("2024-01-15T10:30:45+05:30").unwrap();
        assert_eq!(dt.hour(), 10);
    }

    #[test]
    fn test_iso_year_month() {
        let dt = isoparse("2024-01").unwrap();
        assert_eq!(dt.date(), NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    }

    #[test]
    fn test_iso_year_only() {
        let dt = isoparse("2024").unwrap();
        assert_eq!(dt.date(), NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    }

    #[test]
    fn test_iso_space_separator() {
        let dt = isoparse("2024-01-15 10:30:45").unwrap();
        assert_eq!(dt.hour(), 10);
    }

    #[test]
    fn test_iso_empty() {
        assert!(isoparse("").is_err());
    }

    #[test]
    fn test_iso_invalid() {
        assert!(isoparse("not-a-date").is_err());
    }

    #[test]
    fn test_iso_compact_date() {
        let dt = isoparse("20240115").unwrap();
        assert_eq!(dt.date(), NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
    }

    #[test]
    fn test_iso_hhmm() {
        let dt = isoparse("2024-01-15T1030").unwrap();
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_iso_frac_3_digits() {
        let dt = isoparse("2024-01-15T10:30:45.123").unwrap();
        assert_eq!(dt.nanosecond() / 1000, 123000);
    }

    #[test]
    fn test_iso_invalid_date_feb_30() {
        assert!(isoparse("2024-02-30").is_err());
    }

    #[test]
    fn test_iso_invalid_date_month_13() {
        assert!(isoparse("2024-13-01").is_err());
    }

    #[test]
    fn test_iso_invalid_date_day_00() {
        assert!(isoparse("2024-01-00").is_err());
    }

    #[test]
    fn test_iso_leap_year_feb_29() {
        let dt = isoparse("2024-02-29").unwrap();
        assert_eq!(dt.date(), NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
    }

    #[test]
    fn test_iso_non_leap_year_feb_29() {
        assert!(isoparse("2023-02-29").is_err());
    }

    #[test]
    fn test_iso_midnight() {
        let dt = isoparse("2024-01-15T00:00:00").unwrap();
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn test_iso_end_of_day() {
        let dt = isoparse("2024-01-15T23:59:59").unwrap();
        assert_eq!(dt.hour(), 23);
        assert_eq!(dt.minute(), 59);
        assert_eq!(dt.second(), 59);
    }

    #[test]
    fn test_iso_frac_1_digit() {
        let dt = isoparse("2024-01-15T10:30:45.1").unwrap();
        assert_eq!(dt.nanosecond() / 1000, 100000);
    }

    #[test]
    fn test_iso_frac_2_digits() {
        let dt = isoparse("2024-01-15T10:30:45.12").unwrap();
        assert_eq!(dt.nanosecond() / 1000, 120000);
    }

    #[test]
    fn test_iso_frac_4_digits() {
        let dt = isoparse("2024-01-15T10:30:45.1234").unwrap();
        assert_eq!(dt.nanosecond() / 1000, 123400);
    }

    #[test]
    fn test_iso_frac_5_digits() {
        let dt = isoparse("2024-01-15T10:30:45.12345").unwrap();
        assert_eq!(dt.nanosecond() / 1000, 123450);
    }

    #[test]
    fn test_iso_frac_6_digits() {
        let dt = isoparse("2024-01-15T10:30:45.123456").unwrap();
        assert_eq!(dt.nanosecond() / 1000, 123456);
    }

    #[test]
    fn test_iso_frac_more_than_6_digits_truncates() {
        // Only first 6 fractional digits used
        let dt = isoparse("2024-01-15T10:30:45.1234567").unwrap();
        assert_eq!(dt.nanosecond() / 1000, 123456);
    }

    #[test]
    fn test_iso_compact_hhmmss() {
        let dt = isoparse("20240115T103045").unwrap();
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }

    #[test]
    fn test_iso_compact_tz_offset() {
        let dt = isoparse("2024-01-15T10:30:45+0530").unwrap();
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_iso_compact_negative_tz() {
        let dt = isoparse("2024-01-15T10:30:45-0800").unwrap();
        assert_eq!(dt.hour(), 10);
    }

    #[test]
    fn test_iso_hh_only() {
        let dt = isoparse("2024-01-15T10").unwrap();
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 0);
    }

    #[test]
    fn test_iso_leading_trailing_whitespace() {
        let dt = isoparse("  2024-01-15T10:30:45  ").unwrap();
        assert_eq!(dt.date(), NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        assert_eq!(dt.hour(), 10);
    }

    #[test]
    fn test_iso_whitespace_only() {
        assert!(isoparse("   ").is_err());
    }

    #[test]
    fn test_iso_year_boundaries() {
        let dt = isoparse("0001-01-01").unwrap();
        assert_eq!(dt.date(), NaiveDate::from_ymd_opt(1, 1, 1).unwrap());

        let dt = isoparse("9999-12-31").unwrap();
        assert_eq!(dt.date(), NaiveDate::from_ymd_opt(9999, 12, 31).unwrap());
    }

    #[test]
    fn test_iso_dec_31() {
        let dt = isoparse("2024-12-31T23:59:59.999999").unwrap();
        assert_eq!(dt.date(), NaiveDate::from_ymd_opt(2024, 12, 31).unwrap());
        assert_eq!(dt.hour(), 23);
        assert_eq!(dt.nanosecond() / 1000, 999999);
    }

    #[test]
    fn test_iso_partial_formats_invalid() {
        assert!(isoparse("202").is_err()); // 3 digits
        assert!(isoparse("2024-1").is_err()); // incomplete month
        assert!(isoparse("2024-01-1").is_err()); // incomplete day
    }

    #[test]
    fn test_iso_invalid_time_hour_24() {
        assert!(isoparse("2024-01-15T24:00:00").is_err());
    }

    #[test]
    fn test_iso_invalid_time_minute_60() {
        assert!(isoparse("2024-01-15T10:60:00").is_err());
    }

    #[test]
    fn test_iso_invalid_time_second_60() {
        assert!(isoparse("2024-01-15T10:30:60").is_err());
    }

    #[test]
    fn test_iso_compact_with_z() {
        let dt = isoparse("20240115T103045Z").unwrap();
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }

    #[test]
    fn test_parse_int_edge_cases() {
        assert_eq!(parse_int("0").unwrap(), 0);
        assert_eq!(parse_int("000000").unwrap(), 0);
        assert_eq!(parse_int("999999").unwrap(), 999999);
        assert!(parse_int("").is_err());
        assert!(parse_int("abc").is_err());
        assert!(parse_int("12.3").is_err());
    }
}
