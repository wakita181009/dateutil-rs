use dateutil::common;
use pyo3::prelude::*;

/// Python wrapper for dateutil::common::Weekday.
#[pyclass(name = "weekday", frozen, from_py_object)]
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
    /// Calling with the same N returns the same object (identity).
    #[pyo3(signature = (n=None))]
    fn __call__(slf: Py<Self>, py: Python<'_>, n: Option<i32>) -> PyResult<Py<Self>> {
        if n == Some(0) {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "N must not be 0",
            ));
        }
        // Weekday is Copy — borrow once
        let inner = slf.borrow(py).inner;
        // If the N-th value is the same, return the same object (identity)
        if inner.n() == n {
            return Ok(slf);
        }
        Py::new(py, Self { inner: inner.with_n(n) })
    }

    #[getter]
    pub fn weekday(&self) -> u8 {
        self.inner.weekday()
    }

    #[getter]
    pub fn n(&self) -> Option<i32> {
        self.inner.n()
    }

    fn __hash__(&self) -> isize {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish() as isize
    }

    fn __eq__(&self, _py: Python<'_>, other: &Bound<'_, PyAny>) -> bool {
        // Fast path: other is a PyWeekday
        if let Ok(wd) = other.extract::<PyWeekday>() {
            return self.inner == wd.inner;
        }
        // Duck-type: must have both `weekday` AND `n` attributes
        let Ok(wd_attr) = other.getattr("weekday") else {
            return false;
        };
        let Ok(n_attr) = other.getattr("n") else {
            return false;
        };
        let Ok(wd_val) = wd_attr.extract::<u8>() else {
            return false;
        };
        let Ok(n_val) = n_attr.extract::<Option<i32>>() else {
            return false;
        };
        self.inner.weekday() == wd_val && self.inner.n() == n_val
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
