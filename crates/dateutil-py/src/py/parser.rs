use std::collections::HashMap;

use super::conv::{make_py_tz, make_py_utc, ndt_to_py_datetime};
use chrono::{Datelike, Timelike};
use dateutil::parser;
use dateutil::parser::{IsoParser, IsoTz, ParserInfo};
use pyo3::prelude::*;
use pyo3::types::{PyDate, PyDict, PyTime, PyType, PyTzInfo};

// ---------------------------------------------------------------------------
// parserinfo — Rust pyclass (internal base)
// ---------------------------------------------------------------------------

/// Convert a Python list of `str | tuple[str, …]` into a lowercased
/// `HashMap<String, usize>` where each string maps to its group index.
fn convert(attr_name: &str, list: &Bound<'_, pyo3::PyAny>) -> PyResult<HashMap<String, usize>> {
    let seq: Vec<Bound<'_, pyo3::PyAny>> = list.extract()?;
    let mut map = HashMap::with_capacity(seq.len() * 2);
    for (i, item) in seq.iter().enumerate() {
        if let Ok(s) = item.extract::<String>() {
            map.insert(s.to_lowercase(), i);
        } else {
            let parts: Vec<String> = item.extract().map_err(|_| {
                let type_name = item
                    .get_type()
                    .qualname()
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
    let months = months_raw.into_iter().map(|(k, v)| (k, v + 1)).collect();
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
            " ", ".", ",", ";", "-", "/", "'", "at", "on", "and", "ad", "m", "t", "of", "st", "nd",
            "rd", "th",
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

    let res = py
        .detach(|| parser::parse_to_result(timestr, df, yf, info_ref))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let now = chrono::Local::now().naive_local();
    let default_dt = default.unwrap_or_else(|| now.date().and_hms_opt(0, 0, 0).unwrap());
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

    let res = py
        .detach(|| parser::parse_to_result(timestr, df, yf, info_ref))
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

// ---------------------------------------------------------------------------
// isoparser class
// ---------------------------------------------------------------------------

/// Extract a UTF-8 string from Python `str` or `bytes`.
fn extract_iso_string(obj: &Bound<'_, pyo3::PyAny>) -> PyResult<String> {
    if let Ok(s) = obj.extract::<String>() {
        return Ok(s);
    }
    if let Ok(b) = obj.extract::<Vec<u8>>() {
        return String::from_utf8(b).map_err(|_| {
            pyo3::exceptions::PyValueError::new_err(
                "ISO-8601 strings should contain only ASCII characters",
            )
        });
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected str or bytes",
    ))
}

/// Build a `dateutil.tz.tzutc()` instance.
fn make_dateutil_utc<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyTzInfo>> {
    let tz_mod = py.import("dateutil.tz")?;
    let obj = tz_mod.getattr("tzutc")?.call0()?;
    obj.downcast_into::<PyTzInfo>()
        .map_err(|e| pyo3::exceptions::PyTypeError::new_err(e.to_string()))
}

/// Build a `dateutil.tz.tzoffset(None, seconds)` instance.
fn make_dateutil_offset<'py>(py: Python<'py>, secs: i32) -> PyResult<Bound<'py, PyTzInfo>> {
    let tz_mod = py.import("dateutil.tz")?;
    let obj = tz_mod.getattr("tzoffset")?.call1((py.None(), secs))?;
    obj.downcast_into::<PyTzInfo>()
        .map_err(|e| pyo3::exceptions::PyTypeError::new_err(e.to_string()))
}

/// Convert `IsoTz` to a Python `datetime.tzinfo` subclass.
fn isotz_to_py<'py>(py: Python<'py>, tz: IsoTz) -> PyResult<Bound<'py, PyTzInfo>> {
    match tz {
        IsoTz::Utc => make_dateutil_utc(py),
        IsoTz::Offset(secs) => make_dateutil_offset(py, secs),
    }
}

#[pyclass(name = "isoparser")]
pub struct PyIsoParser {
    inner: IsoParser,
}

#[pymethods]
impl PyIsoParser {
    #[new]
    #[pyo3(signature = (sep=None))]
    fn new(sep: Option<&Bound<'_, pyo3::PyAny>>) -> PyResult<Self> {
        let sep_byte = match sep {
            None => None,
            Some(obj) => {
                let s = if let Ok(s) = obj.extract::<String>() {
                    s
                } else if let Ok(b) = obj.extract::<Vec<u8>>() {
                    String::from_utf8(b).map_err(|_| {
                        pyo3::exceptions::PyValueError::new_err(
                            "Separator must be a single, non-numeric ASCII character",
                        )
                    })?
                } else {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "Separator must be a single, non-numeric ASCII character",
                    ));
                };
                if s.len() != 1 {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "Separator must be a single, non-numeric ASCII character",
                    ));
                }
                let b = s.as_bytes()[0];
                if b >= 128 || (b as char).is_ascii_digit() {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "Separator must be a single, non-numeric ASCII character",
                    ));
                }
                Some(b)
            }
        };
        let inner = IsoParser::new(sep_byte)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    fn isoparse<'py>(
        &self,
        py: Python<'py>,
        dt_str: &Bound<'py, pyo3::PyAny>,
    ) -> PyResult<Bound<'py, pyo3::PyAny>> {
        let s = extract_iso_string(dt_str)?;
        let result = self
            .inner
            .isoparse(&s)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let tzinfo = match result.tz {
            Some(tz) => Some(isotz_to_py(py, tz)?),
            None => None,
        };
        ndt_to_py_datetime(py, result.datetime, tzinfo.as_ref())
    }

    fn parse_isodate<'py>(
        &self,
        py: Python<'py>,
        datestr: &Bound<'py, pyo3::PyAny>,
    ) -> PyResult<Bound<'py, pyo3::PyAny>> {
        let s = extract_iso_string(datestr)?;
        let date = self
            .inner
            .parse_isodate(&s)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(PyDate::new(py, date.year(), date.month() as u8, date.day() as u8)?.into_any())
    }

    fn parse_isotime<'py>(
        &self,
        py: Python<'py>,
        timestr: &Bound<'py, pyo3::PyAny>,
    ) -> PyResult<Bound<'py, pyo3::PyAny>> {
        let s = extract_iso_string(timestr)?;
        let result = self
            .inner
            .parse_isotime(&s)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let tzinfo = match result.tz {
            Some(tz) => Some(isotz_to_py(py, tz)?),
            None => None,
        };
        let us = (result.time.nanosecond() / 1000) % 1_000_000;
        Ok(PyTime::new(
            py,
            result.time.hour() as u8,
            result.time.minute() as u8,
            result.time.second() as u8,
            us,
            tzinfo.as_ref(),
        )?
        .into_any())
    }

    #[pyo3(signature = (tzstr, zero_as_utc=true))]
    fn parse_tzstr<'py>(
        &self,
        py: Python<'py>,
        tzstr: &Bound<'py, pyo3::PyAny>,
        zero_as_utc: bool,
    ) -> PyResult<Bound<'py, pyo3::PyAny>> {
        let s = extract_iso_string(tzstr)?;
        let result = self
            .inner
            .parse_tzstr(&s, zero_as_utc)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(isotz_to_py(py, result)?.into_any())
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyParserInfo>()?;
    m.add_class::<PyIsoParser>()?;
    m.add_function(pyo3::wrap_pyfunction!(parse_py, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_to_dict_py, m)?)?;
    Ok(())
}
