use pyo3::prelude::*;

pub mod common;
pub mod easter;
pub mod parser;
pub mod relativedelta;
pub mod rrule;

/// Register all v1 bindings on the given module.
pub fn register_all(m: &Bound<'_, PyModule>) -> PyResult<()> {
    common::register(m)?;
    easter::register(m)?;
    parser::register(m)?;
    relativedelta::register(m)?;
    rrule::register(m)?;
    Ok(())
}

/// Standalone entry point — only emitted for benchmark builds
/// (`cargo rustc -p dateutil-py -F python,standalone --crate-type cdylib`).
#[cfg(feature = "standalone")]
#[pymodule]
pub fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    register_all(m)
}
