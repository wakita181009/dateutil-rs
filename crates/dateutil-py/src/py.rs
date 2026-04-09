use pyo3::prelude::*;

mod common;
mod easter;
mod parser;
mod relativedelta;
mod rrule;

/// The `_native` module exposed to Python via PyO3.
#[pymodule]
pub fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    common::register(m)?;
    easter::register(m)?;
    parser::register(m)?;
    relativedelta::register(m)?;
    rrule::register(m)?;
    Ok(())
}
