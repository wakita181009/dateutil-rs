use dateutil::easter::{self, EasterMethod};
use pyo3::prelude::*;

/// Python constant values matching python-dateutil convention.
pub const EASTER_JULIAN: i32 = 1;
pub const EASTER_ORTHODOX: i32 = 2;
pub const EASTER_WESTERN: i32 = 3;

/// Compute the date of Easter for a given year and method.
///
/// method: 1=Julian, 2=Orthodox, 3=Western (default)
#[pyfunction]
#[pyo3(name = "easter", signature = (year, method=3))]
fn easter_py(year: i32, method: i32) -> PyResult<chrono::NaiveDate> {
    let m = EasterMethod::from_i32(method)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    easter::easter(year, m).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("EASTER_JULIAN", EASTER_JULIAN)?;
    m.add("EASTER_ORTHODOX", EASTER_ORTHODOX)?;
    m.add("EASTER_WESTERN", EASTER_WESTERN)?;
    m.add_function(pyo3::wrap_pyfunction!(easter_py, m)?)?;
    Ok(())
}
