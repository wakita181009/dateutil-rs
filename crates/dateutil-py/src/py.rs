use pyo3::prelude::*;

pub mod common;
mod conv;
pub mod easter;
pub mod parser;
pub mod relativedelta;
pub mod rrule;
pub mod tz;

/// Register all v1 bindings on the given module.
pub fn register_all(m: &Bound<'_, PyModule>) -> PyResult<()> {
    common::register(m)?;
    easter::register(m)?;
    parser::register(m)?;
    relativedelta::register(m)?;
    rrule::register(m)?;
    tz::register(m)?;
    Ok(())
}

/// PyO3 module entry point.
#[cfg(feature = "python")]
#[pymodule]
pub fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    register_all(m)
}
