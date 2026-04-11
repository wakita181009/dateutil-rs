use super::common::PyWeekday;
use super::conv;
use chrono::{Datelike, NaiveDateTime};
use dateutil_core::common;
use dateutil_core::relativedelta::{RelativeDelta, RelativeDeltaBuilder};
use pyo3::prelude::*;
use pyo3::types::{PyDate, PyDateTime, PyDelta, PyDeltaAccess, PyTzInfoAccess};

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
        dt1=None, dt2=None,
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
        dt1: Option<&Bound<'_, PyAny>>,
        dt2: Option<&Bound<'_, PyAny>>,
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
        weekday: Option<Bound<'_, PyAny>>,
        yearday: Option<i32>,
        nlyearday: Option<i32>,
        hour: Option<i32>,
        minute: Option<i32>,
        second: Option<i32>,
        microsecond: Option<i32>,
    ) -> PyResult<Self> {
        // If both dt1 and dt2 are provided, compute the difference
        // (matches python-dateutil's relativedelta(dt1, dt2) API)
        if let (Some(d1), Some(d2)) = (dt1, dt2) {
            let (ndt1, aware1) = py_any_to_ndt_for_diff(d1)?;
            let (ndt2, aware2) = py_any_to_ndt_for_diff(d2)?;

            if aware1 != aware2 {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "can't compare offset-naive and offset-aware datetimes",
                ));
            }

            return Ok(Self {
                inner: RelativeDelta::from_diff(ndt1, ndt2),
            });
        }

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
        if let Some(ref wd) = weekday {
            let core_wd = if let Ok(py_wd) = wd.extract::<PyWeekday>() {
                py_wd.into()
            } else {
                let day: u8 = wd.extract()?;
                common::Weekday::try_from(day)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
            };
            builder = builder.weekday(core_wd);
        }
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

    /// Create a relativedelta from the difference between two dates/datetimes.
    ///
    /// Both arguments must be either both naive or both aware. Mixed
    /// naive/aware raises `TypeError`, matching python-dateutil behaviour.
    /// When both are aware, datetimes are UTC-normalised before diffing.
    #[staticmethod]
    fn from_diff(dt1: &Bound<'_, PyAny>, dt2: &Bound<'_, PyAny>) -> PyResult<Self> {
        let (ndt1, aware1) = py_any_to_ndt_for_diff(dt1)?;
        let (ndt2, aware2) = py_any_to_ndt_for_diff(dt2)?;

        if aware1 != aware2 {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "can't compare offset-naive and offset-aware datetimes",
            ));
        }

        Ok(Self {
            inner: RelativeDelta::from_diff(ndt1, ndt2),
        })
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
    #[setter]
    fn set_weeks(&mut self, val: i32) { self.inner.set_weeks(val); }
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

    fn __add__<'py>(
        &self,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = other.py();

        // relativedelta + relativedelta
        if let Ok(rd) = other.extract::<PyRef<'_, PyRelativeDelta>>() {
            let result = Self { inner: self.inner.add_rd(&rd.inner) };
            return Ok(Bound::new(py, result)?.into_any());
        }

        // relativedelta + timedelta
        if let Ok(td) = other.cast::<PyDelta>() {
            let td_rd = timedelta_to_rd(td)?;
            let result = Self { inner: self.inner.add_rd(&td_rd) };
            return Ok(Bound::new(py, result)?.into_any());
        }

        // relativedelta + datetime (check BEFORE date — datetime is a date subclass)
        if let Ok(dt) = other.cast::<PyDateTime>() {
            let ndt = conv::pydt_to_naive(dt);
            let result = self.inner.add_to_naive_datetime(ndt);
            let tzinfo = dt.get_tzinfo();
            return conv::ndt_to_py_datetime(py, result, tzinfo.as_ref());
        }

        // relativedelta + date
        if other.cast::<PyDate>().is_ok() {
            if self.inner.has_time() {
                let ndt = conv::py_any_to_naive_datetime(other)?;
                let result = self.inner.add_to_naive_datetime(ndt);
                return conv::ndt_to_py_datetime(py, result, None);
            }
            let nd = conv::py_any_to_naive_date(other)?;
            let result = self.inner.add_to_naive_date(nd);
            let obj = PyDate::new(py, result.year(), result.month() as u8, result.day() as u8)?;
            return Ok(obj.into_any());
        }

        Ok(py.NotImplemented().into_bound(py).into_any())
    }

    fn __radd__<'py>(
        &self,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.__add__(other)
    }

    fn __sub__<'py>(
        &self,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = other.py();
        if let Ok(rd) = other.extract::<PyRef<'_, PyRelativeDelta>>() {
            let result = Self { inner: self.inner.sub_rd(&rd.inner) };
            return Ok(Bound::new(py, result)?.into_any());
        }
        Ok(py.NotImplemented().into_bound(py).into_any())
    }

    fn __rsub__<'py>(
        &self,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let neg = Self { inner: self.inner.neg() };
        neg.__add__(other)
    }

    fn __neg__(&self) -> Self {
        Self {
            inner: self.inner.neg(),
        }
    }

    fn __mul__<'py>(
        &self,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = other.py();
        match other.extract::<f64>() {
            Ok(f) => Ok(Bound::new(py, Self { inner: self.inner.mul(f) })?.into_any()),
            Err(_) => Ok(py.NotImplemented().into_bound(py).into_any()),
        }
    }

    fn __rmul__<'py>(
        &self,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.__mul__(other)
    }

    fn __truediv__<'py>(
        &self,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = other.py();
        match other.extract::<f64>() {
            Ok(f) => Ok(Bound::new(py, Self { inner: self.inner.div(f) })?.into_any()),
            Err(_) => Ok(py.NotImplemented().into_bound(py).into_any()),
        }
    }

    fn __abs__(&self) -> Self {
        Self {
            inner: self.inner.abs(),
        }
    }

    fn __eq__(&self, other: &PyRelativeDelta) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
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

// --- timedelta → RelativeDelta conversion ---

/// Convert a Python `datetime.timedelta` to a `RelativeDelta` with
/// equivalent days, seconds, and microseconds.
fn timedelta_to_rd(td: &Bound<'_, PyDelta>) -> PyResult<RelativeDelta> {
    RelativeDeltaBuilder::new()
        .days(td.get_days())
        .seconds(td.get_seconds())
        .microseconds(td.get_microseconds() as i64)
        .build()
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

// --- from_diff-specific helper (awareness check) ---

/// Extract `NaiveDateTime` from a Python datetime/date for `from_diff`,
/// along with an awareness flag. Wall clock time is preserved as-is
/// (no UTC normalisation) to match python-dateutil semantics where
/// `relativedelta(dt1, dt2)` operates on local wall time.
/// `date` objects are always naive.
fn py_any_to_ndt_for_diff(obj: &Bound<'_, PyAny>) -> PyResult<(NaiveDateTime, bool)> {
    // datetime first (subclass of date)
    if let Ok(dt) = obj.cast::<PyDateTime>() {
        let ndt = conv::pydt_to_naive(dt);
        let aware = if let Some(ref tzinfo) = dt.get_tzinfo() {
            let utcoffset_obj = tzinfo.call_method1("utcoffset", (obj,))?;
            !utcoffset_obj.is_none()
        } else {
            false
        };
        return Ok((ndt, aware));
    }
    // date or other — always naive
    Ok((conv::py_any_to_naive_datetime(obj)?, false))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRelativeDelta>()?;
    Ok(())
}
