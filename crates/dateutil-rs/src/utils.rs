use chrono::{NaiveDateTime, TimeDelta};

/// Returns true if the absolute difference between dt1 and dt2 is within the
/// given delta (inclusive). The delta is treated as its absolute value,
/// matching Python's `abs(delta)` behavior.
pub fn within_delta(dt1: NaiveDateTime, dt2: NaiveDateTime, delta: TimeDelta) -> bool {
    let delta = if delta < TimeDelta::zero() {
        -delta
    } else {
        delta
    };
    let difference = dt1 - dt2;
    -delta <= difference && difference <= delta
}

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
fn py_datetime_to_naive(dt: &Bound<'_, pyo3::types::PyDateTime>) -> PyResult<NaiveDateTime> {
    use chrono::NaiveDate;
    use pyo3::types::{PyDateAccess, PyTimeAccess};

    let date = NaiveDate::from_ymd_opt(
        dt.get_year(),
        dt.get_month() as u32,
        dt.get_day() as u32,
    )
    .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("invalid date"))?;
    let time = chrono::NaiveTime::from_hms_micro_opt(
        dt.get_hour() as u32,
        dt.get_minute() as u32,
        dt.get_second() as u32,
        dt.get_microsecond(),
    )
    .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("invalid time"))?;

    Ok(NaiveDateTime::new(date, time))
}

#[cfg(feature = "python")]
fn py_delta_to_timedelta(delta: &Bound<'_, pyo3::types::PyDelta>) -> PyResult<TimeDelta> {
    use pyo3::types::PyDeltaAccess;

    let days = delta.get_days() as i64;
    let seconds = delta.get_seconds() as i64;
    let microseconds = delta.get_microseconds() as i64;
    let total_us = days * 86_400_000_000 + seconds * 1_000_000 + microseconds;
    Ok(TimeDelta::microseconds(total_us))
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "within_delta")]
pub fn within_delta_py(
    dt1: &Bound<'_, pyo3::types::PyDateTime>,
    dt2: &Bound<'_, pyo3::types::PyDateTime>,
    delta: &Bound<'_, pyo3::types::PyDelta>,
) -> PyResult<bool> {
    let ndt1 = py_datetime_to_naive(dt1)?;
    let ndt2 = py_datetime_to_naive(dt2)?;
    let td = py_delta_to_timedelta(delta)?;
    Ok(within_delta(ndt1, ndt2, td))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn ndt(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32, us: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_micro_opt(h, min, s, us)
            .unwrap()
    }

    #[test]
    fn test_within_delta_true() {
        let d1 = ndt(2016, 1, 1, 12, 14, 1, 9);
        let d2 = ndt(2016, 1, 1, 12, 14, 1, 15);
        assert!(within_delta(d1, d2, TimeDelta::seconds(1)));
    }

    #[test]
    fn test_within_delta_false() {
        let d1 = ndt(2016, 1, 1, 12, 14, 1, 9);
        let d2 = ndt(2016, 1, 1, 12, 14, 1, 15);
        assert!(!within_delta(d1, d2, TimeDelta::microseconds(1)));
    }

    #[test]
    fn test_within_delta_negative_delta() {
        let d1 = ndt(2016, 1, 1, 0, 0, 0, 0);
        let d2 = ndt(2015, 12, 31, 0, 0, 0, 0);
        assert!(within_delta(d2, d1, TimeDelta::try_days(-1).unwrap()));
    }

    #[test]
    fn test_within_delta_exact_boundary() {
        let d1 = ndt(2016, 1, 1, 0, 0, 0, 0);
        let d2 = ndt(2016, 1, 1, 0, 0, 1, 0);
        assert!(within_delta(d1, d2, TimeDelta::seconds(1)));
    }

    #[test]
    fn test_within_delta_zero_difference() {
        let d1 = ndt(2016, 1, 1, 0, 0, 0, 0);
        assert!(within_delta(d1, d1, TimeDelta::zero()));
    }
}
