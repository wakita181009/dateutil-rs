use super::common::PyWeekday;
use dateutil_core::relativedelta::{RelativeDelta, RelativeDeltaBuilder};
use pyo3::prelude::*;

/// Python wrapper for dateutil_core::relativedelta::RelativeDelta.
#[pyclass(name = "relativedelta", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyRelativeDelta {
    inner: RelativeDelta,
}

#[pymethods]
impl PyRelativeDelta {
    #[new]
    #[pyo3(signature = (
        years=0, months=0, days=0, weeks=0,
        hours=0, minutes=0, seconds=0, microseconds=0,
        leapdays=0,
        year=None, month=None, day=None,
        weekday=None,
        yearday=None, nlyearday=None,
        hour=None, minute=None, second=None, microsecond=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        years: i32,
        months: i32,
        days: i32,
        weeks: i32,
        hours: i32,
        minutes: i32,
        seconds: i32,
        microseconds: i64,
        leapdays: i32,
        year: Option<i32>,
        month: Option<i32>,
        day: Option<i32>,
        weekday: Option<PyWeekday>,
        yearday: Option<i32>,
        nlyearday: Option<i32>,
        hour: Option<i32>,
        minute: Option<i32>,
        second: Option<i32>,
        microsecond: Option<i32>,
    ) -> PyResult<Self> {
        let mut builder = RelativeDeltaBuilder::new()
            .years(years)
            .months(months)
            .days(days)
            .weeks(weeks)
            .hours(hours)
            .minutes(minutes)
            .seconds(seconds)
            .microseconds(microseconds)
            .leapdays(leapdays);

        if let Some(v) = year { builder = builder.year(v); }
        if let Some(v) = month { builder = builder.month(v); }
        if let Some(v) = day { builder = builder.day(v); }
        if let Some(wd) = weekday { builder = builder.weekday(wd.into()); }
        if let Some(v) = yearday { builder = builder.yearday(v); }
        if let Some(v) = nlyearday { builder = builder.nlyearday(v); }
        if let Some(v) = hour { builder = builder.hour(v); }
        if let Some(v) = minute { builder = builder.minute(v); }
        if let Some(v) = second { builder = builder.second(v); }
        if let Some(v) = microsecond { builder = builder.microsecond(v); }

        let inner = builder
            .build()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Create a relativedelta from the difference between two datetimes.
    #[staticmethod]
    fn from_diff(dt1: chrono::NaiveDateTime, dt2: chrono::NaiveDateTime) -> Self {
        Self {
            inner: RelativeDelta::from_diff(dt1, dt2),
        }
    }

    /// Add this delta to a datetime.
    fn add_to_datetime(&self, dt: chrono::NaiveDateTime) -> chrono::NaiveDateTime {
        self.inner.add_to_naive_datetime(dt)
    }

    /// Add this delta to a date (date-only arithmetic).
    fn add_to_date(&self, dt: chrono::NaiveDate) -> chrono::NaiveDate {
        self.inner.add_to_naive_date(dt)
    }

    // Relative field getters
    #[getter]
    fn years(&self) -> i32 { self.inner.years() }
    #[getter]
    fn months(&self) -> i32 { self.inner.months() }
    #[getter]
    fn days(&self) -> i32 { self.inner.days() }
    #[getter]
    fn hours(&self) -> i32 { self.inner.hours() }
    #[getter]
    fn minutes(&self) -> i32 { self.inner.minutes() }
    #[getter]
    fn seconds(&self) -> i32 { self.inner.seconds() }
    #[getter]
    fn microseconds(&self) -> i64 { self.inner.microseconds() }
    #[getter]
    fn weeks(&self) -> i32 { self.inner.weeks() }
    #[getter]
    fn leapdays(&self) -> i32 { self.inner.leapdays() }

    // Absolute field getters (None if not set)
    #[getter]
    fn year(&self) -> Option<i32> { self.inner.year() }
    #[getter]
    fn month(&self) -> Option<i32> { self.inner.month() }
    #[getter]
    fn day(&self) -> Option<i32> { self.inner.day() }
    #[getter]
    fn hour(&self) -> Option<i32> { self.inner.hour() }
    #[getter]
    fn minute(&self) -> Option<i32> { self.inner.minute() }
    #[getter]
    fn second(&self) -> Option<i32> { self.inner.second() }
    #[getter]
    fn microsecond(&self) -> Option<i32> { self.inner.microsecond() }
    #[getter]
    fn weekday(&self) -> Option<PyWeekday> {
        self.inner.weekday().map(|w| PyWeekday::from(*w))
    }

    fn has_time(&self) -> bool { self.inner.has_time() }
    fn is_zero(&self) -> bool { self.inner.is_zero() }

    // Arithmetic operations

    fn __add__(&self, other: &PyRelativeDelta) -> Self {
        Self {
            inner: self.inner.add_rd(&other.inner),
        }
    }

    fn __sub__(&self, other: &PyRelativeDelta) -> Self {
        Self {
            inner: self.inner.sub_rd(&other.inner),
        }
    }

    fn __neg__(&self) -> Self {
        Self {
            inner: self.inner.neg(),
        }
    }

    fn __mul__(&self, factor: f64) -> Self {
        Self {
            inner: self.inner.mul(factor),
        }
    }

    fn __rmul__(&self, factor: f64) -> Self {
        Self {
            inner: self.inner.mul(factor),
        }
    }

    fn __eq__(&self, other: &PyRelativeDelta) -> bool {
        self.inner == other.inner
    }

    fn __bool__(&self) -> bool {
        !self.inner.is_zero()
    }

    fn __repr__(&self) -> String {
        self.inner.to_string()
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRelativeDelta>()?;
    Ok(())
}
