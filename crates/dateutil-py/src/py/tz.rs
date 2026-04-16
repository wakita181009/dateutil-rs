use chrono::NaiveDateTime;
use dateutil::tz::{self, TimeZone, TzFile, TzLocal, TzOffset, TzOps};
use pyo3::prelude::*;
use pyo3::types::{PyDateTime, PyDelta, PyTzInfo};

use super::conv::{extract_ndt, extract_ndt_fold, ndt_to_py_datetime_with_fold, secs_to_pydelta};

// ---------------------------------------------------------------------------
// PyTzUtc — extends datetime.tzinfo
// ---------------------------------------------------------------------------

#[pyclass(name = "tzutc", extends = PyTzInfo, frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyTzUtc;

#[pymethods]
impl PyTzUtc {
    #[new]
    fn new() -> Self {
        Self
    }

    fn utcoffset<'py>(
        &self,
        py: Python<'py>,
        _dt: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyDelta>> {
        secs_to_pydelta(py, 0)
    }

    fn dst<'py>(
        &self,
        py: Python<'py>,
        _dt: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyDelta>> {
        secs_to_pydelta(py, 0)
    }

    fn tzname(&self, _dt: Option<&Bound<'_, PyAny>>) -> &str {
        "UTC"
    }

    fn is_ambiguous(&self, _dt: &Bound<'_, PyAny>) -> bool {
        false
    }

    fn fromutc<'py>(
        slf: &Bound<'py, Self>,
        dt: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyDateTime>> {
        let py = slf.py();
        let ndt = extract_ndt(dt)?;
        let tz = slf.cast::<PyTzInfo>()?;
        ndt_to_py_datetime_with_fold(py, ndt, tz, false)
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if other.is_instance_of::<PyTzUtc>() {
            return Ok(true);
        }
        if let Ok(off) = other.extract::<PyRef<'_, PyTzOffset>>() {
            return Ok(off.inner.offset_seconds() == 0);
        }
        Ok(false)
    }

    fn __hash__(&self) -> u64 {
        0 // Consistent: all UTC-equivalent timezones hash the same
    }

    fn __repr__(&self) -> &str {
        "tzutc()"
    }

    fn __str__(&self) -> &str {
        "UTC"
    }
}

// ---------------------------------------------------------------------------
// PyTzOffset — extends datetime.tzinfo
// ---------------------------------------------------------------------------

#[pyclass(name = "tzoffset", extends = PyTzInfo, frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyTzOffset {
    inner: TzOffset,
}

#[pymethods]
impl PyTzOffset {
    #[new]
    #[pyo3(signature = (name=None, offset=None))]
    fn new(name: Option<&str>, offset: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let secs: i32 = match offset {
            Some(obj) if obj.is_instance_of::<PyDelta>() => {
                // Accept timedelta — convert to total seconds (matches python-dateutil)
                let total: f64 = obj.call_method0("total_seconds")?.extract()?;
                total as i32
            }
            Some(obj) => obj.extract::<i32>()?,
            None => 0,
        };
        Ok(Self {
            inner: TzOffset::new(name, secs),
        })
    }

    fn utcoffset<'py>(
        &self,
        py: Python<'py>,
        _dt: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyDelta>> {
        secs_to_pydelta(py, self.inner.offset_seconds())
    }

    fn dst<'py>(
        &self,
        py: Python<'py>,
        _dt: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyDelta>> {
        secs_to_pydelta(py, 0)
    }

    fn tzname(&self, _dt: Option<&Bound<'_, PyAny>>) -> &str {
        self.inner.display_name()
    }

    fn is_ambiguous(&self, _dt: &Bound<'_, PyAny>) -> bool {
        false
    }

    fn fromutc<'py>(
        slf: &Bound<'py, Self>,
        dt: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyDateTime>> {
        let py = slf.py();
        let ndt = extract_ndt(dt)?;
        let wall = slf.borrow().inner.fromutc(ndt);
        let tz = slf.cast::<PyTzInfo>()?;
        ndt_to_py_datetime_with_fold(py, wall, tz, false)
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(off) = other.extract::<PyRef<'_, PyTzOffset>>() {
            return Ok(self.inner.offset_seconds() == off.inner.offset_seconds());
        }
        if other.is_instance_of::<PyTzUtc>() {
            return Ok(self.inner.offset_seconds() == 0);
        }
        Ok(false)
    }

    fn __hash__(&self) -> u64 {
        self.inner.offset_seconds() as u64
    }

    fn __repr__(&self) -> String {
        match self.inner.name() {
            Some(name) => format!("tzoffset('{}', {})", name, self.inner.offset_seconds()),
            None => format!("tzoffset(None, {})", self.inner.offset_seconds()),
        }
    }
}

// ---------------------------------------------------------------------------
// PyTzFile — extends datetime.tzinfo
// ---------------------------------------------------------------------------

#[pyclass(name = "tzfile", extends = PyTzInfo, frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyTzFile {
    inner: TzFile,
}

#[pymethods]
impl PyTzFile {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let inner = TzFile::from_path(path)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    fn utcoffset<'py>(
        &self,
        py: Python<'py>,
        dt: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyDelta>> {
        let (ndt, fold) = match dt {
            Some(d) => extract_ndt_fold(d)?,
            None => return secs_to_pydelta(py, 0),
        };
        secs_to_pydelta(py, self.inner.utcoffset(ndt, fold))
    }

    fn dst<'py>(
        &self,
        py: Python<'py>,
        dt: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyDelta>> {
        let (ndt, fold) = match dt {
            Some(d) => extract_ndt_fold(d)?,
            None => return secs_to_pydelta(py, 0),
        };
        secs_to_pydelta(py, self.inner.dst(ndt, fold))
    }

    fn tzname<'py>(&self, dt: Option<&Bound<'py, PyAny>>) -> PyResult<&str> {
        let (ndt, fold) = match dt {
            Some(d) => extract_ndt_fold(d)?,
            None => return Ok(""),
        };
        Ok(self.inner.tzname(ndt, fold))
    }

    fn is_ambiguous(&self, dt: &Bound<'_, PyAny>) -> PyResult<bool> {
        let ndt = extract_ndt(dt)?;
        Ok(self.inner.is_ambiguous(ndt))
    }

    fn fromutc<'py>(
        slf: &Bound<'py, Self>,
        dt: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyDateTime>> {
        let py = slf.py();
        let ndt = extract_ndt(dt)?;
        let inner = &slf.borrow().inner;
        let wall = inner.fromutc(ndt);
        let fold = inner.is_ambiguous(wall);
        let tz = slf.cast::<PyTzInfo>()?;
        ndt_to_py_datetime_with_fold(py, wall, tz, fold)
    }

    fn __repr__(&self) -> String {
        match self.inner.filename() {
            Some(f) => format!("tzfile('{}')", f),
            None => "tzfile()".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// PyTzLocal — extends datetime.tzinfo
// ---------------------------------------------------------------------------

#[pyclass(name = "tzlocal", extends = PyTzInfo, frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyTzLocal {
    inner: TzLocal,
}

#[pymethods]
impl PyTzLocal {
    #[new]
    fn new() -> Self {
        Self {
            inner: TzLocal::new(),
        }
    }

    fn utcoffset<'py>(
        &self,
        py: Python<'py>,
        dt: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyDelta>> {
        let (ndt, fold) = match dt {
            Some(d) => extract_ndt_fold(d)?,
            None => return secs_to_pydelta(py, 0),
        };
        secs_to_pydelta(py, self.inner.utcoffset(ndt, fold))
    }

    fn dst<'py>(
        &self,
        py: Python<'py>,
        dt: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyDelta>> {
        let (ndt, fold) = match dt {
            Some(d) => extract_ndt_fold(d)?,
            None => return secs_to_pydelta(py, 0),
        };
        secs_to_pydelta(py, self.inner.dst(ndt, fold))
    }

    fn tzname<'py>(&self, dt: Option<&Bound<'py, PyAny>>) -> PyResult<&str> {
        let (ndt, fold) = match dt {
            Some(d) => extract_ndt_fold(d)?,
            None => return Ok(""),
        };
        Ok(self.inner.tzname(ndt, fold))
    }

    fn is_ambiguous(&self, dt: &Bound<'_, PyAny>) -> PyResult<bool> {
        let ndt = extract_ndt(dt)?;
        Ok(self.inner.is_ambiguous(ndt))
    }

    fn fromutc<'py>(
        slf: &Bound<'py, Self>,
        dt: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyDateTime>> {
        let py = slf.py();
        let ndt = extract_ndt(dt)?;
        let inner = &slf.borrow().inner;
        let wall = inner.fromutc(ndt);
        let fold = inner.is_ambiguous(wall);
        let tz = slf.cast::<PyTzInfo>()?;
        ndt_to_py_datetime_with_fold(py, wall, tz, fold)
    }

    fn __repr__(&self) -> String {
        format!("tzlocal('{}')", self.inner.iana_name())
    }
}

// ---------------------------------------------------------------------------
// gettz() — factory function
// ---------------------------------------------------------------------------

#[pyfunction]
#[pyo3(name = "gettz", signature = (name=None))]
fn gettz_py(py: Python<'_>, name: Option<&str>) -> PyResult<Py<PyAny>> {
    let tz = py
        .detach(|| tz::gettz(name))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    match tz {
        TimeZone::Utc(_) => {
            let obj = Bound::new(py, PyTzUtc::new())?;
            Ok(obj.into_any().unbind())
        }
        TimeZone::Offset(inner) => {
            let obj = Bound::new(py, PyTzOffset { inner })?;
            Ok(obj.into_any().unbind())
        }
        TimeZone::File(inner) => {
            let obj = Bound::new(py, PyTzFile { inner })?;
            Ok(obj.into_any().unbind())
        }
        TimeZone::Local(inner) => {
            let obj = Bound::new(py, PyTzLocal { inner })?;
            Ok(obj.into_any().unbind())
        }
    }
}

// ---------------------------------------------------------------------------
// PyTimezone — FromPyObject enum for single-dispatch extraction
// ---------------------------------------------------------------------------

#[derive(FromPyObject)]
enum PyTimezone<'py> {
    #[allow(dead_code)]
    Utc(PyRef<'py, PyTzUtc>),
    Offset(PyRef<'py, PyTzOffset>),
    File(PyRef<'py, PyTzFile>),
    Local(PyRef<'py, PyTzLocal>),
}

impl TzOps for PyTimezone<'_> {
    #[inline]
    fn utcoffset(&self, dt: NaiveDateTime, fold: bool) -> i32 {
        match self {
            PyTimezone::Utc(_) => 0,
            PyTimezone::Offset(tz) => tz.inner.offset_seconds(),
            PyTimezone::File(tz) => tz.inner.utcoffset(dt, fold),
            PyTimezone::Local(tz) => tz.inner.utcoffset(dt, fold),
        }
    }

    #[inline]
    fn dst(&self, dt: NaiveDateTime, fold: bool) -> i32 {
        match self {
            PyTimezone::Utc(_) | PyTimezone::Offset(_) => 0,
            PyTimezone::File(tz) => tz.inner.dst(dt, fold),
            PyTimezone::Local(tz) => tz.inner.dst(dt, fold),
        }
    }

    #[inline]
    fn tzname(&self, dt: NaiveDateTime, fold: bool) -> &str {
        match self {
            PyTimezone::Utc(_) => "UTC",
            PyTimezone::Offset(tz) => tz.inner.display_name(),
            PyTimezone::File(tz) => tz.inner.tzname(dt, fold),
            PyTimezone::Local(tz) => tz.inner.tzname(dt, fold),
        }
    }

    #[inline]
    fn is_ambiguous(&self, dt: NaiveDateTime) -> bool {
        match self {
            PyTimezone::Utc(_) | PyTimezone::Offset(_) => false,
            PyTimezone::File(tz) => tz.inner.is_ambiguous(dt),
            PyTimezone::Local(tz) => tz.inner.is_ambiguous(dt),
        }
    }

    #[inline]
    fn fromutc(&self, dt: NaiveDateTime) -> NaiveDateTime {
        match self {
            PyTimezone::Utc(_) => dt,
            PyTimezone::Offset(tz) => tz.inner.fromutc(dt),
            PyTimezone::File(tz) => tz.inner.fromutc(dt),
            PyTimezone::Local(tz) => tz.inner.fromutc(dt),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper functions — zero-clone via TzOps trait
// ---------------------------------------------------------------------------

#[pyfunction]
#[pyo3(name = "datetime_exists")]
fn datetime_exists_py(dt: NaiveDateTime, tz: PyTimezone<'_>) -> bool {
    tz::datetime_exists(dt, &tz)
}

#[pyfunction]
#[pyo3(name = "datetime_ambiguous")]
fn datetime_ambiguous_py(dt: NaiveDateTime, tz: PyTimezone<'_>) -> bool {
    tz::datetime_ambiguous(dt, &tz)
}

#[pyfunction]
#[pyo3(name = "resolve_imaginary")]
fn resolve_imaginary_py(dt: NaiveDateTime, tz: PyTimezone<'_>) -> NaiveDateTime {
    tz::resolve_imaginary(dt, &tz)
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTzUtc>()?;
    m.add_class::<PyTzOffset>()?;
    m.add_class::<PyTzFile>()?;
    m.add_class::<PyTzLocal>()?;
    m.add_function(pyo3::wrap_pyfunction!(gettz_py, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(datetime_exists_py, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(datetime_ambiguous_py, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(resolve_imaginary_py, m)?)?;
    Ok(())
}
