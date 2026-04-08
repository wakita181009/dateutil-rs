mod isoparser;
pub mod tokenizer;

pub use isoparser::isoparse;

use crate::error::ParseError;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};
use std::borrow::Cow;

// ---------------------------------------------------------------------------
// PHF lookup tables — compile-time perfect hash, zero runtime allocation
// ---------------------------------------------------------------------------

/// Jump words (ignored during parsing).
static JUMP: phf::Set<&'static str> = phf::phf_set! {
    " ", ".", ",", ";", "-", "/", "'",
    "at", "on", "and", "ad", "m", "t", "of",
    "st", "nd", "rd", "th",
};

/// Weekday name → 0-based index (Mon=0 .. Sun=6).
static WEEKDAYS: phf::Map<&'static str, usize> = phf::phf_map! {
    "mon" => 0, "monday" => 0,
    "tue" => 1, "tuesday" => 1,
    "wed" => 2, "wednesday" => 2,
    "thu" => 3, "thursday" => 3,
    "fri" => 4, "friday" => 4,
    "sat" => 5, "saturday" => 5,
    "sun" => 6, "sunday" => 6,
};

/// Month name → 1-based index.
static MONTHS: phf::Map<&'static str, usize> = phf::phf_map! {
    "jan" => 1, "january" => 1,
    "feb" => 2, "february" => 2,
    "mar" => 3, "march" => 3,
    "apr" => 4, "april" => 4,
    "may" => 5,
    "jun" => 6, "june" => 6,
    "jul" => 7, "july" => 7,
    "aug" => 8, "august" => 8,
    "sep" => 9, "sept" => 9, "september" => 9,
    "oct" => 10, "october" => 10,
    "nov" => 11, "november" => 11,
    "dec" => 12, "december" => 12,
};

/// HMS indicator → 0=hour, 1=minute, 2=second.
static HMS: phf::Map<&'static str, usize> = phf::phf_map! {
    "h" => 0, "hour" => 0, "hours" => 0,
    "m" => 1, "minute" => 1, "minutes" => 1,
    "s" => 2, "second" => 2, "seconds" => 2,
};

/// AM/PM → 0=AM, 1=PM.
static AMPM: phf::Map<&'static str, usize> = phf::phf_map! {
    "am" => 0, "a" => 0,
    "pm" => 1, "p" => 1,
};

static PERTAIN: phf::Set<&'static str> = phf::phf_set! { "of" };

static UTCZONE: phf::Set<&'static str> = phf::phf_set! { "utc", "gmt", "z" };

// ---------------------------------------------------------------------------
// Lookup helpers — avoid allocation by lowercasing into a stack buffer
// ---------------------------------------------------------------------------

/// Lowercase a token into a stack buffer (max 16 bytes).
/// Returns None if too long or contains non-ASCII bytes (safety guard for unsafe lower_str).
#[inline]
fn lowercase_buf(s: &str) -> Option<[u8; 16]> {
    let bytes = s.as_bytes();
    if bytes.len() > 16 || !bytes.iter().all(|b| b.is_ascii()) {
        return None;
    }
    let mut buf = [0u8; 16];
    for (i, &b) in bytes.iter().enumerate() {
        buf[i] = b.to_ascii_lowercase();
    }
    Some(buf)
}

#[inline]
fn lower_str<'a>(s: &str, buf: &'a [u8; 16]) -> &'a str {
    // SAFETY: lowercase_buf() validates all bytes are ASCII before lowercasing,
    // so the result is guaranteed valid UTF-8.
    unsafe { std::str::from_utf8_unchecked(&buf[..s.len()]) }
}

#[inline]
fn lookup_jump(s: &str) -> bool {
    if let Some(buf) = lowercase_buf(s) {
        JUMP.contains(lower_str(s, &buf))
    } else {
        false
    }
}

#[inline]
fn lookup_weekday(s: &str) -> Option<usize> {
    let buf = lowercase_buf(s)?;
    let low = lower_str(s, &buf);
    if let Some(&v) = WEEKDAYS.get(low) {
        return Some(v);
    }
    // Prefix match for 4+ letter abbreviations
    if s.len() >= 4 {
        let prefix = &low[..3];
        if let Some(&v) = WEEKDAYS.get(prefix) {
            return Some(v);
        }
    }
    None
}

#[inline]
fn lookup_month(s: &str) -> Option<usize> {
    let buf = lowercase_buf(s)?;
    MONTHS.get(lower_str(s, &buf)).copied()
}

#[inline]
fn lookup_hms(s: &str) -> Option<usize> {
    let buf = lowercase_buf(s)?;
    HMS.get(lower_str(s, &buf)).copied()
}

#[inline]
fn lookup_ampm(s: &str) -> Option<usize> {
    let buf = lowercase_buf(s)?;
    AMPM.get(lower_str(s, &buf)).copied()
}

#[inline]
fn lookup_pertain(s: &str) -> bool {
    if let Some(buf) = lowercase_buf(s) {
        PERTAIN.contains(lower_str(s, &buf))
    } else {
        false
    }
}

#[inline]
fn lookup_utczone(s: &str) -> bool {
    if let Some(buf) = lowercase_buf(s) {
        UTCZONE.contains(lower_str(s, &buf))
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// ParseResult
// ---------------------------------------------------------------------------

/// Result of parsing a date/time string.
#[derive(Debug, Default, Clone)]
pub struct ParseResult<'a> {
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub day: Option<u32>,
    pub weekday: Option<usize>,
    pub hour: Option<u32>,
    pub minute: Option<u32>,
    pub second: Option<u32>,
    pub microsecond: Option<u32>,
    pub tzname: Option<Cow<'a, str>>,
    pub tzoffset: Option<i32>,
    century_specified: bool,
}

impl ParseResult<'_> {
    fn field_count(&self) -> usize {
        let mut n = 0;
        if self.year.is_some() { n += 1; }
        if self.month.is_some() { n += 1; }
        if self.day.is_some() { n += 1; }
        if self.weekday.is_some() { n += 1; }
        if self.hour.is_some() { n += 1; }
        if self.minute.is_some() { n += 1; }
        if self.second.is_some() { n += 1; }
        if self.microsecond.is_some() { n += 1; }
        if self.tzname.is_some() { n += 1; }
        if self.tzoffset.is_some() { n += 1; }
        n
    }
}

// ---------------------------------------------------------------------------
// YMD resolver
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct Ymd {
    values: [i32; 3],
    count: usize,
    century_specified: bool,
    ystridx: Option<usize>,
    mstridx: Option<usize>,
    #[allow(dead_code)]
    dstridx: Option<usize>,
}

impl Ymd {
    fn push(&mut self, val: i32) {
        if self.count < 3 {
            self.values[self.count] = val;
            self.count += 1;
        }
    }

    #[allow(dead_code)]
    fn could_be_day(&self, value: i32) -> bool {
        if self.dstridx.is_some() {
            return false;
        }
        (1..=31).contains(&value)
    }

    fn resolve(
        &self,
        dayfirst: bool,
        yearfirst: bool,
    ) -> Result<(Option<i32>, Option<u32>, Option<u32>), ParseError>
    {
        #![allow(clippy::type_complexity)]
        let len = self.count;
        if len == 0 {
            return Ok((None, None, None));
        }

        let mut year = None;
        let mut month = None;
        let mut day = None;

        if let Some(mi) = self.mstridx {
            month = Some(self.values[mi] as u32);
            match len {
                2 => {
                    let other = if mi == 0 { 1 } else { 0 };
                    let v = self.values[other];
                    if v > 31 {
                        year = Some(v);
                    } else {
                        day = Some(v as u32);
                    }
                }
                3 => {
                    let (a, b) = match mi {
                        0 => (1, 2),
                        1 => (0, 2),
                        _ => (0, 1),
                    };
                    let va = self.values[a];
                    let vb = self.values[b];
                    if va > 31 {
                        year = Some(va);
                        day = Some(vb as u32);
                    } else if vb > 31 {
                        year = Some(vb);
                        day = Some(va as u32);
                    } else if dayfirst && a < b {
                        day = Some(va as u32);
                        year = Some(vb);
                    } else {
                        day = Some(vb as u32);
                        year = Some(va);
                    }
                }
                _ => {
                    return Ok((None, month, None));
                }
            }
        } else {
            match len {
                1 => {
                    if self.values[0] > 31 {
                        year = Some(self.values[0]);
                    } else {
                        day = Some(self.values[0] as u32);
                    }
                }
                2 => {
                    let (v0, v1) = (self.values[0], self.values[1]);
                    if v0 > 31 {
                        year = Some(v0);
                        month = Some(v1 as u32);
                    } else if v1 > 31 {
                        month = Some(v0 as u32);
                        year = Some(v1);
                    } else if dayfirst {
                        day = Some(v0 as u32);
                        month = Some(v1 as u32);
                    } else {
                        month = Some(v0 as u32);
                        day = Some(v1 as u32);
                    }
                }
                3 => {
                    let (v0, v1, v2) = (self.values[0], self.values[1], self.values[2]);
                    if v0 > 31 || self.ystridx == Some(0) || (yearfirst && v1 <= 12 && v2 <= 31) {
                        year = Some(v0);
                        month = Some(v1 as u32);
                        day = Some(v2 as u32);
                    } else if v0 > 12 || (dayfirst && v1 <= 12) {
                        day = Some(v0 as u32);
                        month = Some(v1 as u32);
                        year = Some(v2);
                    } else {
                        month = Some(v0 as u32);
                        day = Some(v1 as u32);
                        year = Some(v2);
                    }
                }
                _ => {}
            }
        }

        Ok((year, month, day))
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

fn convertyear(year: i32, century_specified: bool) -> i32 {
    if year < 100 && !century_specified {
        let now_year = chrono::Local::now().year();
        let century = now_year / 100 * 100;
        let mut y = year + century;
        if y >= now_year + 50 {
            y -= 100;
        } else if y < now_year - 50 {
            y += 100;
        }
        y
    } else {
        year
    }
}

#[allow(dead_code)]
fn days_in_month(year: i32, month: u32) -> u32 {
    const DAYS: [u32; 13] = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if month == 2 && (year % 4 == 0 && year % 100 != 0 || year % 400 == 0) {
        29
    } else if (1..=12).contains(&month) {
        DAYS[month as usize]
    } else {
        31
    }
}

// ---------------------------------------------------------------------------
// parse() — main entry point
// ---------------------------------------------------------------------------

/// Parse a date/time string into a NaiveDateTime.
///
/// This is a Rust-optimized port of `dateutil.parser.parse()` with:
/// - Zero-copy tokenizer (no String allocations during tokenization)
/// - Compile-time phf hash maps for token lookup
/// - Stack-allocated buffers for lowercase conversion
pub fn parse(timestr: &str, dayfirst: bool, yearfirst: bool) -> Result<NaiveDateTime, ParseError> {
    let (res, _) = parse_to_result(timestr, dayfirst, yearfirst)?;
    build_naive(timestr, &res)
}

/// Parse returning the raw ParseResult (for advanced usage).
pub fn parse_to_result<'a>(
    timestr: &'a str,
    dayfirst: bool,
    yearfirst: bool,
) -> Result<(ParseResult<'a>, Vec<Cow<'a, str>>), ParseError> {
    let tokens = tokenizer::tokenize(timestr);
    let mut res = ParseResult::default();
    let mut ymd = Ymd::default();
    let mut skipped_tokens: Vec<Cow<'a, str>> = Vec::new();

    let len = tokens.len();
    let mut i = 0;

    while i < len {
        let token_found = try_parse_token(
            &tokens, i, len, &mut res, &mut ymd, &mut skipped_tokens, dayfirst,
        );

        if !token_found {
            skipped_tokens.push(tokens[i].clone());
        }

        i += 1;
    }

    // Resolve YMD
    let (year, month, day) = ymd.resolve(dayfirst, yearfirst)?;
    res.year = year;
    res.month = month;
    res.day = day;
    res.century_specified = ymd.century_specified;

    // Validate
    if let Some(y) = res.year {
        res.year = Some(convertyear(y, res.century_specified));
    }

    // UTC zone normalization
    if (res.tzoffset == Some(0) && res.tzname.is_none())
        || res.tzname.as_deref() == Some("Z")
        || res.tzname.as_deref() == Some("z")
    {
        res.tzname = Some("UTC".into());
        res.tzoffset = Some(0);
    }

    if res.field_count() == 0 {
        return Err(ParseError::NoDate(timestr.to_string()));
    }

    Ok((res, skipped_tokens))
}

#[inline]
fn try_parse_token<'a>(
    tokens: &[Cow<'a, str>],
    i: usize,
    len: usize,
    res: &mut ParseResult<'a>,
    ymd: &mut Ymd,
    _skipped: &mut Vec<Cow<'a, str>>,
    _dayfirst: bool,
) -> bool {
    let token = &tokens[i];

    // Try as number — integer first (fast path avoids expensive f64 parse)
    let num: Option<(i32, f64)> = if let Ok(vi) = token.parse::<i32>() {
        Some((vi, vi as f64))
    } else if let Ok(vf) = token.parse::<f64>() {
        Some((vf as i32, vf))
    } else {
        None
    };
    if let Some((value_i, value)) = num {
        // Check for HH:MM:SS pattern (number followed by ":" or HMS word)
        if i + 1 < len && tokens[i + 1] == ":" {
            // Time component — handled by caller advancing through ":"
            return try_parse_time_component(tokens, i, len, res, value);
        }

        // Check if next token is HMS
        if i + 1 < len {
            if let Some(hms_idx) = lookup_hms(&tokens[i + 1]) {
                assign_hms(res, hms_idx, value);
                return true;
            }
        }

        // Check for decimal seconds
        if token.contains('.') && res.second.is_some() && res.microsecond.is_none() {
            let frac = value - value.floor();
            res.microsecond = Some((frac * 1_000_000.0).round() as u32);
            return true;
        }

        // Date component
        let slen = token.len();
        if ymd.count < 3 {
            if slen == 4 || (slen >= 5 && !token.contains('.')) {
                // Likely a year (4+ digits) or concatenated date
                ymd.century_specified = true;
            }
            ymd.push(value_i);
            return true;
        }

        // Try as hour if no hour set
        if res.hour.is_none() && (0..24).contains(&value_i) {
            res.hour = Some(value_i as u32);
            return true;
        }

        return false;
    }

    // Try as weekday
    if let Some(wd) = lookup_weekday(token) {
        res.weekday = Some(wd);
        return true;
    }

    // Try as month
    if let Some(mo) = lookup_month(token) {
        ymd.mstridx = Some(ymd.count);
        ymd.push(mo as i32);
        return true;
    }

    // Try as AM/PM
    if let Some(ampm) = lookup_ampm(token) {
        if let Some(h) = res.hour {
            if ampm == 1 && h < 12 {
                res.hour = Some(h + 12);
            } else if ampm == 0 && h == 12 {
                res.hour = Some(0);
            }
        }
        return true;
    }

    // Timezone offset: "+0530", "-05:00", or tokenized as ["+", "05", ":", "30"]
    if (token == "+" || token == "-") && i + 1 < len {
        // Reconstruct offset from subsequent tokens into stack buffer (no heap alloc)
        let mut buf = [0u8; 16];
        buf[0] = token.as_bytes()[0]; // '+' or '-'
        let mut blen = 1usize;
        let mut j = i + 1;
        while j < len && blen < 16 {
            let tj = tokens[j].as_bytes();
            if tj.iter().all(|b| b.is_ascii_digit()) || tokens[j] == ":" {
                if blen + tj.len() > 16 { break; }
                buf[blen..blen + tj.len()].copy_from_slice(tj);
                blen += tj.len();
                j += 1;
            } else {
                break;
            }
        }
        // SAFETY: all bytes are ASCII (sign, digits, colons)
        let offset_str = unsafe { std::str::from_utf8_unchecked(&buf[..blen]) };
        if let Some(offset) = parse_tzoffset(offset_str) {
            res.tzoffset = Some(offset);
            if i > 0 && !tokens[i - 1].chars().next().is_some_and(|c| c.is_ascii_digit()) {
                let prev = &tokens[i - 1];
                if !lookup_jump(prev) && prev != ":" {
                    res.tzname = Some(prev.clone());
                }
            }
            return true;
        }
    }
    if (token.starts_with('+') || token.starts_with('-')) && token.len() >= 3 {
        if let Some(offset) = parse_tzoffset(token) {
            res.tzoffset = Some(offset);
            return true;
        }
    }

    // UTC zone
    if lookup_utczone(token) {
        res.tzname = Some("UTC".into());
        res.tzoffset = Some(0);
        return true;
    }

    // Jump word
    if lookup_jump(token) {
        return true;
    }

    // Pertain word
    if lookup_pertain(token) {
        return true;
    }

    // Timezone name (alphabetic, not matched above)
    if token.chars().all(|c| c.is_alphabetic()) && res.tzname.is_none() && res.hour.is_some() {
        res.tzname = Some(token.clone());
        // Check next for offset
        if i + 1 < len
            && (tokens[i + 1].starts_with('+') || tokens[i + 1].starts_with('-'))
        {
            if let Some(offset) = parse_tzoffset(&tokens[i + 1]) {
                res.tzoffset = Some(offset);
            }
        }
        return true;
    }

    false
}

#[inline]
fn try_parse_time_component(
    tokens: &[Cow<'_, str>],
    i: usize,
    len: usize,
    res: &mut ParseResult<'_>,
    value: f64,
) -> bool {
    // Pattern: HH:MM or HH:MM:SS or HH:MM:SS.ffffff
    if res.hour.is_none() {
        res.hour = Some(value as u32);

        // Look for :MM
        if i + 2 < len && tokens[i + 1] == ":" {
            if let Ok(min) = tokens[i + 2].parse::<f64>() {
                res.minute = Some(min as u32);
                // Look for :SS
                if i + 4 < len && tokens[i + 3] == ":" {
                    if let Ok(sec_str) = tokens[i + 4].parse::<f64>() {
                        res.second = Some(sec_str as u32);
                        let frac = sec_str - sec_str.floor();
                        if frac > 0.0 {
                            res.microsecond = Some((frac * 1_000_000.0).round() as u32);
                        }
                    }
                }
            }
        }
        return true;
    }
    false
}

#[inline]
fn assign_hms(res: &mut ParseResult<'_>, hms_idx: usize, value: f64) {
    match hms_idx {
        0 => res.hour = Some(value as u32),
        1 => res.minute = Some(value as u32),
        2 => {
            res.second = Some(value as u32);
            let frac = value - value.floor();
            if frac > 0.0 {
                res.microsecond = Some((frac * 1_000_000.0).round() as u32);
            }
        }
        _ => {}
    }
}

#[inline]
fn parse_tzoffset(s: &str) -> Option<i32> {
    let (sign, rest) = if let Some(r) = s.strip_prefix('+') {
        (1, r)
    } else if let Some(r) = s.strip_prefix('-') {
        (-1, r)
    } else {
        return None;
    };

    // Validate: only digits and colons allowed
    if !rest.bytes().all(|b| b.is_ascii_digit() || b == b':') {
        return None;
    }

    // Count digits for minimum check
    let digit_count = rest.bytes().filter(|b| b.is_ascii_digit()).count();
    if digit_count < 2 {
        return None;
    }

    // Parse based on format — zero allocation (no String::replace)
    let (hours, minutes) = if let Some(colon_pos) = rest.find(':') {
        // HH:MM or H:MM
        let h = rest[..colon_pos].parse::<i32>().ok()?;
        let m = rest[colon_pos + 1..].parse::<i32>().ok()?;
        (h, m)
    } else if rest.len() <= 2 {
        // HH only
        (rest.parse::<i32>().ok()?, 0)
    } else {
        // HHMM — last 2 digits are minutes
        let h = rest[..rest.len() - 2].parse::<i32>().ok()?;
        let m = rest[rest.len() - 2..].parse::<i32>().ok()?;
        (h, m)
    };

    Some(sign * (hours * 3600 + minutes * 60))
}

fn build_naive(_timestr: &str, res: &ParseResult<'_>) -> Result<NaiveDateTime, ParseError> {
    let now = chrono::Local::now().naive_local();

    let year = res.year.unwrap_or(now.year());
    let month = res.month.unwrap_or(now.month());
    let day = res.day.unwrap_or(now.day());
    let hour = res.hour.unwrap_or(0);
    let minute = res.minute.unwrap_or(0);
    let second = res.second.unwrap_or(0);
    let microsecond = res.microsecond.unwrap_or(0);

    let date = NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        ParseError::ValueError(format!("invalid date: {year}-{month}-{day}"))
    })?;
    let time = NaiveTime::from_hms_micro_opt(hour, minute, second, microsecond).ok_or_else(
        || ParseError::ValueError(format!("invalid time: {hour}:{minute}:{second}.{microsecond}")),
    )?;

    Ok(NaiveDateTime::new(date, time))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_parse_iso_basic() {
        let dt = parse("2024-01-15", false, false).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_datetime() {
        let dt = parse("2024-01-15 10:30:45", false, false).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }

    #[test]
    fn test_parse_month_name() {
        let dt = parse("January 15, 2024", false, false).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_month_abbrev() {
        let dt = parse("15 Jan 2024", false, false).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_us_format() {
        // MM/DD/YYYY (default, dayfirst=false)
        let dt = parse("01/15/2024", false, false).unwrap();
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_dayfirst() {
        // DD/MM/YYYY
        let dt = parse("15/01/2024", true, false).unwrap();
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.month(), 1);
    }

    #[test]
    fn test_parse_yearfirst() {
        let dt = parse("2024/01/15", false, true).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_with_time_and_tz() {
        let (res, _) = parse_to_result("2024-01-15 10:30:45 UTC", false, false).unwrap();
        assert_eq!(res.hour, Some(10));
        assert_eq!(res.minute, Some(30));
        assert_eq!(res.second, Some(45));
        assert_eq!(res.tzname, Some("UTC".into()));
        assert_eq!(res.tzoffset, Some(0));
    }

    #[test]
    fn test_parse_ampm() {
        let dt = parse("January 15, 2024 3:30 PM", false, false).unwrap();
        assert_eq!(dt.hour(), 15);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_parse_microseconds() {
        let dt = parse("2024-01-15 10:30:45.123456", false, false).unwrap();
        assert_eq!(dt.second(), 45);
        assert_eq!(dt.nanosecond() / 1000, 123456);
    }

    #[test]
    fn test_parse_tz_offset() {
        let (res, _) = parse_to_result("2024-01-15 10:30:45+05:30", false, false).unwrap();
        assert_eq!(res.tzoffset, Some(5 * 3600 + 30 * 60));
    }

    #[test]
    fn test_parse_tz_negative() {
        let (res, _) = parse_to_result("2024-01-15 10:30:45-0800", false, false).unwrap();
        assert_eq!(res.tzoffset, Some(-(8 * 3600)));
    }

    #[test]
    fn test_parse_weekday() {
        let (res, _) = parse_to_result("Monday, January 15, 2024", false, false).unwrap();
        assert_eq!(res.weekday, Some(0)); // Monday
    }

    #[test]
    fn test_parse_empty_string() {
        assert!(parse("", false, false).is_err());
    }

    #[test]
    fn test_parse_no_date() {
        assert!(parse("hello world", false, false).is_err());
    }

    #[test]
    fn test_phf_lookup_month() {
        assert_eq!(lookup_month("January"), Some(1));
        assert_eq!(lookup_month("jan"), Some(1));
        assert_eq!(lookup_month("DECEMBER"), Some(12));
        assert_eq!(lookup_month("sept"), Some(9));
        assert_eq!(lookup_month("xyz"), None);
    }

    #[test]
    fn test_phf_lookup_weekday() {
        assert_eq!(lookup_weekday("Monday"), Some(0));
        assert_eq!(lookup_weekday("fri"), Some(4));
        assert_eq!(lookup_weekday("Frid"), Some(4)); // prefix match
        assert_eq!(lookup_weekday("xyz"), None);
    }
}
