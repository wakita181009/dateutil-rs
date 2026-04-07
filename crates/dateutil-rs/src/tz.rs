pub mod file;
pub mod local;
pub mod offset;
pub mod range;
pub mod utc;

pub use file::TzFile;
pub use local::TzLocal;
pub use offset::TzOffset;
pub use range::{TzRange, TzStr};
pub use utc::TzUtc;

use chrono::{Duration, NaiveDateTime};

// ============================================================================
// Tz — Unified timezone enum
// ============================================================================

/// A timezone value (any supported kind).
#[derive(Debug, Clone)]
pub enum Tz {
    Utc(TzUtc),
    Offset(TzOffset),
    Range(TzRange),
    Str(TzStr),
    File(TzFile),
    Local(TzLocal),
}

impl Tz {
    pub fn utcoffset(&self, dt: Option<NaiveDateTime>, fold: bool) -> Option<Duration> {
        match self {
            Tz::Utc(tz) => tz.utcoffset(dt),
            Tz::Offset(tz) => tz.utcoffset(dt),
            Tz::Range(tz) => tz.utcoffset(dt, fold),
            Tz::Str(tz) => tz.utcoffset(dt, fold),
            Tz::File(tz) => tz.utcoffset(dt, fold),
            Tz::Local(tz) => tz.utcoffset(dt, fold),
        }
    }

    pub fn dst(&self, dt: Option<NaiveDateTime>, fold: bool) -> Option<Duration> {
        match self {
            Tz::Utc(tz) => tz.dst(dt),
            Tz::Offset(tz) => tz.dst(dt),
            Tz::Range(tz) => tz.dst(dt, fold),
            Tz::Str(tz) => tz.dst(dt, fold),
            Tz::File(tz) => tz.dst(dt, fold),
            Tz::Local(tz) => tz.dst(dt, fold),
        }
    }

    pub fn tzname(&self, dt: Option<NaiveDateTime>, fold: bool) -> Option<String> {
        match self {
            Tz::Utc(tz) => tz.tzname(dt),
            Tz::Offset(tz) => tz.tzname(dt),
            Tz::Range(tz) => tz.tzname(dt, fold),
            Tz::Str(tz) => tz.tzname(dt, fold),
            Tz::File(tz) => tz.tzname(dt, fold),
            Tz::Local(tz) => tz.tzname(dt, fold),
        }
    }

    pub fn is_ambiguous(&self, dt: NaiveDateTime) -> bool {
        match self {
            Tz::Utc(tz) => tz.is_ambiguous(dt),
            Tz::Offset(tz) => tz.is_ambiguous(dt),
            Tz::Range(tz) => tz.is_ambiguous(dt),
            Tz::Str(tz) => tz.is_ambiguous(dt),
            Tz::File(tz) => tz.is_ambiguous(dt),
            Tz::Local(tz) => tz.is_ambiguous(dt),
        }
    }

    pub fn fromutc(&self, dt: NaiveDateTime) -> (NaiveDateTime, bool) {
        match self {
            Tz::Utc(tz) => (tz.fromutc(dt), false),
            Tz::Offset(tz) => (tz.fromutc(dt), false),
            Tz::Range(tz) => tz.fromutc(dt),
            Tz::Str(tz) => tz.fromutc(dt),
            Tz::File(tz) => tz.fromutc(dt),
            Tz::Local(tz) => tz.fromutc(dt),
        }
    }
}

// ============================================================================
// gettz() — Timezone factory
// ============================================================================

/// Search paths for IANA timezone files on Unix-like systems.
#[cfg(not(target_os = "windows"))]
const TZPATHS: &[&str] = &[
    "/usr/share/zoneinfo",
    "/usr/lib/zoneinfo",
    "/usr/share/lib/zoneinfo",
    "/etc/zoneinfo",
];

/// Files to try when resolving local timezone (no name given).
#[cfg(not(target_os = "windows"))]
const TZFILES: &[&str] = &["/etc/localtime", "localtime"];

/// Get a timezone by name.
///
/// Matches python-dateutil's `dateutil.tz.gettz()` lookup order:
/// 1. `None` / `""` / `":"` → `$TZ` env → TZFILES → `TzLocal`
/// 2. Strip leading `":"` prefix
/// 3. Absolute path → `TzFile`
/// 4. IANA name (e.g. `"America/New_York"`) → search TZPATHS (with space→underscore)
/// 5. `"UTC"` / `"GMT"` → `TzUtc`
/// 6. System timezone name (e.g. `"JST"`) → `TzLocal`
/// 7. POSIX TZ string (contains digits) → `TzStr`
pub fn gettz(name: Option<&str>) -> Option<Tz> {
    let name = match name {
        None | Some("") | Some(":") => {
            // Try $TZ environment variable first
            if let Ok(tz_env) = std::env::var("TZ") {
                if !tz_env.is_empty() && tz_env != ":" {
                    return gettz(Some(&tz_env));
                }
            }
            // Try TZFILES (absolute and relative paths in TZPATHS)
            #[cfg(not(target_os = "windows"))]
            for filepath in TZFILES {
                if filepath.starts_with('/') {
                    // Absolute path — try directly
                    if let Ok(tzf) = TzFile::from_path(filepath) {
                        return Some(Tz::File(tzf));
                    }
                } else {
                    // Relative — search in TZPATHS
                    for base in TZPATHS {
                        let full = format!("{base}/{filepath}");
                        if let Ok(tzf) = TzFile::from_path(&full) {
                            return Some(Tz::File(tzf));
                        }
                    }
                }
            }
            // Fall back to TzLocal
            return Some(Tz::Local(TzLocal::new()));
        }
        Some(n) => n,
    };

    // Strip leading ":" (POSIX TZ convention)
    let name = name.strip_prefix(':').unwrap_or(name).trim();

    if name.is_empty() {
        return gettz(None);
    }

    // Absolute path
    if name.starts_with('/') {
        return TzFile::from_path(name).ok().map(Tz::File);
    }

    // Search TZPATHS for IANA timezone name
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(tz) = lookup_tzfile(name) {
            return Some(tz);
        }
    }

    // UTC / GMT
    if name.eq_ignore_ascii_case("UTC") || name.eq_ignore_ascii_case("GMT") {
        return Some(Tz::Utc(TzUtc::new()));
    }

    // Check system timezone abbreviations (e.g. "JST", "EST")
    if is_system_tzname(name) {
        return Some(Tz::Local(TzLocal::new()));
    }

    // Try as a POSIX TZ string (if it contains a digit)
    if name.chars().any(|c| c.is_ascii_digit()) {
        if let Ok(tz_str) = TzStr::parse(name, false) {
            return Some(Tz::Str(tz_str));
        }
    }

    None
}

/// Search TZPATHS for a timezone file, trying space→underscore replacement.
#[cfg(not(target_os = "windows"))]
fn lookup_tzfile(name: &str) -> Option<Tz> {
    use std::path::Path;

    for base in TZPATHS {
        let path = format!("{base}/{name}");
        if Path::new(&path).is_file() {
            if let Ok(tzf) = TzFile::from_path(&path) {
                return Some(Tz::File(tzf));
            }
        }
        // Try with spaces replaced by underscores
        if name.contains(' ') {
            let alt = format!("{base}/{}", name.replace(' ', "_"));
            if Path::new(&alt).is_file() {
                if let Ok(tzf) = TzFile::from_path(&alt) {
                    return Some(Tz::File(tzf));
                }
            }
        }
    }
    None
}

/// Check if `name` matches the system's current timezone abbreviation(s).
fn is_system_tzname(name: &str) -> bool {
    // Compare against POSIX tzname (standard and DST abbreviations).
    // This mirrors python-dateutil's `name in time.tzname` check.
    #[cfg(not(target_os = "windows"))]
    {
        extern "C" {
            fn tzset();
            static tzname: [*const std::ffi::c_char; 2];
        }

        // Safety: tzset() initializes the tzname globals. The pointers are
        // valid NUL-terminated strings on all POSIX systems after tzset().
        unsafe {
            tzset();

            for ptr in &tzname {
                if !ptr.is_null() {
                    if let Ok(s) = std::ffi::CStr::from_ptr(*ptr).to_str() {
                        if s == name {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

// ============================================================================
// Helper functions
// ============================================================================

/// Check if a wall-clock datetime exists in the given timezone.
///
/// Returns `false` for times that fall in a DST gap (spring forward).
pub fn datetime_exists(dt: NaiveDateTime, tz: &Tz) -> bool {
    let offset = match tz.utcoffset(Some(dt), false) {
        Some(o) => o,
        None => return true,
    };
    let utc = dt - offset;
    let (wall, _fold) = tz.fromutc(utc);
    wall == dt
}

/// Check if a wall-clock datetime is ambiguous in the given timezone.
///
/// Returns `true` for times that fall in a DST overlap (fall back).
pub fn datetime_ambiguous(dt: NaiveDateTime, tz: &Tz) -> bool {
    tz.is_ambiguous(dt)
}

/// Resolve an imaginary datetime (one that falls in a DST gap) by shifting
/// it forward to the correct wall time after the transition.
pub fn resolve_imaginary(dt: NaiveDateTime, tz: &Tz) -> NaiveDateTime {
    if datetime_exists(dt, tz) {
        return dt;
    }
    // Convert via UTC to get the correct wall time
    let offset = tz.utcoffset(Some(dt), false).unwrap_or(Duration::zero());
    let utc = dt - offset;
    let (wall, _fold) = tz.fromutc(utc);
    wall
}

// ============================================================================
// PyO3 bindings
// ============================================================================

#[cfg(feature = "python")]
pub mod python {
    use chrono::{Datelike, Timelike};
    use pyo3::prelude::*;
    use pyo3::types::PyDelta;

    use super::*;

    /// Internal UTC timezone helper.
    #[pyclass(name = "_TzUtc", skip_from_py_object)]
    #[derive(Debug, Clone)]
    pub struct TzUtcPy {
        _inner: TzUtc,
    }

    #[pymethods]
    impl TzUtcPy {
        #[new]
        fn new() -> Self {
            TzUtcPy {
                _inner: TzUtc::new(),
            }
        }

        fn utcoffset<'py>(&self, py: Python<'py>, _dt: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyDelta>> {
            PyDelta::new(py, 0, 0, 0, false)
        }

        fn dst<'py>(&self, py: Python<'py>, _dt: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyDelta>> {
            PyDelta::new(py, 0, 0, 0, false)
        }

        fn tzname(&self, _dt: &Bound<'_, PyAny>) -> String {
            "UTC".to_string()
        }

        fn is_ambiguous(&self, _dt: &Bound<'_, PyAny>) -> bool {
            false
        }

        fn __repr__(&self) -> String {
            "tzutc()".to_string()
        }

        fn __str__(&self) -> String {
            "tzutc()".to_string()
        }
    }

    /// Internal fixed-offset timezone helper.
    #[pyclass(name = "_TzOffset", skip_from_py_object)]
    #[derive(Debug, Clone)]
    pub struct TzOffsetPy {
        inner: TzOffset,
    }

    #[pymethods]
    impl TzOffsetPy {
        #[new]
        #[pyo3(signature = (name=None, offset=0))]
        fn new(name: Option<String>, offset: i32) -> Self {
            TzOffsetPy {
                inner: TzOffset::new(name, offset),
            }
        }

        fn utcoffset<'py>(&self, py: Python<'py>, _dt: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyDelta>> {
            let secs = self.inner.offset_seconds();
            duration_to_pydelta(py, secs)
        }

        fn dst<'py>(&self, py: Python<'py>, _dt: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyDelta>> {
            PyDelta::new(py, 0, 0, 0, false)
        }

        fn tzname(&self, _dt: &Bound<'_, PyAny>) -> Option<String> {
            self.inner.name().map(|s| s.to_string())
        }

        fn is_ambiguous(&self, _dt: &Bound<'_, PyAny>) -> bool {
            false
        }

        fn offset_seconds(&self) -> i64 {
            self.inner.offset_seconds()
        }

        fn name(&self) -> Option<String> {
            self.inner.name().map(|s| s.to_string())
        }

        fn __repr__(&self) -> String {
            format!("{}", self.inner)
        }
    }

    /// Internal TzFile helper.
    #[pyclass(name = "_TzFile", skip_from_py_object)]
    #[derive(Debug, Clone)]
    pub struct TzFilePy {
        inner: TzFile,
    }

    #[pymethods]
    impl TzFilePy {
        #[new]
        #[pyo3(signature = (path))]
        fn new(path: String) -> PyResult<Self> {
            let inner = TzFile::from_path(&path)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            Ok(TzFilePy { inner })
        }

        fn utcoffset<'py>(&self, py: Python<'py>, dt: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
            if dt.is_none() {
                return Ok(py.None());
            }
            let (naive, fold) = extract_dt_and_fold(dt)?;
            match self.inner.utcoffset(Some(naive), fold) {
                Some(d) => Ok(duration_to_pydelta(py, d.num_seconds())?.into_any().unbind()),
                None => Ok(py.None()),
            }
        }

        fn dst<'py>(&self, py: Python<'py>, dt: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
            if dt.is_none() {
                return Ok(py.None());
            }
            let (naive, fold) = extract_dt_and_fold(dt)?;
            match self.inner.dst(Some(naive), fold) {
                Some(d) => Ok(duration_to_pydelta(py, d.num_seconds())?.into_any().unbind()),
                None => Ok(py.None()),
            }
        }

        fn tzname(&self, dt: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
            if dt.is_none() {
                return Ok(None);
            }
            let (naive, fold) = extract_dt_and_fold(dt)?;
            Ok(self.inner.tzname(Some(naive), fold))
        }

        fn is_ambiguous(&self, dt: &Bound<'_, PyAny>) -> PyResult<bool> {
            let (naive, _fold) = extract_dt_and_fold(dt)?;
            Ok(self.inner.is_ambiguous(naive))
        }

        /// Convert a UTC datetime to wall time, returning (wall_naive, fold).
        fn fromutc_naive(&self, dt: &Bound<'_, PyAny>) -> PyResult<(i32, u32, u32, u32, u32, u32, u32, bool)> {
            fromutc_result_to_tuple(dt, &self.inner)
        }

        fn filename(&self) -> Option<String> {
            self.inner.filename().map(|s| s.to_string())
        }

        fn __repr__(&self) -> String {
            format!("{}", self.inner)
        }
    }

    /// Internal TzLocal helper.
    #[pyclass(name = "_TzLocal", skip_from_py_object)]
    #[derive(Debug, Clone)]
    pub struct TzLocalPy {
        inner: TzLocal,
    }

    #[pymethods]
    impl TzLocalPy {
        #[new]
        fn new() -> Self {
            TzLocalPy {
                inner: TzLocal::new(),
            }
        }

        fn utcoffset<'py>(&self, py: Python<'py>, dt: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
            if dt.is_none() {
                return Ok(py.None());
            }
            let (naive, fold) = extract_dt_and_fold(dt)?;
            match self.inner.utcoffset(Some(naive), fold) {
                Some(d) => Ok(duration_to_pydelta(py, d.num_seconds())?.into_any().unbind()),
                None => Ok(py.None()),
            }
        }

        fn dst<'py>(&self, py: Python<'py>, dt: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
            if dt.is_none() {
                return Ok(py.None());
            }
            let (naive, fold) = extract_dt_and_fold(dt)?;
            match self.inner.dst(Some(naive), fold) {
                Some(d) => Ok(duration_to_pydelta(py, d.num_seconds())?.into_any().unbind()),
                None => Ok(py.None()),
            }
        }

        fn tzname(&self, dt: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
            if dt.is_none() {
                return Ok(None);
            }
            let (naive, fold) = extract_dt_and_fold(dt)?;
            Ok(self.inner.tzname(Some(naive), fold))
        }

        fn is_ambiguous(&self, dt: &Bound<'_, PyAny>) -> PyResult<bool> {
            let (naive, _fold) = extract_dt_and_fold(dt)?;
            Ok(self.inner.is_ambiguous(naive))
        }

        fn fromutc_naive(&self, dt: &Bound<'_, PyAny>) -> PyResult<(i32, u32, u32, u32, u32, u32, u32, bool)> {
            let (naive, _fold) = extract_dt_and_fold(dt)?;
            let (wall, fold) = self.inner.fromutc(naive);
            Ok((
                wall.date().year(),
                wall.date().month(),
                wall.date().day(),
                wall.time().hour(),
                wall.time().minute(),
                wall.time().second(),
                wall.time().nanosecond() / 1000,
                fold,
            ))
        }

        fn __repr__(&self) -> String {
            "tzlocal()".to_string()
        }
    }

    /// Internal TzStr helper.
    #[pyclass(name = "_TzStr", skip_from_py_object)]
    #[derive(Debug, Clone)]
    pub struct TzStrPy {
        inner: TzStr,
    }

    #[pymethods]
    impl TzStrPy {
        #[new]
        #[pyo3(signature = (s, posix_offset=false))]
        fn new(s: String, posix_offset: bool) -> PyResult<Self> {
            let inner = TzStr::parse(&s, posix_offset)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            Ok(TzStrPy { inner })
        }

        fn utcoffset<'py>(&self, py: Python<'py>, dt: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
            if dt.is_none() {
                return Ok(py.None());
            }
            let (naive, fold) = extract_dt_and_fold(dt)?;
            match self.inner.utcoffset(Some(naive), fold) {
                Some(d) => Ok(duration_to_pydelta(py, d.num_seconds())?.into_any().unbind()),
                None => Ok(py.None()),
            }
        }

        fn dst<'py>(&self, py: Python<'py>, dt: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
            if dt.is_none() {
                return Ok(py.None());
            }
            let (naive, fold) = extract_dt_and_fold(dt)?;
            match self.inner.dst(Some(naive), fold) {
                Some(d) => Ok(duration_to_pydelta(py, d.num_seconds())?.into_any().unbind()),
                None => Ok(py.None()),
            }
        }

        fn tzname(&self, dt: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
            if dt.is_none() {
                return Ok(None);
            }
            let (naive, fold) = extract_dt_and_fold(dt)?;
            Ok(self.inner.tzname(Some(naive), fold))
        }

        fn is_ambiguous(&self, dt: &Bound<'_, PyAny>) -> PyResult<bool> {
            let (naive, _fold) = extract_dt_and_fold(dt)?;
            Ok(self.inner.is_ambiguous(naive))
        }

        fn fromutc_naive(&self, dt: &Bound<'_, PyAny>) -> PyResult<(i32, u32, u32, u32, u32, u32, u32, bool)> {
            let (naive, _fold) = extract_dt_and_fold(dt)?;
            let (wall, fold) = self.inner.fromutc(naive);
            Ok((
                wall.date().year(),
                wall.date().month(),
                wall.date().day(),
                wall.time().hour(),
                wall.time().minute(),
                wall.time().second(),
                wall.time().nanosecond() / 1000,
                fold,
            ))
        }

        fn source(&self) -> String {
            self.inner.source().to_string()
        }

        fn __repr__(&self) -> String {
            format!("{}", self.inner)
        }
    }

    /// Internal TzRange helper.
    #[pyclass(name = "_TzRange", skip_from_py_object)]
    #[derive(Debug, Clone)]
    pub struct TzRangePy {
        inner: TzRange,
    }

    #[pymethods]
    impl TzRangePy {
        #[new]
        #[pyo3(signature = (std_abbr, std_offset=None, dst_abbr=None, dst_offset=None, start=None, end=None))]
        fn new(
            std_abbr: String,
            std_offset: Option<i64>,
            dst_abbr: Option<String>,
            dst_offset: Option<i64>,
            start: Option<(u32, u32, u32, i32)>,
            end: Option<(u32, u32, u32, i32)>,
        ) -> Self {
            let std_off = std_offset.map(Duration::seconds);
            let dst_off = dst_offset.map(Duration::seconds);
            let start_rule = start.map(|(m, w, d, t)| {
                super::range::TransitionRule::new(
                    super::range::DateRule::MonthWeekDay {
                        month: m,
                        week: w,
                        weekday: d,
                    },
                    t,
                )
            });
            let end_rule = end.map(|(m, w, d, t)| {
                super::range::TransitionRule::new(
                    super::range::DateRule::MonthWeekDay {
                        month: m,
                        week: w,
                        weekday: d,
                    },
                    t,
                )
            });
            TzRangePy {
                inner: TzRange::new(std_abbr, std_off, dst_abbr, dst_off, start_rule, end_rule),
            }
        }

        fn utcoffset<'py>(&self, py: Python<'py>, dt: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
            if dt.is_none() {
                return Ok(py.None());
            }
            let (naive, fold) = extract_dt_and_fold(dt)?;
            match self.inner.utcoffset(Some(naive), fold) {
                Some(d) => Ok(duration_to_pydelta(py, d.num_seconds())?.into_any().unbind()),
                None => Ok(py.None()),
            }
        }

        fn dst<'py>(&self, py: Python<'py>, dt: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
            if dt.is_none() {
                return Ok(py.None());
            }
            let (naive, fold) = extract_dt_and_fold(dt)?;
            match self.inner.dst(Some(naive), fold) {
                Some(d) => Ok(duration_to_pydelta(py, d.num_seconds())?.into_any().unbind()),
                None => Ok(py.None()),
            }
        }

        fn tzname(&self, dt: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
            if dt.is_none() {
                return Ok(None);
            }
            let (naive, fold) = extract_dt_and_fold(dt)?;
            Ok(self.inner.tzname(Some(naive), fold))
        }

        fn is_ambiguous(&self, dt: &Bound<'_, PyAny>) -> PyResult<bool> {
            let (naive, _fold) = extract_dt_and_fold(dt)?;
            Ok(self.inner.is_ambiguous(naive))
        }

        fn fromutc_naive(&self, dt: &Bound<'_, PyAny>) -> PyResult<(i32, u32, u32, u32, u32, u32, u32, bool)> {
            let (naive, _fold) = extract_dt_and_fold(dt)?;
            let (wall, fold) = self.inner.fromutc(naive);
            Ok((
                wall.date().year(),
                wall.date().month(),
                wall.date().day(),
                wall.time().hour(),
                wall.time().minute(),
                wall.time().second(),
                wall.time().nanosecond() / 1000,
                fold,
            ))
        }

        fn std_abbr(&self) -> String {
            self.inner.std_abbr.clone()
        }

        fn dst_abbr(&self) -> Option<String> {
            self.inner.dst_abbr.clone()
        }

        fn __repr__(&self) -> String {
            format!("{}", self.inner)
        }
    }

    /// Python-accessible gettz function.
    #[pyfunction]
    #[pyo3(name = "gettz", signature = (name=None))]
    pub fn gettz_py(py: Python<'_>, name: Option<&str>) -> PyResult<Py<PyAny>> {
        match gettz(name) {
            Some(tz) => match tz {
                super::Tz::Utc(_) => {
                    let obj = Bound::new(py, TzUtcPy::new())?;
                    Ok(obj.into_any().unbind())
                }
                super::Tz::Offset(o) => {
                    let obj = Bound::new(py, TzOffsetPy { inner: o })?;
                    Ok(obj.into_any().unbind())
                }
                super::Tz::File(f) => {
                    let obj = Bound::new(py, TzFilePy { inner: f })?;
                    Ok(obj.into_any().unbind())
                }
                super::Tz::Str(s) => {
                    let obj = Bound::new(py, TzStrPy { inner: s })?;
                    Ok(obj.into_any().unbind())
                }
                super::Tz::Local(_) => {
                    let obj = Bound::new(py, TzLocalPy::new())?;
                    Ok(obj.into_any().unbind())
                }
                super::Tz::Range(r) => {
                    let obj = Bound::new(py, TzRangePy { inner: r })?;
                    Ok(obj.into_any().unbind())
                }
            },
            None => Ok(py.None()),
        }
    }

    /// Python-accessible datetime_exists function.
    #[pyfunction]
    #[pyo3(name = "datetime_exists")]
    pub fn datetime_exists_py(dt: &Bound<'_, PyAny>, tz: &Bound<'_, PyAny>) -> PyResult<bool> {
        // Call utcoffset on the tz
        let offset_obj = tz.call_method1("utcoffset", (dt,))?;
        if offset_obj.is_none() {
            return Ok(true);
        }
        let offset_secs: f64 = offset_obj.call_method0("total_seconds")?.extract()?;

        let (naive, _fold) = extract_dt_and_fold(dt)?;
        let offset = Duration::seconds(offset_secs as i64);
        let utc = naive - offset;

        // Build a UTC datetime with the tz attached, then call fromutc
        let py = dt.py();
        let datetime_mod = py.import("datetime")?;
        let datetime_cls = datetime_mod.getattr("datetime")?;

        let utc_dt = datetime_cls.call1((
            utc.date().year(),
            utc.date().month() as i32,
            utc.date().day() as i32,
            utc.time().hour() as i32,
            utc.time().minute() as i32,
            utc.time().second() as i32,
            (utc.time().nanosecond() / 1000) as i32,
            tz,
        ))?;
        let wall = tz.call_method1("fromutc", (&utc_dt,))?;
        let wall_replace_args = pyo3::types::PyDict::new(py);
        wall_replace_args.set_item("tzinfo", py.None())?;
        let wall_naive = wall.call_method("replace", (), Some(&wall_replace_args))?;

        let dt_replace_args = pyo3::types::PyDict::new(py);
        dt_replace_args.set_item("tzinfo", py.None())?;
        let dt_naive = dt.call_method("replace", (), Some(&dt_replace_args))?;

        wall_naive.eq(&dt_naive)
    }

    /// Python-accessible datetime_ambiguous function.
    #[pyfunction]
    #[pyo3(name = "datetime_ambiguous")]
    pub fn datetime_ambiguous_py(dt: &Bound<'_, PyAny>, tz: &Bound<'_, PyAny>) -> PyResult<bool> {
        // Try is_ambiguous method first
        if let Ok(result) = tz.call_method1("is_ambiguous", (dt,)) {
            return result.extract();
        }

        // Fallback: compare fold=0 and fold=1
        let py = dt.py();
        let kwargs0 = pyo3::types::PyDict::new(py);
        kwargs0.set_item("fold", 0)?;
        let dt0 = dt.call_method("replace", (), Some(&kwargs0))?;

        let kwargs1 = pyo3::types::PyDict::new(py);
        kwargs1.set_item("fold", 1)?;
        let dt1 = dt.call_method("replace", (), Some(&kwargs1))?;

        let off0 = tz.call_method1("utcoffset", (&dt0,))?;
        let off1 = tz.call_method1("utcoffset", (&dt1,))?;

        Ok(!off0.eq(&off1)?)
    }

    // Helpers

    fn extract_dt_and_fold(dt: &Bound<'_, PyAny>) -> PyResult<(NaiveDateTime, bool)> {
        use chrono::{NaiveDate, NaiveTime};

        let year: i32 = dt.getattr("year")?.extract()?;
        let month: u32 = dt.getattr("month")?.extract()?;
        let day: u32 = dt.getattr("day")?.extract()?;
        let hour: u32 = dt.getattr("hour")?.extract()?;
        let minute: u32 = dt.getattr("minute")?.extract()?;
        let second: u32 = dt.getattr("second")?.extract()?;
        let microsecond: u32 = dt.getattr("microsecond")?.extract()?;

        let date = NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("invalid date"))?;
        let time = NaiveTime::from_hms_micro_opt(hour, minute, second, microsecond)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("invalid time"))?;

        let fold: bool = dt
            .getattr("fold")
            .and_then(|f| f.extract::<u8>())
            .map(|f| f != 0)
            .unwrap_or(false);

        Ok((NaiveDateTime::new(date, time), fold))
    }

    /// Convert a Rust fromutc result to a Python-friendly tuple.
    fn fromutc_result_to_tuple(
        dt: &Bound<'_, PyAny>,
        tz: &impl FromUtcRust,
    ) -> PyResult<(i32, u32, u32, u32, u32, u32, u32, bool)> {
        let (naive, _fold) = extract_dt_and_fold(dt)?;
        let (wall, fold) = tz.fromutc_rust(naive);
        Ok((
            wall.date().year(),
            wall.date().month(),
            wall.date().day(),
            wall.time().hour(),
            wall.time().minute(),
            wall.time().second(),
            wall.time().nanosecond() / 1000,
            fold,
        ))
    }

    /// Trait to allow generic fromutc dispatch across tz types.
    trait FromUtcRust {
        fn fromutc_rust(&self, dt: NaiveDateTime) -> (NaiveDateTime, bool);
    }

    impl FromUtcRust for TzFile {
        fn fromutc_rust(&self, dt: NaiveDateTime) -> (NaiveDateTime, bool) {
            self.fromutc(dt)
        }
    }

    impl FromUtcRust for TzLocal {
        fn fromutc_rust(&self, dt: NaiveDateTime) -> (NaiveDateTime, bool) {
            self.fromutc(dt)
        }
    }

    impl FromUtcRust for TzStr {
        fn fromutc_rust(&self, dt: NaiveDateTime) -> (NaiveDateTime, bool) {
            self.fromutc(dt)
        }
    }

    impl FromUtcRust for TzRange {
        fn fromutc_rust(&self, dt: NaiveDateTime) -> (NaiveDateTime, bool) {
            self.fromutc(dt)
        }
    }

    fn duration_to_pydelta<'py>(py: Python<'py>, total_seconds: i64) -> PyResult<Bound<'py, PyDelta>> {
        let days = total_seconds.div_euclid(86400) as i32;
        let remaining = total_seconds.rem_euclid(86400) as i32;
        PyDelta::new(py, days, remaining, 0, false)
    }

    /// Register tz module classes and functions with the parent module.
    pub fn register(m: &Bound<'_, pyo3::types::PyModule>) -> PyResult<()> {
        m.add_class::<TzUtcPy>()?;
        m.add_class::<TzOffsetPy>()?;
        m.add_class::<TzFilePy>()?;
        m.add_class::<TzLocalPy>()?;
        m.add_class::<TzStrPy>()?;
        m.add_class::<TzRangePy>()?;
        m.add_function(pyo3::wrap_pyfunction!(gettz_py, m)?)?;
        m.add_function(pyo3::wrap_pyfunction!(datetime_exists_py, m)?)?;
        m.add_function(pyo3::wrap_pyfunction!(datetime_ambiguous_py, m)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_gettz_utc() {
        let tz = gettz(Some("UTC"));
        assert!(tz.is_some());
        let dt = NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tz.unwrap().utcoffset(Some(dt), false), Some(Duration::zero()));
    }

    #[test]
    fn test_gettz_gmt() {
        let tz = gettz(Some("GMT"));
        assert!(tz.is_some());
    }

    #[test]
    fn test_gettz_iana() {
        // Skip if timezone files not available
        if let Some(tz) = gettz(Some("America/New_York")) {
            let winter = NaiveDate::from_ymd_opt(2020, 1, 15)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap();
            assert_eq!(
                tz.utcoffset(Some(winter), false),
                Some(Duration::seconds(-18000))
            );
        }
    }

    #[test]
    fn test_gettz_posix_string() {
        let tz = gettz(Some("EST5EDT,M3.2.0/2,M11.1.0/2"));
        assert!(tz.is_some());
    }

    #[test]
    fn test_gettz_none() {
        // Should return local timezone
        let tz = gettz(None);
        assert!(tz.is_some());
    }

    #[test]
    fn test_datetime_exists() {
        if let Some(tz) = gettz(Some("America/New_York")) {
            // Normal time — should exist
            let normal = NaiveDate::from_ymd_opt(2020, 1, 15)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap();
            assert!(datetime_exists(normal, &tz));
        }
    }

    #[test]
    fn test_datetime_ambiguous() {
        if let Some(tz) = gettz(Some("America/New_York")) {
            // Normal time — not ambiguous
            let normal = NaiveDate::from_ymd_opt(2020, 1, 15)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap();
            assert!(!datetime_ambiguous(normal, &tz));
        }
    }

    // --- Tz enum dispatch tests ---

    #[test]
    fn test_tz_utc_variant() {
        let tz = Tz::Utc(TzUtc::new());
        let dt = NaiveDate::from_ymd_opt(2020, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tz.utcoffset(Some(dt), false), Some(Duration::zero()));
        assert_eq!(tz.dst(Some(dt), false), Some(Duration::zero()));
        assert_eq!(tz.tzname(Some(dt), false), Some("UTC".into()));
        assert!(!tz.is_ambiguous(dt));
        assert_eq!(tz.fromutc(dt), (dt, false));
    }

    #[test]
    fn test_tz_offset_variant() {
        let tz = Tz::Offset(TzOffset::new(Some("EST".into()), -18000));
        let dt = NaiveDate::from_ymd_opt(2020, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tz.utcoffset(Some(dt), false), Some(Duration::seconds(-18000)));
        assert_eq!(tz.dst(Some(dt), false), Some(Duration::zero()));
        assert_eq!(tz.tzname(Some(dt), false), Some("EST".into()));
        assert!(!tz.is_ambiguous(dt));
    }

    #[test]
    fn test_tz_str_variant() {
        let tzstr = TzStr::parse("EST5EDT,M3.2.0/2,M11.1.0/2", false).unwrap();
        let tz = Tz::Str(tzstr);
        let winter = NaiveDate::from_ymd_opt(2020, 1, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tz.utcoffset(Some(winter), false), Some(Duration::seconds(-18000)));
        assert_eq!(tz.dst(Some(winter), false), Some(Duration::zero()));
        assert_eq!(tz.tzname(Some(winter), false), Some("EST".into()));
        assert!(!tz.is_ambiguous(winter));
    }

    #[test]
    fn test_tz_range_variant() {
        let tzrange = TzRange::new(
            "EST".into(),
            Some(Duration::seconds(-18000)),
            Some("EDT".into()),
            Some(Duration::seconds(-14400)),
            None,
            None,
        );
        let tz = Tz::Range(tzrange);
        let summer = NaiveDate::from_ymd_opt(2020, 7, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tz.utcoffset(Some(summer), false), Some(Duration::seconds(-14400)));
        assert_eq!(tz.dst(Some(summer), false), Some(Duration::seconds(3600)));
        assert_eq!(tz.tzname(Some(summer), false), Some("EDT".into()));
    }

    #[test]
    fn test_tz_file_variant() {
        if let Some(tz) = gettz(Some("America/New_York")) {
            let summer = NaiveDate::from_ymd_opt(2020, 7, 15)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap();
            assert!(tz.utcoffset(Some(summer), false).is_some());
            assert!(tz.dst(Some(summer), false).is_some());
            assert!(tz.tzname(Some(summer), false).is_some());
        }
    }

    #[test]
    fn test_tz_local_variant() {
        let tz = Tz::Local(TzLocal::new());
        let dt = NaiveDate::from_ymd_opt(2020, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert!(tz.utcoffset(Some(dt), false).is_some());
        assert!(tz.dst(Some(dt), false).is_some());
        assert!(tz.tzname(Some(dt), false).is_some());
        let _ = tz.fromutc(dt);
    }

    // --- resolve_imaginary ---

    #[test]
    fn test_resolve_imaginary_normal() {
        if let Some(tz) = gettz(Some("America/New_York")) {
            let normal = NaiveDate::from_ymd_opt(2020, 1, 15)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap();
            assert_eq!(resolve_imaginary(normal, &tz), normal);
        }
    }

    #[test]
    fn test_resolve_imaginary_utc() {
        let tz = Tz::Utc(TzUtc::new());
        let dt = NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(resolve_imaginary(dt, &tz), dt);
    }

    // --- gettz edge cases ---

    #[test]
    fn test_gettz_empty_string() {
        let tz = gettz(Some(""));
        assert!(tz.is_some());
    }

    #[test]
    fn test_gettz_utc_lowercase() {
        let tz = gettz(Some("utc"));
        assert!(tz.is_some());
    }

    #[test]
    fn test_gettz_absolute_path() {
        if let Some(tz) = gettz(Some("/usr/share/zoneinfo/UTC")) {
            let dt = NaiveDate::from_ymd_opt(2020, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
            assert_eq!(tz.utcoffset(Some(dt), false), Some(Duration::zero()));
        }
    }

    #[test]
    fn test_gettz_invalid_name() {
        let tz = gettz(Some("Not/A/Real/Timezone"));
        assert!(tz.is_none());
    }

    #[test]
    fn test_gettz_colon_prefix() {
        // ":America/New_York" should strip the colon and resolve
        if let Some(tz) = gettz(Some(":America/New_York")) {
            let winter = NaiveDate::from_ymd_opt(2020, 1, 15)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap();
            assert_eq!(
                tz.utcoffset(Some(winter), false),
                Some(Duration::seconds(-18000))
            );
        }
    }

    #[test]
    fn test_gettz_colon_only() {
        // ":" alone should resolve to local timezone
        let tz = gettz(Some(":"));
        assert!(tz.is_some());
    }

    #[test]
    fn test_gettz_system_tzname() {
        // The system should know its own timezone abbreviations
        // This test just verifies is_system_tzname doesn't panic
        // and that unknown names don't match
        assert!(!is_system_tzname("FAKE_TZ_THAT_DOES_NOT_EXIST"));
    }

    #[test]
    fn test_datetime_exists_utc() {
        let tz = Tz::Utc(TzUtc::new());
        let dt = NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert!(datetime_exists(dt, &tz));
    }

    #[test]
    fn test_datetime_ambiguous_utc() {
        let tz = Tz::Utc(TzUtc::new());
        let dt = NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert!(!datetime_ambiguous(dt, &tz));
    }
}
