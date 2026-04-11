//! Shared Python ↔ chrono conversion helpers.
//!
//! Centralises NaiveDateTime extraction, Python datetime construction,
//! and timezone/timedelta building so that parser, tz, and relativedelta
//! modules share a single implementation.

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use pyo3::prelude::*;
use pyo3::types::{
    PyDate, PyDateAccess, PyDateTime, PyDelta, PyTimeAccess, PyTzInfo,
};

// ---------------------------------------------------------------------------
// Python → chrono
// ---------------------------------------------------------------------------

/// Extract `NaiveDateTime` and `fold` flag from a Python `datetime.datetime`
/// in a single cast. All fields use C-level getters (zero Python attribute
/// lookups).
#[inline]
pub fn extract_ndt_fold(dt: &Bound<'_, PyAny>) -> PyResult<(NaiveDateTime, bool)> {
    let pydt = dt.cast::<PyDateTime>()?;
    // Python datetime always carries valid components — unwrap is safe.
    let date = NaiveDate::from_ymd_opt(
        pydt.get_year(),
        pydt.get_month().into(),
        pydt.get_day().into(),
    )
    .unwrap();
    let time = NaiveTime::from_hms_micro_opt(
        pydt.get_hour().into(),
        pydt.get_minute().into(),
        pydt.get_second().into(),
        pydt.get_microsecond(),
    )
    .unwrap();
    Ok((NaiveDateTime::new(date, time), pydt.get_fold()))
}

/// Extract `NaiveDateTime` only (ignoring fold). Used where fold is
/// irrelevant.
#[inline]
pub fn extract_ndt(dt: &Bound<'_, PyAny>) -> PyResult<NaiveDateTime> {
    extract_ndt_fold(dt).map(|(ndt, _)| ndt)
}

/// Extract `NaiveDateTime` from an already-cast `PyDateTime` reference.
/// Python datetime components are always valid, so this never fails.
#[inline]
pub fn pydt_to_naive(dt: &Bound<'_, PyDateTime>) -> NaiveDateTime {
    let date = NaiveDate::from_ymd_opt(
        dt.get_year(),
        dt.get_month() as u32,
        dt.get_day() as u32,
    )
    .unwrap();
    let time = NaiveTime::from_hms_micro_opt(
        dt.get_hour() as u32,
        dt.get_minute() as u32,
        dt.get_second() as u32,
        dt.get_microsecond(),
    )
    .unwrap();
    NaiveDateTime::new(date, time)
}

/// Convert a Python `datetime.date` or `datetime.datetime` to
/// `NaiveDateTime`. datetime → extract all fields; date → midnight.
pub fn py_any_to_naive_datetime(obj: &Bound<'_, PyAny>) -> PyResult<NaiveDateTime> {
    // datetime first (subclass of date)
    if let Ok(dt) = obj.cast::<PyDateTime>() {
        return Ok(pydt_to_naive(dt));
    }
    if let Ok(d) = obj.cast::<PyDate>() {
        let date = NaiveDate::from_ymd_opt(
            d.get_year(),
            d.get_month() as u32,
            d.get_day() as u32,
        )
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("invalid date"))?;
        return Ok(date.and_hms_opt(0, 0, 0).unwrap());
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected datetime.date or datetime.datetime",
    ))
}

/// Extract `NaiveDate` from a Python `datetime.date` object.
pub fn py_any_to_naive_date(obj: &Bound<'_, PyAny>) -> PyResult<NaiveDate> {
    if let Ok(d) = obj.cast::<PyDate>() {
        return NaiveDate::from_ymd_opt(
            d.get_year(),
            d.get_month() as u32,
            d.get_day() as u32,
        )
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("invalid date"));
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected datetime.date",
    ))
}

// ---------------------------------------------------------------------------
// chrono → Python
// ---------------------------------------------------------------------------

/// Build a Python `datetime.datetime` from `NaiveDateTime` + optional
/// tzinfo via C-API.
#[inline]
pub fn ndt_to_py_datetime<'py>(
    py: Python<'py>,
    ndt: NaiveDateTime,
    tzinfo: Option<&Bound<'py, PyTzInfo>>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyDateTime::new(
        py,
        ndt.year(),
        ndt.month() as u8,
        ndt.day() as u8,
        ndt.hour() as u8,
        ndt.minute() as u8,
        ndt.second() as u8,
        (ndt.nanosecond() / 1000) % 1_000_000,
        tzinfo,
    )?
    .into_any())
}

/// Build an aware `datetime.datetime` with explicit fold flag.
#[inline]
pub fn ndt_to_py_datetime_with_fold<'py>(
    py: Python<'py>,
    ndt: NaiveDateTime,
    tz: &Bound<'py, PyTzInfo>,
    fold: bool,
) -> PyResult<Bound<'py, PyDateTime>> {
    PyDateTime::new_with_fold(
        py,
        ndt.year(),
        ndt.month() as u8,
        ndt.day() as u8,
        ndt.hour() as u8,
        ndt.minute() as u8,
        ndt.second() as u8,
        (ndt.nanosecond() / 1000) % 1_000_000,
        Some(tz),
        fold,
    )
}

// ---------------------------------------------------------------------------
// Scalar-or-sequence extraction helpers
// ---------------------------------------------------------------------------

/// Accept either a single `i32` or a list of `i32`.
pub fn extract_i32_list(obj: &Bound<'_, PyAny>) -> PyResult<Vec<i32>> {
    if let Ok(v) = obj.extract::<i32>() {
        return Ok(vec![v]);
    }
    obj.extract::<Vec<i32>>()
}

/// Accept either a single `u8` or a list of `u8`.
pub fn extract_u8_list(obj: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    if let Ok(v) = obj.extract::<u8>() {
        return Ok(vec![v]);
    }
    obj.extract::<Vec<u8>>()
}

// ---------------------------------------------------------------------------
// Timezone / timedelta helpers
// ---------------------------------------------------------------------------

/// Build a fixed-offset `datetime.timezone` via C-API (no `py.import`).
#[inline]
pub fn make_py_tz<'py>(py: Python<'py>, offset_seconds: i32) -> PyResult<Bound<'py, PyTzInfo>> {
    let days = offset_seconds.div_euclid(86400);
    let remaining = offset_seconds.rem_euclid(86400);
    let td = PyDelta::new(py, days, remaining, 0, false)?;
    PyTzInfo::fixed_offset(py, td)
}

/// Get `datetime.timezone.utc` via C-API (cached internally by PyO3).
#[inline]
pub fn make_py_utc<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyTzInfo>> {
    PyTzInfo::utc(py).map(|b| b.to_owned())
}

/// Convert total seconds to a Python `datetime.timedelta`.
#[inline]
pub fn secs_to_pydelta<'py>(py: Python<'py>, total_secs: i32) -> PyResult<Bound<'py, PyDelta>> {
    let days = total_secs.div_euclid(86400);
    let remaining = total_secs.rem_euclid(86400);
    PyDelta::new(py, days, remaining, 0, false)
}
