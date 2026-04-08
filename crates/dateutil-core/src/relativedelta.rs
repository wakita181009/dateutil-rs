use crate::common::Weekday;
use crate::error::RelativeDeltaError;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Timelike};
use std::fmt;

const YDAY_IDX: [i32; 12] = [31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 366];

#[inline]
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[inline]
fn days_in_month(year: i32, month: u32) -> u32 {
    debug_assert!((1..=12).contains(&month), "month out of range: {month}");
    const DAYS: [u32; 13] = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if month == 2 && is_leap_year(year) {
        29
    } else {
        DAYS[month as usize]
    }
}

/// Normalized relative time, stored as a single microsecond count.
/// This avoids cascading overflow checks in fix() and gives one
/// integer multiplication in add_to_datetime() instead of four.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RelativeTime {
    /// Total relative time in microseconds (days not included — kept separate for date arithmetic)
    total_us: i64,
}

impl RelativeTime {
    const ZERO: Self = Self { total_us: 0 };

    #[inline]
    fn new(hours: i32, minutes: i32, seconds: i32, microseconds: i64) -> Self {
        let total_us = hours as i64 * 3_600_000_000
            + minutes as i64 * 60_000_000
            + seconds as i64 * 1_000_000
            + microseconds;
        Self { total_us }
    }

    #[inline]
    fn hours(&self) -> i32 {
        ((self.total_us % 86_400_000_000) / 3_600_000_000) as i32
    }
    #[inline]
    fn minutes(&self) -> i32 {
        ((self.total_us % 3_600_000_000) / 60_000_000) as i32
    }
    #[inline]
    fn seconds(&self) -> i32 {
        ((self.total_us % 60_000_000) / 1_000_000) as i32
    }
    #[inline]
    fn microseconds(&self) -> i64 {
        self.total_us % 1_000_000
    }
    #[inline]
    fn extra_days(&self) -> i32 {
        (self.total_us / 86_400_000_000) as i32
    }
    #[inline]
    fn is_nonzero(&self) -> bool {
        self.total_us != 0
    }
}

impl std::ops::Add for RelativeTime {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            total_us: self.total_us + rhs.total_us,
        }
    }
}

impl std::ops::Sub for RelativeTime {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            total_us: self.total_us - rhs.total_us,
        }
    }
}

impl std::ops::Neg for RelativeTime {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self {
            total_us: -self.total_us,
        }
    }
}

/// Absolute date/time fields. Uses a bitflag to track which fields are set,
/// avoiding 7 separate Option discriminants.
#[derive(Clone, Copy, Debug, Default)]
struct AbsoluteFields {
    flags: u8,
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: i32,
    microsecond: i32,
}

impl AbsoluteFields {
    const YEAR: u8 = 1 << 0;
    const MONTH: u8 = 1 << 1;
    const DAY: u8 = 1 << 2;
    const HOUR: u8 = 1 << 3;
    const MINUTE: u8 = 1 << 4;
    const SECOND: u8 = 1 << 5;
    const MICROSECOND: u8 = 1 << 6;

    #[inline]
    fn has(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    #[inline]
    fn year_or(&self, default: i32) -> i32 {
        if self.has(Self::YEAR) { self.year } else { default }
    }
    #[inline]
    fn month_or(&self, default: i32) -> i32 {
        if self.has(Self::MONTH) { self.month } else { default }
    }
    #[inline]
    fn day_or(&self, default: i32) -> i32 {
        if self.has(Self::DAY) { self.day } else { default }
    }
    #[inline]
    fn hour_or(&self, default: i32) -> i32 {
        if self.has(Self::HOUR) { self.hour } else { default }
    }
    #[inline]
    fn minute_or(&self, default: i32) -> i32 {
        if self.has(Self::MINUTE) { self.minute } else { default }
    }
    #[inline]
    fn second_or(&self, default: i32) -> i32 {
        if self.has(Self::SECOND) { self.second } else { default }
    }
    #[inline]
    fn microsecond_or(&self, default: i32) -> i32 {
        if self.has(Self::MICROSECOND) { self.microsecond } else { default }
    }

    #[inline]
    fn set_year(&mut self, v: i32) { self.flags |= Self::YEAR; self.year = v; }
    #[inline]
    fn set_month(&mut self, v: i32) { self.flags |= Self::MONTH; self.month = v; }
    #[inline]
    fn set_day(&mut self, v: i32) { self.flags |= Self::DAY; self.day = v; }
    #[inline]
    fn set_hour(&mut self, v: i32) { self.flags |= Self::HOUR; self.hour = v; }
    #[inline]
    fn set_minute(&mut self, v: i32) { self.flags |= Self::MINUTE; self.minute = v; }
    #[inline]
    fn set_second(&mut self, v: i32) { self.flags |= Self::SECOND; self.second = v; }
    #[inline]
    fn set_microsecond(&mut self, v: i32) { self.flags |= Self::MICROSECOND; self.microsecond = v; }

    #[inline]
    fn has_time(&self) -> bool {
        self.flags & (Self::HOUR | Self::MINUTE | Self::SECOND | Self::MICROSECOND) != 0
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.flags == 0
    }

    fn merge_prefer_other(&self, other: &Self) -> Self {
        let mut result = *self;
        let mask = other.flags;
        if mask & Self::YEAR != 0 { result.year = other.year; }
        if mask & Self::MONTH != 0 { result.month = other.month; }
        if mask & Self::DAY != 0 { result.day = other.day; }
        if mask & Self::HOUR != 0 { result.hour = other.hour; }
        if mask & Self::MINUTE != 0 { result.minute = other.minute; }
        if mask & Self::SECOND != 0 { result.second = other.second; }
        if mask & Self::MICROSECOND != 0 { result.microsecond = other.microsecond; }
        result.flags |= mask;
        result
    }

    fn get_year(&self) -> Option<i32> {
        if self.has(Self::YEAR) { Some(self.year) } else { None }
    }
    fn get_month(&self) -> Option<i32> {
        if self.has(Self::MONTH) { Some(self.month) } else { None }
    }
    fn get_day(&self) -> Option<i32> {
        if self.has(Self::DAY) { Some(self.day) } else { None }
    }
    fn get_hour(&self) -> Option<i32> {
        if self.has(Self::HOUR) { Some(self.hour) } else { None }
    }
    fn get_minute(&self) -> Option<i32> {
        if self.has(Self::MINUTE) { Some(self.minute) } else { None }
    }
    fn get_second(&self) -> Option<i32> {
        if self.has(Self::SECOND) { Some(self.second) } else { None }
    }
    fn get_microsecond(&self) -> Option<i32> {
        if self.has(Self::MICROSECOND) { Some(self.microsecond) } else { None }
    }
}

/// Relative date/time delta with optimized internal representation.
///
/// Internally uses:
/// - Packed `RelativeTime` (single i64) for hours/minutes/seconds/microseconds
/// - Bitflag-based `AbsoluteFields` for optional absolute values
/// - Normalized months (years * 12 + months) to avoid cascading overflow
#[derive(Clone, Copy, Debug)]
pub struct RelativeDelta {
    /// Relative months, normalized: always -11..=11, excess folded into years
    months: i32,
    years: i32,
    days: i32,
    leapdays: i32,
    /// Packed relative time (hours + minutes + seconds + microseconds as single i64)
    time: RelativeTime,
    /// Absolute fields with bitflag tracking
    abs: AbsoluteFields,
    weekday: Option<Weekday>,
}

/// Builder for RelativeDelta.
#[derive(Default)]
pub struct RelativeDeltaBuilder {
    years: i32,
    months: i32,
    days: i32,
    leapdays: i32,
    hours: i32,
    minutes: i32,
    seconds: i32,
    microseconds: i64,
    abs: AbsoluteFields,
    weekday: Option<Weekday>,
    yearday: Option<i32>,
    nlyearday: Option<i32>,
}

impl RelativeDeltaBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn years(mut self, v: i32) -> Self { self.years = v; self }
    pub fn months(mut self, v: i32) -> Self { self.months = v; self }
    pub fn days(mut self, v: i32) -> Self { self.days = v; self }
    pub fn weeks(mut self, v: i32) -> Self { self.days += v * 7; self }
    pub fn hours(mut self, v: i32) -> Self { self.hours = v; self }
    pub fn minutes(mut self, v: i32) -> Self { self.minutes = v; self }
    pub fn seconds(mut self, v: i32) -> Self { self.seconds = v; self }
    pub fn microseconds(mut self, v: i64) -> Self { self.microseconds = v; self }
    pub fn leapdays(mut self, v: i32) -> Self { self.leapdays = v; self }

    pub fn year(mut self, v: i32) -> Self { self.abs.set_year(v); self }
    pub fn month(mut self, v: i32) -> Self { self.abs.set_month(v); self }
    pub fn day(mut self, v: i32) -> Self { self.abs.set_day(v); self }
    pub fn hour(mut self, v: i32) -> Self { self.abs.set_hour(v); self }
    pub fn minute(mut self, v: i32) -> Self { self.abs.set_minute(v); self }
    pub fn second(mut self, v: i32) -> Self { self.abs.set_second(v); self }
    pub fn microsecond(mut self, v: i32) -> Self { self.abs.set_microsecond(v); self }

    pub fn weekday(mut self, v: Weekday) -> Self { self.weekday = Some(v); self }
    pub fn yearday(mut self, v: i32) -> Self { self.yearday = Some(v); self }
    pub fn nlyearday(mut self, v: i32) -> Self { self.nlyearday = Some(v); self }

    pub fn build(self) -> Result<RelativeDelta, RelativeDeltaError> {
        RelativeDelta::new(
            self.years,
            self.months,
            self.days,
            self.leapdays,
            self.hours,
            self.minutes,
            self.seconds,
            self.microseconds,
            self.abs.get_year(),
            self.abs.get_month(),
            self.abs.get_day(),
            self.weekday,
            self.yearday,
            self.nlyearday,
            self.abs.get_hour(),
            self.abs.get_minute(),
            self.abs.get_second(),
            self.abs.get_microsecond(),
        )
    }
}

impl RelativeDelta {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        years: i32,
        months: i32,
        days: i32,
        leapdays: i32,
        hours: i32,
        minutes: i32,
        seconds: i32,
        microseconds: i64,
        year: Option<i32>,
        month: Option<i32>,
        day: Option<i32>,
        weekday: Option<Weekday>,
        yearday: Option<i32>,
        nlyearday: Option<i32>,
        hour: Option<i32>,
        minute: Option<i32>,
        second: Option<i32>,
        microsecond: Option<i32>,
    ) -> Result<Self, RelativeDeltaError> {
        let mut abs = AbsoluteFields::default();
        if let Some(v) = year { abs.set_year(v); }
        if let Some(v) = month { abs.set_month(v); }
        if let Some(v) = day { abs.set_day(v); }
        if let Some(v) = hour { abs.set_hour(v); }
        if let Some(v) = minute { abs.set_minute(v); }
        if let Some(v) = second { abs.set_second(v); }
        if let Some(v) = microsecond { abs.set_microsecond(v); }

        let mut leapdays = leapdays;

        // Handle yearday / nlyearday
        let mut yday = 0i32;
        if let Some(nly) = nlyearday {
            if nly != 0 {
                yday = nly;
            }
        } else if let Some(yd) = yearday {
            if yd != 0 {
                yday = yd;
                if yd > 59 {
                    leapdays = -1;
                }
            }
        }
        if yday != 0 {
            let mut found = false;
            for (idx, &ydays) in YDAY_IDX.iter().enumerate() {
                if yday <= ydays {
                    abs.set_month((idx + 1) as i32);
                    let d = if idx == 0 {
                        yday
                    } else {
                        yday - YDAY_IDX[idx - 1]
                    };
                    abs.set_day(d);
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(RelativeDeltaError::InvalidYearDay(yday));
            }
        }

        // Pack time into single i64 — no cascading fix() needed
        let time = RelativeTime::new(hours, minutes, seconds, microseconds);
        let extra_days = time.extra_days();

        // Normalize months
        let total_months = years * 12 + months;

        Ok(Self {
            years: total_months / 12,
            months: total_months % 12,
            days: days + extra_days,
            leapdays,
            time: RelativeTime {
                total_us: time.total_us % 86_400_000_000,
            },
            abs,
            weekday,
        })
    }

    pub fn builder() -> RelativeDeltaBuilder {
        RelativeDeltaBuilder::new()
    }

    /// Build from the difference between two `NaiveDateTime` values.
    pub fn from_diff(dt1: NaiveDateTime, dt2: NaiveDateTime) -> Self {
        let total_months_init =
            (dt1.year() - dt2.year()) * 12 + (dt1.month() as i32 - dt2.month() as i32);
        let mut rd = Self {
            years: total_months_init / 12,
            months: total_months_init % 12,
            days: 0,
            leapdays: 0,
            time: RelativeTime::ZERO,
            abs: AbsoluteFields::default(),
            weekday: None,
        };

        let mut dtm = rd.add_to_naive_datetime(dt2);
        let mut total_months = total_months_init;

        let (cmp, inc): (fn(NaiveDateTime, NaiveDateTime) -> bool, i32) = if dt1 < dt2 {
            (|a, b| a > b, 1)
        } else {
            (|a, b| a < b, -1)
        };

        while cmp(dt1, dtm) {
            total_months += inc;
            rd.years = total_months / 12;
            rd.months = total_months % 12;
            dtm = rd.add_to_naive_datetime(dt2);
        }

        let total_us = (dt1 - dtm)
            .num_microseconds()
            .expect("microsecond overflow in diff");
        rd.time = RelativeTime { total_us: total_us % 86_400_000_000 };
        rd.days = (total_us / 86_400_000_000) as i32;

        rd
    }

    /// Apply this delta to a `NaiveDateTime`.
    #[inline]
    pub fn add_to_naive_datetime(&self, other: NaiveDateTime) -> NaiveDateTime {
        let (year, month, day) = self.resolve_date(other.year(), other.month(), other.day());
        let hour = self.abs.hour_or(other.hour() as i32) as u32;
        let minute = self.abs.minute_or(other.minute() as i32) as u32;
        let second = self.abs.second_or(other.second() as i32) as u32;
        let usec = self.abs.microsecond_or((other.nanosecond() / 1000) as i32) as u32;

        let date = NaiveDate::from_ymd_opt(year, month as u32, day).unwrap();
        let time = NaiveTime::from_hms_micro_opt(hour, minute, second, usec).unwrap();
        let mut ret = NaiveDateTime::new(date, time);

        // Relative days + time as single TimeDelta
        let mut day_us = self.days as i64 * 86_400_000_000;
        if self.leapdays != 0 && month > 2 && is_leap_year(year) {
            day_us += self.leapdays as i64 * 86_400_000_000;
        }
        let delta = TimeDelta::microseconds(day_us + self.time.total_us);
        ret += delta;

        if let Some(wd) = self.weekday {
            ret = apply_weekday(ret, wd);
        }
        ret
    }

    /// Apply this delta to a `NaiveDate` (date-only).
    #[inline]
    pub fn add_to_naive_date(&self, other: NaiveDate) -> NaiveDate {
        let (year, month, day) = self.resolve_date(other.year(), other.month(), other.day());
        let mut ret = NaiveDate::from_ymd_opt(year, month as u32, day).unwrap();

        let mut extra_days = self.days as i64;
        if self.leapdays != 0 && month > 2 && is_leap_year(year) {
            extra_days += self.leapdays as i64;
        }
        ret += TimeDelta::days(extra_days);

        if let Some(wd) = self.weekday {
            let dt = ret.and_hms_opt(0, 0, 0).unwrap();
            ret = apply_weekday(dt, wd).date();
        }
        ret
    }

    // ---- arithmetic ----

    pub fn add_rd(&self, other: &RelativeDelta) -> Self {
        let total_months = (self.years + other.years) * 12 + self.months + other.months;
        let combined_time = self.time + other.time;
        Self {
            years: total_months / 12,
            months: total_months % 12,
            days: self.days + other.days + combined_time.extra_days(),
            leapdays: if other.leapdays != 0 {
                other.leapdays
            } else {
                self.leapdays
            },
            time: RelativeTime {
                total_us: combined_time.total_us % 86_400_000_000,
            },
            abs: self.abs.merge_prefer_other(&other.abs),
            weekday: other.weekday.or(self.weekday),
        }
    }

    pub fn sub_rd(&self, other: &RelativeDelta) -> Self {
        let total_months = (self.years - other.years) * 12 + self.months - other.months;
        let combined_time = self.time - other.time;
        Self {
            years: total_months / 12,
            months: total_months % 12,
            days: self.days - other.days + combined_time.extra_days(),
            leapdays: if self.leapdays != 0 {
                self.leapdays
            } else {
                other.leapdays
            },
            time: RelativeTime {
                total_us: combined_time.total_us % 86_400_000_000,
            },
            abs: self.abs.merge_prefer_other(&other.abs),
            weekday: self.weekday.or(other.weekday),
        }
    }

    pub fn neg(&self) -> Self {
        Self {
            years: -self.years,
            months: -self.months,
            days: -self.days,
            leapdays: self.leapdays,
            time: -self.time,
            abs: self.abs,
            weekday: self.weekday,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.years == 0
            && self.months == 0
            && self.days == 0
            && self.leapdays == 0
            && !self.time.is_nonzero()
            && self.abs.is_empty()
            && self.weekday.is_none()
    }

    #[inline]
    pub fn has_time(&self) -> bool {
        self.time.is_nonzero() || self.abs.has_time()
    }

    pub fn weeks(&self) -> i32 {
        self.days / 7
    }

    // Getters — decompose packed fields on demand
    pub fn years(&self) -> i32 { self.years }
    pub fn months(&self) -> i32 { self.months }
    pub fn days(&self) -> i32 { self.days }
    pub fn hours(&self) -> i32 { self.time.hours() }
    pub fn minutes(&self) -> i32 { self.time.minutes() }
    pub fn seconds(&self) -> i32 { self.time.seconds() }
    pub fn microseconds(&self) -> i64 { self.time.microseconds() }

    // ---- internals ----

    #[inline]
    fn resolve_date(&self, base_year: i32, base_month: u32, base_day: u32) -> (i32, i32, u32) {
        let mut year = self.abs.year_or(base_year) + self.years;
        let mut month = self.abs.month_or(base_month as i32);

        if self.months != 0 {
            // Convert to 0-based, apply delta, convert back.
            // div_euclid/rem_euclid handle negative values correctly.
            let total = (month - 1) + self.months;
            year += total.div_euclid(12);
            month = total.rem_euclid(12) + 1;
        }

        let dim = days_in_month(year, month as u32);
        let day = dim.min(self.abs.day_or(base_day as i32) as u32);

        (year, month, day)
    }
}

#[inline]
fn apply_weekday(dt: NaiveDateTime, wd: Weekday) -> NaiveDateTime {
    let weekday = wd.weekday() as i64;
    let nth = match wd.n() {
        Some(0) | None => 1i64,
        Some(n) => n as i64,
    };
    let mut jumpdays = (nth.abs() - 1) * 7;
    let ret_wd = dt.weekday().num_days_from_monday() as i64;
    if nth > 0 {
        jumpdays += (7 - ret_wd + weekday).rem_euclid(7);
    } else {
        jumpdays += (ret_wd - weekday).rem_euclid(7);
        jumpdays *= -1;
    }
    dt + TimeDelta::days(jumpdays)
}

impl PartialEq for RelativeDelta {
    fn eq(&self, other: &Self) -> bool {
        self.years == other.years
            && self.months == other.months
            && self.days == other.days
            && self.leapdays == other.leapdays
            && self.time == other.time
            && self.abs.flags == other.abs.flags
            && (!self.abs.has(AbsoluteFields::YEAR) || self.abs.year == other.abs.year)
            && (!self.abs.has(AbsoluteFields::MONTH) || self.abs.month == other.abs.month)
            && (!self.abs.has(AbsoluteFields::DAY) || self.abs.day == other.abs.day)
            && (!self.abs.has(AbsoluteFields::HOUR) || self.abs.hour == other.abs.hour)
            && (!self.abs.has(AbsoluteFields::MINUTE) || self.abs.minute == other.abs.minute)
            && (!self.abs.has(AbsoluteFields::SECOND) || self.abs.second == other.abs.second)
            && (!self.abs.has(AbsoluteFields::MICROSECOND)
                || self.abs.microsecond == other.abs.microsecond)
            && weekday_eq(&self.weekday, &other.weekday)
    }
}

impl Eq for RelativeDelta {}

fn weekday_eq(a: &Option<Weekday>, b: &Option<Weekday>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(wa), Some(wb)) => {
            wa.weekday() == wb.weekday() && normalize_n(wa.n()) == normalize_n(wb.n())
        }
        _ => false,
    }
}

fn normalize_n(n: Option<i32>) -> Option<i32> {
    match n {
        None | Some(0) | Some(1) => None,
        other => other,
    }
}

impl fmt::Display for RelativeDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "relativedelta(")?;
        let mut first = true;
        macro_rules! part {
            ($name:expr, $val:expr) => {
                if $val != 0 {
                    if !first { write!(f, ", ")?; }
                    write!(f, "{}={}", $name, $val)?;
                    #[allow(unused_assignments)]
                    { first = false; }
                }
            };
        }
        part!("years", self.years);
        part!("months", self.months);
        part!("days", self.days);
        part!("leapdays", self.leapdays);
        part!("hours", self.time.hours());
        part!("minutes", self.time.minutes());
        part!("seconds", self.time.seconds());
        part!("microseconds", self.time.microseconds());
        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, mi, s)
            .unwrap()
    }

    fn rd(years: i32, months: i32, days: i32) -> RelativeDelta {
        RelativeDelta::builder()
            .years(years)
            .months(months)
            .days(days)
            .build()
            .unwrap()
    }

    #[test]
    fn test_builder() {
        let delta = RelativeDelta::builder()
            .years(1)
            .months(2)
            .days(3)
            .hours(4)
            .build()
            .unwrap();
        assert_eq!(delta.years(), 1);
        assert_eq!(delta.months(), 2);
        assert_eq!(delta.days(), 3);
        assert_eq!(delta.hours(), 4);
    }

    #[test]
    fn test_add_months() {
        let base = dt(2024, 1, 31, 0, 0, 0);
        let delta = rd(0, 1, 0);
        let result = delta.add_to_naive_datetime(base);
        assert_eq!(result, dt(2024, 2, 29, 0, 0, 0));
    }

    #[test]
    fn test_add_months_non_leap() {
        let base = dt(2023, 1, 31, 0, 0, 0);
        let delta = rd(0, 1, 0);
        let result = delta.add_to_naive_datetime(base);
        assert_eq!(result, dt(2023, 2, 28, 0, 0, 0));
    }

    #[test]
    fn test_add_years() {
        let base = dt(2024, 2, 29, 12, 0, 0);
        let delta = rd(1, 0, 0);
        let result = delta.add_to_naive_datetime(base);
        assert_eq!(result, dt(2025, 2, 28, 12, 0, 0));
    }

    #[test]
    fn test_add_days_and_hours() {
        let base = dt(2024, 1, 1, 10, 30, 0);
        let delta = RelativeDelta::new(
            0, 0, 5, 0, 3, 0, 0, 0, None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        let result = delta.add_to_naive_datetime(base);
        assert_eq!(result, dt(2024, 1, 6, 13, 30, 0));
    }

    #[test]
    fn test_absolute_fields() {
        let base = dt(2024, 6, 15, 10, 30, 45);
        let delta = RelativeDelta::builder()
            .year(2025)
            .month(1)
            .day(1)
            .hour(0)
            .minute(0)
            .second(0)
            .build()
            .unwrap();
        let result = delta.add_to_naive_datetime(base);
        assert_eq!(result, dt(2025, 1, 1, 0, 0, 0));
    }

    #[test]
    fn test_weekday_next_monday() {
        let base = dt(2024, 1, 3, 0, 0, 0);
        let mo = Weekday::new(0, None).unwrap();
        let delta = RelativeDelta::builder().weekday(mo).build().unwrap();
        let result = delta.add_to_naive_datetime(base);
        assert_eq!(result, dt(2024, 1, 8, 0, 0, 0));
    }

    #[test]
    fn test_neg() {
        let delta = rd(1, 2, 3);
        let neg = delta.neg();
        assert_eq!(neg.years(), -1);
        assert_eq!(neg.months(), -2);
        assert_eq!(neg.days(), -3);
    }

    #[test]
    fn test_add_rd() {
        let a = rd(1, 2, 3);
        let b = rd(0, 3, 7);
        let result = a.add_rd(&b);
        assert_eq!(result.years(), 1);
        assert_eq!(result.months(), 5);
        assert_eq!(result.days(), 10);
    }

    #[test]
    fn test_from_diff() {
        let dt1 = dt(2025, 3, 15, 10, 30, 0);
        let dt2 = dt(2024, 1, 10, 8, 15, 0);
        let delta = RelativeDelta::from_diff(dt1, dt2);
        let result = delta.add_to_naive_datetime(dt2);
        assert_eq!(result, dt1);
    }

    #[test]
    fn test_is_zero() {
        let delta = RelativeDelta::new(
            0, 0, 0, 0, 0, 0, 0, 0, None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        assert!(delta.is_zero());
    }

    #[test]
    fn test_time_normalization() {
        // 90 seconds → 1 minute 30 seconds, packed into single i64
        let delta = RelativeDelta::new(
            0, 0, 0, 0, 0, 0, 90, 0, None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        assert_eq!(delta.minutes(), 1);
        assert_eq!(delta.seconds(), 30);
    }

    #[test]
    fn test_hours_overflow_to_days() {
        let delta = RelativeDelta::new(
            0, 0, 0, 0, 25, 0, 0, 0, None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        assert_eq!(delta.days(), 1);
        assert_eq!(delta.hours(), 1);
    }

    #[test]
    fn test_months_overflow() {
        let delta = RelativeDelta::new(
            0, 14, 0, 0, 0, 0, 0, 0, None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        assert_eq!(delta.years(), 1);
        assert_eq!(delta.months(), 2);
    }

    #[test]
    fn test_microseconds_overflow() {
        let delta = RelativeDelta::new(
            0, 0, 0, 0, 0, 0, 0, 2_500_000, None, None, None, None, None, None, None, None,
            None, None,
        )
        .unwrap();
        assert_eq!(delta.seconds(), 2);
        assert_eq!(delta.microseconds(), 500_000);
    }

    #[test]
    fn test_yearday() {
        let delta = RelativeDelta::new(
            0, 0, 0, 0, 0, 0, 0, 0, Some(2024), None, None, None, Some(60), None, None, None,
            None, None,
        )
        .unwrap();
        let base = dt(2024, 1, 1, 0, 0, 0);
        let result = delta.add_to_naive_datetime(base);
        assert_eq!(result.month(), 2);
    }

    #[test]
    fn test_add_to_naive_date() {
        let base = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let delta = rd(0, 1, 0);
        let result = delta.add_to_naive_date(base);
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
    }

    #[test]
    fn test_display_nonzero_only() {
        let delta = rd(1, 2, 3);
        assert_eq!(delta.to_string(), "relativedelta(years=1, months=2, days=3)");
    }

    #[test]
    fn test_display_empty() {
        let delta = rd(0, 0, 0);
        assert_eq!(delta.to_string(), "relativedelta()");
    }

    #[test]
    fn test_display_time() {
        let delta = RelativeDelta::builder().hours(1).minutes(30).build().unwrap();
        assert_eq!(delta.to_string(), "relativedelta(hours=1, minutes=30)");
    }

    #[test]
    fn test_has_time() {
        let no_time = rd(1, 0, 0);
        assert!(!no_time.has_time());

        let with_time = RelativeDelta::builder().hours(1).build().unwrap();
        assert!(with_time.has_time());

        let with_abs_time = RelativeDelta::builder().hour(12).build().unwrap();
        assert!(with_abs_time.has_time());
    }

    #[test]
    fn test_add_rd_time_overflow_to_days() {
        // 23h + 2h = 25h → 1 day + 1h
        let a = RelativeDelta::builder().hours(23).build().unwrap();
        let b = RelativeDelta::builder().hours(2).build().unwrap();
        let result = a.add_rd(&b);
        assert_eq!(result.days(), 1);
        assert_eq!(result.hours(), 1);
    }

    #[test]
    fn test_sub_rd_time_underflow_to_days() {
        // 1h - 3h = -2h → -1 day + 22h? No: total_us = -7200s, extra_days=0, time=-2h
        // Actually: -2h total_us = -7_200_000_000. extra_days = -7200000000/86400000000 = 0.
        // So days=0, hours=-2. But for sub where base has days:
        let a = RelativeDelta::builder().days(1).hours(1).build().unwrap();
        let b = RelativeDelta::builder().hours(3).build().unwrap();
        let result = a.sub_rd(&b);
        // days=1, time = 1h - 3h = -2h. extra_days=0. So days=1, hours=-2.
        // net: 1 day - 2 hours = 22 hours. Representation: days=1, time=-2h.
        // This is consistent — add_to_datetime will compute 1*86400 + (-2*3600) = 79200s = 22h.
        assert_eq!(result.days(), 1);
        assert_eq!(result.hours(), -2);
    }

    // --- P0 regression tests (negative months) ---

    #[test]
    fn test_subtract_1_month_from_january() {
        // Jan 15 - 1 month = Dec 15 of previous year
        let base = dt(2024, 1, 15, 0, 0, 0);
        let delta = rd(0, -1, 0);
        let result = delta.add_to_naive_datetime(base);
        assert_eq!(result, dt(2023, 12, 15, 0, 0, 0));
    }

    #[test]
    fn test_subtract_13_months() {
        // Jan 15 2024 - 13 months = Dec 15 2022
        let base = dt(2024, 1, 15, 0, 0, 0);
        let delta = rd(0, -13, 0);
        let result = delta.add_to_naive_datetime(base);
        assert_eq!(result, dt(2022, 12, 15, 0, 0, 0));
    }

    #[test]
    fn test_subtract_12_months() {
        // Mar 15 2024 - 12 months = Mar 15 2023
        let base = dt(2024, 3, 15, 0, 0, 0);
        let delta = rd(0, -12, 0);
        let result = delta.add_to_naive_datetime(base);
        assert_eq!(result, dt(2023, 3, 15, 0, 0, 0));
    }

    #[test]
    fn test_subtract_25_months() {
        // Mar 15 2024 - 25 months = Feb 15 2022
        let base = dt(2024, 3, 15, 0, 0, 0);
        let delta = rd(0, -25, 0);
        let result = delta.add_to_naive_datetime(base);
        assert_eq!(result, dt(2022, 2, 15, 0, 0, 0));
    }

    #[test]
    fn test_add_large_positive_months() {
        // Jan 15 2024 + 25 months = Feb 15 2026
        let base = dt(2024, 1, 15, 0, 0, 0);
        let delta = rd(0, 25, 0);
        let result = delta.add_to_naive_datetime(base);
        assert_eq!(result, dt(2026, 2, 15, 0, 0, 0));
    }

    #[test]
    fn test_negative_months_date_only() {
        // Same bug for NaiveDate path
        let base = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let delta = rd(0, -13, 0);
        let result = delta.add_to_naive_date(base);
        assert_eq!(result, NaiveDate::from_ymd_opt(2022, 12, 15).unwrap());
    }

    #[test]
    fn test_struct_size() {
        // Verify the struct is compact
        let size = std::mem::size_of::<RelativeDelta>();
        // Should be significantly smaller than 17 * Option<i32> = 136 bytes
        assert!(size <= 80, "RelativeDelta is {size} bytes, expected <= 80");
    }
}
