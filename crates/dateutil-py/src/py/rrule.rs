use std::sync::{Arc, Mutex, OnceLock};

use super::common::PyWeekday;
use dateutil_core::common::Weekday;
use dateutil_core::rrule::iter::RRuleIter as CoreRRuleIter;
use dateutil_core::rrule::{Frequency, Recurrence, RRule, RRuleBuilder};
use dateutil_core::rrule::parse::{rrulestr as core_rrulestr, RRuleStrResult};
use dateutil_core::rrule::set::{RRuleSet, RRuleSetIter as CoreRRuleSetIter};
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
    cache_enabled: bool,
    cache: OnceLock<Arc<Vec<chrono::NaiveDateTime>>>,
}

impl PyRRule {
    /// Return the cached result list if caching is enabled.
    fn get_cache(&self) -> Option<&Arc<Vec<chrono::NaiveDateTime>>> {
        if !self.cache_enabled {
            return None;
        }
        Some(self.cache.get_or_init(|| Arc::new(self.inner.all())))
    }
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
        cache=false,
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
        cache: bool,
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
        let cache_enabled = cache && inner.is_finite();
        Ok(Self {
            inner,
            cache_enabled,
            cache: OnceLock::new(),
        })
    }

    // -----------------------------------------------------------------------
    // Property getters
    // -----------------------------------------------------------------------

    #[getter]
    fn freq(&self) -> u8 {
        self.inner.freq() as u8
    }

    #[getter]
    fn dtstart(&self) -> chrono::NaiveDateTime {
        self.inner.dtstart()
    }

    #[getter]
    fn interval(&self) -> u32 {
        self.inner.interval()
    }

    #[getter]
    fn wkst(&self) -> u8 {
        self.inner.wkst()
    }

    #[getter]
    fn count(&self) -> Option<u32> {
        self.inner.count()
    }

    #[getter]
    fn until(&self) -> Option<chrono::NaiveDateTime> {
        self.inner.until()
    }

    #[getter]
    fn bysetpos(&self) -> Option<Vec<i32>> {
        self.inner.bysetpos().map(|s| s.to_vec())
    }

    #[getter]
    fn bymonth(&self) -> Option<Vec<u32>> {
        self.inner
            .bymonth()
            .map(|s| s.iter().map(|&v| v as u32).collect())
    }

    #[getter]
    fn byyearday(&self) -> Option<Vec<i32>> {
        self.inner.byyearday().map(|s| s.to_vec())
    }

    #[getter]
    fn byeaster(&self) -> Option<Vec<i32>> {
        self.inner.byeaster().map(|s| s.to_vec())
    }

    #[getter]
    fn byweekno(&self) -> Option<Vec<i32>> {
        self.inner.byweekno().map(|s| s.to_vec())
    }

    #[getter]
    fn byweekday(&self) -> Option<Vec<PyWeekday>> {
        self.inner
            .byweekday()
            .map(|wds| wds.iter().copied().map(PyWeekday::from).collect())
    }

    #[getter]
    fn byhour(&self) -> Option<Vec<u32>> {
        self.inner
            .byhour()
            .map(|s| s.iter().map(|&v| v as u32).collect())
    }

    #[getter]
    fn byminute(&self) -> Option<Vec<u32>> {
        self.inner
            .byminute()
            .map(|s| s.iter().map(|&v| v as u32).collect())
    }

    #[getter]
    fn bysecond(&self) -> Option<Vec<u32>> {
        self.inner
            .bysecond()
            .map(|s| s.iter().map(|&v| v as u32).collect())
    }

    // -----------------------------------------------------------------------
    // Query methods (with cache support)
    // -----------------------------------------------------------------------

    fn all(&self) -> PyResult<Vec<chrono::NaiveDateTime>> {
        if let Some(cached) = self.get_cache() {
            return Ok(cached.to_vec());
        }
        if !self.inner.is_finite() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "all() called on infinite recurrence (set count or until)",
            ));
        }
        Ok(self.inner.all())
    }

    #[pyo3(signature = (dt, inc=false))]
    fn before(&self, dt: chrono::NaiveDateTime, inc: bool) -> Option<chrono::NaiveDateTime> {
        if let Some(cached) = self.get_cache() {
            let idx = if inc {
                cached.partition_point(|&x| x <= dt)
            } else {
                cached.partition_point(|&x| x < dt)
            };
            return if idx > 0 { Some(cached[idx - 1]) } else { None };
        }
        self.inner.before(dt, inc)
    }

    #[pyo3(signature = (dt, inc=false))]
    fn after(&self, dt: chrono::NaiveDateTime, inc: bool) -> Option<chrono::NaiveDateTime> {
        if let Some(cached) = self.get_cache() {
            let idx = if inc {
                cached.partition_point(|&x| x < dt)
            } else {
                cached.partition_point(|&x| x <= dt)
            };
            return cached.get(idx).copied();
        }
        self.inner.after(dt, inc)
    }

    #[pyo3(signature = (after, before, inc=false))]
    fn between(
        &self,
        after: chrono::NaiveDateTime,
        before: chrono::NaiveDateTime,
        inc: bool,
    ) -> Vec<chrono::NaiveDateTime> {
        if let Some(cached) = self.get_cache() {
            let start = if inc {
                cached.partition_point(|&x| x < after)
            } else {
                cached.partition_point(|&x| x <= after)
            };
            let end = if inc {
                cached.partition_point(|&x| x <= before)
            } else {
                cached.partition_point(|&x| x < before)
            };
            return cached[start..end].to_vec();
        }
        self.inner.between(after, before, inc)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRRuleIter {
        if let Some(cached) = slf.get_cache() {
            return PyRRuleIter {
                inner: PyRRuleIterInner::Cached { data: Arc::clone(cached), idx: 0 },
            };
        }
        PyRRuleIter {
            inner: PyRRuleIterInner::Lazy(Box::new(slf.inner.iter())),
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
// PyRRuleIter — Python iterator for rrule (lazy or cached)
// ---------------------------------------------------------------------------

enum PyRRuleIterInner {
    Lazy(Box<CoreRRuleIter>),
    Cached { data: Arc<Vec<chrono::NaiveDateTime>>, idx: usize },
}

#[pyclass]
struct PyRRuleIter {
    inner: PyRRuleIterInner,
}

#[pymethods]
impl PyRRuleIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<chrono::NaiveDateTime> {
        match &mut self.inner {
            PyRRuleIterInner::Lazy(iter) => iter.next(),
            PyRRuleIterInner::Cached { data, idx } => {
                let val = data.get(*idx).copied();
                if val.is_some() { *idx += 1; }
                val
            }
        }
    }
}

// ---------------------------------------------------------------------------
// PyRRuleSet
// ---------------------------------------------------------------------------

#[pyclass(name = "rruleset")]
pub struct PyRRuleSet {
    inner: RRuleSet,
    cache_enabled: bool,
    cache: Mutex<Option<Arc<Vec<chrono::NaiveDateTime>>>>,
}

impl PyRRuleSet {
    /// Return the cached result list, populating it on first access if caching is enabled.
    fn get_or_populate_cache(&self) -> Option<Arc<Vec<chrono::NaiveDateTime>>> {
        if !self.cache_enabled {
            return None;
        }
        let mut guard = self.cache.lock().unwrap();
        if let Some(ref cached) = *guard {
            return Some(Arc::clone(cached));
        }
        if !self.inner.is_finite() {
            return None;
        }
        let result = Arc::new(self.inner.all());
        *guard = Some(Arc::clone(&result));
        Some(result)
    }
}

#[pymethods]
impl PyRRuleSet {
    #[new]
    #[pyo3(signature = (cache=false))]
    fn new(cache: bool) -> Self {
        Self {
            inner: RRuleSet::new(),
            cache_enabled: cache,
            cache: Mutex::new(None),
        }
    }

    fn rrule(&mut self, rule: &PyRRule) {
        self.inner.rrule(rule.inner.clone());
        *self.cache.get_mut().unwrap() = None;
    }

    fn rdate(&mut self, dt: chrono::NaiveDateTime) {
        self.inner.rdate(dt);
        *self.cache.get_mut().unwrap() = None;
    }

    fn exrule(&mut self, rule: &PyRRule) {
        self.inner.exrule(rule.inner.clone());
        *self.cache.get_mut().unwrap() = None;
    }

    fn exdate(&mut self, dt: chrono::NaiveDateTime) {
        self.inner.exdate(dt);
        *self.cache.get_mut().unwrap() = None;
    }

    fn all(&self) -> PyResult<Vec<chrono::NaiveDateTime>> {
        if let Some(cached) = self.get_or_populate_cache() {
            return Ok(cached.to_vec());
        }
        if !self.inner.is_finite() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "all() called on infinite recurrence (set count or until)",
            ));
        }
        Ok(self.inner.all())
    }

    #[pyo3(signature = (dt, inc=false))]
    fn before(&self, dt: chrono::NaiveDateTime, inc: bool) -> Option<chrono::NaiveDateTime> {
        if let Some(cached) = self.get_or_populate_cache() {
            let idx = if inc {
                cached.partition_point(|&x| x <= dt)
            } else {
                cached.partition_point(|&x| x < dt)
            };
            return if idx > 0 { Some(cached[idx - 1]) } else { None };
        }
        self.inner.before(dt, inc)
    }

    #[pyo3(signature = (dt, inc=false))]
    fn after(&self, dt: chrono::NaiveDateTime, inc: bool) -> Option<chrono::NaiveDateTime> {
        if let Some(cached) = self.get_or_populate_cache() {
            let idx = if inc {
                cached.partition_point(|&x| x < dt)
            } else {
                cached.partition_point(|&x| x <= dt)
            };
            return cached.get(idx).copied();
        }
        self.inner.after(dt, inc)
    }

    #[pyo3(signature = (after, before, inc=false))]
    fn between(
        &self,
        after: chrono::NaiveDateTime,
        before: chrono::NaiveDateTime,
        inc: bool,
    ) -> Vec<chrono::NaiveDateTime> {
        if let Some(cached) = self.get_or_populate_cache() {
            let start = if inc {
                cached.partition_point(|&x| x < after)
            } else {
                cached.partition_point(|&x| x <= after)
            };
            let end = if inc {
                cached.partition_point(|&x| x <= before)
            } else {
                cached.partition_point(|&x| x < before)
            };
            return cached[start..end].to_vec();
        }
        self.inner.between(after, before, inc)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRRuleSetIter {
        if let Some(cached) = slf.get_or_populate_cache() {
            return PyRRuleSetIter {
                inner: PyRRuleSetIterInner::Cached { data: cached, idx: 0 },
            };
        }
        PyRRuleSetIter {
            inner: PyRRuleSetIterInner::Lazy(Box::new(slf.inner.iter())),
        }
    }
}

// ---------------------------------------------------------------------------
// PyRRuleSetIter
// ---------------------------------------------------------------------------

enum PyRRuleSetIterInner {
    Lazy(Box<CoreRRuleSetIter>),
    Cached { data: Arc<Vec<chrono::NaiveDateTime>>, idx: usize },
}

#[pyclass]
struct PyRRuleSetIter {
    inner: PyRRuleSetIterInner,
}

#[pymethods]
impl PyRRuleSetIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<chrono::NaiveDateTime> {
        match &mut self.inner {
            PyRRuleSetIterInner::Lazy(iter) => iter.next(),
            PyRRuleSetIterInner::Cached { data, idx } => {
                let val = data.get(*idx).copied();
                if val.is_some() { *idx += 1; }
                val
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rrulestr function
// ---------------------------------------------------------------------------

#[pyfunction]
#[pyo3(name = "rrulestr", signature = (s, dtstart=None, forceset=false, compatible=false, unfold=false, cache=false))]
fn rrulestr_py(
    py: Python<'_>,
    s: &str,
    dtstart: Option<chrono::NaiveDateTime>,
    forceset: bool,
    compatible: bool,
    unfold: bool,
    cache: bool,
) -> PyResult<Py<PyAny>> {
    let result = core_rrulestr(s, dtstart, forceset, compatible, unfold)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    match result {
        RRuleStrResult::Single(rule) => {
            let cache_enabled = cache && rule.is_finite();
            Ok(PyRRule {
                inner: *rule,
                cache_enabled,
                cache: OnceLock::new(),
            }
            .into_pyobject(py)?
            .into_any()
            .unbind())
        }
        RRuleStrResult::Set(set) => {
            Ok(PyRRuleSet {
                inner: set,
                cache_enabled: cache,
                cache: Mutex::new(None),
            }
            .into_pyobject(py)?
            .into_any()
            .unbind())
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
