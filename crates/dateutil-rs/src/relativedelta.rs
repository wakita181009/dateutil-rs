use crate::common::Weekday;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Timelike};
use std::fmt;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const YDAY_IDX: [i32; 12] = [31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 366];

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => panic!("invalid month: {month}"),
    }
}

fn sign_f(x: f64) -> f64 {
    if x >= 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// Python-style divmod for positive dividend (after abs).
fn divmod_f(x: f64, d: f64) -> (f64, f64) {
    let q = (x / d).floor();
    (q, x - q * d)
}

// ---------------------------------------------------------------------------
// Struct
// ---------------------------------------------------------------------------

/// Rust port of `dateutil.relativedelta.relativedelta`.
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(name = "relativedelta", from_py_object)
)]
pub struct RelativeDelta {
    // Relative (plural) fields
    years: i32,
    months: i32,
    days: f64,
    leapdays: i32,
    hours: f64,
    minutes: f64,
    seconds: f64,
    microseconds: f64,
    // Absolute (singular) fields — None means "not set"
    year: Option<i32>,
    month: Option<i32>,
    day: Option<i32>,
    hour: Option<i32>,
    minute: Option<i32>,
    second: Option<i32>,
    microsecond: Option<i32>,
    // Special
    weekday: Option<Weekday>,
    // Internal
    has_time: bool,
}

// ---------------------------------------------------------------------------
// Pure-Rust implementation
// ---------------------------------------------------------------------------

impl RelativeDelta {
    /// Keyword-style constructor (weeks already folded into days by caller).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        years: i32,
        months: i32,
        days: f64,
        leapdays: i32,
        hours: f64,
        minutes: f64,
        seconds: f64,
        microseconds: f64,
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
    ) -> Result<Self, String> {
        let mut rd = Self {
            years,
            months,
            days,
            leapdays,
            hours,
            minutes,
            seconds,
            microseconds,
            year,
            month,
            day,
            weekday,
            hour,
            minute,
            second,
            microsecond,
            has_time: false,
        };

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
                    rd.leapdays = -1;
                }
            }
        }
        if yday != 0 {
            let mut found = false;
            for (idx, &ydays) in YDAY_IDX.iter().enumerate() {
                if yday <= ydays {
                    rd.month = Some((idx + 1) as i32);
                    rd.day = if idx == 0 {
                        Some(yday)
                    } else {
                        Some(yday - YDAY_IDX[idx - 1])
                    };
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(format!("invalid year day ({yday})"));
            }
        }

        rd.fix();
        Ok(rd)
    }

    /// Build from the difference between two `NaiveDateTime` values.
    pub fn from_diff(dt1: NaiveDateTime, dt2: NaiveDateTime) -> Self {
        let mut rd = Self::zero();

        let mut months = (dt1.year() - dt2.year()) * 12 + (dt1.month() as i32 - dt2.month() as i32);
        rd.set_months_internal(months);

        let mut dtm = rd.add_to_naive_datetime(dt2);

        let (cmp, inc): (fn(NaiveDateTime, NaiveDateTime) -> bool, i32) = if dt1 < dt2 {
            (|a, b| a > b, 1)
        } else {
            (|a, b| a < b, -1)
        };

        while cmp(dt1, dtm) {
            months += inc;
            rd.set_months_internal(months);
            dtm = rd.add_to_naive_datetime(dt2);
        }

        let total_us = (dt1 - dtm)
            .num_microseconds()
            .expect("microsecond overflow in diff");
        // Match Python: seconds = delta.days*86400 + delta.seconds; microseconds = delta.microseconds
        rd.seconds = total_us.div_euclid(1_000_000) as f64;
        rd.microseconds = total_us.rem_euclid(1_000_000) as f64;

        rd.fix();
        rd
    }

    // ---- core operations ----

    /// Apply this delta to a `NaiveDateTime`, returning a new `NaiveDateTime`.
    pub fn add_to_naive_datetime(&self, other: NaiveDateTime) -> NaiveDateTime {
        let mut year = self.year.unwrap_or(other.year()) + self.years;
        let mut month = self.month.unwrap_or(other.month() as i32);

        if self.months != 0 {
            month += self.months;
            if month > 12 {
                year += 1;
                month -= 12;
            } else if month < 1 {
                year -= 1;
                month += 12;
            }
        }

        let dim = days_in_month(year, month as u32);
        let day = dim.min(self.day.unwrap_or(other.day() as i32) as u32);

        let hour = self.hour.unwrap_or(other.hour() as i32) as u32;
        let minute = self.minute.unwrap_or(other.minute() as i32) as u32;
        let second = self.second.unwrap_or(other.second() as i32) as u32;
        let usec = self
            .microsecond
            .unwrap_or((other.nanosecond() / 1000) as i32) as u32;

        let date = NaiveDate::from_ymd_opt(year, month as u32, day).unwrap();
        let time = NaiveTime::from_hms_micro_opt(hour, minute, second, usec).unwrap();
        let mut ret = NaiveDateTime::new(date, time);

        // relative time
        let mut extra_days = self.days;
        if self.leapdays != 0 && month > 2 && is_leap_year(year) {
            extra_days += self.leapdays as f64;
        }
        let total_us = extra_days * 86_400_000_000.0
            + self.hours * 3_600_000_000.0
            + self.minutes * 60_000_000.0
            + self.seconds * 1_000_000.0
            + self.microseconds;
        ret += TimeDelta::microseconds(total_us as i64);

        // weekday
        if let Some(ref wd) = self.weekday {
            ret = apply_weekday(ret, wd);
        }
        ret
    }

    /// Apply this delta to a `NaiveDate` (date-only, ignoring time fields).
    pub fn add_to_naive_date(&self, other: NaiveDate) -> NaiveDate {
        let mut year = self.year.unwrap_or(other.year()) + self.years;
        let mut month = self.month.unwrap_or(other.month() as i32);

        if self.months != 0 {
            month += self.months;
            if month > 12 {
                year += 1;
                month -= 12;
            } else if month < 1 {
                year -= 1;
                month += 12;
            }
        }

        let dim = days_in_month(year, month as u32);
        let day = dim.min(self.day.unwrap_or(other.day() as i32) as u32);

        let mut ret = NaiveDate::from_ymd_opt(year, month as u32, day).unwrap();

        let mut extra_days = self.days;
        if self.leapdays != 0 && month > 2 && is_leap_year(year) {
            extra_days += self.leapdays as f64;
        }
        ret += TimeDelta::days(extra_days as i64);

        if let Some(ref wd) = self.weekday {
            let dt = ret.and_hms_opt(0, 0, 0).unwrap();
            ret = apply_weekday(dt, wd).date();
        }
        ret
    }

    // ---- arithmetic between RelativeDeltas ----

    pub fn add_rd(&self, other: &RelativeDelta) -> Self {
        let mut rd = Self {
            years: other.years + self.years,
            months: other.months + self.months,
            days: other.days + self.days,
            leapdays: if other.leapdays != 0 {
                other.leapdays
            } else {
                self.leapdays
            },
            hours: other.hours + self.hours,
            minutes: other.minutes + self.minutes,
            seconds: other.seconds + self.seconds,
            microseconds: other.microseconds + self.microseconds,
            year: other.year.or(self.year),
            month: other.month.or(self.month),
            day: other.day.or(self.day),
            weekday: other.weekday.or(self.weekday),
            hour: other.hour.or(self.hour),
            minute: other.minute.or(self.minute),
            second: other.second.or(self.second),
            microsecond: other.microsecond.or(self.microsecond),
            has_time: false,
        };
        rd.fix();
        rd
    }

    pub fn sub_rd(&self, other: &RelativeDelta) -> Self {
        let mut rd = Self {
            years: self.years - other.years,
            months: self.months - other.months,
            days: self.days - other.days,
            leapdays: if self.leapdays != 0 {
                self.leapdays
            } else {
                other.leapdays
            },
            hours: self.hours - other.hours,
            minutes: self.minutes - other.minutes,
            seconds: self.seconds - other.seconds,
            microseconds: self.microseconds - other.microseconds,
            year: self.year.or(other.year),
            month: self.month.or(other.month),
            day: self.day.or(other.day),
            weekday: self.weekday.or(other.weekday),
            hour: self.hour.or(other.hour),
            minute: self.minute.or(other.minute),
            second: self.second.or(other.second),
            microsecond: self.microsecond.or(other.microsecond),
            has_time: false,
        };
        rd.fix();
        rd
    }

    pub fn neg(&self) -> Self {
        let mut rd = Self {
            years: -self.years,
            months: -self.months,
            days: -self.days,
            leapdays: self.leapdays,
            hours: -self.hours,
            minutes: -self.minutes,
            seconds: -self.seconds,
            microseconds: -self.microseconds,
            year: self.year,
            month: self.month,
            day: self.day,
            weekday: self.weekday,
            hour: self.hour,
            minute: self.minute,
            second: self.second,
            microsecond: self.microsecond,
            has_time: false,
        };
        rd.fix();
        rd
    }

    pub fn abs(&self) -> Self {
        let mut rd = Self {
            years: self.years.abs(),
            months: self.months.abs(),
            days: self.days.abs(),
            leapdays: self.leapdays,
            hours: self.hours.abs(),
            minutes: self.minutes.abs(),
            seconds: self.seconds.abs(),
            microseconds: self.microseconds.abs(),
            year: self.year,
            month: self.month,
            day: self.day,
            weekday: self.weekday,
            hour: self.hour,
            minute: self.minute,
            second: self.second,
            microsecond: self.microsecond,
            has_time: false,
        };
        rd.fix();
        rd
    }

    pub fn mul(&self, f: f64) -> Self {
        let mut rd = Self {
            years: (self.years as f64 * f) as i32,
            months: (self.months as f64 * f) as i32,
            days: (self.days * f).trunc(),
            leapdays: self.leapdays,
            hours: (self.hours * f).trunc(),
            minutes: (self.minutes * f).trunc(),
            seconds: (self.seconds * f).trunc(),
            microseconds: (self.microseconds * f).trunc(),
            year: self.year,
            month: self.month,
            day: self.day,
            weekday: self.weekday,
            hour: self.hour,
            minute: self.minute,
            second: self.second,
            microsecond: self.microsecond,
            has_time: false,
        };
        rd.fix();
        rd
    }

    pub fn is_zero(&self) -> bool {
        self.years == 0
            && self.months == 0
            && self.days == 0.0
            && self.leapdays == 0
            && self.hours == 0.0
            && self.minutes == 0.0
            && self.seconds == 0.0
            && self.microseconds == 0.0
            && self.year.is_none()
            && self.month.is_none()
            && self.day.is_none()
            && self.weekday.is_none()
            && self.hour.is_none()
            && self.minute.is_none()
            && self.second.is_none()
            && self.microsecond.is_none()
    }

    pub fn weeks(&self) -> f64 {
        (self.days / 7.0).trunc()
    }

    pub fn has_time(&self) -> bool {
        self.has_time
    }

    // ---- internals ----

    fn zero() -> Self {
        Self {
            years: 0,
            months: 0,
            days: 0.0,
            leapdays: 0,
            hours: 0.0,
            minutes: 0.0,
            seconds: 0.0,
            microseconds: 0.0,
            year: None,
            month: None,
            day: None,
            weekday: None,
            hour: None,
            minute: None,
            second: None,
            microsecond: None,
            has_time: false,
        }
    }

    fn fix(&mut self) {
        if self.microseconds.abs() > 999_999.0 {
            let s = sign_f(self.microseconds);
            let (d, m) = divmod_f(self.microseconds * s, 1_000_000.0);
            self.microseconds = m * s;
            self.seconds += d * s;
        }
        if self.seconds.abs() > 59.0 {
            let s = sign_f(self.seconds);
            let (d, m) = divmod_f(self.seconds * s, 60.0);
            self.seconds = m * s;
            self.minutes += d * s;
        }
        if self.minutes.abs() > 59.0 {
            let s = sign_f(self.minutes);
            let (d, m) = divmod_f(self.minutes * s, 60.0);
            self.minutes = m * s;
            self.hours += d * s;
        }
        if self.hours.abs() > 23.0 {
            let s = sign_f(self.hours);
            let (d, m) = divmod_f(self.hours * s, 24.0);
            self.hours = m * s;
            self.days += d * s;
        }
        if self.months.abs() > 11 {
            let s = if self.months >= 0 { 1 } else { -1 };
            let v = self.months.abs();
            self.years += (v / 12) * s;
            self.months = (v % 12) * s;
        }
        self.has_time = self.hours != 0.0
            || self.minutes != 0.0
            || self.seconds != 0.0
            || self.microseconds != 0.0
            || self.hour.is_some()
            || self.minute.is_some()
            || self.second.is_some()
            || self.microsecond.is_some();
    }

    fn set_months_internal(&mut self, months: i32) {
        self.months = months;
        if self.months.abs() > 11 {
            let s = if self.months >= 0 { 1 } else { -1 };
            let v = self.months.abs();
            self.years = (v / 12) * s;
            self.months = (v % 12) * s;
        } else {
            self.years = 0;
        }
    }
}

fn apply_weekday(dt: NaiveDateTime, wd: &Weekday) -> NaiveDateTime {
    let weekday = wd.weekday() as i64;
    let nth = match wd.n() {
        Some(0) | None => 1i64,
        Some(n) => n as i64,
    };
    let mut jumpdays = (nth.abs() - 1) * 7;
    let ret_wd = dt.weekday().num_days_from_monday() as i64;
    if nth > 0 {
        // rem_euclid matches Python's % for negative dividends
        jumpdays += (7 - ret_wd + weekday).rem_euclid(7);
    } else {
        jumpdays += (ret_wd - weekday).rem_euclid(7);
        jumpdays *= -1;
    }
    dt + TimeDelta::days(jumpdays)
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

impl fmt::Display for RelativeDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "relativedelta(")?;
        let mut parts: Vec<String> = Vec::new();

        // Relative fields with sign
        macro_rules! push_rel {
            ($name:expr, $val:expr, $is_f64:expr) => {
                if $is_f64 {
                    let v: f64 = $val as f64;
                    if v != 0.0 {
                        if v == v.trunc() {
                            parts.push(format!("{}={:+}", $name, v as i64));
                        } else {
                            parts.push(format!("{}={:+}", $name, v));
                        }
                    }
                } else {
                    let v: i64 = $val as i64;
                    if v != 0 {
                        parts.push(format!("{}={:+}", $name, v));
                    }
                }
            };
        }
        push_rel!("years", self.years, false);
        push_rel!("months", self.months, false);
        push_rel!("days", self.days, true);
        push_rel!("leapdays", self.leapdays, false);
        push_rel!("hours", self.hours, true);
        push_rel!("minutes", self.minutes, true);
        push_rel!("seconds", self.seconds, true);
        push_rel!("microseconds", self.microseconds, true);

        // Absolute fields
        macro_rules! push_abs {
            ($name:expr, $val:expr) => {
                if let Some(v) = $val {
                    parts.push(format!("{}={}", $name, v));
                }
            };
        }
        push_abs!("year", self.year);
        push_abs!("month", self.month);
        push_abs!("day", self.day);
        if let Some(ref wd) = self.weekday {
            parts.push(format!("weekday={wd}"));
        }
        push_abs!("hour", self.hour);
        push_abs!("minute", self.minute);
        push_abs!("second", self.second);
        push_abs!("microsecond", self.microsecond);

        write!(f, "{})", parts.join(", "))
    }
}

// ---------------------------------------------------------------------------
// PyO3 bindings
// ---------------------------------------------------------------------------

#[cfg(feature = "python")]
mod py {
    use super::*;
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;
    use pyo3::types::{
        PyDate, PyDateAccess, PyDateTime, PyDelta, PyInt, PyTimeAccess, PyTzInfo,
        PyTzInfoAccess,
    };

    /// Extract a NaiveDateTime from a Python date or datetime.
    fn py_to_ndt(obj: &Bound<'_, pyo3::PyAny>) -> PyResult<NaiveDateTime> {
        if let Ok(dt) = obj.cast::<PyDateTime>() {
            let d = NaiveDate::from_ymd_opt(
                dt.get_year(),
                dt.get_month() as u32,
                dt.get_day() as u32,
            )
            .ok_or_else(|| PyValueError::new_err("invalid date"))?;
            let t = NaiveTime::from_hms_micro_opt(
                dt.get_hour() as u32,
                dt.get_minute() as u32,
                dt.get_second() as u32,
                dt.get_microsecond(),
            )
            .ok_or_else(|| PyValueError::new_err("invalid time"))?;
            Ok(NaiveDateTime::new(d, t))
        } else if let Ok(d) = obj.cast::<PyDate>() {
            let date = NaiveDate::from_ymd_opt(
                d.get_year(),
                d.get_month() as u32,
                d.get_day() as u32,
            )
            .ok_or_else(|| PyValueError::new_err("invalid date"))?;
            Ok(date.and_hms_opt(0, 0, 0).unwrap())
        } else {
            Err(pyo3::exceptions::PyTypeError::new_err(
                "relativedelta only diffs datetime/date",
            ))
        }
    }

    /// Create a Python datetime from chrono, preserving optional tzinfo.
    fn ndt_to_py<'py>(
        py: Python<'py>,
        dt: NaiveDateTime,
        tzinfo: Option<&Bound<'py, PyTzInfo>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let obj = PyDateTime::new(
            py,
            dt.year(),
            dt.month() as u8,
            dt.day() as u8,
            dt.hour() as u8,
            dt.minute() as u8,
            dt.second() as u8,
            dt.nanosecond() / 1000,
            tzinfo,
        )?;
        Ok(obj.into_any())
    }

    #[pymethods]
    impl RelativeDelta {
        #[new]
        #[pyo3(signature = (dt1=None, dt2=None, *, years=0.0, months=0.0, days=0.0, leapdays=0, weeks=0.0, hours=0.0, minutes=0.0, seconds=0.0, microseconds=0.0, year=None, month=None, day=None, weekday=None, yearday=None, nlyearday=None, hour=None, minute=None, second=None, microsecond=None))]
        #[allow(clippy::too_many_arguments)]
        fn py_new(
            dt1: Option<&Bound<'_, pyo3::PyAny>>,
            dt2: Option<&Bound<'_, pyo3::PyAny>>,
            years: f64,
            months: f64,
            days: f64,
            leapdays: i32,
            weeks: f64,
            hours: f64,
            minutes: f64,
            seconds: f64,
            microseconds: f64,
            year: Option<i32>,
            month: Option<i32>,
            day: Option<i32>,
            weekday: Option<&Bound<'_, pyo3::PyAny>>,
            yearday: Option<i32>,
            nlyearday: Option<i32>,
            hour: Option<i32>,
            minute: Option<i32>,
            second: Option<i32>,
            microsecond: Option<i32>,
        ) -> PyResult<Self> {
            // Diff mode
            if let (Some(d1), Some(d2)) = (dt1, dt2) {
                let ndt1 = py_to_ndt(d1)?;
                let ndt2 = py_to_ndt(d2)?;
                return Ok(Self::from_diff(ndt1, ndt2));
            }

            // Validate years/months are integers (match Python behavior)
            if years.fract() != 0.0 || months.fract() != 0.0 {
                return Err(PyValueError::new_err(
                    "Non-integer years and months are ambiguous and not currently supported.",
                ));
            }
            let years = years as i32;
            let months = months as i32;

            // Parse weekday (int 0-6 or Weekday instance)
            let wd = match weekday {
                Some(w) => {
                    if w.is_instance_of::<PyInt>() {
                        let i: u8 = w.extract()?;
                        if i > 6 {
                            return Err(PyValueError::new_err(format!(
                                "weekday must be 0..=6, got {i}"
                            )));
                        }
                        Some(Weekday::new(i, None))
                    } else {
                        Some(w.extract::<Weekday>()?)
                    }
                }
                None => None,
            };

            Self::new(
                years,
                months,
                days + weeks * 7.0,
                leapdays,
                hours,
                minutes,
                seconds,
                microseconds,
                year,
                month,
                day,
                wd,
                yearday,
                nlyearday,
                hour,
                minute,
                second,
                microsecond,
            )
            .map_err(PyValueError::new_err)
        }

        // ---- arithmetic ----

        fn __add__<'py>(
            &self,
            other: &Bound<'py, pyo3::PyAny>,
        ) -> PyResult<Bound<'py, pyo3::PyAny>> {
            let py = other.py();

            // relativedelta + relativedelta
            if let Ok(rd) = other.extract::<pyo3::PyRef<'_, RelativeDelta>>() {
                let result = self.add_rd(&rd);
                return Ok(Bound::new(py, result)?.into_any());
            }

            // relativedelta + timedelta
            if let Ok(td) = other.cast::<PyDelta>() {
                use pyo3::types::PyDeltaAccess;
                let result = self.add_timedelta(
                    td.get_days() as i64,
                    td.get_seconds() as i64,
                    td.get_microseconds() as i64,
                );
                return Ok(Bound::new(py, result)?.into_any());
            }

            // relativedelta + datetime (check BEFORE date, datetime is subclass)
            if let Ok(dt) = other.cast::<PyDateTime>() {
                let ndt = py_to_ndt(other)?;
                let result = self.add_to_naive_datetime(ndt);
                let tzinfo = dt.get_tzinfo();
                return ndt_to_py(py, result, tzinfo.as_ref());
            }

            // relativedelta + date
            if other.cast::<PyDate>().is_ok() {
                if self.has_time {
                    let ndt = py_to_ndt(other)?;
                    let result = self.add_to_naive_datetime(ndt);
                    return ndt_to_py(py, result, None);
                } else {
                    let ndt = py_to_ndt(other)?;
                    let result = self.add_to_naive_date(ndt.date());
                    let obj = PyDate::new(py, result.year(), result.month() as u8, result.day() as u8)?;
                    return Ok(obj.into_any());
                }
            }

            // NotImplemented
            Ok(py.NotImplemented().into_bound(py).into_any())
        }

        fn __radd__<'py>(
            &self,
            other: &Bound<'py, pyo3::PyAny>,
        ) -> PyResult<Bound<'py, pyo3::PyAny>> {
            self.__add__(other)
        }

        fn __sub__<'py>(
            &self,
            other: &Bound<'py, pyo3::PyAny>,
        ) -> PyResult<Bound<'py, pyo3::PyAny>> {
            let py = other.py();
            if let Ok(rd) = other.extract::<pyo3::PyRef<'_, RelativeDelta>>() {
                let result = self.sub_rd(&rd);
                return Ok(Bound::new(py, result)?.into_any());
            }
            Ok(py.NotImplemented().into_bound(py).into_any())
        }

        fn __rsub__<'py>(
            &self,
            other: &Bound<'py, pyo3::PyAny>,
        ) -> PyResult<Bound<'py, pyo3::PyAny>> {
            self.neg().__add__(other)
        }

        fn __neg__(&self) -> Self {
            self.neg()
        }

        fn __abs__(&self) -> Self {
            self.abs()
        }

        fn __mul__<'py>(
            &self,
            other: &Bound<'py, pyo3::PyAny>,
        ) -> PyResult<Bound<'py, pyo3::PyAny>> {
            let py = other.py();
            match other.extract::<f64>() {
                Ok(f) => Ok(Bound::new(py, self.mul(f))?.into_any()),
                Err(_) => Ok(py.NotImplemented().into_bound(py).into_any()),
            }
        }

        fn __rmul__<'py>(
            &self,
            other: &Bound<'py, pyo3::PyAny>,
        ) -> PyResult<Bound<'py, pyo3::PyAny>> {
            self.__mul__(other)
        }

        fn __truediv__<'py>(
            &self,
            other: &Bound<'py, pyo3::PyAny>,
        ) -> PyResult<Bound<'py, pyo3::PyAny>> {
            let py = other.py();
            match other.extract::<f64>() {
                Ok(f) => Ok(Bound::new(py, self.mul(1.0 / f))?.into_any()),
                Err(_) => Ok(py.NotImplemented().into_bound(py).into_any()),
            }
        }

        fn __bool__(&self) -> bool {
            !self.is_zero()
        }

        fn __eq__<'py>(
            &self,
            other: &Bound<'py, pyo3::PyAny>,
        ) -> PyResult<Bound<'py, pyo3::PyAny>> {
            let py = other.py();
            let Ok(o) = other.extract::<pyo3::PyRef<'_, RelativeDelta>>() else {
                return Ok(py.NotImplemented().into_bound(py).into_any());
            };
            // Weekday comparison with None/0/1 equivalence
            let wd_eq = match (&self.weekday, &o.weekday) {
                (Some(a), Some(b)) => {
                    if a.weekday() != b.weekday() {
                        false
                    } else {
                        let n1 = a.n();
                        let n2 = b.n();
                        let default =
                            |n: Option<i32>| matches!(n, None | Some(0) | Some(1));
                        n1 == n2 || (default(n1) && default(n2))
                    }
                }
                (None, None) => true,
                _ => false,
            };
            let result = wd_eq
                && self.years == o.years
                && self.months == o.months
                && self.days == o.days
                && self.hours == o.hours
                && self.minutes == o.minutes
                && self.seconds == o.seconds
                && self.microseconds == o.microseconds
                && self.leapdays == o.leapdays
                && self.year == o.year
                && self.month == o.month
                && self.day == o.day
                && self.hour == o.hour
                && self.minute == o.minute
                && self.second == o.second
                && self.microsecond == o.microsecond;
            Ok(pyo3::types::PyBool::new(py, result).to_owned().into_any())
        }

        fn __hash__(&self) -> u64 {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            self.weekday.hash(&mut h);
            self.years.hash(&mut h);
            self.months.hash(&mut h);
            self.days.to_bits().hash(&mut h);
            self.hours.to_bits().hash(&mut h);
            self.minutes.to_bits().hash(&mut h);
            self.seconds.to_bits().hash(&mut h);
            self.microseconds.to_bits().hash(&mut h);
            self.leapdays.hash(&mut h);
            self.year.hash(&mut h);
            self.month.hash(&mut h);
            self.day.hash(&mut h);
            self.hour.hash(&mut h);
            self.minute.hash(&mut h);
            self.second.hash(&mut h);
            self.microsecond.hash(&mut h);
            h.finish()
        }

        fn __repr__(&self) -> String {
            self.to_string()
        }

        fn normalized(&self) -> PyResult<Self> {
            let days = self.days.trunc();
            let hours_f = round_to(self.hours + 24.0 * (self.days - days), 11);
            let hours = hours_f.trunc();
            let minutes_f = round_to(self.minutes + 60.0 * (hours_f - hours), 10);
            let minutes = minutes_f.trunc();
            let seconds_f = round_to(self.seconds + 60.0 * (minutes_f - minutes), 8);
            let seconds = seconds_f.trunc();
            let microseconds = (self.microseconds + 1e6 * (seconds_f - seconds)).round();

            Self::new(
                self.years,
                self.months,
                days,
                self.leapdays,
                hours,
                minutes,
                seconds,
                microseconds,
                self.year,
                self.month,
                self.day,
                self.weekday,
                None,
                None,
                self.hour,
                self.minute,
                self.second,
                self.microsecond,
            )
            .map_err(PyValueError::new_err)
        }

        // ---- properties ----

        #[getter]
        fn get_years(&self) -> i32 {
            self.years
        }
        #[getter]
        fn get_months(&self) -> i32 {
            self.months
        }
        #[getter]
        fn get_days(&self) -> i64 {
            self.days as i64
        }
        #[getter]
        fn get_leapdays(&self) -> i32 {
            self.leapdays
        }
        #[getter]
        fn get_hours(&self) -> i64 {
            self.hours as i64
        }
        #[getter]
        fn get_minutes(&self) -> i64 {
            self.minutes as i64
        }
        #[getter]
        fn get_seconds(&self) -> i64 {
            self.seconds as i64
        }
        #[getter]
        fn get_microseconds(&self) -> i64 {
            self.microseconds as i64
        }
        #[getter]
        fn get_year(&self) -> Option<i32> {
            self.year
        }
        #[getter]
        fn get_month(&self) -> Option<i32> {
            self.month
        }
        #[getter]
        fn get_day(&self) -> Option<i32> {
            self.day
        }
        #[getter]
        fn get_hour(&self) -> Option<i32> {
            self.hour
        }
        #[getter]
        fn get_minute(&self) -> Option<i32> {
            self.minute
        }
        #[getter]
        fn get_second(&self) -> Option<i32> {
            self.second
        }
        #[getter]
        fn get_microsecond(&self) -> Option<i32> {
            self.microsecond
        }
        #[getter]
        fn get_weekday(&self) -> Option<Weekday> {
            self.weekday
        }
        #[getter]
        fn get_weeks(&self) -> i64 {
            self.weeks() as i64
        }
        #[setter]
        fn set_weeks(&mut self, value: f64) {
            self.days = self.days - self.weeks() * 7.0 + value * 7.0;
        }

        // Setters for mutable fields (Python's relativedelta allows mutation)
        #[setter]
        fn set_years(&mut self, v: i32) {
            self.years = v;
        }
        #[setter]
        fn set_months(&mut self, v: i32) {
            self.months = v;
        }
        #[setter]
        fn set_days(&mut self, v: f64) {
            self.days = v;
        }
        #[setter]
        fn set_leapdays(&mut self, v: i32) {
            self.leapdays = v;
        }
        #[setter]
        fn set_hours(&mut self, v: f64) {
            self.hours = v;
        }
        #[setter]
        fn set_minutes(&mut self, v: f64) {
            self.minutes = v;
        }
        #[setter]
        fn set_seconds(&mut self, v: f64) {
            self.seconds = v;
        }
        #[setter]
        fn set_microseconds(&mut self, v: f64) {
            self.microseconds = v;
        }
        #[setter]
        fn set_year(&mut self, v: Option<i32>) {
            self.year = v;
        }
        #[setter]
        fn set_month(&mut self, v: Option<i32>) {
            self.month = v;
        }
        #[setter]
        fn set_day(&mut self, v: Option<i32>) {
            self.day = v;
        }
        #[setter]
        fn set_hour(&mut self, v: Option<i32>) {
            self.hour = v;
        }
        #[setter]
        fn set_minute(&mut self, v: Option<i32>) {
            self.minute = v;
        }
        #[setter]
        fn set_second(&mut self, v: Option<i32>) {
            self.second = v;
        }
        #[setter]
        fn set_microsecond(&mut self, v: Option<i32>) {
            self.microsecond = v;
        }
        #[setter]
        fn set_weekday(&mut self, v: Option<Weekday>) {
            self.weekday = v;
        }
    }

    // Private helper for add + timedelta
    impl RelativeDelta {
        fn add_timedelta(&self, td_days: i64, td_seconds: i64, td_microseconds: i64) -> Self {
            let mut rd = Self {
                years: self.years,
                months: self.months,
                days: self.days + td_days as f64,
                leapdays: self.leapdays,
                hours: self.hours,
                minutes: self.minutes,
                seconds: self.seconds + td_seconds as f64,
                microseconds: self.microseconds + td_microseconds as f64,
                year: self.year,
                month: self.month,
                day: self.day,
                weekday: self.weekday,
                hour: self.hour,
                minute: self.minute,
                second: self.second,
                microsecond: self.microsecond,
                has_time: false,
            };
            rd.fix();
            rd
        }
    }

    fn round_to(x: f64, decimals: i32) -> f64 {
        let factor = 10f64.powi(decimals);
        (x * factor).round() / factor
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn rd(years: i32, months: i32, days: i32) -> RelativeDelta {
        RelativeDelta::new(
            years, months, days as f64, 0, 0.0, 0.0, 0.0, 0.0, None, None, None, None, None,
            None, None, None, None, None,
        )
        .unwrap()
    }

    fn ndt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, mi, s)
            .unwrap()
    }

    #[test]
    fn test_add_months() {
        let r = rd(0, 1, 0);
        let dt = ndt(2024, 1, 15, 14, 30, 0);
        let result = r.add_to_naive_datetime(dt);
        assert_eq!(result, ndt(2024, 2, 15, 14, 30, 0));
    }

    #[test]
    fn test_month_end_overflow() {
        let r = rd(0, 1, 0);
        let dt = ndt(2024, 1, 31, 0, 0, 0);
        let result = r.add_to_naive_datetime(dt);
        // Jan 31 + 1 month = Feb 29 (2024 is leap year)
        assert_eq!(result, ndt(2024, 2, 29, 0, 0, 0));
    }

    #[test]
    fn test_add_years() {
        let r = rd(1, 0, 0);
        let dt = ndt(2024, 3, 15, 10, 0, 0);
        let result = r.add_to_naive_datetime(dt);
        assert_eq!(result, ndt(2025, 3, 15, 10, 0, 0));
    }

    #[test]
    fn test_fix_cascade() {
        // 90 minutes should cascade to 1 hour 30 minutes
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 90.0, 0.0, 0.0, None, None, None, None, None, None, None,
            None, None, None,
        )
        .unwrap();
        assert_eq!(r.hours, 1.0);
        assert_eq!(r.minutes, 30.0);
    }

    #[test]
    fn test_fix_months_cascade() {
        // 14 months = 1 year + 2 months
        let r = rd(0, 14, 0);
        assert_eq!(r.years, 1);
        assert_eq!(r.months, 2);
    }

    #[test]
    fn test_diff_same_date() {
        let dt = ndt(2024, 3, 15, 10, 0, 0);
        let r = RelativeDelta::from_diff(dt, dt);
        assert!(r.is_zero());
    }

    #[test]
    fn test_diff_months() {
        let dt1 = ndt(2024, 3, 15, 10, 0, 0);
        let dt2 = ndt(2024, 1, 15, 10, 0, 0);
        let r = RelativeDelta::from_diff(dt1, dt2);
        assert_eq!(r.years, 0);
        assert_eq!(r.months, 2);
        assert_eq!(r.days, 0.0);
    }

    #[test]
    fn test_neg() {
        let r = rd(1, 2, 3);
        let n = r.neg();
        assert_eq!(n.years, -1);
        assert_eq!(n.months, -2);
        assert_eq!(n.days, -3.0);
    }

    #[test]
    fn test_mul() {
        let r = rd(0, 1, 5);
        let m = r.mul(3.0);
        assert_eq!(m.months, 3);
        assert_eq!(m.days, 15.0);
    }

    #[test]
    fn test_weekday_next_monday() {
        let wd = Weekday::new(0, Some(1)); // MO(+1)
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 0.0, 0.0, None, None, None, Some(wd), None, None, None,
            None, None, None,
        )
        .unwrap();
        // 2024-01-15 is a Monday -> no change
        let dt = ndt(2024, 1, 15, 14, 30, 0);
        assert_eq!(r.add_to_naive_datetime(dt), dt);

        // 2024-01-16 is a Tuesday -> next Monday = 2024-01-22
        let dt2 = ndt(2024, 1, 16, 14, 30, 0);
        assert_eq!(
            r.add_to_naive_datetime(dt2),
            ndt(2024, 1, 22, 14, 30, 0)
        );
    }

    #[test]
    fn test_yearday() {
        // yearday=60 => March 1 (in non-leap year sense, leapdays=-1)
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 0.0, 0.0, None, None, None, None, Some(60), None, None,
            None, None, None,
        )
        .unwrap();
        assert_eq!(r.month, Some(3));
        assert_eq!(r.day, Some(1));
        assert_eq!(r.leapdays, -1);
    }

    // --- nlyearday ---

    #[test]
    fn test_nlyearday() {
        // nlyearday=60 => March 1 (no leapday adjustment)
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 0.0, 0.0, None, None, None, None, None, Some(60), None,
            None, None, None,
        )
        .unwrap();
        assert_eq!(r.month, Some(3));
        assert_eq!(r.day, Some(1));
        assert_eq!(r.leapdays, 0); // no leapday adjustment for nlyearday
    }

    #[test]
    fn test_nlyearday_january() {
        // nlyearday=15 => January 15
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 0.0, 0.0, None, None, None, None, None, Some(15), None,
            None, None, None,
        )
        .unwrap();
        assert_eq!(r.month, Some(1));
        assert_eq!(r.day, Some(15));
    }

    #[test]
    fn test_invalid_yearday() {
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 0.0, 0.0, None, None, None, None, Some(367), None, None,
            None, None, None,
        );
        assert!(r.is_err());
    }

    // --- from_diff with dt1 < dt2 (negative diff) ---

    #[test]
    fn test_diff_negative() {
        let dt1 = ndt(2024, 1, 15, 10, 0, 0);
        let dt2 = ndt(2024, 3, 15, 10, 0, 0);
        let r = RelativeDelta::from_diff(dt1, dt2);
        assert_eq!(r.years, 0);
        assert_eq!(r.months, -2);
        assert_eq!(r.days, 0.0);
    }

    #[test]
    fn test_diff_with_time() {
        let dt1 = ndt(2024, 3, 15, 14, 30, 45);
        let dt2 = ndt(2024, 3, 15, 10, 0, 0);
        let r = RelativeDelta::from_diff(dt1, dt2);
        assert_eq!(r.years, 0);
        assert_eq!(r.months, 0);
        // 4h30m45s = 16245 seconds total, stored as hours+minutes+seconds after fix()
        let total_secs = r.hours * 3600.0 + r.minutes * 60.0 + r.seconds;
        assert_eq!(total_secs, 16245.0);
    }

    // --- add_to_naive_date ---

    #[test]
    fn test_add_to_naive_date() {
        let r = rd(0, 1, 0);
        let d = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let result = r.add_to_naive_date(d);
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 2, 15).unwrap());
    }

    #[test]
    fn test_add_to_naive_date_with_days() {
        let r = rd(0, 0, 10);
        let d = NaiveDate::from_ymd_opt(2024, 1, 25).unwrap();
        let result = r.add_to_naive_date(d);
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 2, 4).unwrap());
    }

    #[test]
    fn test_add_to_naive_date_month_overflow() {
        let r = rd(0, 1, 0);
        let d = NaiveDate::from_ymd_opt(2024, 12, 15).unwrap();
        let result = r.add_to_naive_date(d);
        assert_eq!(result, NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());
    }

    #[test]
    fn test_add_to_naive_date_month_underflow() {
        let r = rd(0, -1, 0);
        let d = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let result = r.add_to_naive_date(d);
        assert_eq!(result, NaiveDate::from_ymd_opt(2023, 12, 15).unwrap());
    }

    #[test]
    fn test_add_to_naive_date_with_weekday() {
        let wd = Weekday::new(4, Some(1)); // FR(+1)
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 0.0, 0.0, None, None, None, Some(wd), None, None, None,
            None, None, None,
        )
        .unwrap();
        // 2024-01-15 is Monday -> next Friday = 2024-01-19
        let d = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let result = r.add_to_naive_date(d);
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 19).unwrap());
    }

    // --- add_to_naive_datetime with month overflow/underflow ---

    #[test]
    fn test_add_datetime_month_overflow() {
        let r = rd(0, 1, 0);
        let dt = ndt(2024, 12, 15, 10, 0, 0);
        let result = r.add_to_naive_datetime(dt);
        assert_eq!(result, ndt(2025, 1, 15, 10, 0, 0));
    }

    #[test]
    fn test_add_datetime_month_underflow() {
        let r = rd(0, -1, 0);
        let dt = ndt(2024, 1, 15, 10, 0, 0);
        let result = r.add_to_naive_datetime(dt);
        assert_eq!(result, ndt(2023, 12, 15, 10, 0, 0));
    }

    // --- leapdays ---

    #[test]
    fn test_leapdays_in_leap_year() {
        // leapdays=1 should add 1 day when month > 2 in a leap year
        let r = RelativeDelta::new(
            0, 0, 0.0, 1, 0.0, 0.0, 0.0, 0.0, None, None, None, None, None, None, None,
            None, None, None,
        )
        .unwrap();
        let dt = ndt(2024, 3, 1, 0, 0, 0); // March 1, 2024 (leap year)
        let result = r.add_to_naive_datetime(dt);
        assert_eq!(result, ndt(2024, 3, 2, 0, 0, 0));
    }

    #[test]
    fn test_leapdays_in_non_leap_year() {
        let r = RelativeDelta::new(
            0, 0, 0.0, 1, 0.0, 0.0, 0.0, 0.0, None, None, None, None, None, None, None,
            None, None, None,
        )
        .unwrap();
        let dt = ndt(2023, 3, 1, 0, 0, 0); // not a leap year
        let result = r.add_to_naive_datetime(dt);
        assert_eq!(result, ndt(2023, 3, 1, 0, 0, 0)); // no change
    }

    #[test]
    fn test_leapdays_in_date() {
        let r = RelativeDelta::new(
            0, 0, 0.0, 1, 0.0, 0.0, 0.0, 0.0, None, None, None, None, None, None, None,
            None, None, None,
        )
        .unwrap();
        let d = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
        let result = r.add_to_naive_date(d);
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 3, 2).unwrap());
    }

    // --- add_rd / sub_rd ---

    #[test]
    fn test_add_rd() {
        let a = rd(1, 2, 3);
        let b = rd(0, 3, 7);
        let result = a.add_rd(&b);
        assert_eq!(result.years, 1);
        assert_eq!(result.months, 5);
        assert_eq!(result.days, 10.0);
    }

    #[test]
    fn test_sub_rd() {
        let a = rd(2, 5, 10);
        let b = rd(1, 2, 3);
        let result = a.sub_rd(&b);
        assert_eq!(result.years, 1);
        assert_eq!(result.months, 3);
        assert_eq!(result.days, 7.0);
    }

    // --- abs ---

    #[test]
    fn test_abs() {
        let r = rd(-1, -2, -3);
        let a = r.abs();
        assert_eq!(a.years, 1);
        assert_eq!(a.months, 2);
        assert_eq!(a.days, 3.0);
    }

    // --- weeks / has_time ---

    #[test]
    fn test_weeks() {
        let r = rd(0, 0, 14);
        assert_eq!(r.weeks(), 2.0);
    }

    #[test]
    fn test_weeks_partial() {
        let r = rd(0, 0, 10);
        assert_eq!(r.weeks(), 1.0); // trunc(10/7) = 1
    }

    #[test]
    fn test_has_time_false() {
        let r = rd(1, 2, 3);
        assert!(!r.has_time());
    }

    #[test]
    fn test_has_time_true() {
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 5.0, 0.0, 0.0, 0.0, None, None, None, None, None, None, None,
            None, None, None,
        )
        .unwrap();
        assert!(r.has_time());
    }

    #[test]
    fn test_has_time_absolute() {
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 0.0, 0.0, None, None, None, None, None, None,
            Some(10), None, None, None,
        )
        .unwrap();
        assert!(r.has_time());
    }

    // --- is_zero ---

    #[test]
    fn test_is_zero_true() {
        let r = rd(0, 0, 0);
        assert!(r.is_zero());
    }

    #[test]
    fn test_is_zero_false() {
        let r = rd(0, 0, 1);
        assert!(!r.is_zero());
    }

    // --- fix cascade for large values ---

    #[test]
    fn test_fix_microseconds_cascade() {
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 0.0, 2_500_000.0, None, None, None, None, None, None,
            None, None, None, None,
        )
        .unwrap();
        assert_eq!(r.seconds, 2.0);
        assert_eq!(r.microseconds, 500_000.0);
    }

    #[test]
    fn test_fix_seconds_cascade() {
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 150.0, 0.0, None, None, None, None, None, None, None,
            None, None, None,
        )
        .unwrap();
        assert_eq!(r.minutes, 2.0);
        assert_eq!(r.seconds, 30.0);
    }

    #[test]
    fn test_fix_hours_cascade() {
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 50.0, 0.0, 0.0, 0.0, None, None, None, None, None, None, None,
            None, None, None,
        )
        .unwrap();
        assert_eq!(r.days, 2.0);
        assert_eq!(r.hours, 2.0);
    }

    #[test]
    fn test_fix_negative_months_cascade() {
        let r = rd(0, -14, 0);
        assert_eq!(r.years, -1);
        assert_eq!(r.months, -2);
    }

    // --- Display ---

    #[test]
    fn test_display_zero() {
        let r = rd(0, 0, 0);
        assert_eq!(format!("{}", r), "relativedelta()");
    }

    #[test]
    fn test_display_relative() {
        let r = rd(1, 2, 3);
        let s = format!("{}", r);
        assert!(s.contains("years=+1"));
        assert!(s.contains("months=+2"));
        assert!(s.contains("days=+3"));
    }

    #[test]
    fn test_display_absolute() {
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 0.0, 0.0, Some(2024), Some(3), Some(15), None, None,
            None, Some(10), Some(30), Some(0), None,
        )
        .unwrap();
        let s = format!("{}", r);
        assert!(s.contains("year=2024"));
        assert!(s.contains("month=3"));
        assert!(s.contains("day=15"));
        assert!(s.contains("hour=10"));
        assert!(s.contains("minute=30"));
    }

    #[test]
    fn test_display_with_weekday() {
        let wd = Weekday::new(0, Some(1));
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 0.0, 0.0, None, None, None, Some(wd), None, None, None,
            None, None, None,
        )
        .unwrap();
        let s = format!("{}", r);
        assert!(s.contains("weekday=MO(+1)"));
    }

    // --- apply_weekday with negative n ---

    #[test]
    fn test_weekday_negative_n() {
        let wd = Weekday::new(0, Some(-1)); // MO(-1) = last Monday
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 0.0, 0.0, None, None, None, Some(wd), None, None, None,
            None, None, None,
        )
        .unwrap();
        // 2024-01-17 is Wednesday -> last Monday before = 2024-01-15
        let dt = ndt(2024, 1, 17, 14, 0, 0);
        let result = r.add_to_naive_datetime(dt);
        assert_eq!(result, ndt(2024, 1, 15, 14, 0, 0));
    }

    #[test]
    fn test_weekday_negative_2() {
        let wd = Weekday::new(4, Some(-2)); // FR(-2) = 2nd-to-last Friday
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 0.0, 0.0, None, None, None, Some(wd), None, None, None,
            None, None, None,
        )
        .unwrap();
        // 2024-01-20 is Saturday -> previous Friday = Jan 19, 2nd previous = Jan 12
        let dt = ndt(2024, 1, 20, 0, 0, 0);
        let result = r.add_to_naive_datetime(dt);
        assert_eq!(result, ndt(2024, 1, 12, 0, 0, 0));
    }

    // --- absolute fields in add_to_naive_datetime ---

    #[test]
    fn test_absolute_year_month_day() {
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 0.0, 0.0, Some(2025), Some(6), Some(15), None, None,
            None, None, None, None, None,
        )
        .unwrap();
        let dt = ndt(2024, 1, 1, 12, 0, 0);
        let result = r.add_to_naive_datetime(dt);
        assert_eq!(result, ndt(2025, 6, 15, 12, 0, 0));
    }

    #[test]
    fn test_absolute_hour_minute_second() {
        let r = RelativeDelta::new(
            0, 0, 0.0, 0, 0.0, 0.0, 0.0, 0.0, None, None, None, None, None, None,
            Some(14), Some(30), Some(45), None,
        )
        .unwrap();
        let dt = ndt(2024, 1, 15, 0, 0, 0);
        let result = r.add_to_naive_datetime(dt);
        assert_eq!(result, ndt(2024, 1, 15, 14, 30, 45));
    }
}
