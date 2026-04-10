use chrono::NaiveDateTime;
use dateutil_core::tz::{
    self, TimeZone, TzFile, TzLocal, TzOffset, TzUtc,
};
use pyo3::prelude::*;
use pyo3::types::PyDelta;

// ---------------------------------------------------------------------------
// Helper: i32 seconds → Python timedelta
// ---------------------------------------------------------------------------

fn secs_to_pydelta<'py>(py: Python<'py>, total_secs: i32) -> PyResult<Bound<'py, PyDelta>> {
    let days = total_secs.div_euclid(86400);
    let remaining = total_secs.rem_euclid(86400);
    PyDelta::new(py, days, remaining, 0, false)
}

// ---------------------------------------------------------------------------
// PyTzUtc
// ---------------------------------------------------------------------------

#[pyclass(name = "tzutc", frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyTzUtc {
    inner: TzUtc,
}

#[pymethods]
impl PyTzUtc {
    #[new]
    fn new() -> Self {
        Self { inner: TzUtc }
    }

    fn utcoffset<'py>(&self, py: Python<'py>, _dt: Option<NaiveDateTime>) -> PyResult<Bound<'py, PyDelta>> {
        secs_to_pydelta(py, 0)
    }

    fn dst<'py>(&self, py: Python<'py>, _dt: Option<NaiveDateTime>) -> PyResult<Bound<'py, PyDelta>> {
        secs_to_pydelta(py, 0)
    }

    fn tzname(&self, _dt: Option<NaiveDateTime>) -> &str {
        "UTC"
    }

    fn is_ambiguous(&self, _dt: NaiveDateTime) -> bool {
        false
    }

    fn fromutc(&self, dt: NaiveDateTime) -> NaiveDateTime {
        dt
    }

    fn __repr__(&self) -> &str {
        "tzutc()"
    }

    fn __str__(&self) -> &str {
        "UTC"
    }
}

// ---------------------------------------------------------------------------
// PyTzOffset
// ---------------------------------------------------------------------------

#[pyclass(name = "tzoffset", frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyTzOffset {
    inner: TzOffset,
}

#[pymethods]
impl PyTzOffset {
    #[new]
    #[pyo3(signature = (name=None, offset=0))]
    fn new(name: Option<&str>, offset: i32) -> Self {
        Self {
            inner: TzOffset::new(name, offset),
        }
    }

    fn utcoffset<'py>(&self, py: Python<'py>, _dt: Option<NaiveDateTime>) -> PyResult<Bound<'py, PyDelta>> {
        secs_to_pydelta(py, self.inner.offset_seconds())
    }

    fn dst<'py>(&self, py: Python<'py>, _dt: Option<NaiveDateTime>) -> PyResult<Bound<'py, PyDelta>> {
        secs_to_pydelta(py, 0)
    }

    fn tzname(&self, _dt: Option<NaiveDateTime>) -> &str {
        self.inner.display_name()
    }

    fn is_ambiguous(&self, _dt: NaiveDateTime) -> bool {
        false
    }

    fn fromutc(&self, dt: NaiveDateTime) -> NaiveDateTime {
        self.inner.fromutc(dt)
    }

    fn __repr__(&self) -> String {
        match self.inner.name() {
            Some(name) => format!("tzoffset('{}', {})", name, self.inner.offset_seconds()),
            None => format!("tzoffset(None, {})", self.inner.offset_seconds()),
        }
    }
}

// ---------------------------------------------------------------------------
// PyTzFile
// ---------------------------------------------------------------------------

#[pyclass(name = "tzfile", frozen, skip_from_py_object)]
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

    #[pyo3(signature = (dt, fold=false))]
    fn utcoffset<'py>(&self, py: Python<'py>, dt: NaiveDateTime, fold: bool) -> PyResult<Bound<'py, PyDelta>> {
        secs_to_pydelta(py, self.inner.utcoffset(dt, fold))
    }

    #[pyo3(signature = (dt, fold=false))]
    fn dst<'py>(&self, py: Python<'py>, dt: NaiveDateTime, fold: bool) -> PyResult<Bound<'py, PyDelta>> {
        secs_to_pydelta(py, self.inner.dst(dt, fold))
    }

    #[pyo3(signature = (dt, fold=false))]
    fn tzname(&self, dt: NaiveDateTime, fold: bool) -> &str {
        self.inner.tzname(dt, fold)
    }

    fn is_ambiguous(&self, dt: NaiveDateTime) -> bool {
        self.inner.is_ambiguous(dt)
    }

    fn fromutc(&self, dt: NaiveDateTime) -> NaiveDateTime {
        self.inner.fromutc(dt)
    }

    fn __repr__(&self) -> String {
        match self.inner.filename() {
            Some(f) => format!("tzfile('{}')", f),
            None => "tzfile()".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// PyTzLocal
// ---------------------------------------------------------------------------

#[pyclass(name = "tzlocal", frozen, skip_from_py_object)]
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

    #[pyo3(signature = (dt, fold=false))]
    fn utcoffset<'py>(&self, py: Python<'py>, dt: NaiveDateTime, fold: bool) -> PyResult<Bound<'py, PyDelta>> {
        secs_to_pydelta(py, self.inner.utcoffset(dt, fold))
    }

    #[pyo3(signature = (dt, fold=false))]
    fn dst<'py>(&self, py: Python<'py>, dt: NaiveDateTime, fold: bool) -> PyResult<Bound<'py, PyDelta>> {
        secs_to_pydelta(py, self.inner.dst(dt, fold))
    }

    #[pyo3(signature = (dt, fold=false))]
    fn tzname(&self, dt: NaiveDateTime, fold: bool) -> &str {
        self.inner.tzname(dt, fold)
    }

    fn is_ambiguous(&self, dt: NaiveDateTime) -> bool {
        self.inner.is_ambiguous(dt)
    }

    fn fromutc(&self, dt: NaiveDateTime) -> NaiveDateTime {
        self.inner.fromutc(dt)
    }

    fn __repr__(&self) -> String {
        format!("tzlocal('{}')", self.inner.iana_name())
    }
}

// ---------------------------------------------------------------------------
// gettz() — factory function returning appropriate type
// ---------------------------------------------------------------------------

#[pyfunction]
#[pyo3(name = "gettz", signature = (name=None))]
fn gettz_py(py: Python<'_>, name: Option<&str>) -> PyResult<Py<PyAny>> {
    let tz = tz::gettz(name)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    match tz {
        TimeZone::Utc(_) => {
            let obj = Bound::new(py, PyTzUtc::new())?;
            Ok(obj.into_any().unbind())
        }
        TimeZone::Offset(inner) => {
            let obj = Bound::new(py, PyTzOffset {
                inner,
            })?;
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
    Utc(PyRef<'py, PyTzUtc>),
    Offset(PyRef<'py, PyTzOffset>),
    File(PyRef<'py, PyTzFile>),
    Local(PyRef<'py, PyTzLocal>),
}

impl PyTimezone<'_> {
    fn to_timezone(&self) -> TimeZone {
        match self {
            PyTimezone::Utc(tz) => TimeZone::Utc(tz.inner),
            PyTimezone::Offset(tz) => TimeZone::Offset(tz.inner.clone()),
            PyTimezone::File(tz) => TimeZone::File(tz.inner.clone()),
            PyTimezone::Local(tz) => TimeZone::Local(tz.inner.clone()),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

#[pyfunction]
#[pyo3(name = "datetime_exists")]
fn datetime_exists_py(dt: NaiveDateTime, tz: PyTimezone<'_>) -> bool {
    tz::datetime_exists(dt, &tz.to_timezone())
}

#[pyfunction]
#[pyo3(name = "datetime_ambiguous")]
fn datetime_ambiguous_py(dt: NaiveDateTime, tz: PyTimezone<'_>) -> bool {
    tz::datetime_ambiguous(dt, &tz.to_timezone())
}

#[pyfunction]
#[pyo3(name = "resolve_imaginary")]
fn resolve_imaginary_py(dt: NaiveDateTime, tz: PyTimezone<'_>) -> NaiveDateTime {
    tz::resolve_imaginary(dt, &tz.to_timezone())
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
