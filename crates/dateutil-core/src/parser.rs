mod compact;
mod hms;
mod isoparser;
mod parserinfo;
pub mod tokenizer;

use compact::try_parse_compact;
use hms::assign_hms;
pub use isoparser::{isoparse, IsoParsed, IsoParser, IsoTimeParsed, IsoTz};
pub use parserinfo::ParserInfo;
use parserinfo::{
    do_ampm_lc, do_hms, do_jump, do_jump_lc, do_month_lc, do_pertain_lc, do_tzoffset_lc,
    do_utczone_lc, do_weekday_lc,
};

use crate::error::ParseError;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use std::borrow::Cow;

// ---------------------------------------------------------------------------
// Fast integer parse — hand-rolled for short ASCII digit strings
// ---------------------------------------------------------------------------

/// Fast integer parse for pure-digit ASCII strings (e.g. "2024", "01", "30").
/// Returns None if the string contains non-digit bytes or is empty.
#[inline]
fn fast_parse_int(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let mut n: i32 = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?.checked_add((b - b'0') as i32)?;
    }
    Some(n)
}

/// Fast decimal parse for "NN" or "NN.FFFFFF" patterns.
/// Returns (integer_part, microseconds) using pure integer arithmetic — no f64.
#[inline]
fn fast_parse_decimal(s: &str) -> Option<(i32, u32)> {
    let dot_pos = s.as_bytes().iter().position(|&b| b == b'.')?;
    let int_part = fast_parse_int(&s[..dot_pos])?;
    let frac_str = &s[dot_pos + 1..];
    if frac_str.is_empty() {
        return Some((int_part, 0));
    }
    let us = match frac_str.len() {
        1..=5 => {
            const SCALE: [u32; 5] = [100_000, 10_000, 1_000, 100, 10];
            fast_parse_int(frac_str)? as u32 * SCALE[frac_str.len() - 1]
        }
        _ => fast_parse_int(&frac_str[..6])? as u32,
    };
    Some((int_part, us))
}

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
pub(crate) fn lowercase_buf(s: &str) -> Option<[u8; 16]> {
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
pub(crate) fn lower_str<'a>(s: &str, buf: &'a [u8; 16]) -> &'a str {
    // SAFETY: lowercase_buf() validates all bytes are ASCII before lowercasing,
    // so the result is guaranteed valid UTF-8.
    unsafe { std::str::from_utf8_unchecked(&buf[..s.len()]) }
}

#[inline]
pub(crate) fn lookup_jump_lc(low: Option<&str>) -> bool {
    low.is_some_and(|l| JUMP.contains(l))
}

#[inline]
pub(crate) fn lookup_weekday_lc(original_len: usize, low: Option<&str>) -> Option<usize> {
    let low = low?;
    if let Some(&v) = WEEKDAYS.get(low) {
        return Some(v);
    }
    if original_len >= 4 {
        if let Some(&v) = WEEKDAYS.get(&low[..3]) {
            return Some(v);
        }
    }
    None
}

#[inline]
pub(crate) fn lookup_month_lc(low: Option<&str>) -> Option<usize> {
    low.and_then(|l| MONTHS.get(l).copied())
}

#[inline]
pub(crate) fn lookup_hms_lc(low: Option<&str>) -> Option<usize> {
    low.and_then(|l| HMS.get(l).copied())
}

#[inline]
pub(crate) fn lookup_ampm_lc(low: Option<&str>) -> Option<usize> {
    low.and_then(|l| AMPM.get(l).copied())
}

#[inline]
pub(crate) fn lookup_pertain_lc(low: Option<&str>) -> bool {
    low.is_some_and(|l| PERTAIN.contains(l))
}

#[inline]
pub(crate) fn lookup_utczone_lc(low: Option<&str>) -> bool {
    low.is_some_and(|l| UTCZONE.contains(l))
}

// ---- Wrappers that recompute lowercase_buf ----
// Test-only variants (non-_lc) keep existing regression tests readable.
// The jump/hms wrappers are also used by the rare non-hot-path dispatchers
// in parserinfo.rs (do_jump on prev token, do_hms on lookahead token).

#[inline]
fn lookup_jump(s: &str) -> bool {
    let buf = lowercase_buf(s);
    lookup_jump_lc(buf.as_ref().map(|b| lower_str(s, b)))
}

#[cfg(test)]
#[inline]
fn lookup_weekday(s: &str) -> Option<usize> {
    let buf = lowercase_buf(s);
    lookup_weekday_lc(s.len(), buf.as_ref().map(|b| lower_str(s, b)))
}

#[cfg(test)]
#[inline]
fn lookup_month(s: &str) -> Option<usize> {
    let buf = lowercase_buf(s);
    lookup_month_lc(buf.as_ref().map(|b| lower_str(s, b)))
}

// lookup_hms retained for the non-test caller do_hms() below.
#[inline]
fn lookup_hms(s: &str) -> Option<usize> {
    let buf = lowercase_buf(s);
    lookup_hms_lc(buf.as_ref().map(|b| lower_str(s, b)))
}

#[cfg(test)]
#[inline]
fn lookup_ampm(s: &str) -> Option<usize> {
    let buf = lowercase_buf(s);
    lookup_ampm_lc(buf.as_ref().map(|b| lower_str(s, b)))
}

#[cfg(test)]
#[inline]
fn lookup_pertain(s: &str) -> bool {
    let buf = lowercase_buf(s);
    lookup_pertain_lc(buf.as_ref().map(|b| lower_str(s, b)))
}

#[cfg(test)]
#[inline]
fn lookup_utczone(s: &str) -> bool {
    let buf = lowercase_buf(s);
    lookup_utczone_lc(buf.as_ref().map(|b| lower_str(s, b)))
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
}

impl ParseResult<'_> {
    fn field_count(&self) -> usize {
        self.year.is_some() as usize
            + self.month.is_some() as usize
            + self.day.is_some() as usize
            + self.weekday.is_some() as usize
            + self.hour.is_some() as usize
            + self.minute.is_some() as usize
            + self.second.is_some() as usize
            + self.microsecond.is_some() as usize
            + self.tzname.is_some() as usize
            + self.tzoffset.is_some() as usize
    }
}

#[derive(Debug, Default)]
struct ParseState<'a> {
    result: ParseResult<'a>,
    century_specified: bool,
    ampm_no_hour: bool,
    ampm_out_of_range: bool,
    malformed_time: bool,
    last_hms_idx: Option<u8>,
}

impl<'a> std::ops::Deref for ParseState<'a> {
    type Target = ParseResult<'a>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.result
    }
}

impl<'a> std::ops::DerefMut for ParseState<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.result
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
        debug_assert!(
            self.count < 3,
            "Ymd::push called with count={}, val={val}",
            self.count
        );
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
    ) -> Result<(Option<i32>, Option<u32>, Option<u32>), ParseError> {
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
                    if v > 31 || self.ystridx == Some(other) {
                        year = Some(v);
                    } else {
                        day = Some(v as u32);
                    }
                }
                3 => {
                    // Matches python-dateutil _parse: when a month token is
                    // present, the remaining two positions are assigned by
                    // month-position and value magnitude. `dayfirst` is NOT
                    // consulted here (upstream reserves it for the all-
                    // numeric case); `yearfirst` only affects mi=1.
                    //
                    //   mi=0 (M X Y): va>31 ? Y D : D Y (default M D Y)
                    //   mi=1 (X M Y): va>31 || (yearfirst && vb<=31) ? Y D
                    //                                                 : D Y
                    //   mi=2 (X Y M): vb>31 ? D Y : Y D (default Y D M)
                    //
                    // An explicit `ystridx` set when a 4+ digit token was
                    // consumed (e.g. "0031 Nov 03") overrides the positional
                    // default.
                    let (a, b) = match mi {
                        0 => (1, 2),
                        1 => (0, 2),
                        _ => (0, 1),
                    };
                    let va = self.values[a];
                    let vb = self.values[b];
                    let va_is_year = if self.ystridx == Some(a) {
                        true
                    } else if self.ystridx == Some(b) {
                        false
                    } else {
                        match mi {
                            0 => va > 31,
                            1 => va > 31 || (yearfirst && vb <= 31),
                            _ => vb <= 31,
                        }
                    };
                    if va_is_year {
                        year = Some(va);
                        day = Some(vb as u32);
                    } else {
                        day = Some(va as u32);
                        year = Some(vb);
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

fn convertyear(year: i32, century_specified: bool, now_year: i32) -> i32 {
    if year < 100 && !century_specified {
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

// ---------------------------------------------------------------------------
// parse() — main entry point
// ---------------------------------------------------------------------------

/// Parse a date/time string into a `NaiveDateTime`.
///
/// Missing fields are filled from `default`. If `None`, uses today at midnight
/// (matching python-dateutil). Two-digit years are always resolved against the
/// current year.
///
/// Pass `info` to override the default English lookup tables (month names,
/// weekday names, etc.) for non-English or customised parsing.
pub fn parse(
    timestr: &str,
    dayfirst: bool,
    yearfirst: bool,
    default: Option<NaiveDateTime>,
    info: Option<&ParserInfo>,
) -> Result<NaiveDateTime, ParseError> {
    let (default, current_year) = match default {
        Some(d) => (d, None),
        None => {
            let now = chrono::Local::now().naive_local();
            (now.date().and_hms_opt(0, 0, 0).unwrap(), Some(now.year()))
        }
    };
    let res = parse_to_result_with_year(timestr, dayfirst, yearfirst, current_year, info)?;
    build_naive(&res, default)
}

/// Parse returning the raw `ParseResult` (for advanced usage).
///
/// Use this when you need access to individual parsed fields (e.g. `tzname`,
/// `tzoffset`, `weekday`) that are not available from `NaiveDateTime`.
pub fn parse_to_result<'a>(
    timestr: &'a str,
    dayfirst: bool,
    yearfirst: bool,
    info: Option<&ParserInfo>,
) -> Result<ParseResult<'a>, ParseError> {
    parse_to_result_with_year(timestr, dayfirst, yearfirst, None, info)
}

fn parse_to_result_with_year<'a>(
    timestr: &'a str,
    dayfirst: bool,
    yearfirst: bool,
    current_year: Option<i32>,
    info: Option<&ParserInfo>,
) -> Result<ParseResult<'a>, ParseError> {
    let tokens = tokenizer::tokenize(timestr);
    let mut state = ParseState::default();
    let mut ymd = Ymd::default();

    let len = tokens.len();
    let mut i = 0;

    while i < len {
        let consumed = try_parse_token(&tokens, i, len, &mut state, &mut ymd, dayfirst, info);

        if consumed == 0 {
            i += 1;
        } else {
            i += consumed;
        }
    }

    // Resolve YMD
    let (year, month, day) = ymd.resolve(dayfirst, yearfirst)?;
    state.result.year = year;
    state.result.month = month;
    state.result.day = day;
    state.century_specified = ymd.century_specified;

    if let Some(y) = state.result.year {
        if y < 100 && !state.century_specified {
            let now_year =
                current_year.unwrap_or_else(|| chrono::Local::now().naive_local().year());
            state.result.year = Some(convertyear(y, false, now_year));
        }
    }

    // UTC zone normalization
    if (state.result.tzoffset == Some(0) && state.result.tzname.is_none())
        || state.result.tzname.as_deref() == Some("Z")
        || state.result.tzname.as_deref() == Some("z")
    {
        state.result.tzname = Some("UTC".into());
        state.result.tzoffset = Some(0);
    }

    if state.result.field_count() == 0 {
        return Err(ParseError::NoDate(timestr.into()));
    }

    if state.ampm_no_hour {
        return Err(ParseError::ValueError(
            "No hour specified with AM or PM flag.".into(),
        ));
    }
    if state.ampm_out_of_range {
        return Err(ParseError::ValueError(
            "Invalid hour specified for 12-hour clock.".into(),
        ));
    }
    if state.malformed_time {
        return Err(ParseError::UnknownFormat(timestr.into()));
    }

    Ok(state.result)
}

// ---------------------------------------------------------------------------
// Compact / dot-separated date helpers
// ---------------------------------------------------------------------------

/// Try to parse a dot-separated date token (e.g., "2003.09.25", "09.25.2003").
/// Returns true if the token was consumed as a date (3 values pushed to YMD).
#[inline]
fn try_parse_dot_date(token: &str, ymd: &mut Ymd) -> bool {
    if !token.as_bytes().first().is_some_and(|b| b.is_ascii_digit()) {
        return false;
    }

    let mut parts = token.splitn(4, '.');
    let p0 = match parts.next() {
        Some(s) if !s.is_empty() => s,
        _ => return false,
    };
    let p1 = match parts.next() {
        Some(s) if !s.is_empty() => s,
        _ => return false,
    };
    let p2 = match parts.next() {
        Some(s) if !s.is_empty() => s,
        _ => return false,
    };
    if parts.next().is_some() {
        return false;
    }

    let Some(v0) = fast_parse_int(p0) else {
        return false;
    };
    let Some(v1) = fast_parse_int(p1) else {
        return false;
    };
    let Some(v2) = fast_parse_int(p2) else {
        return false;
    };

    if p0.len() >= 4 || p2.len() >= 4 {
        ymd.century_specified = true;
    }

    ymd.push(v0);
    ymd.push(v1);
    ymd.push(v2);

    true
}

#[inline]
/// Returns the number of tokens consumed (0 = not matched).
fn try_parse_token<'a>(
    tokens: &[Cow<'a, str>],
    i: usize,
    len: usize,
    res: &mut ParseState<'a>,
    ymd: &mut Ymd,
    _dayfirst: bool,
    info: Option<&ParserInfo>,
) -> usize {
    let token = &tokens[i];

    // Handle dot-separated dates (e.g., "2003.09.25", "09.25.2003")
    if ymd.count == 0 && try_parse_dot_date(token, ymd) {
        return 1;
    }

    // Handle compact all-digit tokens first so 12/14-digit forms (which overflow
    // i32) like "199709020908" and "19970902090807" still reach try_parse_compact.
    if !token.is_empty() && token.as_bytes().iter().all(|b| b.is_ascii_digit()) {
        let compact = try_parse_compact(tokens, i, len, res, ymd, token);
        if compact > 0 {
            return compact;
        }
    }

    // Try as number — fast integer path first, then decimal (no f64)
    let num: Option<(i32, u32)> = if let Some(vi) = fast_parse_int(token) {
        Some((vi, 0))
    } else {
        fast_parse_decimal(token)
    };
    if let Some((value_i, value_us)) = num {
        // Compact HHMMSS.ffffff after date (e.g., "20030925T104941.5-0300")
        if value_us > 0
            && ymd.count == 3
            && res.hour.is_none()
            && token.as_bytes().iter().position(|&b| b == b'.') == Some(6)
        {
            let int_part = &token[..6];
            let Some(hour) = fast_parse_int(&int_part[0..2]) else {
                return 0;
            };
            let Some(minute) = fast_parse_int(&int_part[2..4]) else {
                return 0;
            };
            let Some(second) = fast_parse_int(&int_part[4..6]) else {
                return 0;
            };
            if hour <= 23 && minute <= 59 && second <= 59 {
                res.hour = Some(hour as u32);
                res.minute = Some(minute as u32);
                res.second = Some(second as u32);
                res.microsecond = Some(value_us);
                return 1;
            }
        }

        // Check for HH:MM:SS pattern (number followed by ":" or HMS word)
        if i + 1 < len && tokens[i + 1] == ":" {
            return try_parse_time_component(tokens, i, len, res, value_i as u32);
        }

        // Check if next token is HMS (skip intervening jump-but-not-HMS
        // tokens; "m"/"t" appear in both sets, so we must stop on HMS match).
        {
            let mut j = i + 1;
            while j < len && do_jump(&tokens[j], info) && do_hms(&tokens[j], info).is_none() {
                j += 1;
            }
            if j < len {
                if let Some(hms_idx) = do_hms(&tokens[j], info) {
                    assign_hms(res, hms_idx, value_i as u32, value_us);
                    return j + 1 - i;
                }
            }
        }

        // Continuation of an HMS run: after hour/minute via HMS word, a bare
        // number means the next smaller unit ("01h02" → hour=1, minute=2,
        // "10 h 36.5" → hour=10, minute=36, second=30).
        if let Some(prev) = res.last_hms_idx {
            let next_idx = (prev as usize) + 1;
            if next_idx <= 2 && (0..60).contains(&value_i) {
                let already_set = match next_idx {
                    1 => res.minute.is_some(),
                    2 => res.second.is_some(),
                    _ => true,
                };
                if !already_set {
                    assign_hms(res, next_idx, value_i as u32, value_us);
                    return 1;
                }
            }
        }

        // Check if next token is AM/PM (e.g., "10am", "10pm")
        if value_us == 0 && res.hour.is_none() && (0..=24).contains(&value_i) && i + 1 < len {
            let next_lc_buf = lowercase_buf(&tokens[i + 1]);
            let next_lc = next_lc_buf.as_ref().map(|b| lower_str(&tokens[i + 1], b));
            if let Some(ampm) = do_ampm_lc(next_lc, info) {
                if value_i > 12 {
                    res.ampm_out_of_range = true;
                } else {
                    let mut hour = value_i as u32;
                    if ampm == 1 && hour < 12 {
                        hour += 12;
                    } else if ampm == 0 && hour == 12 {
                        hour = 0;
                    }
                    res.hour = Some(hour);
                }
                return 2;
            }
        }

        // Check for decimal seconds (only accept pure fractional values like "0.5")
        if value_us > 0 && value_i == 0 && res.second.is_some() && res.microsecond.is_none() {
            res.microsecond = Some(value_us);
            return 1;
        }

        // Date component
        let slen = token.len();
        if ymd.count < 3 {
            if slen == 4 || (slen >= 5 && !token.contains('.')) {
                // Likely a year (4+ digits) or concatenated date
                ymd.century_specified = true;
                if ymd.ystridx.is_none() {
                    ymd.ystridx = Some(ymd.count);
                }
            }
            ymd.push(value_i);
            return 1;
        }

        // Try as hour if no hour set
        if res.hour.is_none() && (0..24).contains(&value_i) {
            res.hour = Some(value_i as u32);
            return 1;
        }

        return 0;
    }

    // Lower-case the token once and reuse the view across all alphabetic
    // dispatch helpers. Non-ASCII / overlong tokens get `None` — none of the
    // PHF tables or ParserInfo HashMaps will match those.
    let lc_buf = lowercase_buf(token);
    let lc: Option<&str> = lc_buf.as_ref().map(|b| lower_str(token, b));
    let token_len = token.len();

    // Try as weekday
    if let Some(wd) = do_weekday_lc(token_len, lc, info) {
        res.weekday = Some(wd);
        return 1;
    }

    // Try as month
    if let Some(mo) = do_month_lc(lc, info) {
        ymd.mstridx = Some(ymd.count);
        ymd.push(mo as i32);
        return 1;
    }

    // Try as AM/PM
    if let Some(ampm) = do_ampm_lc(lc, info) {
        match res.hour {
            None => res.ampm_no_hour = true,
            Some(h) if h > 12 => res.ampm_out_of_range = true,
            Some(h) => {
                if ampm == 1 && h < 12 {
                    res.hour = Some(h + 12);
                } else if ampm == 0 && h == 12 {
                    res.hour = Some(0);
                }
            }
        }
        return 1;
    }

    // Timezone offset: "+0530", "-05:00", or tokenized as ["+", "05", ":", "30"]
    // Guard: only treat standalone +/- as tz offset after time has been parsed,
    // otherwise date separators like "-" in "2024-01-15" would be misinterpreted.
    if (token == "+" || token == "-") && i + 1 < len && res.hour.is_some() {
        // Reconstruct offset from subsequent tokens into stack buffer (no heap alloc)
        let mut buf = [0u8; 16];
        buf[0] = token.as_bytes()[0]; // '+' or '-'
        let mut blen = 1usize;
        let mut j = i + 1;
        while j < len && blen < 16 {
            let tj = tokens[j].as_bytes();
            if tj.iter().all(|b| b.is_ascii_digit()) || tokens[j] == ":" {
                if blen + tj.len() > 16 {
                    break;
                }
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
            if i > 0
                && !tokens[i - 1]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit())
            {
                let prev = &tokens[i - 1];
                if !do_jump(prev, info) && prev != ":" {
                    res.tzname = Some(prev.clone());
                }
            }
            return j - i; // consume sign + all lookahead tokens
        }
    }
    if (token.starts_with('+') || token.starts_with('-')) && token.len() >= 3 {
        if let Some(offset) = parse_tzoffset(token) {
            res.tzoffset = Some(offset);
            return 1;
        }
    }

    // Known timezone abbreviation from parserinfo (e.g. EST → -18000)
    if let Some(offset) = do_tzoffset_lc(lc, info) {
        res.tzname = Some(token.clone());
        res.tzoffset = Some(offset);
        return 1;
    }

    // UTC zone
    if do_utczone_lc(lc, info) {
        res.tzname = Some("UTC".into());
        res.tzoffset = Some(0);
        return 1;
    }

    // Pertain word (e.g. "Sep of 03" — "03" is the year, not the day).
    // Checked before jump because "of" is in both sets.
    if do_pertain_lc(lc, info) {
        if ymd.count == 1 && ymd.mstridx.is_some() {
            // Look past intervening jump/whitespace tokens to find a number.
            let mut j = i + 1;
            while j < len && do_jump(&tokens[j], info) {
                j += 1;
            }
            if j < len {
                if let Some(yr) = fast_parse_int(&tokens[j]) {
                    ymd.ystridx = Some(ymd.count);
                    ymd.push(yr);
                    return j + 1 - i;
                }
            }
        }
        return 1;
    }

    // Jump word
    if do_jump_lc(lc, info) {
        return 1;
    }

    // Timezone name (alphabetic, not matched above)
    if token.chars().all(|c| c.is_alphabetic()) && res.tzname.is_none() && res.hour.is_some() {
        res.tzname = Some(token.clone());
        // Check next for offset
        if i + 1 < len && (tokens[i + 1].starts_with('+') || tokens[i + 1].starts_with('-')) {
            if let Some(offset) = parse_tzoffset(&tokens[i + 1]) {
                res.tzoffset = Some(offset);
                return 2; // tzname + offset token
            }
        }
        return 1;
    }

    0
}

#[inline]
fn try_parse_time_component(
    tokens: &[Cow<'_, str>],
    i: usize,
    len: usize,
    res: &mut ParseState<'_>,
    value: u32,
) -> usize {
    // Pattern: HH:MM or HH:MM:SS or HH:MM:SS.ffffff
    if res.hour.is_none() {
        res.hour = Some(value);
        let mut consumed = 1; // the hour number

        // Look for :MM — minutes are always integers, use fast path
        if i + 2 < len && tokens[i + 1] == ":" {
            if let Some(min) = fast_parse_int(&tokens[i + 2]) {
                res.minute = Some(min as u32);
                consumed = 3; // hour + ":" + minute
                              // Look for :SS — seconds may have fractional part
                if i + 4 < len && tokens[i + 3] == ":" {
                    if let Some(sec) = fast_parse_int(&tokens[i + 4]) {
                        // Pure integer seconds (fast path)
                        res.second = Some(sec as u32);
                        consumed = 5;
                    } else if let Some((sec, us)) = fast_parse_decimal(&tokens[i + 4]) {
                        // Fractional seconds (e.g. "45.123456") — integer arithmetic only
                        res.second = Some(sec as u32);
                        if us > 0 {
                            res.microsecond = Some(us);
                        }
                        consumed = 5;
                    }
                }
            } else {
                // Colon present but no valid minute — e.g. "1: test"
                res.malformed_time = true;
            }
        } else if i + 1 < len && tokens[i + 1] == ":" {
            // Trailing colon at end of input — also malformed.
            res.malformed_time = true;
        }
        return consumed;
    }
    0
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

/// Build a `NaiveDateTime` from a `ParseResult`, filling missing fields from
/// `default`.
pub fn build_naive(
    res: &ParseResult<'_>,
    default: NaiveDateTime,
) -> Result<NaiveDateTime, ParseError> {
    // If only a weekday was given (no day), advance the default date to the
    // next occurrence of that weekday (same day if it already matches).
    let default = if let (Some(target_wd), None) = (res.weekday, res.day) {
        let current_wd = default.weekday().num_days_from_monday() as i64;
        let days = ((target_wd as i64) - current_wd).rem_euclid(7);
        default + chrono::Duration::days(days)
    } else {
        default
    };

    let year = res.year.unwrap_or(default.year());
    let month = res.month.unwrap_or(default.month());
    // If no day was specified, clamp the default's day to the last day of the
    // effective year/month so e.g. default 2010-01-31 + "April 2009" yields
    // 2009-04-30 rather than an invalid 2009-04-31.
    let day = match res.day {
        Some(d) => d,
        None => default.day().min(crate::common::days_in_month(year, month)),
    };
    let hour = res.hour.unwrap_or(default.hour());
    let minute = res.minute.unwrap_or(default.minute());
    let second = res.second.unwrap_or(default.second());
    let microsecond = res.microsecond.unwrap_or(default.nanosecond() / 1000);

    let date = NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        ParseError::ValueError(format!("invalid date: {year}-{month}-{day}").into_boxed_str())
    })?;
    let time =
        NaiveTime::from_hms_micro_opt(hour, minute, second, microsecond).ok_or_else(|| {
            ParseError::ValueError(
                format!("invalid time: {hour}:{minute}:{second}.{microsecond}").into_boxed_str(),
            )
        })?;

    Ok(NaiveDateTime::new(date, time))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_parse_iso_basic() {
        let dt = parse("2024-01-15", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_datetime() {
        let dt = parse("2024-01-15 10:30:45", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }

    #[test]
    fn test_parse_us_format() {
        // MM/DD/YYYY (default, dayfirst=false)
        let dt = parse("01/15/2024", false, false, None, None).unwrap();
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_dayfirst() {
        // DD/MM/YYYY
        let dt = parse("15/01/2024", true, false, None, None).unwrap();
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.month(), 1);
    }

    #[test]
    fn test_parse_yearfirst() {
        let dt = parse("2024/01/15", false, true, None, None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_with_time_and_tz() {
        let res = parse_to_result("2024-01-15 10:30:45 UTC", false, false, None).unwrap();
        assert_eq!(res.hour, Some(10));
        assert_eq!(res.minute, Some(30));
        assert_eq!(res.second, Some(45));
        assert_eq!(res.tzname, Some("UTC".into()));
        assert_eq!(res.tzoffset, Some(0));
    }

    #[test]
    fn test_parse_ampm() {
        let dt = parse("January 15, 2024 3:30 PM", false, false, None, None).unwrap();
        assert_eq!(dt.hour(), 15);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_parse_microseconds() {
        let dt = parse("2024-01-15 10:30:45.123456", false, false, None, None).unwrap();
        assert_eq!(dt.second(), 45);
        assert_eq!(dt.nanosecond() / 1000, 123456);
    }

    #[test]
    fn test_parse_empty_string() {
        assert!(parse("", false, false, None, None).is_err());
    }

    #[test]
    fn test_parse_no_date() {
        assert!(parse("hello world", false, false, None, None).is_err());
    }

    #[test]
    fn test_time_only_no_date_leak() {
        let res = parse_to_result("3:30 PM", false, false, None).unwrap();
        assert_eq!(res.hour, Some(15));
        assert_eq!(res.minute, Some(30));
        assert!(
            res.day.is_none(),
            "minute '30' leaked into day: {:?}",
            res.day
        );
    }

    #[test]
    fn test_time_with_tz_no_date_leak() {
        let res = parse_to_result("10:30:45-05:00", false, false, None).unwrap();
        assert_eq!(res.hour, Some(10));
        assert_eq!(res.minute, Some(30));
        assert_eq!(res.second, Some(45));
        assert_eq!(res.tzoffset, Some(-(5 * 3600)));
        assert!(
            res.year.is_none(),
            "tz digits leaked into year: {:?}",
            res.year
        );
        assert!(
            res.month.is_none(),
            "tz digits leaked into month: {:?}",
            res.month
        );
    }

    #[test]
    fn test_date_separator_not_tz() {
        let res = parse_to_result("2024-01-15", false, false, None).unwrap();
        assert_eq!(res.year, Some(2024));
        assert_eq!(res.month, Some(1));
        assert_eq!(res.day, Some(15));
        assert!(
            res.tzoffset.is_none(),
            "date separator '-' set tzoffset: {:?}",
            res.tzoffset
        );
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

    #[test]
    fn test_parse_12am_midnight() {
        let dt = parse("January 15, 2024 12:00 AM", false, false, None, None).unwrap();
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
    }

    #[test]
    fn test_parse_12pm_noon() {
        let dt = parse("January 15, 2024 12:00 PM", false, false, None, None).unwrap();
        assert_eq!(dt.hour(), 12);
        assert_eq!(dt.minute(), 0);
    }

    #[test]
    fn test_parse_midnight_2359() {
        let dt = parse("2024-01-15 23:59:59", false, false, None, None).unwrap();
        assert_eq!(dt.hour(), 23);
        assert_eq!(dt.minute(), 59);
        assert_eq!(dt.second(), 59);
    }

    #[test]
    fn test_parse_whitespace_only() {
        assert!(parse("   ", false, false, None, None).is_err());
    }

    #[test]
    fn test_parse_gmt_timezone() {
        let res = parse_to_result("2024-01-15 10:30:00 GMT", false, false, None).unwrap();
        assert_eq!(res.tzname, Some("UTC".into()));
        assert_eq!(res.tzoffset, Some(0));
    }

    #[test]
    fn test_parse_z_timezone() {
        let res = parse_to_result("2024-01-15 10:30:00 Z", false, false, None).unwrap();
        assert_eq!(res.tzname, Some("UTC".into()));
        assert_eq!(res.tzoffset, Some(0));
    }

    #[test]
    fn test_parse_month_all_names() {
        let months = [
            ("January", 1),
            ("February", 2),
            ("March", 3),
            ("April", 4),
            ("May", 5),
            ("June", 6),
            ("July", 7),
            ("August", 8),
            ("September", 9),
            ("October", 10),
            ("November", 11),
            ("December", 12),
        ];
        for (name, expected) in months {
            let dt = parse(&format!("{name} 15, 2024"), false, false, None, None).unwrap();
            assert_eq!(dt.month(), expected, "Failed for {name}");
        }
    }

    #[test]
    fn test_parse_month_all_abbrevs() {
        let months = [
            ("Jan", 1),
            ("Feb", 2),
            ("Mar", 3),
            ("Apr", 4),
            ("May", 5),
            ("Jun", 6),
            ("Jul", 7),
            ("Aug", 8),
            ("Sep", 9),
            ("Oct", 10),
            ("Nov", 11),
            ("Dec", 12),
        ];
        for (name, expected) in months {
            let dt = parse(&format!("15 {name} 2024"), false, false, None, None).unwrap();
            assert_eq!(dt.month(), expected, "Failed for {name}");
        }
    }

    #[test]
    fn test_parse_sept_abbreviation() {
        let dt = parse("15 Sept 2024", false, false, None, None).unwrap();
        assert_eq!(dt.month(), 9);
    }

    #[test]
    fn test_parse_all_weekday_names() {
        let weekdays = [
            ("Monday", 0),
            ("Tuesday", 1),
            ("Wednesday", 2),
            ("Thursday", 3),
            ("Friday", 4),
            ("Saturday", 5),
            ("Sunday", 6),
        ];
        for (name, expected) in weekdays {
            let s = format!("{name}, January 15, 2024");
            let res = parse_to_result(&s, false, false, None).unwrap();
            assert_eq!(res.weekday, Some(expected), "Failed for {name}");
        }
    }

    #[test]
    fn test_parse_weekday_abbrev() {
        let weekdays = [
            ("Mon", 0),
            ("Tue", 1),
            ("Wed", 2),
            ("Thu", 3),
            ("Fri", 4),
            ("Sat", 5),
            ("Sun", 6),
        ];
        for (name, expected) in weekdays {
            let s = format!("{name}, January 15, 2024");
            let res = parse_to_result(&s, false, false, None).unwrap();
            assert_eq!(res.weekday, Some(expected), "Failed for {name}");
        }
    }

    #[test]
    fn test_parse_hms_word_after_date() {
        // HMS word after number with date context
        let res = parse_to_result("2024-01-15 10 hours", false, false, None).unwrap();
        assert_eq!(res.hour, Some(10));
    }

    #[test]
    fn test_parse_single_digit_values() {
        let dt = parse("2024-1-5", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 5);
    }

    #[test]
    fn test_parse_dayfirst_ambiguous() {
        // 05/06/2024 — dayfirst=true → June 5; dayfirst=false → May 6
        let dt_df = parse("05/06/2024", true, false, None, None).unwrap();
        assert_eq!(dt_df.day(), 5);
        assert_eq!(dt_df.month(), 6);

        let dt_mf = parse("05/06/2024", false, false, None, None).unwrap();
        assert_eq!(dt_mf.day(), 6);
        assert_eq!(dt_mf.month(), 5);
    }

    #[test]
    fn test_parse_yearfirst_ambiguous() {
        // 2024/05/06 yearfirst=true → May 6
        let dt = parse("2024/05/06", false, true, None, None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 5);
        assert_eq!(dt.day(), 6);
    }

    #[test]
    fn test_parse_tz_named_with_offset() {
        let res = parse_to_result("2024-01-15 10:30:00 EST-0500", false, false, None).unwrap();
        assert_eq!(res.tzname, Some("EST".into()));
        assert_eq!(res.tzoffset, Some(-(5 * 3600)));
    }

    #[test]
    fn test_parse_case_insensitive_month() {
        assert_eq!(lookup_month("JANUARY"), Some(1));
        assert_eq!(lookup_month("january"), Some(1));
        assert_eq!(lookup_month("JaN"), Some(1));
    }

    #[test]
    fn test_parse_case_insensitive_weekday() {
        assert_eq!(lookup_weekday("MONDAY"), Some(0));
        assert_eq!(lookup_weekday("monday"), Some(0));
        assert_eq!(lookup_weekday("MoN"), Some(0));
    }

    #[test]
    fn test_parse_case_insensitive_ampm() {
        assert_eq!(lookup_ampm("AM"), Some(0));
        assert_eq!(lookup_ampm("am"), Some(0));
        assert_eq!(lookup_ampm("PM"), Some(1));
        assert_eq!(lookup_ampm("pm"), Some(1));
        assert_eq!(lookup_ampm("A"), Some(0));
        assert_eq!(lookup_ampm("P"), Some(1));
    }

    #[test]
    fn test_parse_tzoffset_formats() {
        // Various valid offset formats
        assert_eq!(parse_tzoffset("+05:30"), Some(5 * 3600 + 30 * 60));
        assert_eq!(parse_tzoffset("-08:00"), Some(-(8 * 3600)));
        assert_eq!(parse_tzoffset("+0530"), Some(5 * 3600 + 30 * 60));
        assert_eq!(parse_tzoffset("-0800"), Some(-(8 * 3600)));
        assert_eq!(parse_tzoffset("+05"), Some(5 * 3600));
        assert_eq!(parse_tzoffset("-08"), Some(-(8 * 3600)));
        assert_eq!(parse_tzoffset("+00:00"), Some(0));
        assert_eq!(parse_tzoffset("+0000"), Some(0));
    }

    #[test]
    fn test_parse_tzoffset_invalid() {
        assert_eq!(parse_tzoffset("abc"), None);
        assert_eq!(parse_tzoffset(""), None);
        assert_eq!(parse_tzoffset("+"), None);
        assert_eq!(parse_tzoffset("+a"), None);
    }

    #[test]
    fn test_lowercase_buf_too_long() {
        // 17 characters — exceeds 16-byte buffer
        assert!(lowercase_buf("abcdefghijklmnopq").is_none());
    }

    #[test]
    fn test_lowercase_buf_non_ascii() {
        assert!(lowercase_buf("日本語").is_none());
    }

    #[test]
    fn test_lookup_jump_words() {
        assert!(lookup_jump(" "));
        assert!(lookup_jump("."));
        assert!(lookup_jump(","));
        assert!(lookup_jump("at"));
        assert!(lookup_jump("on"));
        assert!(lookup_jump("of"));
        assert!(!lookup_jump("foo"));
    }

    #[test]
    fn test_lookup_utczone() {
        assert!(lookup_utczone("UTC"));
        assert!(lookup_utczone("utc"));
        assert!(lookup_utczone("GMT"));
        assert!(lookup_utczone("gmt"));
        assert!(lookup_utczone("Z"));
        assert!(lookup_utczone("z"));
        assert!(!lookup_utczone("EST"));
    }

    #[test]
    fn test_parse_only_year() {
        // A 4-digit number alone should be treated as a year
        let res = parse_to_result("2024", false, false, None).unwrap();
        assert_eq!(res.year, Some(2024));
    }

    #[test]
    fn test_lookup_non_ascii_returns_false() {
        assert!(!lookup_jump("日本語"));
        assert!(!lookup_pertain("日本語"));
        assert!(!lookup_utczone("日本語"));
    }

    #[test]
    fn test_ymd_could_be_day_direct() {
        let mut ymd = Ymd::default();
        assert!(ymd.could_be_day(1));
        assert!(ymd.could_be_day(31));
        assert!(!ymd.could_be_day(0));
        assert!(!ymd.could_be_day(32));
        ymd.dstridx = Some(0);
        assert!(!ymd.could_be_day(15));
    }

    #[test]
    fn test_ymd_resolve_mstridx_len2_year() {
        let res = parse_to_result("Jan 2024", false, false, None).unwrap();
        assert_eq!(res.month, Some(1));
        assert_eq!(res.year, Some(2024));
        assert!(res.day.is_none());
    }

    #[test]
    fn test_ymd_resolve_mstridx_len2_day() {
        let res = parse_to_result("Jan 15", false, false, None).unwrap();
        assert_eq!(res.month, Some(1));
        assert_eq!(res.day, Some(15));
    }

    #[test]
    fn test_ymd_resolve_mstridx_len1() {
        let res = parse_to_result("January", false, false, None).unwrap();
        assert_eq!(res.month, Some(1));
        assert!(res.year.is_none());
        assert!(res.day.is_none());
    }

    #[test]
    fn test_ymd_resolve_no_mstridx_len1_day() {
        let res = parse_to_result("15", false, false, None).unwrap();
        assert_eq!(res.day, Some(15));
    }

    #[test]
    fn test_ymd_resolve_no_mstridx_len2_v0_gt31() {
        let res = parse_to_result("2024 01", false, false, None).unwrap();
        assert_eq!(res.year, Some(2024));
        assert_eq!(res.month, Some(1));
    }

    #[test]
    fn test_ymd_resolve_no_mstridx_len2_v1_gt31() {
        let res = parse_to_result("01 2024", false, false, None).unwrap();
        assert_eq!(res.month, Some(1));
        assert_eq!(res.year, Some(2024));
    }

    #[test]
    fn test_ymd_resolve_no_mstridx_len2_dayfirst() {
        let res = parse_to_result("15/06", true, false, None).unwrap();
        assert_eq!(res.day, Some(15));
        assert_eq!(res.month, Some(6));
    }

    #[test]
    fn test_ymd_resolve_no_mstridx_len2_monthfirst() {
        let res = parse_to_result("05/06", false, false, None).unwrap();
        assert_eq!(res.month, Some(5));
        assert_eq!(res.day, Some(6));
    }

    #[test]
    fn test_ymd_resolve_mstridx_len3_mi2() {
        let dt = parse("15 20 Jan", false, false, None, None).unwrap();
        assert_eq!(dt.month(), 1);
    }

    #[test]
    fn test_ymd_resolve_mstridx_len3_va_gt31() {
        let dt = parse("2024 Jan 15", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_ymd_resolve_mstridx_len3_dayfirst() {
        let dt = parse("15 Jan 20", true, false, None, None).unwrap();
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.year(), 2020);
    }

    #[test]
    fn test_ymd_resolve_mstridx_len3_not_dayfirst() {
        let dt = parse("15 Jan 20", false, false, None, None).unwrap();
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.year(), 2020);
    }

    #[test]
    fn test_convertyear_high_two_digit() {
        let dt = parse("Jan 15 99", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 1999);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_hms_words_no_space() {
        let res =
            parse_to_result("2024-01-15 10hours30minutes45.5seconds", false, false, None).unwrap();
        assert_eq!(res.hour, Some(10));
        assert_eq!(res.minute, Some(30));
        assert_eq!(res.second, Some(45));
        assert_eq!(res.microsecond, Some(500000));
    }

    #[test]
    fn test_parse_decimal_seconds_postfact() {
        let res = parse_to_result("10:30:45 0.5", false, false, None).unwrap();
        assert_eq!(res.second, Some(45));
        assert_eq!(res.microsecond, Some(500000));
    }

    #[test]
    fn test_parse_ymd_full_invalid_hour() {
        let res = parse_to_result("2024-01-15 25", false, false, None).unwrap();
        assert_eq!(res.year, Some(2024));
        assert!(res.hour.is_none());
    }

    #[test]
    fn test_parse_tz_offset_break_on_alpha() {
        let res = parse_to_result("10:30:00 +05abc", false, false, None).unwrap();
        assert_eq!(res.tzoffset, Some(5 * 3600));
    }

    #[test]
    fn test_parse_time_hour_already_set_colon() {
        let res = parse_to_result("10:30 5:00", false, false, None).unwrap();
        assert_eq!(res.hour, Some(10));
        assert_eq!(res.minute, Some(30));
    }

    #[test]
    fn test_parse_invalid_date() {
        assert!(parse("2024-02-30", false, false, None, None).is_err());
    }

    #[test]
    fn test_parse_invalid_time() {
        assert!(parse("2024-01-15 99:00:00", false, false, None, None).is_err());
    }

    // ==== Edge case tests ====

    // ---- Year boundary cases ----

    #[test]
    fn test_parse_year_1() {
        let dt = parse("0001-01-01", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 1);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
    }

    #[test]
    fn test_parse_year_9999() {
        let dt = parse("9999-12-31", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 9999);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 31);
    }

    // ---- dayfirst + yearfirst combined ----

    #[test]
    fn test_parse_dayfirst_and_yearfirst_both_true() {
        let dt = parse("2024/05/06", true, true, None, None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 5);
        assert_eq!(dt.day(), 6);
    }

    // ---- 2-digit year conversion ----

    #[test]
    fn test_parse_two_digit_year_00() {
        let dt = parse("01/15/00", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 2000);
    }

    // ---- Leap year edge cases ----

    #[test]
    fn test_parse_feb29_leap_year() {
        let dt = parse("February 29, 2024", false, false, None, None).unwrap();
        assert_eq!(dt.month(), 2);
        assert_eq!(dt.day(), 29);
    }

    #[test]
    fn test_parse_feb29_non_leap_year_error() {
        assert!(parse("February 29, 2023", false, false, None, None).is_err());
    }

    #[test]
    fn test_parse_feb29_century_non_leap() {
        assert!(parse("February 29, 1900", false, false, None, None).is_err());
    }

    #[test]
    fn test_parse_feb29_century_leap() {
        let dt = parse("February 29, 2000", false, false, None, None).unwrap();
        assert_eq!(dt.day(), 29);
    }

    // ---- Extreme timezone offsets ----

    #[test]
    fn test_parse_tz_offset_max_plus_14() {
        let res = parse_to_result("2024-01-15 10:30:00+14:00", false, false, None).unwrap();
        assert_eq!(res.tzoffset, Some(50400));
    }

    #[test]
    fn test_parse_tz_offset_max_minus_12() {
        let res = parse_to_result("2024-01-15 10:30:00-12:00", false, false, None).unwrap();
        assert_eq!(res.tzoffset, Some(-43200));
    }

    // ---- Non-ASCII and special inputs ----

    #[test]
    fn test_parse_only_digits_and_separators() {
        // All-numeric input with various separators should not panic
        let result = parse("2024!01!15", false, false, None, None);
        let _ = result; // may fail but should not panic
    }

    // ---- Month day boundaries ----

    #[test]
    fn test_parse_jan31() {
        let dt = parse("January 31, 2024", false, false, None, None).unwrap();
        assert_eq!(dt.day(), 31);
    }

    #[test]
    fn test_parse_apr30() {
        let dt = parse("April 30, 2024", false, false, None, None).unwrap();
        assert_eq!(dt.day(), 30);
    }

    #[test]
    fn test_parse_apr31_invalid() {
        assert!(parse("April 31, 2024", false, false, None, None).is_err());
    }

    #[test]
    fn test_parse_jun30_valid() {
        let dt = parse("June 30, 2024", false, false, None, None).unwrap();
        assert_eq!(dt.day(), 30);
    }

    #[test]
    fn test_parse_jun31_invalid() {
        assert!(parse("June 31, 2024", false, false, None, None).is_err());
    }

    // ---- Time edge cases ----

    #[test]
    fn test_parse_midnight_0000() {
        let dt = parse("2024-01-15 00:00:00", false, false, None, None).unwrap();
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
        assert_eq!(dt.second(), 0);
    }

    // ---- Fractional seconds precision ----

    #[test]
    fn test_parse_fractional_1_digit() {
        let dt = parse("2024-01-15 10:30:45.1", false, false, None, None).unwrap();
        assert_eq!(dt.nanosecond() / 1000, 100_000);
    }

    // ---- Only time ----

    #[test]
    fn test_parse_time_only_no_panic() {
        let _ = parse("10:30:45", false, false, None, None);
    }

    // ---- parse_with_default ----

    #[test]
    fn test_parse_with_default_fills_date() {
        let default = NaiveDate::from_ymd_opt(2020, 6, 15)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let dt = parse("10:30", false, false, Some(default), None).unwrap();
        assert_eq!(dt.year(), 2020);
        assert_eq!(dt.month(), 6);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn test_parse_with_default_fills_time() {
        let default = NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(14, 30, 45)
            .unwrap();
        let dt = parse("2024-03-20", false, false, Some(default), None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 20);
        // Time from default
        assert_eq!(dt.hour(), 14);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }

    #[test]
    fn test_parse_with_default_full_override() {
        let default = NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let dt = parse("2024-03-20 10:30:45", false, false, Some(default), None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 20);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }

    #[test]
    fn test_parse_with_default_year_only() {
        let default = NaiveDate::from_ymd_opt(2020, 6, 15)
            .unwrap()
            .and_hms_opt(8, 0, 0)
            .unwrap();
        // Parsing just a month-day should fill year from default
        let dt = parse("March 10", false, false, Some(default), None).unwrap();
        assert_eq!(dt.year(), 2020);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 10);
        assert_eq!(dt.hour(), 8);
    }

    // ---- Coverage: compact parsing (try_parse_compact) ----

    #[test]
    fn test_parse_compact_yyyymmdd() {
        let dt = parse("20240315", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_compact_yyyymmddt_hhmmss() {
        // YYYYMMDD + T separator + HHMMSS (6-digit time after date)
        let dt = parse("20240115T103045", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }

    #[test]
    fn test_parse_compact_yymmdd() {
        let default = NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let dt = parse("240315", false, false, Some(default), None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_compact_yyyymm() {
        // "202403" → YYMMDD (20/24/03) fails because month=24 > 12
        // → YYYYMM fallback: year=2024, month=03
        let dt = parse("202403", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
    }

    #[test]
    fn test_parse_compact_hhmmss_after_date() {
        // Date then T then HHMMSS
        let dt = parse("2024-03-15T103045", false, false, None, None).unwrap();
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }

    #[test]
    fn test_parse_compact_yyyymmddhh() {
        let dt = parse("2024031510", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
    }

    #[test]
    fn test_parse_compact_yyyymmddhh_with_minutes() {
        let dt = parse("2024031510:30", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_parse_compact_yyyymmddhh_with_seconds() {
        let dt = parse("2024031510:30:45", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }

    #[test]
    fn test_parse_compact_yyyymmddhh_with_decimal_seconds() {
        let dt = parse("2024031510:30:45.5", false, false, None, None).unwrap();
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }

    // ---- Coverage: dot-separated date (try_parse_dot_date) ----

    #[test]
    fn test_parse_dot_separated_date() {
        let dt = parse("2003.09.25", false, false, None, None).unwrap();
        assert_eq!(dt.year(), 2003);
        assert_eq!(dt.month(), 9);
        assert_eq!(dt.day(), 25);
    }

    #[test]
    fn test_parse_dot_separated_date_dmy() {
        let dt = parse("25.09.2003", true, false, None, None).unwrap();
        assert_eq!(dt.year(), 2003);
        assert_eq!(dt.month(), 9);
        assert_eq!(dt.day(), 25);
    }

    // ---- Coverage: timezone parsing ----

    #[test]
    fn test_parse_tz_offset_single_token() {
        let res = parse_to_result("2024-01-15 10:30:45 +0500", false, false, None).unwrap();
        assert_eq!(res.tzoffset, Some(18000));
    }

    #[test]
    fn test_parse_tz_name_with_offset() {
        let res = parse_to_result("2024-01-15 10:30:45 EST -0500", false, false, None).unwrap();
        assert_eq!(res.tzname.as_deref(), Some("EST"));
        assert_eq!(res.tzoffset, Some(-18000));
    }

    #[test]
    fn test_parse_tz_with_parserinfo() {
        let mut info = ParserInfo::default();
        info.tzoffset.insert("est".into(), -18000);
        let res = parse_to_result("2024-01-15 10:30:45 EST", false, false, Some(&info)).unwrap();
        assert_eq!(res.tzname.as_deref(), Some("EST"));
        assert_eq!(res.tzoffset, Some(-18000));
    }

    // ---- Coverage: pertain word ----

    #[test]
    fn test_parse_pertain_of() {
        let dt = parse("15 of January 2024", false, false, None, None).unwrap();
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    // ---- Coverage: HMS assign with microseconds ----

    #[test]
    fn test_parse_hms_label_no_space() {
        // "10h30m45s" — HMS labels immediately after numbers
        let res = parse_to_result("10h30m45s", false, false, None).unwrap();
        assert_eq!(res.hour, Some(10));
        assert_eq!(res.minute, Some(30));
        assert_eq!(res.second, Some(45));
    }

    // ---- Coverage: convertyear edge cases ----

    #[test]
    fn test_parse_two_digit_year_old() {
        // Two-digit year far in the past gets + 100
        let default = NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let dt = parse("01/15/70", false, false, Some(default), None).unwrap();
        // 70 should map to 1970 (2070 - 100 = 1970... or 2070 if < now+50)
        assert!(dt.year() == 1970 || dt.year() == 2070);
    }

    // ---- Coverage: fast_parse_int / fast_parse_decimal edge cases ----

    #[test]
    fn test_fast_parse_int_empty() {
        assert_eq!(fast_parse_int(""), None);
    }

    #[test]
    fn test_fast_parse_int_overflow() {
        // 10-digit number exceeding i32::MAX must return None, not panic
        assert_eq!(fast_parse_int("9999999999"), None);
        assert_eq!(fast_parse_int("99999999999999"), None);
        // i32::MAX boundary
        assert_eq!(fast_parse_int("2147483647"), Some(i32::MAX));
        assert_eq!(fast_parse_int("2147483648"), None);
    }

    #[test]
    fn test_fast_parse_decimal_empty_frac() {
        // "10." — dot at end, empty frac part → (10, 0)
        assert_eq!(fast_parse_decimal("10."), Some((10, 0)));
    }

    // ---- Coverage: malformed colon-time rejection ----

    #[test]
    fn test_parse_malformed_colon_time_word() {
        // "1: test" — colon with non-numeric after must raise
        let r = parse("1: test", false, false, None, None);
        assert!(r.is_err());
    }

    #[test]
    fn test_parse_malformed_colon_time_no_space() {
        // "1:test" — same, without the space
        let r = parse("1:test", false, false, None, None);
        assert!(r.is_err());
    }

    #[test]
    fn test_parse_trailing_colon() {
        // "2024-01-15 10:" — bare trailing colon at end of input
        let r = parse("2024-01-15 10:", false, false, None, None);
        assert!(r.is_err());
    }

    #[test]
    fn test_parse_valid_colon_time_still_parses() {
        // Sanity: valid colon-time still works after malformed guard
        let res = parse_to_result("10:30", false, false, None).unwrap();
        assert_eq!(res.hour, Some(10));
        assert_eq!(res.minute, Some(30));
    }

    // ---- Coverage: AM/PM validation ----

    #[test]
    fn test_parse_ampm_no_hour_alone() {
        // Bare "AM" — NoDate (no fields at all), still raises
        assert!(parse("AM", false, false, None, None).is_err());
    }

    #[test]
    fn test_parse_ampm_no_hour_with_date() {
        // "Jan 20, 2015 PM" — PM without any hour raises
        assert!(parse("Jan 20, 2015 PM", false, false, None, None).is_err());
    }

    #[test]
    fn test_parse_ampm_out_of_range_word() {
        // "13:44 AM" — hour 13 is invalid for 12-hour clock
        assert!(parse("13:44 AM", false, false, None, None).is_err());
    }

    #[test]
    fn test_parse_ampm_out_of_range_adjacent_pm() {
        // "23:13 PM" — hour 23 is invalid for 12-hour clock
        assert!(parse("January 25, 1921 23:13 PM", false, false, None, None).is_err());
    }

    #[test]
    fn test_parse_hour_only_ampm() {
        // "10am" — AM-adjacent hour in 12-hour range
        let res = parse_to_result("10am", false, false, None).unwrap();
        assert_eq!(res.hour, Some(10));
    }

    #[test]
    fn test_parse_hour_only_pm_converts() {
        // "10pm" → hour=22
        let res = parse_to_result("10pm", false, false, None).unwrap();
        assert_eq!(res.hour, Some(22));
    }

    #[test]
    fn test_parse_12am_converts_to_zero() {
        let res = parse_to_result("12am", false, false, None).unwrap();
        assert_eq!(res.hour, Some(0));
    }

    #[test]
    fn test_parse_ampm_adjacent_out_of_range() {
        // "13am" — bare number adjacent to AM, hour > 12
        assert!(parse("13am", false, false, None, None).is_err());
    }

    // ---- Coverage: HMS compound with whitespace ----

    #[test]
    fn test_parse_hms_label_with_space_hour() {
        // "10 h 36" — space between number and HMS label
        let res = parse_to_result("10 h 36", false, false, None).unwrap();
        assert_eq!(res.hour, Some(10));
        assert_eq!(res.minute, Some(36));
    }

    #[test]
    fn test_parse_hms_label_with_space_fractional() {
        // "10 h 36.5" — continuation with fractional minute → carry seconds
        let res = parse_to_result("10 h 36.5", false, false, None).unwrap();
        assert_eq!(res.hour, Some(10));
        assert_eq!(res.minute, Some(36));
        assert_eq!(res.second, Some(30));
    }

    #[test]
    fn test_parse_hms_minute_label_continuation() {
        // "36 m 05 s" — minute label followed by second label, both spaced
        let res = parse_to_result("36 m 05 s", false, false, None).unwrap();
        assert_eq!(res.minute, Some(36));
        assert_eq!(res.second, Some(5));
    }

    #[test]
    fn test_parse_hms_continuation_bare_number() {
        // "01h02" — bare number after "01h" becomes minute (next unit)
        let res = parse_to_result("01h02", false, false, None).unwrap();
        assert_eq!(res.hour, Some(1));
        assert_eq!(res.minute, Some(2));
    }

    #[test]
    fn test_parse_hms_continuation_chain() {
        // "01h02m03" — cascade: hour via "h", minute via "m", bare 03 → second
        let res = parse_to_result("01h02m03", false, false, None).unwrap();
        assert_eq!(res.hour, Some(1));
        assert_eq!(res.minute, Some(2));
        assert_eq!(res.second, Some(3));
    }

    #[test]
    fn test_parse_hms_minute_continuation_to_second() {
        // "01m02" — bare number after "01m" becomes second
        let res = parse_to_result("01m02", false, false, None).unwrap();
        assert_eq!(res.minute, Some(1));
        assert_eq!(res.second, Some(2));
    }

    #[test]
    fn test_parse_fractional_hour_carries_minutes() {
        // "5.6h" — 0.6 h = 36 min
        let res = parse_to_result("5.6h", false, false, None).unwrap();
        assert_eq!(res.hour, Some(5));
        assert_eq!(res.minute, Some(36));
    }

    #[test]
    fn test_parse_fractional_minute_carries_seconds() {
        // "5.6m" — 0.6 min = 36 s
        let res = parse_to_result("5.6m", false, false, None).unwrap();
        assert_eq!(res.minute, Some(5));
        assert_eq!(res.second, Some(36));
    }

    // ---- Coverage: pertain + ystridx resolver ----

    #[test]
    fn test_parse_pertain_short_year() {
        // "Sep of 03" — "03" is year via pertain, not day
        let res = parse_to_result("Sep of 03", false, false, None).unwrap();
        assert_eq!(res.month, Some(9));
        assert_eq!(res.year, Some(2003));
        assert_eq!(res.day, None);
    }

    #[test]
    fn test_parse_pertain_full_year() {
        let res = parse_to_result("Sep of 2020", false, false, None).unwrap();
        assert_eq!(res.month, Some(9));
        assert_eq!(res.year, Some(2020));
    }

    #[test]
    fn test_parse_ystridx_overrides_positional_default() {
        // "0031 Nov 03" — 4-digit "0031" forces ystridx=0; year=31, day=3
        let res = parse_to_result("0031 Nov 03", false, false, None).unwrap();
        assert_eq!(res.year, Some(31));
        assert_eq!(res.month, Some(11));
        assert_eq!(res.day, Some(3));
    }

    #[test]
    fn test_parse_zero_year_via_mi1() {
        // "31-Dec-00" — both va=31 and vb=0 are ≤31; mi=1 default picks
        // day=31, year=0 → convertyear → 2000.
        let res = parse_to_result("31-Dec-00", false, false, None).unwrap();
        assert_eq!(res.day, Some(31));
        assert_eq!(res.month, Some(12));
        assert_eq!(res.year, Some(2000));
    }

    #[test]
    fn test_parse_weekday_alone_advances_default() {
        // Default 2003-09-25 is Thursday; "Wed" advances to 2003-10-01.
        let default = NaiveDate::from_ymd_opt(2003, 9, 25)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let dt = parse("Wed", false, false, Some(default), None).unwrap();
        assert_eq!(dt.year(), 2003);
        assert_eq!(dt.month(), 10);
        assert_eq!(dt.day(), 1);
    }

    #[test]
    fn test_parse_weekday_alone_same_day() {
        // Default Thursday + "Thu" keeps the same day.
        let default = NaiveDate::from_ymd_opt(2003, 9, 25)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let dt = parse("Thu", false, false, Some(default), None).unwrap();
        assert_eq!(dt.day(), 25);
    }

    // ---- Coverage: day clamp when unspecified ----

    #[test]
    fn test_parse_unspecified_day_clamped_to_month_end() {
        // "April 2009" with default day=31 → clamp to 30.
        let default = NaiveDate::from_ymd_opt(2010, 1, 31)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let dt = parse("April 2009", false, false, Some(default), None).unwrap();
        assert_eq!(dt.year(), 2009);
        assert_eq!(dt.month(), 4);
        assert_eq!(dt.day(), 30);
    }

    #[test]
    fn test_parse_unspecified_day_feb_non_leap() {
        let default = NaiveDate::from_ymd_opt(2010, 1, 31)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let dt = parse("Feb 2007", false, false, Some(default), None).unwrap();
        assert_eq!(dt.day(), 28);
    }

    #[test]
    fn test_parse_unspecified_day_feb_leap() {
        let default = NaiveDate::from_ymd_opt(2010, 1, 31)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let dt = parse("Feb 2008", false, false, Some(default), None).unwrap();
        assert_eq!(dt.day(), 29);
    }

    // ---- Coverage: compact ISO overflow (12/14 digit no-separator) ----

    #[test]
    fn test_parse_compact_12_digit() {
        // "199709020908" — YYYYMMDDHHMM without separators (value overflows i32)
        let res = parse_to_result("199709020908", false, false, None).unwrap();
        assert_eq!(res.year, Some(1997));
        assert_eq!(res.month, Some(9));
        assert_eq!(res.day, Some(2));
        assert_eq!(res.hour, Some(9));
        assert_eq!(res.minute, Some(8));
    }

    #[test]
    fn test_parse_compact_14_digit() {
        // "19970902090807" — YYYYMMDDHHMMSS without separators
        let res = parse_to_result("19970902090807", false, false, None).unwrap();
        assert_eq!(res.hour, Some(9));
        assert_eq!(res.minute, Some(8));
        assert_eq!(res.second, Some(7));
    }

    #[test]
    fn test_parse_compact_iso_t_hhmm() {
        // "20030925T1049" — 8-digit date, T, 4-digit HHMM
        let res = parse_to_result("20030925T1049", false, false, None).unwrap();
        assert_eq!(res.hour, Some(10));
        assert_eq!(res.minute, Some(49));
        assert_eq!(res.second, None);
    }

    #[test]
    fn test_parse_compact_iso_fractional_seconds_with_offset() {
        // "20030925T104941.5-0300" — fractional seconds immediately before tz
        let res = parse_to_result("20030925T104941.5-0300", false, false, None).unwrap();
        assert_eq!(res.hour, Some(10));
        assert_eq!(res.minute, Some(49));
        assert_eq!(res.second, Some(41));
        assert_eq!(res.microsecond, Some(500_000));
        assert_eq!(res.tzoffset, Some(-10800));
    }
}
