use std::sync::{Arc, Mutex, OnceLock};

use super::common::PyWeekday;
use super::conv::{extract_i32_list, extract_u8_list, py_any_to_naive_datetime};
use dateutil::common::Weekday;
use dateutil::rrule::iter::RRuleIter as CoreRRuleIter;
use dateutil::rrule::parse::{rrulestr as core_rrulestr, RRuleStrResult};
use dateutil::rrule::set::{RRuleSet, RRuleSetIter as CoreRRuleSetIter};
use dateutil::rrule::{
    search_after, search_before, search_between, search_xafter, signed_index, slice_sorted,
    Frequency, RRule, RRuleBuilder, Recurrence,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyList, PySlice};

// ---------------------------------------------------------------------------
// Helper: accept weekday scalar-or-sequence for byweekday parameter
// ---------------------------------------------------------------------------

/// Accept a single weekday/int or a list/tuple of weekday/int and return `Vec<Weekday>`.
fn extract_byweekday_any(obj: &Bound<'_, PyAny>) -> PyResult<Vec<Weekday>> {
    // Try as a single weekday object
    if let Ok(wd) = obj.extract::<PyWeekday>() {
        return Ok(vec![wd.into()]);
    }
    // Try as a single int (0-6)
    if let Ok(n) = obj.extract::<u8>() {
        let wd = Weekday::new(n, None).map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(vec![wd]);
    }
    // Then try as any iterable (list, tuple, etc.)
    let iter = obj.try_iter().map_err(|_| {
        pyo3::exceptions::PyTypeError::new_err(
            "byweekday must be a weekday, int, or iterable of weekdays/ints",
        )
    })?;
    let mut result = Vec::new();
    for item in iter {
        let item = item?;
        if let Ok(wd) = item.extract::<PyWeekday>() {
            result.push(wd.into());
        } else if let Ok(n) = item.extract::<u8>() {
            let wd = Weekday::new(n, None).map_err(|e| PyValueError::new_err(e.to_string()))?;
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
// Frequency constants (match python-dateutil convention)
// ---------------------------------------------------------------------------

const YEARLY: u8 = Frequency::Yearly as u8;
const MONTHLY: u8 = Frequency::Monthly as u8;
const WEEKLY: u8 = Frequency::Weekly as u8;
const DAILY: u8 = Frequency::Daily as u8;
const HOURLY: u8 = Frequency::Hourly as u8;
const MINUTELY: u8 = Frequency::Minutely as u8;
const SECONDLY: u8 = Frequency::Secondly as u8;

// ---------------------------------------------------------------------------
// PyRRule
// ---------------------------------------------------------------------------

#[pyclass(name = "rrule", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyRRule {
    inner: Arc<RRule>,
    cache_enabled: bool,
    cache: OnceLock<Arc<Vec<chrono::NaiveDateTime>>>,
}

impl PyRRule {
    /// Return the cached result list if caching is enabled.
    fn get_cache(&self) -> Option<&Arc<Vec<chrono::NaiveDateTime>>> {
        if !self.cache_enabled {
            return None;
        }
        Some(self.cache.get_or_init(|| {
            Arc::new(
                self.inner
                    .all()
                    .expect("cache init: finite rrule should not fail"),
            )
        }))
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
        py: Python<'_>,
        freq: u8,
        dtstart: Option<&Bound<'_, PyAny>>,
        interval: u32,
        wkst: Option<Bound<'_, PyAny>>,
        count: Option<u32>,
        until: Option<&Bound<'_, PyAny>>,
        bysetpos: Option<Bound<'_, PyAny>>,
        bymonth: Option<Bound<'_, PyAny>>,
        bymonthday: Option<Bound<'_, PyAny>>,
        byyearday: Option<Bound<'_, PyAny>>,
        byeaster: Option<Bound<'_, PyAny>>,
        byweekno: Option<Bound<'_, PyAny>>,
        byweekday: Option<Bound<'_, PyAny>>,
        byhour: Option<Bound<'_, PyAny>>,
        byminute: Option<Bound<'_, PyAny>>,
        bysecond: Option<Bound<'_, PyAny>>,
        cache: bool,
    ) -> PyResult<Self> {
        let f = Frequency::try_from(freq).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let mut builder = RRuleBuilder::new(f).interval(interval);

        if let Some(obj) = dtstart {
            builder = builder.dtstart(py_any_to_naive_datetime(obj)?);
        }
        if let Some(ref w) = wkst {
            let val = if let Ok(wd) = w.extract::<PyWeekday>() {
                wd.weekday()
            } else {
                w.extract::<u8>()?
            };
            builder = builder.wkst(val);
        }
        // RFC 5545 3.3.10: UNTIL and COUNT are mutually exclusive
        if count.is_some() && until.is_some() {
            let warnings = py.import("warnings")?;
            warnings.call_method1(
                "warn",
                (
                    "Using both 'count' and 'until' is inconsistent with \
                     RFC 5545 and has been deprecated in dateutil. Future \
                     versions will raise an error.",
                    py.get_type::<pyo3::exceptions::PyDeprecationWarning>(),
                ),
            )?;
        }
        if let Some(c) = count {
            builder = builder.count(c);
        }
        if let Some(obj) = until {
            builder = builder.until(py_any_to_naive_datetime(obj)?);
        }
        if let Some(ref v) = bysetpos {
            builder = builder.bysetpos(extract_i32_list(v)?);
        }
        if let Some(ref v) = bymonth {
            builder = builder.bymonth(extract_u8_list(v)?);
        }
        if let Some(ref v) = bymonthday {
            builder = builder.bymonthday(extract_i32_list(v)?);
        }
        if let Some(ref v) = byyearday {
            builder = builder.byyearday(extract_i32_list(v)?);
        }
        if let Some(ref v) = byeaster {
            builder = builder.byeaster(extract_i32_list(v)?);
        }
        if let Some(ref v) = byweekno {
            builder = builder.byweekno(extract_i32_list(v)?);
        }
        if let Some(ref v) = byweekday {
            builder = builder.byweekday(extract_byweekday_any(v)?);
        }
        if let Some(ref v) = byhour {
            builder = builder.byhour(extract_u8_list(v)?);
        }
        if let Some(ref v) = byminute {
            builder = builder.byminute(extract_u8_list(v)?);
        }
        if let Some(ref v) = bysecond {
            builder = builder.bysecond(extract_u8_list(v)?);
        }

        let inner = builder
            .build()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let cache_enabled = cache && inner.is_finite();
        Ok(Self {
            inner: Arc::new(inner),
            cache_enabled,
            cache: OnceLock::new(),
        })
    }

    // -----------------------------------------------------------------------
    // Property getters
    // -----------------------------------------------------------------------

    #[getter]
    fn _cache<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyList>>> {
        match self.cache.get() {
            Some(arc) => Ok(Some(PyList::new(py, arc.iter().copied())?)),
            None => Ok(None),
        }
    }

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
    fn _count(&self) -> Option<u32> {
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

    fn all(&self, py: Python<'_>) -> PyResult<Vec<chrono::NaiveDateTime>> {
        if let Some(cached) = self.get_cache() {
            return Ok(cached.to_vec());
        }
        if !self.inner.is_finite() {
            return Err(PyValueError::new_err(
                "all() called on infinite recurrence (set count or until)",
            ));
        }
        let inner = &self.inner;
        py.detach(|| inner.all())
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[pyo3(signature = (dt, inc=false))]
    fn before(&self, dt: chrono::NaiveDateTime, inc: bool) -> Option<chrono::NaiveDateTime> {
        if let Some(cached) = self.get_cache() {
            return search_before(cached, dt, inc);
        }
        self.inner.before(dt, inc)
    }

    #[pyo3(signature = (dt, inc=false))]
    fn after(&self, dt: chrono::NaiveDateTime, inc: bool) -> Option<chrono::NaiveDateTime> {
        if let Some(cached) = self.get_cache() {
            return search_after(cached, dt, inc);
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
            return search_between(cached, after, before, inc).to_vec();
        }
        self.inner.between(after, before, inc)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRRuleIter {
        if let Some(cached) = slf.get_cache() {
            return PyRRuleIter {
                inner: PyRRuleIterInner::Cached {
                    data: Arc::clone(cached),
                    idx: 0,
                },
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

    // -----------------------------------------------------------------------
    // Sequence protocol: __getitem__, count, __contains__
    // -----------------------------------------------------------------------

    fn __getitem__(&self, py: Python<'_>, item: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        if let Ok(idx) = item.extract::<isize>() {
            return self.getitem_int(py, idx);
        }
        if let Ok(slice) = item.cast::<PySlice>() {
            return self.getitem_slice(py, slice);
        }
        Err(pyo3::exceptions::PyTypeError::new_err(
            "rrule indices must be integers or slices",
        ))
    }

    /// Return the number of occurrences (finite rules only).
    fn count(&self) -> PyResult<usize> {
        if let Some(cached) = self.get_cache() {
            return Ok(cached.len());
        }
        self.inner.len().ok_or_else(|| {
            PyValueError::new_err("count() called on infinite recurrence (set count or until)")
        })
    }

    fn __contains__(&self, dt: chrono::NaiveDateTime) -> bool {
        if let Some(cached) = self.get_cache() {
            return cached.binary_search(&dt).is_ok();
        }
        self.inner.contains(dt)
    }

    /// Return an iterator yielding `count` occurrences after `dt`.
    #[pyo3(signature = (dt, count=1, inc=false))]
    fn xafter(
        &self,
        dt: chrono::NaiveDateTime,
        count: usize,
        inc: bool,
    ) -> Vec<chrono::NaiveDateTime> {
        if let Some(cached) = self.get_cache() {
            return search_xafter(cached, dt, count, inc);
        }
        self.inner.xafter(dt, count, inc)
    }

    /// Return a new rrule with specified parameters replaced.
    #[pyo3(signature = (
        freq=None,
        dtstart=None,
        interval=None,
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
        cache=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn replace(
        &self,
        freq: Option<u8>,
        dtstart: Option<&Bound<'_, PyAny>>,
        interval: Option<u32>,
        wkst: Option<Bound<'_, PyAny>>,
        count: Option<u32>,
        until: Option<&Bound<'_, PyAny>>,
        bysetpos: Option<Bound<'_, PyAny>>,
        bymonth: Option<Bound<'_, PyAny>>,
        bymonthday: Option<Bound<'_, PyAny>>,
        byyearday: Option<Bound<'_, PyAny>>,
        byeaster: Option<Bound<'_, PyAny>>,
        byweekno: Option<Bound<'_, PyAny>>,
        byweekday: Option<Bound<'_, PyAny>>,
        byhour: Option<Bound<'_, PyAny>>,
        byminute: Option<Bound<'_, PyAny>>,
        bysecond: Option<Bound<'_, PyAny>>,
        cache: Option<bool>,
    ) -> PyResult<Self> {
        let mut builder = self.inner.to_builder();

        if let Some(fv) = freq {
            builder = builder
                .freq(Frequency::try_from(fv).map_err(|e| PyValueError::new_err(e.to_string()))?);
        }
        if let Some(obj) = dtstart {
            builder = builder.dtstart(py_any_to_naive_datetime(obj)?);
        }
        if let Some(v) = interval {
            builder = builder.interval(v);
        }
        if let Some(ref w) = wkst {
            let val = if let Ok(wd) = w.extract::<PyWeekday>() {
                wd.weekday()
            } else {
                w.extract::<u8>()?
            };
            builder = builder.wkst(val);
        }
        if let Some(c) = count {
            builder = builder.count(c);
        }
        if let Some(obj) = until {
            builder = builder.until(py_any_to_naive_datetime(obj)?);
        }
        if let Some(ref v) = bysetpos {
            builder = builder.bysetpos(extract_i32_list(v)?);
        }
        if let Some(ref v) = bymonth {
            builder = builder.bymonth(extract_u8_list(v)?);
        }
        if let Some(ref v) = bymonthday {
            builder = builder.bymonthday(extract_i32_list(v)?);
        }
        if let Some(ref v) = byyearday {
            builder = builder.byyearday(extract_i32_list(v)?);
        }
        if let Some(ref v) = byeaster {
            builder = builder.byeaster(extract_i32_list(v)?);
        }
        if let Some(ref v) = byweekno {
            builder = builder.byweekno(extract_i32_list(v)?);
        }
        if let Some(ref v) = byweekday {
            builder = builder.byweekday(extract_byweekday_any(v)?);
        }
        if let Some(ref v) = byhour {
            builder = builder.byhour(extract_u8_list(v)?);
        }
        if let Some(ref v) = byminute {
            builder = builder.byminute(extract_u8_list(v)?);
        }
        if let Some(ref v) = bysecond {
            builder = builder.bysecond(extract_u8_list(v)?);
        }

        let inner = builder
            .build()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let cache_val = cache.unwrap_or(self.cache_enabled);
        let cache_enabled = cache_val && inner.is_finite();
        Ok(Self {
            inner: Arc::new(inner),
            cache_enabled,
            cache: OnceLock::new(),
        })
    }
}

impl PyRRule {
    fn getitem_int(&self, py: Python<'_>, idx: isize) -> PyResult<Py<PyAny>> {
        let dt = if let Some(cached) = self.get_cache() {
            signed_index(cached, idx)
        } else {
            if idx < 0 && !self.inner.is_finite() {
                return Err(PyValueError::new_err(
                    "negative index on infinite recurrence",
                ));
            }
            self.inner.signed_nth(idx)
        };
        dt.ok_or_else(|| pyo3::exceptions::PyIndexError::new_err("rrule index out of range"))
            .and_then(|dt| Ok(dt.into_pyobject(py)?.into_any().unbind()))
    }

    fn getitem_slice(&self, py: Python<'_>, slice: &Bound<'_, PySlice>) -> PyResult<Py<PyAny>> {
        if let Some(cached) = self.get_cache() {
            let indices = slice.indices(cached.len() as isize)?;
            let result = slice_sorted(cached, indices.start, indices.stop, indices.step);
            return Ok(result.into_pyobject(py)?.into_any().unbind());
        }

        let start: Option<isize> = slice.getattr("start")?.extract().ok();
        let stop: Option<isize> = slice.getattr("stop")?.extract().ok();
        let step: Option<isize> = slice.getattr("step")?.extract().ok();
        let step_val = step.unwrap_or(1);

        if step_val == 0 {
            return Err(PyValueError::new_err("slice step cannot be zero"));
        }

        if step_val < 0 || start.is_some_and(|s| s < 0) || stop.is_some_and(|s| s < 0) {
            if !self.inner.is_finite() {
                return Err(PyValueError::new_err(
                    "negative slice index/step on infinite recurrence",
                ));
            }
            let all = self
                .inner
                .all()
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            let indices = slice.indices(all.len() as isize)?;
            let result = slice_sorted(&all, indices.start, indices.stop, indices.step);
            return Ok(result.into_pyobject(py)?.into_any().unbind());
        }

        let s = start.unwrap_or(0) as usize;
        let e = stop.map(|v| v as usize).unwrap_or(usize::MAX);
        let step_u = step_val as usize;
        let result = self.inner.take_slice(s, e, step_u);
        Ok(result.into_pyobject(py)?.into_any().unbind())
    }
}

// ---------------------------------------------------------------------------
// PyRRuleIter — Python iterator for rrule (lazy or cached)
// ---------------------------------------------------------------------------

enum PyRRuleIterInner {
    Lazy(Box<CoreRRuleIter>),
    Cached {
        data: Arc<Vec<chrono::NaiveDateTime>>,
        idx: usize,
    },
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

    fn __next__(&mut self) -> PyResult<Option<chrono::NaiveDateTime>> {
        match &mut self.inner {
            PyRRuleIterInner::Lazy(iter) => {
                let val = iter.next();
                if val.is_none() && iter.diverged() {
                    return Err(PyValueError::new_err(
                        "bad combination of interval and by* filters: \
                         the recurrence rule can never produce results",
                    ));
                }
                Ok(val)
            }
            PyRRuleIterInner::Cached { data, idx } => {
                let val = data.get(*idx).copied();
                *idx += val.is_some() as usize;
                Ok(val)
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
        let result = Arc::new(
            self.inner
                .all()
                .expect("cache init: finite rruleset should not fail"),
        );
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

    #[getter]
    fn _cache<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyList>>> {
        let arc_opt = {
            let guard = self.cache.lock().unwrap();
            guard.as_ref().map(Arc::clone)
        };
        match arc_opt {
            Some(arc) => Ok(Some(PyList::new(py, arc.iter().copied())?)),
            None => Ok(None),
        }
    }

    fn rrule(&mut self, rule: &PyRRule) {
        self.inner.rrule_shared(Arc::clone(&rule.inner));
        *self.cache.get_mut().unwrap() = None;
    }

    fn rdate(&mut self, dt: chrono::NaiveDateTime) {
        self.inner.rdate(dt);
        *self.cache.get_mut().unwrap() = None;
    }

    fn exrule(&mut self, rule: &PyRRule) {
        self.inner.exrule_shared(Arc::clone(&rule.inner));
        *self.cache.get_mut().unwrap() = None;
    }

    fn exdate(&mut self, dt: chrono::NaiveDateTime) {
        self.inner.exdate(dt);
        *self.cache.get_mut().unwrap() = None;
    }

    fn all(&self, py: Python<'_>) -> PyResult<Vec<chrono::NaiveDateTime>> {
        if let Some(cached) = self.get_or_populate_cache() {
            return Ok(cached.to_vec());
        }
        if !self.inner.is_finite() {
            return Err(PyValueError::new_err(
                "all() called on infinite recurrence (set count or until)",
            ));
        }
        let inner = &self.inner;
        py.detach(|| inner.all())
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[pyo3(signature = (dt, inc=false))]
    fn before(&self, dt: chrono::NaiveDateTime, inc: bool) -> Option<chrono::NaiveDateTime> {
        if let Some(cached) = self.get_or_populate_cache() {
            return search_before(&cached, dt, inc);
        }
        self.inner.before(dt, inc)
    }

    #[pyo3(signature = (dt, inc=false))]
    fn after(&self, dt: chrono::NaiveDateTime, inc: bool) -> Option<chrono::NaiveDateTime> {
        if let Some(cached) = self.get_or_populate_cache() {
            return search_after(&cached, dt, inc);
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
            return search_between(&cached, after, before, inc).to_vec();
        }
        self.inner.between(after, before, inc)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRRuleSetIter {
        if let Some(cached) = slf.get_or_populate_cache() {
            return PyRRuleSetIter {
                inner: PyRRuleSetIterInner::Cached {
                    data: cached,
                    idx: 0,
                },
            };
        }
        PyRRuleSetIter {
            inner: PyRRuleSetIterInner::Lazy(Box::new(slf.inner.iter())),
        }
    }

    // -----------------------------------------------------------------------
    // Sequence protocol: __getitem__, count, __contains__
    // -----------------------------------------------------------------------

    fn __getitem__(&self, py: Python<'_>, item: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        if let Ok(idx) = item.extract::<isize>() {
            return self.getitem_int(py, idx);
        }
        if let Ok(slice) = item.cast::<PySlice>() {
            return self.getitem_slice(py, slice);
        }
        Err(pyo3::exceptions::PyTypeError::new_err(
            "rruleset indices must be integers or slices",
        ))
    }

    fn count(&self) -> PyResult<usize> {
        if let Some(cached) = self.get_or_populate_cache() {
            return Ok(cached.len());
        }
        self.inner.len().ok_or_else(|| {
            PyValueError::new_err("count() called on infinite recurrence (set count or until)")
        })
    }

    fn __contains__(&self, dt: chrono::NaiveDateTime) -> bool {
        if let Some(cached) = self.get_or_populate_cache() {
            return cached.binary_search(&dt).is_ok();
        }
        self.inner.contains(dt)
    }
}

impl PyRRuleSet {
    fn getitem_int(&self, py: Python<'_>, idx: isize) -> PyResult<Py<PyAny>> {
        let dt = if let Some(cached) = self.get_or_populate_cache() {
            signed_index(&cached, idx)
        } else {
            if idx < 0 && !self.inner.is_finite() {
                return Err(PyValueError::new_err(
                    "negative index on infinite recurrence",
                ));
            }
            self.inner.signed_nth(idx)
        };
        dt.ok_or_else(|| pyo3::exceptions::PyIndexError::new_err("rruleset index out of range"))
            .and_then(|dt| Ok(dt.into_pyobject(py)?.into_any().unbind()))
    }

    fn getitem_slice(&self, py: Python<'_>, slice: &Bound<'_, PySlice>) -> PyResult<Py<PyAny>> {
        if let Some(cached) = self.get_or_populate_cache() {
            let indices = slice.indices(cached.len() as isize)?;
            let result = slice_sorted(&cached, indices.start, indices.stop, indices.step);
            return Ok(result.into_pyobject(py)?.into_any().unbind());
        }

        let start: Option<isize> = slice.getattr("start")?.extract().ok();
        let stop: Option<isize> = slice.getattr("stop")?.extract().ok();
        let step: Option<isize> = slice.getattr("step")?.extract().ok();
        let step_val = step.unwrap_or(1);

        if step_val == 0 {
            return Err(PyValueError::new_err("slice step cannot be zero"));
        }

        if step_val < 0 || start.is_some_and(|s| s < 0) || stop.is_some_and(|s| s < 0) {
            if !self.inner.is_finite() {
                return Err(PyValueError::new_err(
                    "negative slice index/step on infinite recurrence",
                ));
            }
            let all = self
                .inner
                .all()
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            let indices = slice.indices(all.len() as isize)?;
            let result = slice_sorted(&all, indices.start, indices.stop, indices.step);
            return Ok(result.into_pyobject(py)?.into_any().unbind());
        }

        let s = start.unwrap_or(0) as usize;
        let e = stop.map(|v| v as usize).unwrap_or(usize::MAX);
        let step_u = step_val as usize;
        let result = self.inner.take_slice(s, e, step_u);
        Ok(result.into_pyobject(py)?.into_any().unbind())
    }
}

// ---------------------------------------------------------------------------
// PyRRuleSetIter
// ---------------------------------------------------------------------------

enum PyRRuleSetIterInner {
    Lazy(Box<CoreRRuleSetIter>),
    Cached {
        data: Arc<Vec<chrono::NaiveDateTime>>,
        idx: usize,
    },
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
                *idx += val.is_some() as usize;
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
    dtstart: Option<&Bound<'_, PyAny>>,
    forceset: bool,
    compatible: bool,
    unfold: bool,
    cache: bool,
) -> PyResult<Py<PyAny>> {
    let dtstart = dtstart.map(py_any_to_naive_datetime).transpose()?;
    let result = py
        .detach(|| core_rrulestr(s, dtstart, forceset, compatible, unfold))
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    match result {
        RRuleStrResult::Single(rule) => {
            let cache_enabled = cache && rule.is_finite();
            Ok(PyRRule {
                inner: Arc::from(rule),
                cache_enabled,
                cache: OnceLock::new(),
            }
            .into_pyobject(py)?
            .into_any()
            .unbind())
        }
        RRuleStrResult::Set(set) => Ok(PyRRuleSet {
            inner: set,
            cache_enabled: cache,
            cache: Mutex::new(None),
        }
        .into_pyobject(py)?
        .into_any()
        .unbind()),
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
