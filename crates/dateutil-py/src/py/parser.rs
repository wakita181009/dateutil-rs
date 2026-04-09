use dateutil_core::parser;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Parse a date/time string into a datetime.
///
/// Returns a naive datetime. Use `parse_to_dict` for access to parsed
/// fields including timezone info.
#[pyfunction]
#[pyo3(name = "parse", signature = (timestr, dayfirst=false, yearfirst=false))]
fn parse_py(timestr: &str, dayfirst: bool, yearfirst: bool) -> PyResult<chrono::NaiveDateTime> {
    parser::parse(timestr, dayfirst, yearfirst)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

/// Parse a date/time string and return a dict with all parsed fields.
///
/// Keys: year, month, day, weekday, hour, minute, second, microsecond,
///       tzname, tzoffset. Values are None when not present in the input.
#[pyfunction]
#[pyo3(name = "parse_to_dict", signature = (timestr, dayfirst=false, yearfirst=false))]
fn parse_to_dict_py<'py>(
    py: Python<'py>,
    timestr: &str,
    dayfirst: bool,
    yearfirst: bool,
) -> PyResult<Bound<'py, PyDict>> {
    let res = parser::parse_to_result(timestr, dayfirst, yearfirst)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let dict = PyDict::new(py);
    dict.set_item("year", res.year)?;
    dict.set_item("month", res.month)?;
    dict.set_item("day", res.day)?;
    dict.set_item("weekday", res.weekday)?;
    dict.set_item("hour", res.hour)?;
    dict.set_item("minute", res.minute)?;
    dict.set_item("second", res.second)?;
    dict.set_item("microsecond", res.microsecond)?;
    dict.set_item("tzname", res.tzname.as_deref())?;
    dict.set_item("tzoffset", res.tzoffset)?;
    Ok(dict)
}

/// Parse an ISO-8601 date/time string into a datetime.
#[pyfunction]
#[pyo3(name = "isoparse")]
fn isoparse_py(dt_str: &str) -> PyResult<chrono::NaiveDateTime> {
    parser::isoparse(dt_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(parse_py, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_to_dict_py, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(isoparse_py, m)?)?;
    Ok(())
}
