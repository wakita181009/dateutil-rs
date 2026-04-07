use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};

/// A DST transition date specification (POSIX TZ string format).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateRule {
    /// `Mm.w.d` — The d-th day (0=Sunday) of week w (1–5, 5=last) of month m (1–12).
    MonthWeekDay { month: u32, week: u32, weekday: u32 },
    /// `Jn` — Julian day (1–365, February 29 is never counted).
    JulianNoLeap(u32),
    /// `n` — Zero-based Julian day (0–365, February 29 is counted in leap years).
    JulianLeap(u32),
}

/// A DST transition rule: a date rule plus a time-of-day offset (seconds from midnight).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionRule {
    pub date_rule: DateRule,
    /// Seconds from midnight for the transition (default: 7200 = 02:00).
    pub time: i32,
}

impl TransitionRule {
    pub fn new(date_rule: DateRule, time: i32) -> Self {
        TransitionRule { date_rule, time }
    }

    /// Compute the wall-clock datetime of this transition in the given year.
    pub fn datetime_for_year(&self, year: i32) -> NaiveDateTime {
        let date = match &self.date_rule {
            DateRule::MonthWeekDay {
                month,
                week,
                weekday,
            } => nth_weekday_of_month(year, *month, *weekday, *week),
            DateRule::JulianNoLeap(jday) => julian_no_leap_to_date(year, *jday),
            DateRule::JulianLeap(jday) => julian_leap_to_date(year, *jday),
        };
        let time_secs = self.time;
        let hours = time_secs / 3600;
        let mins = (time_secs % 3600) / 60;
        let secs = time_secs % 60;
        if (0..24).contains(&hours) && mins >= 0 && secs >= 0 {
            date.and_time(
                NaiveTime::from_hms_opt(hours as u32, mins as u32, secs as u32)
                    .unwrap_or(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
            )
        } else {
            // Handle times >= 24:00 or negative by adding duration to midnight
            date.and_hms_opt(0, 0, 0).unwrap() + Duration::seconds(time_secs as i64)
        }
    }
}

/// Default transition rules (US standard before 2007):
/// DST starts: 2nd Sunday of March at 02:00
/// DST ends:   1st Sunday of November at 02:00
pub fn default_start_rule() -> TransitionRule {
    TransitionRule::new(
        DateRule::MonthWeekDay {
            month: 3,
            week: 2,
            weekday: 0,
        },
        7200,
    )
}

pub fn default_end_rule() -> TransitionRule {
    TransitionRule::new(
        DateRule::MonthWeekDay {
            month: 11,
            week: 1,
            weekday: 0,
        },
        7200,
    )
}

/// Compute the n-th occurrence of a weekday in a month.
/// weekday: 0=Sunday, 6=Saturday. week: 1–4 for specific, 5 for last.
fn nth_weekday_of_month(year: i32, month: u32, weekday: u32, week: u32) -> NaiveDate {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    // chrono: num_days_from_sunday() => 0=Sunday
    let first_wd = first.weekday().num_days_from_sunday();
    let diff = ((weekday as i32 - first_wd as i32) + 7) % 7;

    if week == 5 {
        // Last occurrence in the month
        let last_day_of_month = last_day_of_month(year, month);
        let last = NaiveDate::from_ymd_opt(year, month, last_day_of_month).unwrap();
        let last_wd = last.weekday().num_days_from_sunday();
        let diff_back = ((last_wd as i32 - weekday as i32) + 7) % 7;
        NaiveDate::from_ymd_opt(year, month, last_day_of_month - diff_back as u32).unwrap()
    } else {
        let day = 1 + diff as u32 + (week - 1) * 7;
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .unwrap()
    .pred_opt()
    .unwrap()
    .day()
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Convert a Julian day (1–365, no Feb 29) to a NaiveDate.
fn julian_no_leap_to_date(year: i32, jday: u32) -> NaiveDate {
    // Julian day 1 = Jan 1, Feb 29 is never counted.
    // In leap years, dates after Feb 28 (jday >= 60) need their ordinal
    // shifted by 1 to account for Feb 29 in the actual calendar.
    let ordinal = if is_leap_year(year) && jday >= 60 {
        jday + 1
    } else {
        jday
    };
    NaiveDate::from_yo_opt(year, ordinal).unwrap()
}

/// Convert a zero-based Julian day (0–365) to a NaiveDate.
fn julian_leap_to_date(year: i32, jday: u32) -> NaiveDate {
    // Day 0 = Jan 1
    NaiveDate::from_yo_opt(year, jday + 1).unwrap()
}

// ============================================================================
// TzRange — Annual DST transitions
// ============================================================================

/// Timezone with annual DST transitions.
///
/// Mirrors python-dateutil's `tzrange`: fixed standard/DST offsets with
/// rule-based annual transitions.
#[derive(Debug, Clone)]
pub struct TzRange {
    pub std_abbr: String,
    pub dst_abbr: Option<String>,
    pub std_offset: Duration,
    pub dst_offset: Duration,
    pub start: Option<TransitionRule>,
    pub end: Option<TransitionRule>,
    pub hasdst: bool,
}

impl TzRange {
    /// Create a new TzRange.
    ///
    /// - `std_abbr`: standard timezone abbreviation (e.g. "EST")
    /// - `std_offset`: standard UTC offset (default: 0)
    /// - `dst_abbr`: DST abbreviation (e.g. "EDT"); if `Some`, DST is enabled
    /// - `dst_offset`: DST UTC offset (default: std_offset + 1 hour)
    /// - `start`/`end`: DST transition rules (defaults to US rules)
    pub fn new(
        std_abbr: String,
        std_offset: Option<Duration>,
        dst_abbr: Option<String>,
        dst_offset: Option<Duration>,
        start: Option<TransitionRule>,
        end: Option<TransitionRule>,
    ) -> Self {
        let std_offset = std_offset.unwrap_or_else(Duration::zero);
        let hasdst = dst_abbr.is_some();
        let dst_offset = dst_offset.unwrap_or(std_offset + Duration::hours(1));
        let start = if hasdst {
            Some(start.unwrap_or_else(default_start_rule))
        } else {
            None
        };
        let end = if hasdst {
            Some(end.unwrap_or_else(default_end_rule))
        } else {
            None
        };

        TzRange {
            std_abbr,
            dst_abbr,
            std_offset,
            dst_offset,
            start,
            end,
            hasdst,
        }
    }

    /// Return (dston, dstoff) wall-clock datetimes for the given year,
    /// or None if no DST.
    pub fn transitions(&self, year: i32) -> Option<(NaiveDateTime, NaiveDateTime)> {
        if !self.hasdst {
            return None;
        }
        let start = self.start.as_ref()?.datetime_for_year(year);
        let end = self.end.as_ref()?.datetime_for_year(year);
        Some((start, end))
    }

    /// The DST base offset (difference between DST and standard).
    pub fn dst_base_offset(&self) -> Duration {
        self.dst_offset - self.std_offset
    }

    pub fn utcoffset(&self, dt: Option<NaiveDateTime>, fold: bool) -> Option<Duration> {
        let dt = dt?;
        if self.hasdst && self.isdst(dt, fold) {
            Some(self.dst_offset)
        } else {
            Some(self.std_offset)
        }
    }

    pub fn dst(&self, dt: Option<NaiveDateTime>, fold: bool) -> Option<Duration> {
        let dt = dt?;
        if self.hasdst && self.isdst(dt, fold) {
            Some(self.dst_base_offset())
        } else {
            Some(Duration::zero())
        }
    }

    pub fn tzname(&self, dt: Option<NaiveDateTime>, fold: bool) -> Option<String> {
        let dt = dt?;
        if self.hasdst && self.isdst(dt, fold) {
            self.dst_abbr.clone()
        } else {
            Some(self.std_abbr.clone())
        }
    }

    pub fn is_ambiguous(&self, dt: NaiveDateTime) -> bool {
        if !self.hasdst {
            return false;
        }
        if let Some((dston, dstoff)) = self.transitions(dt.date().year()) {
            let dst_diff = self.dst_base_offset();
            if dst_diff.num_seconds() > 0 {
                // Normal DST: spring forward, fall back
                // At fall-back, clocks go from dstoff back by dst_diff.
                // Ambiguous range: [dstoff - dst_diff, dstoff)
                dstoff - dst_diff <= dt && dt < dstoff
            } else {
                // Negative DST (rare): ambiguous during spring transition
                // dst_diff < 0, so dston + dst_diff < dston
                dston + dst_diff <= dt && dt < dston
            }
        } else {
            false
        }
    }

    pub fn fromutc(&self, dt: NaiveDateTime) -> (NaiveDateTime, bool) {
        if !self.hasdst {
            return (dt + self.std_offset, false);
        }
        // Try standard offset first
        let wall_std = dt + self.std_offset;
        let wall_dst = dt + self.dst_offset;

        if let Some((dston, dstoff)) = self.transitions(dt.date().year()) {
            let in_dst = self.naive_isdst(wall_std, dston, dstoff);
            let wall = if in_dst { wall_dst } else { wall_std };
            let fold = self.is_ambiguous(wall);
            (wall, fold)
        } else {
            (wall_std, false)
        }
    }

    fn isdst(&self, dt: NaiveDateTime, fold: bool) -> bool {
        if let Some((dston, dstoff)) = self.transitions(dt.date().year()) {
            let dst_diff = self.dst_base_offset();
            if dst_diff.num_seconds() > 0 {
                // Check ambiguity: fall-back overlap [dstoff - dst_diff, dstoff)
                if dstoff - dst_diff <= dt && dt < dstoff {
                    // In ambiguous period — fold=1 means second occurrence (standard time)
                    return !fold;
                }
                // Check gap: spring-forward gap [dston, dston + dst_diff)
                if dston <= dt && dt < dston + dst_diff {
                    // In gap — fold=1 means DST
                    return fold;
                }
            }
            self.naive_isdst(dt, dston, dstoff)
        } else {
            false
        }
    }

    fn naive_isdst(&self, dt: NaiveDateTime, dston: NaiveDateTime, dstoff: NaiveDateTime) -> bool {
        if dston < dstoff {
            // Northern hemisphere: DST starts before it ends in calendar order
            dston <= dt && dt < dstoff
        } else {
            // Southern hemisphere: DST crosses year boundary
            !(dstoff <= dt && dt < dston)
        }
    }
}

impl std::fmt::Display for TzRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "tzrange({}, {}, {}, {})",
            self.std_abbr,
            self.std_offset.num_seconds(),
            self.dst_abbr.as_deref().unwrap_or("None"),
            self.dst_offset.num_seconds()
        )
    }
}

// ============================================================================
// TzStr — POSIX TZ string parser
// ============================================================================

/// Timezone parsed from a POSIX TZ environment variable string.
///
/// Supports formats like:
/// - `EST5EDT,M3.2.0/2,M11.1.0/2`
/// - `CET-1CEST,M3.5.0,M10.5.0/3`
/// - `<+09>-9`
#[derive(Debug, Clone)]
pub struct TzStr {
    inner: TzRange,
    source: String,
    _posix_offset: bool,
}

impl TzStr {
    /// Parse a POSIX TZ string.
    ///
    /// If `posix_offset` is false (default), offset signs are inverted to match
    /// the ISO convention (positive = east of UTC). POSIX convention is the
    /// opposite: positive = west of UTC.
    pub fn parse(s: &str, posix_offset: bool) -> Result<Self, TzStrError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(TzStrError::Empty);
        }

        let mut parser = TzStrParser::new(trimmed);

        // Parse standard abbreviation and offset
        let std_abbr = parser.parse_abbr()?;
        let std_offset_raw = parser.parse_offset()?;
        let std_offset = if posix_offset {
            std_offset_raw
        } else {
            -std_offset_raw
        };

        // Check for DST part
        if parser.is_empty() {
            // No DST
            let inner = TzRange::new(
                std_abbr,
                Some(Duration::seconds(std_offset)),
                None,
                None,
                None,
                None,
            );
            return Ok(TzStr {
                inner,
                source: s.to_string(),
                _posix_offset: posix_offset,
            });
        }

        // Parse DST abbreviation
        let dst_abbr = parser.parse_abbr()?;

        // Parse optional DST offset
        let dst_offset = if parser.peek_char().is_some_and(|c| {
            c == '-' || c == '+' || c.is_ascii_digit()
        }) {
            let raw = parser.parse_offset()?;
            if posix_offset { raw } else { -raw }
        } else {
            std_offset + 3600 // Default: 1 hour ahead of standard
        };

        // Parse optional transition rules
        let (start, end) = if parser.consume_char(',') {
            let start = parser.parse_rule()?;
            if !parser.consume_char(',') {
                return Err(TzStrError::MissingEndRule);
            }
            let end = parser.parse_rule()?;
            (Some(start), Some(end))
        } else {
            (None, None)
        };

        let inner = TzRange::new(
            std_abbr,
            Some(Duration::seconds(std_offset)),
            Some(dst_abbr),
            Some(Duration::seconds(dst_offset)),
            start,
            end,
        );

        Ok(TzStr {
            inner,
            source: s.to_string(),
            _posix_offset: posix_offset,
        })
    }

    pub fn utcoffset(&self, dt: Option<NaiveDateTime>, fold: bool) -> Option<Duration> {
        self.inner.utcoffset(dt, fold)
    }

    pub fn dst(&self, dt: Option<NaiveDateTime>, fold: bool) -> Option<Duration> {
        self.inner.dst(dt, fold)
    }

    pub fn tzname(&self, dt: Option<NaiveDateTime>, fold: bool) -> Option<String> {
        self.inner.tzname(dt, fold)
    }

    pub fn is_ambiguous(&self, dt: NaiveDateTime) -> bool {
        self.inner.is_ambiguous(dt)
    }

    pub fn fromutc(&self, dt: NaiveDateTime) -> (NaiveDateTime, bool) {
        self.inner.fromutc(dt)
    }

    pub fn transitions(&self, year: i32) -> Option<(NaiveDateTime, NaiveDateTime)> {
        self.inner.transitions(year)
    }

    pub fn hasdst(&self) -> bool {
        self.inner.hasdst
    }

    pub fn std_abbr(&self) -> &str {
        &self.inner.std_abbr
    }

    pub fn dst_abbr(&self) -> Option<&str> {
        self.inner.dst_abbr.as_deref()
    }

    pub fn std_offset(&self) -> Duration {
        self.inner.std_offset
    }

    pub fn dst_offset(&self) -> Duration {
        self.inner.dst_offset
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

impl std::fmt::Display for TzStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tzstr({})", self.source)
    }
}

// ============================================================================
// POSIX TZ string parser internals
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum TzStrError {
    #[error("empty TZ string")]
    Empty,
    #[error("invalid abbreviation")]
    InvalidAbbr,
    #[error("invalid offset")]
    InvalidOffset,
    #[error("invalid transition rule")]
    InvalidRule,
    #[error("missing end rule")]
    MissingEndRule,
    #[error("unexpected character: {0}")]
    UnexpectedChar(char),
}

struct TzStrParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> TzStrParser<'a> {
    fn new(input: &'a str) -> Self {
        TzStrParser { input, pos: 0 }
    }

    fn is_empty(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn peek_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn consume_char(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.pos += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    /// Parse a timezone abbreviation.
    /// Either `<...>` (quoted) or 3+ alpha characters.
    fn parse_abbr(&mut self) -> Result<String, TzStrError> {
        if self.consume_char('<') {
            // Quoted abbreviation
            let start = self.pos;
            while let Some(ch) = self.peek_char() {
                if ch == '>' {
                    let abbr = self.input[start..self.pos].to_string();
                    self.advance(); // consume '>'
                    return Ok(abbr);
                }
                self.advance();
            }
            Err(TzStrError::InvalidAbbr)
        } else {
            // Unquoted: 3 or more alphabetic characters
            let start = self.pos;
            while let Some(ch) = self.peek_char() {
                if !ch.is_ascii_alphabetic() {
                    break;
                }
                self.advance();
            }
            if self.pos - start < 3 {
                return Err(TzStrError::InvalidAbbr);
            }
            Ok(self.input[start..self.pos].to_string())
        }
    }

    /// Parse an offset: `[+|-]hh[:mm[:ss]]`
    fn parse_offset(&mut self) -> Result<i64, TzStrError> {
        let sign: i64 = if self.consume_char('-') {
            -1
        } else {
            self.consume_char('+');
            1
        };

        let hours = self.parse_number()? as i64;
        let mut total = hours * 3600;

        if self.consume_char(':') {
            let mins = self.parse_number()? as i64;
            total += mins * 60;
            if self.consume_char(':') {
                let secs = self.parse_number()? as i64;
                total += secs;
            }
        }

        Ok(sign * total)
    }

    /// Parse a non-negative integer.
    fn parse_number(&mut self) -> Result<u32, TzStrError> {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if !ch.is_ascii_digit() {
                break;
            }
            self.advance();
        }
        if self.pos == start {
            return Err(TzStrError::InvalidOffset);
        }
        self.input[start..self.pos]
            .parse::<u32>()
            .map_err(|_| TzStrError::InvalidOffset)
    }

    /// Parse a transition rule: `Jn`, `n`, or `Mm.w.d`, optionally followed by `/time`.
    fn parse_rule(&mut self) -> Result<TransitionRule, TzStrError> {
        let date_rule = if self.consume_char('J') {
            let day = self.parse_number()?;
            DateRule::JulianNoLeap(day)
        } else if self.consume_char('M') {
            let month = self.parse_number()?;
            if !(1..=12).contains(&month) {
                return Err(TzStrError::InvalidRule);
            }
            if !self.consume_char('.') {
                return Err(TzStrError::InvalidRule);
            }
            let week = self.parse_number()?;
            if !(1..=5).contains(&week) {
                return Err(TzStrError::InvalidRule);
            }
            if !self.consume_char('.') {
                return Err(TzStrError::InvalidRule);
            }
            let weekday = self.parse_number()?;
            if weekday > 6 {
                return Err(TzStrError::InvalidRule);
            }
            DateRule::MonthWeekDay {
                month,
                week,
                weekday,
            }
        } else {
            let day = self.parse_number()?;
            DateRule::JulianLeap(day)
        };

        let time = if self.consume_char('/') {
            self.parse_rule_time()?
        } else {
            7200 // Default: 02:00
        };

        Ok(TransitionRule::new(date_rule, time))
    }

    /// Parse a time-of-day for a transition rule: `[+|-]hh[:mm[:ss]]`
    fn parse_rule_time(&mut self) -> Result<i32, TzStrError> {
        let sign: i32 = if self.consume_char('-') {
            -1
        } else {
            self.consume_char('+');
            1
        };

        let hours = self.parse_number()? as i32;
        let mut total = hours * 3600;

        if self.consume_char(':') {
            let mins = self.parse_number()? as i32;
            total += mins * 60;
            if self.consume_char(':') {
                let secs = self.parse_number()? as i32;
                total += secs;
            }
        }

        Ok(sign * total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== DateRule / TransitionRule tests =====

    #[test]
    fn test_nth_weekday_first_sunday_march_2020() {
        // March 1, 2020 is a Sunday
        let date = nth_weekday_of_month(2020, 3, 0, 1);
        assert_eq!(date, NaiveDate::from_ymd_opt(2020, 3, 1).unwrap());
    }

    #[test]
    fn test_nth_weekday_second_sunday_march_2020() {
        let date = nth_weekday_of_month(2020, 3, 0, 2);
        assert_eq!(date, NaiveDate::from_ymd_opt(2020, 3, 8).unwrap());
    }

    #[test]
    fn test_nth_weekday_last_sunday_october_2020() {
        let date = nth_weekday_of_month(2020, 10, 0, 5);
        assert_eq!(date, NaiveDate::from_ymd_opt(2020, 10, 25).unwrap());
    }

    #[test]
    fn test_transition_rule_datetime() {
        // 2nd Sunday of March 2020 at 02:00
        let rule = TransitionRule::new(
            DateRule::MonthWeekDay {
                month: 3,
                week: 2,
                weekday: 0,
            },
            7200,
        );
        let dt = rule.datetime_for_year(2020);
        assert_eq!(
            dt,
            NaiveDate::from_ymd_opt(2020, 3, 8)
                .unwrap()
                .and_hms_opt(2, 0, 0)
                .unwrap()
        );
    }

    // ===== TzRange tests =====

    #[test]
    fn test_tzrange_no_dst() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            None,
            None,
            None,
            None,
        );
        assert!(!tz.hasdst);
        let dt = NaiveDate::from_ymd_opt(2020, 6, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tz.utcoffset(Some(dt), false), Some(Duration::seconds(-18000)));
        assert_eq!(tz.dst(Some(dt), false), Some(Duration::zero()));
    }

    #[test]
    fn test_tzrange_with_dst() {
        // EST5EDT: standard -5h, DST -4h
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            Some("EDT".to_string()),
            Some(Duration::seconds(-14400)),
            None, // default US rules
            None,
        );
        assert!(tz.hasdst);

        // January — standard time
        let jan = NaiveDate::from_ymd_opt(2020, 1, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tz.utcoffset(Some(jan), false), Some(Duration::seconds(-18000)));
        assert_eq!(tz.tzname(Some(jan), false), Some("EST".to_string()));

        // July — DST
        let jul = NaiveDate::from_ymd_opt(2020, 7, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tz.utcoffset(Some(jul), false), Some(Duration::seconds(-14400)));
        assert_eq!(tz.tzname(Some(jul), false), Some("EDT".to_string()));
    }

    // ===== TzStr parser tests =====

    #[test]
    fn test_parse_simple_no_dst() {
        let tz = TzStr::parse("EST5", false).unwrap();
        assert!(!tz.hasdst());
        assert_eq!(tz.std_abbr(), "EST");
        assert_eq!(tz.std_offset(), Duration::seconds(-18000));
    }

    #[test]
    fn test_parse_with_dst() {
        let tz = TzStr::parse("EST5EDT,M3.2.0/2,M11.1.0/2", false).unwrap();
        assert!(tz.hasdst());
        assert_eq!(tz.std_abbr(), "EST");
        assert_eq!(tz.dst_abbr(), Some("EDT"));
        assert_eq!(tz.std_offset(), Duration::seconds(-18000));
        assert_eq!(tz.dst_offset(), Duration::seconds(-14400));
    }

    #[test]
    fn test_parse_negative_offset() {
        // CET-1CEST: Central European Time (UTC+1, DST UTC+2)
        let tz = TzStr::parse("CET-1CEST,M3.5.0,M10.5.0/3", false).unwrap();
        assert_eq!(tz.std_offset(), Duration::seconds(3600));
        assert_eq!(tz.dst_offset(), Duration::seconds(7200));
    }

    #[test]
    fn test_parse_quoted_abbr() {
        let tz = TzStr::parse("<+09>-9", false).unwrap();
        assert_eq!(tz.std_abbr(), "+09");
        assert_eq!(tz.std_offset(), Duration::seconds(32400));
        assert!(!tz.hasdst());
    }

    #[test]
    fn test_parse_posix_offset_mode() {
        // In POSIX mode, signs are not inverted
        let tz = TzStr::parse("EST5EDT", true).unwrap();
        assert_eq!(tz.std_offset(), Duration::seconds(5 * 3600)); // West of GMT
    }

    #[test]
    fn test_tzstr_utcoffset_summer() {
        let tz = TzStr::parse("EST5EDT,M3.2.0/2,M11.1.0/2", false).unwrap();
        let summer = NaiveDate::from_ymd_opt(2020, 7, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(
            tz.utcoffset(Some(summer), false),
            Some(Duration::seconds(-14400))
        );
    }

    #[test]
    fn test_tzstr_utcoffset_winter() {
        let tz = TzStr::parse("EST5EDT,M3.2.0/2,M11.1.0/2", false).unwrap();
        let winter = NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(
            tz.utcoffset(Some(winter), false),
            Some(Duration::seconds(-18000))
        );
    }

    #[test]
    fn test_tzstr_default_dst_offset() {
        // If no DST offset specified, default is std + 1h
        let tz = TzStr::parse("EST5EDT,M3.2.0/2,M11.1.0/2", false).unwrap();
        let diff = tz.dst_offset() - tz.std_offset();
        assert_eq!(diff, Duration::hours(1));
    }

    #[test]
    fn test_julian_no_leap_leap_year() {
        // J60 = March 1 in all years (Feb 29 is never counted)
        assert_eq!(
            julian_no_leap_to_date(2020, 60),
            NaiveDate::from_ymd_opt(2020, 3, 1).unwrap()
        );
        // J59 = Feb 28 always
        assert_eq!(
            julian_no_leap_to_date(2020, 59),
            NaiveDate::from_ymd_opt(2020, 2, 28).unwrap()
        );
        // Non-leap year: J60 = March 1
        assert_eq!(
            julian_no_leap_to_date(2019, 60),
            NaiveDate::from_ymd_opt(2019, 3, 1).unwrap()
        );
    }

    // ===== Julian leap day rule =====

    #[test]
    fn test_julian_leap_to_date() {
        // Day 0 = Jan 1
        assert_eq!(
            julian_leap_to_date(2020, 0),
            NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()
        );
        // Day 59 = Feb 29 in leap year
        assert_eq!(
            julian_leap_to_date(2020, 59),
            NaiveDate::from_ymd_opt(2020, 2, 29).unwrap()
        );
        // Day 365 = Dec 31 in leap year
        assert_eq!(
            julian_leap_to_date(2020, 365),
            NaiveDate::from_ymd_opt(2020, 12, 31).unwrap()
        );
    }

    // ===== TransitionRule with Julian rules =====

    #[test]
    fn test_transition_rule_julian_no_leap() {
        let rule = TransitionRule::new(DateRule::JulianNoLeap(60), 7200);
        let dt = rule.datetime_for_year(2020);
        assert_eq!(
            dt,
            NaiveDate::from_ymd_opt(2020, 3, 1)
                .unwrap()
                .and_hms_opt(2, 0, 0)
                .unwrap()
        );
    }

    #[test]
    fn test_transition_rule_julian_leap() {
        let rule = TransitionRule::new(DateRule::JulianLeap(59), 7200);
        let dt = rule.datetime_for_year(2020);
        assert_eq!(
            dt,
            NaiveDate::from_ymd_opt(2020, 2, 29)
                .unwrap()
                .and_hms_opt(2, 0, 0)
                .unwrap()
        );
    }

    #[test]
    fn test_transition_rule_custom_time() {
        let rule = TransitionRule::new(
            DateRule::MonthWeekDay {
                month: 3,
                week: 2,
                weekday: 0,
            },
            3600, // 01:00
        );
        let dt = rule.datetime_for_year(2020);
        assert_eq!(
            dt,
            NaiveDate::from_ymd_opt(2020, 3, 8)
                .unwrap()
                .and_hms_opt(1, 0, 0)
                .unwrap()
        );
    }

    #[test]
    fn test_transition_rule_time_over_24h() {
        // Some rules have times >= 24:00
        let rule = TransitionRule::new(
            DateRule::MonthWeekDay {
                month: 3,
                week: 2,
                weekday: 0,
            },
            90000, // 25:00 = next day 01:00
        );
        let dt = rule.datetime_for_year(2020);
        // March 8 at 25:00 = March 9 at 01:00
        assert_eq!(
            dt,
            NaiveDate::from_ymd_opt(2020, 3, 9)
                .unwrap()
                .and_hms_opt(1, 0, 0)
                .unwrap()
        );
    }

    // ===== TzRange: transitions / ambiguity / fromutc =====

    #[test]
    fn test_tzrange_transitions() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            Some("EDT".to_string()),
            Some(Duration::seconds(-14400)),
            None,
            None,
        );
        let tr = tz.transitions(2020);
        assert!(tr.is_some());
        let (start, end) = tr.unwrap();
        // 2nd Sunday of March at 02:00
        assert_eq!(start, NaiveDate::from_ymd_opt(2020, 3, 8).unwrap().and_hms_opt(2, 0, 0).unwrap());
        // 1st Sunday of November at 02:00
        assert_eq!(end, NaiveDate::from_ymd_opt(2020, 11, 1).unwrap().and_hms_opt(2, 0, 0).unwrap());
    }

    #[test]
    fn test_tzrange_no_dst_transitions() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            None,
            None,
            None,
            None,
        );
        assert!(tz.transitions(2020).is_none());
    }

    #[test]
    fn test_tzrange_is_ambiguous_fall_back() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            Some("EDT".to_string()),
            Some(Duration::seconds(-14400)),
            None,
            None,
        );
        // The ambiguous period is [01:00, 02:00) (clocks fall back from 02:00 to 01:00)
        let ambiguous = NaiveDate::from_ymd_opt(2020, 11, 1)
            .unwrap()
            .and_hms_opt(1, 30, 0)
            .unwrap();
        assert!(tz.is_ambiguous(ambiguous));
    }

    #[test]
    fn test_tzrange_not_ambiguous_normal() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            Some("EDT".to_string()),
            Some(Duration::seconds(-14400)),
            None,
            None,
        );
        let normal = NaiveDate::from_ymd_opt(2020, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert!(!tz.is_ambiguous(normal));
    }

    #[test]
    fn test_tzrange_no_dst_not_ambiguous() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            None,
            None,
            None,
            None,
        );
        let dt = NaiveDate::from_ymd_opt(2020, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert!(!tz.is_ambiguous(dt));
    }

    #[test]
    fn test_tzrange_fromutc_no_dst() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            None,
            None,
            None,
            None,
        );
        let utc = NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let (wall, fold) = tz.fromutc(utc);
        assert_eq!(
            wall,
            NaiveDate::from_ymd_opt(2020, 1, 1)
                .unwrap()
                .and_hms_opt(7, 0, 0)
                .unwrap()
        );
        assert!(!fold);
    }

    #[test]
    fn test_tzrange_fromutc_summer() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            Some("EDT".to_string()),
            Some(Duration::seconds(-14400)),
            None,
            None,
        );
        let utc = NaiveDate::from_ymd_opt(2020, 7, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let (wall, _fold) = tz.fromutc(utc);
        assert_eq!(
            wall,
            NaiveDate::from_ymd_opt(2020, 7, 1)
                .unwrap()
                .and_hms_opt(8, 0, 0)
                .unwrap()
        );
    }

    #[test]
    fn test_tzrange_dst_summer() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            Some("EDT".to_string()),
            Some(Duration::seconds(-14400)),
            None,
            None,
        );
        let summer = NaiveDate::from_ymd_opt(2020, 7, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tz.dst(Some(summer), false), Some(Duration::seconds(3600)));
    }

    #[test]
    fn test_tzrange_dst_winter() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            Some("EDT".to_string()),
            Some(Duration::seconds(-14400)),
            None,
            None,
        );
        let winter = NaiveDate::from_ymd_opt(2020, 1, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tz.dst(Some(winter), false), Some(Duration::zero()));
    }

    #[test]
    fn test_tzrange_utcoffset_none() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            None,
            None,
            None,
            None,
        );
        assert_eq!(tz.utcoffset(None, false), None);
    }

    #[test]
    fn test_tzrange_dst_none() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            None,
            None,
            None,
            None,
        );
        assert_eq!(tz.dst(None, false), None);
    }

    #[test]
    fn test_tzrange_tzname_none() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            None,
            None,
            None,
            None,
        );
        assert_eq!(tz.tzname(None, false), None);
    }

    #[test]
    fn test_tzrange_display() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            Some("EDT".to_string()),
            Some(Duration::seconds(-14400)),
            None,
            None,
        );
        let s = format!("{}", tz);
        assert!(s.contains("EST"));
        assert!(s.contains("EDT"));
    }

    // ===== TzStr additional tests =====

    #[test]
    fn test_tzstr_display() {
        let tz = TzStr::parse("EST5EDT,M3.2.0/2,M11.1.0/2", false).unwrap();
        let s = format!("{}", tz);
        assert!(s.contains("EST5EDT"));
    }

    #[test]
    fn test_tzstr_transitions() {
        let tz = TzStr::parse("EST5EDT,M3.2.0/2,M11.1.0/2", false).unwrap();
        let tr = tz.transitions(2020);
        assert!(tr.is_some());
    }

    #[test]
    fn test_tzstr_is_ambiguous() {
        let tz = TzStr::parse("EST5EDT,M3.2.0/2,M11.1.0/2", false).unwrap();
        // Ambiguous period: [01:00, 02:00) — clocks fall back from 02:00 to 01:00
        let ambiguous = NaiveDate::from_ymd_opt(2020, 11, 1)
            .unwrap()
            .and_hms_opt(1, 30, 0)
            .unwrap();
        assert!(tz.is_ambiguous(ambiguous));
    }

    #[test]
    fn test_tzstr_fromutc() {
        let tz = TzStr::parse("EST5EDT,M3.2.0/2,M11.1.0/2", false).unwrap();
        let utc = NaiveDate::from_ymd_opt(2020, 7, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let (wall, _fold) = tz.fromutc(utc);
        assert_eq!(
            wall,
            NaiveDate::from_ymd_opt(2020, 7, 1)
                .unwrap()
                .and_hms_opt(8, 0, 0)
                .unwrap()
        );
    }

    #[test]
    fn test_tzstr_dst() {
        let tz = TzStr::parse("EST5EDT,M3.2.0/2,M11.1.0/2", false).unwrap();
        let summer = NaiveDate::from_ymd_opt(2020, 7, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tz.dst(Some(summer), false), Some(Duration::seconds(3600)));
    }

    #[test]
    fn test_tzstr_tzname() {
        let tz = TzStr::parse("EST5EDT,M3.2.0/2,M11.1.0/2", false).unwrap();
        let summer = NaiveDate::from_ymd_opt(2020, 7, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tz.tzname(Some(summer), false), Some("EDT".into()));
        let winter = NaiveDate::from_ymd_opt(2020, 1, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tz.tzname(Some(winter), false), Some("EST".into()));
    }

    #[test]
    fn test_tzstr_none_dt() {
        let tz = TzStr::parse("EST5EDT,M3.2.0/2,M11.1.0/2", false).unwrap();
        assert_eq!(tz.utcoffset(None, false), None);
        assert_eq!(tz.dst(None, false), None);
        assert_eq!(tz.tzname(None, false), None);
    }

    // ===== TzStr parsing with Julian rules =====

    #[test]
    fn test_tzstr_julian_no_leap_rule() {
        let tz = TzStr::parse("EST5EDT,J60/2,J305/2", false).unwrap();
        assert!(tz.hasdst());
    }

    #[test]
    fn test_tzstr_julian_leap_rule() {
        let tz = TzStr::parse("EST5EDT,59/2,304/2", false).unwrap();
        assert!(tz.hasdst());
    }

    #[test]
    fn test_tzstr_empty() {
        let result = TzStr::parse("", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_tzstr_parse_with_explicit_dst_offset() {
        let tz = TzStr::parse("CET-1CEST-2,M3.5.0,M10.5.0/3", false).unwrap();
        assert_eq!(tz.std_offset(), Duration::seconds(3600));
        assert_eq!(tz.dst_offset(), Duration::seconds(7200));
    }

    // ===== TzRange fold behavior =====

    #[test]
    fn test_tzrange_isdst_fold() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            Some("EDT".to_string()),
            Some(Duration::seconds(-14400)),
            None,
            None,
        );
        // In the ambiguous period [01:00, 02:00), fold=true should return standard time
        let ambiguous = NaiveDate::from_ymd_opt(2020, 11, 1)
            .unwrap()
            .and_hms_opt(1, 30, 0)
            .unwrap();
        // fold=false (first occurrence = DST)
        assert_eq!(
            tz.utcoffset(Some(ambiguous), false),
            Some(Duration::seconds(-14400))
        );
        // fold=true (second occurrence = standard)
        assert_eq!(
            tz.utcoffset(Some(ambiguous), true),
            Some(Duration::seconds(-18000))
        );
    }

    // ===== last_day_of_month =====

    #[test]
    fn test_last_day_of_month_feb_leap() {
        assert_eq!(last_day_of_month(2020, 2), 29);
    }

    #[test]
    fn test_last_day_of_month_feb_nonleap() {
        assert_eq!(last_day_of_month(2019, 2), 28);
    }

    #[test]
    fn test_last_day_of_month_dec() {
        assert_eq!(last_day_of_month(2020, 12), 31);
    }

    // ===== dst_base_offset =====

    #[test]
    fn test_dst_base_offset() {
        let tz = TzRange::new(
            "EST".to_string(),
            Some(Duration::seconds(-18000)),
            Some("EDT".to_string()),
            Some(Duration::seconds(-14400)),
            None,
            None,
        );
        assert_eq!(tz.dst_base_offset(), Duration::seconds(3600));
    }
}
