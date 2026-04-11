use std::collections::HashMap;

use super::conv::{make_py_tz, make_py_utc, ndt_to_py_datetime};
use dateutil::parser;
use dateutil::parser::ParserInfo;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTzInfo, PyType};

// ---------------------------------------------------------------------------
// parserinfo — Rust pyclass (internal base)
// ---------------------------------------------------------------------------

/// Convert a Python list of `str | tuple[str, …]` into a lowercased
/// `HashMap<String, usize>` where each string maps to its group index.
fn convert(attr_name: &str, list: &Bound<'_, pyo3::PyAny>) -> PyResult<HashMap<String, usize>> {
    let mut map = HashMap::new();
    let seq: Vec<Bound<'_, pyo3::PyAny>> = list.extract()?;
    for (i, item) in seq.iter().enumerate() {
        if let Ok(s) = item.extract::<String>() {
            map.insert(s.to_lowercase(), i);
        } else {
            let parts: Vec<String> = item.extract().map_err(|_| {
                let type_name = item.get_type().qualname()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|_| "unknown".into());
                pyo3::exceptions::PyTypeError::new_err(format!(
                    "parserinfo.{attr_name}[{i}]: expected str or sequence of str, got {type_name}",
                ))
            })?;
            for s in parts {
                map.insert(s.to_lowercase(), i);
            }
        }
    }
    Ok(map)
}

/// Extract `ParserInfo`, `dayfirst`, and `yearfirst` from an optional
/// `PyParserInfo` borrow, falling back to defaults when absent.
fn resolve_pi_args<'a>(
    pi_ref: &'a Option<PyRef<'_, PyParserInfo>>,
    dayfirst: Option<bool>,
    yearfirst: Option<bool>,
) -> (Option<&'a ParserInfo>, bool, bool) {
    match pi_ref {
        Some(pi) => (
            Some(&pi.inner),
            dayfirst.unwrap_or(pi.dayfirst),
            yearfirst.unwrap_or(pi.yearfirst),
        ),
        None => (None, dayfirst.unwrap_or(false), yearfirst.unwrap_or(false)),
    }
}

/// Build a `ParserInfo` from class-level attributes read from a Python type.
fn build_parser_info(cls: &Bound<'_, PyType>) -> PyResult<ParserInfo> {
    let jump_map = convert("JUMP", &cls.getattr("JUMP")?)?;
    let weekdays = convert("WEEKDAYS", &cls.getattr("WEEKDAYS")?)?;
    let months_raw = convert("MONTHS", &cls.getattr("MONTHS")?)?;
    let months = months_raw
        .into_iter()
        .map(|(k, v)| (k, v + 1))
        .collect();
    let hms = convert("HMS", &cls.getattr("HMS")?)?;
    let ampm = convert("AMPM", &cls.getattr("AMPM")?)?;
    let utczone_map = convert("UTCZONE", &cls.getattr("UTCZONE")?)?;
    let pertain_map = convert("PERTAIN", &cls.getattr("PERTAIN")?)?;
    let tzoffset: HashMap<String, i32> = cls
        .getattr("TZOFFSET")?
        .extract::<HashMap<String, i32>>()?
        .into_iter()
        .map(|(k, v)| (k.to_lowercase(), v))
        .collect();

    Ok(ParserInfo {
        jump: jump_map.into_keys().collect(),
        weekdays,
        months,
        hms,
        ampm,
        utczone: utczone_map.into_keys().collect(),
        pertain: pertain_map.into_keys().collect(),
        tzoffset,
    })
}

/// Internal base class for ``parserinfo``.
///
/// The public Python class subclasses this and calls ``_build(type(self))``
/// in ``__init__`` so that subclass class-variable overrides are respected.
#[pyclass(name = "_ParserInfoBase", subclass)]
pub struct PyParserInfo {
    inner: ParserInfo,
    #[pyo3(get)]
    dayfirst: bool,
    #[pyo3(get)]
    yearfirst: bool,
}

#[pymethods]
impl PyParserInfo {
    // ---- Class-level defaults (overridable by Python subclasses) ----

    #[classattr]
    #[allow(non_snake_case)]
    fn JUMP() -> Vec<&'static str> {
        vec![
            " ", ".", ",", ";", "-", "/", "'", "at", "on", "and", "ad",
            "m", "t", "of", "st", "nd", "rd", "th",
        ]
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn WEEKDAYS() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Mon", "Monday"),
            ("Tue", "Tuesday"),
            ("Wed", "Wednesday"),
            ("Thu", "Thursday"),
            ("Fri", "Friday"),
            ("Sat", "Saturday"),
            ("Sun", "Sunday"),
        ]
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn MONTHS() -> Vec<Vec<&'static str>> {
        vec![
            vec!["Jan", "January"],
            vec!["Feb", "February"],
            vec!["Mar", "March"],
            vec!["Apr", "April"],
            vec!["May"],
            vec!["Jun", "June"],
            vec!["Jul", "July"],
            vec!["Aug", "August"],
            vec!["Sep", "Sept", "September"],
            vec!["Oct", "October"],
            vec!["Nov", "November"],
            vec!["Dec", "December"],
        ]
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn HMS() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            ("h", "hour", "hours"),
            ("m", "minute", "minutes"),
            ("s", "second", "seconds"),
        ]
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn AMPM() -> Vec<(&'static str, &'static str)> {
        vec![("am", "a"), ("pm", "p")]
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn UTCZONE() -> Vec<&'static str> {
        vec!["UTC", "GMT", "Z", "z"]
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn PERTAIN() -> Vec<&'static str> {
        vec!["of"]
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn TZOFFSET() -> HashMap<String, i32> {
        HashMap::new()
    }

    // ---- Constructor ----

    #[new]
    #[pyo3(signature = (dayfirst=false, yearfirst=false))]
    fn new(dayfirst: bool, yearfirst: bool) -> Self {
        Self {
            inner: ParserInfo::default(),
            dayfirst,
            yearfirst,
        }
    }

    /// Read class-level variables and rebuild internal lookup tables.
    /// Called from Python ``__init__`` with ``type(self)`` so that
    /// subclass overrides are captured.
    fn _build(&mut self, cls: &Bound<'_, PyType>) -> PyResult<()> {
        self.inner = build_parser_info(cls)?;
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!(
            "parserinfo(dayfirst={}, yearfirst={})",
            self.dayfirst, self.yearfirst
        )
    }
}

// ---------------------------------------------------------------------------
// parse()
// ---------------------------------------------------------------------------

/// Parse a date/time string into a datetime.
#[pyfunction]
#[pyo3(name = "parse", signature = (
    timestr,
    parserinfo = None,
    *,
    dayfirst = None,
    yearfirst = None,
    default = None,
    ignoretz = false,
    tzinfos = None,
))]
#[allow(clippy::too_many_arguments)]
fn parse_py<'py>(
    py: Python<'py>,
    timestr: &str,
    parserinfo: Option<Bound<'py, PyParserInfo>>,
    dayfirst: Option<bool>,
    yearfirst: Option<bool>,
    default: Option<chrono::NaiveDateTime>,
    ignoretz: bool,
    tzinfos: Option<Bound<'py, pyo3::PyAny>>,
) -> PyResult<Bound<'py, pyo3::PyAny>> {
    let pi_ref = parserinfo.as_ref().map(|pi| pi.borrow());
    let (info_ref, df, yf) = resolve_pi_args(&pi_ref, dayfirst, yearfirst);

    let res = parser::parse_to_result(timestr, df, yf, info_ref)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let now = chrono::Local::now().naive_local();
    let default_dt =
        default.unwrap_or_else(|| now.date().and_hms_opt(0, 0, 0).unwrap());
    let ndt = parser::build_naive(&res, default_dt)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    if ignoretz {
        return ndt_to_py_datetime(py, ndt, None);
    }

    // tzinfos resolution
    if let Some(ref tzinfos_obj) = tzinfos {
        if let Some(ref tzname) = res.tzname {
            let tzdata = if tzinfos_obj.is_callable() {
                let offset_arg: Bound<'py, pyo3::PyAny> = match res.tzoffset {
                    Some(o) => o.into_pyobject(py)?.into_any(),
                    None => py.None().into_bound(py),
                };
                tzinfos_obj.call1((tzname.as_ref(), offset_arg))?
            } else {
                tzinfos_obj.get_item(tzname.as_ref())?
            };

            if tzdata.is_none() {
                return ndt_to_py_datetime(py, ndt, None);
            } else if let Ok(offset_secs) = tzdata.extract::<i32>() {
                let tz = make_py_tz(py, offset_secs)?;
                return ndt_to_py_datetime(py, ndt, Some(&tz));
            } else {
                let tz = tzdata.cast::<PyTzInfo>()?;
                return ndt_to_py_datetime(py, ndt, Some(tz));
            }
        }
    }

    // Offset-based fallback
    if let Some(offset) = res.tzoffset {
        if offset == 0 {
            let tz = make_py_utc(py)?;
            return ndt_to_py_datetime(py, ndt, Some(&tz));
        }
        let tz = make_py_tz(py, offset)?;
        return ndt_to_py_datetime(py, ndt, Some(&tz));
    }

    ndt_to_py_datetime(py, ndt, None)
}

// ---------------------------------------------------------------------------
// parse_to_dict()
// ---------------------------------------------------------------------------

#[pyfunction]
#[pyo3(name = "parse_to_dict", signature = (timestr, *, parserinfo=None, dayfirst=None, yearfirst=None))]
fn parse_to_dict_py<'py>(
    py: Python<'py>,
    timestr: &str,
    parserinfo: Option<Bound<'py, PyParserInfo>>,
    dayfirst: Option<bool>,
    yearfirst: Option<bool>,
) -> PyResult<Bound<'py, PyDict>> {
    let pi_ref = parserinfo.as_ref().map(|pi| pi.borrow());
    let (info_ref, df, yf) = resolve_pi_args(&pi_ref, dayfirst, yearfirst);

    let res = parser::parse_to_result(timestr, df, yf, info_ref)
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

#[pyfunction]
#[pyo3(name = "isoparse")]
fn isoparse_py(dt_str: &str) -> PyResult<chrono::NaiveDateTime> {
    parser::isoparse(dt_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyParserInfo>()?;
    m.add_function(pyo3::wrap_pyfunction!(parse_py, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_to_dict_py, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(isoparse_py, m)?)?;
    Ok(())
}
