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

// ============================================================================
// PyO3 bindings
// ============================================================================

#[cfg(feature = "python")]
pub mod python {
    use pyo3::prelude::*;

    /// PyO3 wrapper for within_delta.
    #[pyfunction]
    #[pyo3(name = "within_delta")]
    pub fn within_delta_py(
        dt1: chrono::NaiveDateTime,
        dt2: chrono::NaiveDateTime,
        delta: chrono::TimeDelta,
    ) -> bool {
        super::within_delta(dt1, dt2, delta)
    }

    /// Returns a datetime representing the current day at midnight.
    ///
    /// Equivalent to Python's:
    ///   dt = datetime.now(tzinfo)
    ///   return datetime.combine(dt.date(), time(0, tzinfo=tzinfo))
    #[pyfunction]
    #[pyo3(name = "today", signature = (tzinfo=None))]
    pub fn today_py<'py>(py: Python<'py>, tzinfo: Option<&Bound<'py, PyAny>>) -> PyResult<Bound<'py, PyAny>> {
        let datetime_mod = py.import("datetime")?;
        let datetime_cls = datetime_mod.getattr("datetime")?;
        let time_cls = datetime_mod.getattr("time")?;

        // datetime.now(tzinfo)
        let now = match tzinfo {
            Some(tz) => datetime_cls.call_method1("now", (tz,))?,
            None => datetime_cls.call_method0("now")?,
        };

        // dt.date()
        let date = now.call_method0("date")?;

        // time(0, tzinfo=tzinfo)
        let midnight = {
            let kwargs = pyo3::types::PyDict::new(py);
            kwargs.set_item("tzinfo", tzinfo)?;
            time_cls.call((0,), Some(&kwargs))?
        };

        // datetime.combine(date, midnight)
        datetime_cls.call_method1("combine", (date, midnight))
    }

    /// Sets the tzinfo parameter on naive datetimes only.
    ///
    /// If dt already has a tzinfo, returns dt unchanged.
    /// Otherwise, returns dt.replace(tzinfo=tzinfo).
    #[pyfunction]
    #[pyo3(name = "default_tzinfo")]
    pub fn default_tzinfo_py<'py>(
        py: Python<'py>,
        dt: &Bound<'py, PyAny>,
        tzinfo: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let existing = dt.getattr("tzinfo")?;
        if !existing.is_none() {
            return Ok(dt.clone());
        }
        let kwargs = pyo3::types::PyDict::new(py);
        kwargs.set_item("tzinfo", tzinfo)?;
        dt.call_method("replace", (), Some(&kwargs))
    }

    /// Register utils functions with the parent module.
    pub fn register(m: &Bound<'_, pyo3::types::PyModule>) -> PyResult<()> {
        m.add_function(pyo3::wrap_pyfunction!(within_delta_py, m)?)?;
        m.add_function(pyo3::wrap_pyfunction!(today_py, m)?)?;
        m.add_function(pyo3::wrap_pyfunction!(default_tzinfo_py, m)?)?;
        Ok(())
    }
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
