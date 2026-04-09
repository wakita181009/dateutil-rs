pub mod common;
pub mod easter;
pub mod parser;
pub mod relativedelta;
pub mod rrule;
pub mod tz;
pub mod utils;

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Weekday class
    m.add_class::<common::Weekday>()?;

    // Weekday constants
    m.add("MO", common::Weekday::new(0, None))?;
    m.add("TU", common::Weekday::new(1, None))?;
    m.add("WE", common::Weekday::new(2, None))?;
    m.add("TH", common::Weekday::new(3, None))?;
    m.add("FR", common::Weekday::new(4, None))?;
    m.add("SA", common::Weekday::new(5, None))?;
    m.add("SU", common::Weekday::new(6, None))?;

    // Easter constants
    m.add("EASTER_JULIAN", easter::EASTER_JULIAN)?;
    m.add("EASTER_ORTHODOX", easter::EASTER_ORTHODOX)?;
    m.add("EASTER_WESTERN", easter::EASTER_WESTERN)?;

    // Easter function
    m.add_function(wrap_pyfunction!(easter::easter_py, m)?)?;

    // RelativeDelta class
    m.add_class::<relativedelta::RelativeDelta>()?;

    // Utils functions
    utils::python::register(m)?;

    // Parser functions and classes
    parser::python::register(m)?;

    // Timezone classes and functions
    tz::python::register(m)?;

    // RRule classes and functions
    rrule::python::register(m)?;

    // v1 bindings (dateutil-core via dateutil-py)
    #[cfg(feature = "v1")]
    {
        let py = m.py();
        let v1 = PyModule::new(py, "_native")?;
        dateutil_py::py::register_all(&v1)?;
        // Register in sys.modules so `from dateutil_rs.v1._native import ...` works
        let sys_modules = py.import("sys")?.getattr("modules")?;
        sys_modules.set_item("dateutil_rs.v1._native", &v1)?;
    }

    Ok(())
}
