use super::common::PyWeekday;
use dateutil_core::common::Weekday;
use dateutil_core::rrule::{Frequency, Recurrence, RRule, RRuleBuilder};
use dateutil_core::rrule::parse::{rrulestr as core_rrulestr, RRuleStrResult};
use dateutil_core::rrule::set::RRuleSet;
use pyo3::prelude::*;
use pyo3::types::PyList;

// ---------------------------------------------------------------------------
// Frequency constants (match python-dateutil convention)
// ---------------------------------------------------------------------------

const YEARLY: u8 = 0;
const MONTHLY: u8 = 1;
const WEEKLY: u8 = 2;
const DAILY: u8 = 3;
const HOURLY: u8 = 4;
const MINUTELY: u8 = 5;
const SECONDLY: u8 = 6;

fn freq_from_int(v: u8) -> PyResult<Frequency> {
    match v {
        0 => Ok(Frequency::Yearly),
        1 => Ok(Frequency::Monthly),
        2 => Ok(Frequency::Weekly),
        3 => Ok(Frequency::Daily),
        4 => Ok(Frequency::Hourly),
        5 => Ok(Frequency::Minutely),
        6 => Ok(Frequency::Secondly),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "invalid frequency: {v}"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Helper: extract byweekday from heterogeneous Python list
// ---------------------------------------------------------------------------

/// Accept a list of weekday objects or plain ints (0-6).
fn extract_byweekday(list: &Bound<'_, PyList>) -> PyResult<Vec<Weekday>> {
    let mut result = Vec::with_capacity(list.len());
    for item in list.iter() {
        if let Ok(wd) = item.extract::<PyWeekday>() {
            result.push(wd.into());
        } else if let Ok(n) = item.extract::<u8>() {
            let wd = Weekday::new(n, None)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            result.push(wd);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "byweekday items must be weekday objects or ints (0-6)",
            ));
        }
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// PyRRule
// ---------------------------------------------------------------------------

#[pyclass(name = "rrule", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyRRule {
    inner: RRule,
}

#[pymethods]
impl PyRRule {
    #[new]
    #[pyo3(signature = (
        freq,
        dtstart=None,
        interval=1,
        wkst=None,
        count=None,
        until=None,
        bysetpos=None,
        bymonth=None,
        bymonthday=None,
        byyearday=None,
        byeaster=None,
        byweekno=None,
        byweekday=None,
        byhour=None,
        byminute=None,
        bysecond=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        freq: u8,
        dtstart: Option<chrono::NaiveDateTime>,
        interval: u32,
        wkst: Option<u8>,
        count: Option<u32>,
        until: Option<chrono::NaiveDateTime>,
        bysetpos: Option<Vec<i32>>,
        bymonth: Option<Vec<u8>>,
        bymonthday: Option<Vec<i32>>,
        byyearday: Option<Vec<i32>>,
        byeaster: Option<Vec<i32>>,
        byweekno: Option<Vec<i32>>,
        byweekday: Option<Bound<'_, PyList>>,
        byhour: Option<Vec<u8>>,
        byminute: Option<Vec<u8>>,
        bysecond: Option<Vec<u8>>,
    ) -> PyResult<Self> {
        let f = freq_from_int(freq)?;
        let mut builder = RRuleBuilder::new(f).interval(interval);

        if let Some(dt) = dtstart {
            builder = builder.dtstart(dt);
        }
        if let Some(w) = wkst {
            builder = builder.wkst(w);
        }
        if let Some(c) = count {
            builder = builder.count(c);
        }
        if let Some(u) = until {
            builder = builder.until(u);
        }
        if let Some(v) = bysetpos {
            builder = builder.bysetpos(v);
        }
        if let Some(v) = bymonth {
            builder = builder.bymonth(v);
        }
        if let Some(v) = bymonthday {
            builder = builder.bymonthday(v);
        }
        if let Some(v) = byyearday {
            builder = builder.byyearday(v);
        }
        if let Some(v) = byeaster {
            builder = builder.byeaster(v);
        }
        if let Some(v) = byweekno {
            builder = builder.byweekno(v);
        }
        if let Some(list) = byweekday {
            builder = builder.byweekday(extract_byweekday(&list)?);
        }
        if let Some(v) = byhour {
            builder = builder.byhour(v);
        }
        if let Some(v) = byminute {
            builder = builder.byminute(v);
        }
        if let Some(v) = bysecond {
            builder = builder.bysecond(v);
        }

        let inner = builder
            .build()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    #[getter]
    fn freq(&self) -> u8 {
        self.inner.freq() as u8
    }

    #[getter]
    fn dtstart(&self) -> chrono::NaiveDateTime {
        self.inner.dtstart()
    }

    fn all(&self) -> PyResult<Vec<chrono::NaiveDateTime>> {
        if !self.inner.is_finite() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "all() called on infinite recurrence (set count or until)",
            ));
        }
        Ok(self.inner.all())
    }

    #[pyo3(signature = (dt, inc=false))]
    fn before(&self, dt: chrono::NaiveDateTime, inc: bool) -> Option<chrono::NaiveDateTime> {
        self.inner.before(dt, inc)
    }

    #[pyo3(signature = (dt, inc=false))]
    fn after(&self, dt: chrono::NaiveDateTime, inc: bool) -> Option<chrono::NaiveDateTime> {
        self.inner.after(dt, inc)
    }

    #[pyo3(signature = (after, before, inc=false))]
    fn between(
        &self,
        after: chrono::NaiveDateTime,
        before: chrono::NaiveDateTime,
        inc: bool,
    ) -> Vec<chrono::NaiveDateTime> {
        self.inner.between(after, before, inc)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRRuleIter {
        PyRRuleIter {
            inner: slf.inner.iter().collect::<Vec<_>>().into_iter(),
        }
    }

    fn __repr__(&self) -> String {
        self.inner.to_string()
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

// ---------------------------------------------------------------------------
// PyRRuleIter — Python iterator for rrule
// ---------------------------------------------------------------------------

#[pyclass]
struct PyRRuleIter {
    inner: std::vec::IntoIter<chrono::NaiveDateTime>,
}

#[pymethods]
impl PyRRuleIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<chrono::NaiveDateTime> {
        self.inner.next()
    }
}

// ---------------------------------------------------------------------------
// PyRRuleSet
// ---------------------------------------------------------------------------

#[pyclass(name = "rruleset")]
pub struct PyRRuleSet {
    inner: RRuleSet,
}

#[pymethods]
impl PyRRuleSet {
    #[new]
    fn new() -> Self {
        Self {
            inner: RRuleSet::new(),
        }
    }

    fn rrule(&mut self, rule: &PyRRule) {
        self.inner.rrule(rule.inner.clone());
    }

    fn rdate(&mut self, dt: chrono::NaiveDateTime) {
        self.inner.rdate(dt);
    }

    fn exrule(&mut self, rule: &PyRRule) {
        self.inner.exrule(rule.inner.clone());
    }

    fn exdate(&mut self, dt: chrono::NaiveDateTime) {
        self.inner.exdate(dt);
    }

    fn all(&self) -> PyResult<Vec<chrono::NaiveDateTime>> {
        if !self.inner.is_finite() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "all() called on infinite recurrence (set count or until)",
            ));
        }
        Ok(self.inner.all())
    }

    #[pyo3(signature = (dt, inc=false))]
    fn before(&self, dt: chrono::NaiveDateTime, inc: bool) -> Option<chrono::NaiveDateTime> {
        self.inner.before(dt, inc)
    }

    #[pyo3(signature = (dt, inc=false))]
    fn after(&self, dt: chrono::NaiveDateTime, inc: bool) -> Option<chrono::NaiveDateTime> {
        self.inner.after(dt, inc)
    }

    #[pyo3(signature = (after, before, inc=false))]
    fn between(
        &self,
        after: chrono::NaiveDateTime,
        before: chrono::NaiveDateTime,
        inc: bool,
    ) -> Vec<chrono::NaiveDateTime> {
        self.inner.between(after, before, inc)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRRuleSetIter {
        PyRRuleSetIter {
            inner: slf.inner.iter().collect::<Vec<_>>().into_iter(),
        }
    }
}

// ---------------------------------------------------------------------------
// PyRRuleSetIter
// ---------------------------------------------------------------------------

#[pyclass]
struct PyRRuleSetIter {
    inner: std::vec::IntoIter<chrono::NaiveDateTime>,
}

#[pymethods]
impl PyRRuleSetIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<chrono::NaiveDateTime> {
        self.inner.next()
    }
}

// ---------------------------------------------------------------------------
// rrulestr function
// ---------------------------------------------------------------------------

#[pyfunction]
#[pyo3(name = "rrulestr", signature = (s, dtstart=None, forceset=false, compatible=false, unfold=false))]
fn rrulestr_py(
    py: Python<'_>,
    s: &str,
    dtstart: Option<chrono::NaiveDateTime>,
    forceset: bool,
    compatible: bool,
    unfold: bool,
) -> PyResult<Py<PyAny>> {
    let result = core_rrulestr(s, dtstart, forceset, compatible, unfold)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    match result {
        RRuleStrResult::Single(rule) => {
            Ok(PyRRule { inner: *rule }.into_pyobject(py)?.into_any().unbind())
        }
        RRuleStrResult::Set(set) => {
            Ok(PyRRuleSet { inner: set }.into_pyobject(py)?.into_any().unbind())
        }
    }
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("YEARLY", YEARLY)?;
    m.add("MONTHLY", MONTHLY)?;
    m.add("WEEKLY", WEEKLY)?;
    m.add("DAILY", DAILY)?;
    m.add("HOURLY", HOURLY)?;
    m.add("MINUTELY", MINUTELY)?;
    m.add("SECONDLY", SECONDLY)?;
    m.add_class::<PyRRule>()?;
    m.add_class::<PyRRuleSet>()?;
    m.add_function(pyo3::wrap_pyfunction!(rrulestr_py, m)?)?;
    Ok(())
}
