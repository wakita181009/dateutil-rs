use dateutil_core::common;
use pyo3::prelude::*;

/// Python wrapper for dateutil_core::common::Weekday.
#[pyclass(name = "weekday", frozen, hash, eq, from_py_object)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyWeekday {
    inner: common::Weekday,
}

#[pymethods]
impl PyWeekday {
    #[new]
    #[pyo3(signature = (weekday, n=None))]
    fn new(weekday: u8, n: Option<i32>) -> PyResult<Self> {
        let inner = common::Weekday::new(weekday, n)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Create a new weekday with the given N-th occurrence.
    /// e.g. MO(2) means "the 2nd Monday".
    #[pyo3(signature = (n=None))]
    fn __call__(&self, n: Option<i32>) -> Self {
        Self {
            inner: self.inner.with_n(n),
        }
    }

    #[getter]
    fn weekday(&self) -> u8 {
        self.inner.weekday()
    }

    #[getter]
    fn n(&self) -> Option<i32> {
        self.inner.n()
    }

    fn __repr__(&self) -> String {
        self.inner.to_string()
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

impl From<common::Weekday> for PyWeekday {
    fn from(wd: common::Weekday) -> Self {
        Self { inner: wd }
    }
}

impl From<PyWeekday> for common::Weekday {
    fn from(py_wd: PyWeekday) -> Self {
        py_wd.inner
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWeekday>()?;
    m.add("MO", PyWeekday::from(common::MO))?;
    m.add("TU", PyWeekday::from(common::TU))?;
    m.add("WE", PyWeekday::from(common::WE))?;
    m.add("TH", PyWeekday::from(common::TH))?;
    m.add("FR", PyWeekday::from(common::FR))?;
    m.add("SA", PyWeekday::from(common::SA))?;
    m.add("SU", PyWeekday::from(common::SU))?;
    Ok(())
}
