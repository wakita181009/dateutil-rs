//! Timezone types and utilities.
//!
//! Provides timezone types compatible with python-dateutil's tz module,
//! optimized for Rust performance.

mod utc;
mod offset;
mod file;
mod local;

pub use utc::TzUtc;
pub use offset::TzOffset;
pub use file::{TzFile, TzFileData};
pub use local::TzLocal;

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use chrono::{NaiveDateTime, TimeDelta};

use crate::error::TzError;

// ---------------------------------------------------------------------------
// TimeZone — enum dispatch for all timezone types
// ---------------------------------------------------------------------------

/// Unified timezone type. Returned by `gettz()`.
#[derive(Debug, Clone)]
pub enum TimeZone {
    /// UTC (zero offset, no DST).
    Utc(TzUtc),
    /// Fixed UTC offset (no DST).
    Offset(TzOffset),
    /// TZif file-based timezone (DST-aware).
    File(TzFile),
    /// System local timezone.
    Local(TzLocal),
}

impl TimeZone {
    /// UTC offset in seconds for a wall-clock datetime.
    /// `fold` disambiguates repeated wall times (PEP 495).
    #[inline]
    pub fn utcoffset(&self, dt: NaiveDateTime, fold: bool) -> i32 {
        match self {
            TimeZone::Utc(tz) => tz.utcoffset(dt, fold),
            TimeZone::Offset(tz) => tz.utcoffset(dt, fold),
            TimeZone::File(tz) => tz.utcoffset(dt, fold),
            TimeZone::Local(tz) => tz.utcoffset(dt, fold),
        }
    }

    /// DST offset component in seconds.
    #[inline]
    pub fn dst(&self, dt: NaiveDateTime, fold: bool) -> i32 {
        match self {
            TimeZone::Utc(tz) => tz.dst(dt, fold),
            TimeZone::Offset(tz) => tz.dst(dt, fold),
            TimeZone::File(tz) => tz.dst(dt, fold),
            TimeZone::Local(tz) => tz.dst(dt, fold),
        }
    }

    /// Timezone abbreviation string.
    #[inline]
    pub fn tzname(&self, dt: NaiveDateTime, fold: bool) -> &str {
        match self {
            TimeZone::Utc(tz) => tz.tzname(dt, fold),
            TimeZone::Offset(tz) => tz.tzname(dt, fold),
            TimeZone::File(tz) => tz.tzname(dt, fold),
            TimeZone::Local(tz) => tz.tzname(dt, fold),
        }
    }

    /// Whether the given wall time is ambiguous (falls in a DST overlap).
    #[inline]
    pub fn is_ambiguous(&self, dt: NaiveDateTime) -> bool {
        match self {
            TimeZone::Utc(tz) => tz.is_ambiguous(dt),
            TimeZone::Offset(tz) => tz.is_ambiguous(dt),
            TimeZone::File(tz) => tz.is_ambiguous(dt),
            TimeZone::Local(tz) => tz.is_ambiguous(dt),
        }
    }

    /// Convert a UTC datetime to wall time.
    #[inline]
    pub fn fromutc(&self, dt: NaiveDateTime) -> NaiveDateTime {
        match self {
            TimeZone::Utc(tz) => tz.fromutc(dt),
            TimeZone::Offset(tz) => tz.fromutc(dt),
            TimeZone::File(tz) => tz.fromutc(dt),
            TimeZone::Local(tz) => tz.fromutc(dt),
        }
    }

    /// UTC offset as a `chrono::TimeDelta`.
    #[inline]
    pub fn utcoffset_delta(&self, dt: NaiveDateTime, fold: bool) -> TimeDelta {
        TimeDelta::seconds(self.utcoffset(dt, fold) as i64)
    }

    /// DST offset as a `chrono::TimeDelta`.
    #[inline]
    pub fn dst_delta(&self, dt: NaiveDateTime, fold: bool) -> TimeDelta {
        TimeDelta::seconds(self.dst(dt, fold) as i64)
    }
}

// ---------------------------------------------------------------------------
// Convenience constructors
// ---------------------------------------------------------------------------

impl TimeZone {
    /// Create a UTC timezone.
    #[inline]
    pub fn utc() -> Self {
        TimeZone::Utc(TzUtc)
    }

    /// Create a fixed-offset timezone.
    #[inline]
    pub fn offset(name: Option<&str>, offset_secs: i32) -> Self {
        TimeZone::Offset(TzOffset::new(name, offset_secs))
    }

    /// Create a system local timezone.
    #[inline]
    pub fn local() -> Self {
        TimeZone::Local(TzLocal::new())
    }
}

// ---------------------------------------------------------------------------
// gettz() — timezone lookup with caching
// ---------------------------------------------------------------------------

/// Search paths for TZif files.
pub(super) const TZPATHS: &[&str] = &[
    "/usr/share/zoneinfo",
    "/usr/lib/zoneinfo",
    "/usr/share/lib/zoneinfo",
    "/etc/zoneinfo",
];

/// Well-known UTC aliases.
const UTC_NAMES: &[&str] = &["UTC", "utc", "GMT", "gmt", "Z", "z"];

/// Thread-safe timezone cache.
static TZ_CACHE: LazyLock<RwLock<HashMap<Box<str>, TimeZone>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Look up a timezone by name.
///
/// Supports:
/// - IANA timezone names (e.g. "America/New_York", "Asia/Tokyo")
/// - UTC aliases ("UTC", "GMT", "Z")
/// - Absolute file paths (e.g. "/usr/share/zoneinfo/US/Eastern")
/// - `None` or empty string → system local timezone
///
/// Results are cached for the lifetime of the process.
pub fn gettz(name: Option<&str>) -> Result<TimeZone, TzError> {
    let key = name.unwrap_or("");

    // Fast path: check cache under read lock.
    if let Ok(cache) = TZ_CACHE.read() {
        if let Some(tz) = cache.get(key) {
            return Ok(tz.clone());
        }
    }

    // Slow path: resolve, then cache.
    let tz = resolve_tz(key)?;

    // Don't cache local timezone (it can change with TZ env var).
    if !key.is_empty() {
        if let Ok(mut cache) = TZ_CACHE.write() {
            cache.entry(key.into()).or_insert_with(|| tz.clone());
        }
    }

    Ok(tz)
}

/// Clear the timezone cache.
pub fn cache_clear() {
    if let Ok(mut cache) = TZ_CACHE.write() {
        cache.clear();
    }
}

/// Resolve a timezone name to a TimeZone instance.
fn resolve_tz(name: &str) -> Result<TimeZone, TzError> {
    // Empty name or None → local timezone
    if name.is_empty() {
        return Ok(TimeZone::local());
    }

    // Strip leading colon (POSIX convention)
    let name = name.strip_prefix(':').unwrap_or(name);

    // UTC aliases
    if UTC_NAMES.contains(&name) {
        return Ok(TimeZone::utc());
    }

    // Absolute path → TzFile
    if name.starts_with('/') {
        let tz = TzFile::from_path(name)?;
        return Ok(TimeZone::File(tz));
    }

    // Search TZPATHS for IANA name
    let normalized = name.replace(' ', "_");
    for base in TZPATHS {
        let path = format!("{}/{}", base, normalized);
        if let Ok(tz) = TzFile::from_path(&path) {
            return Ok(TimeZone::File(tz));
        }
    }

    Err(TzError::NotFound(name.into()))
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Check if a wall-clock datetime exists in the given timezone.
/// Returns `false` for times in DST gaps (spring forward).
pub fn datetime_exists(dt: NaiveDateTime, tz: &TimeZone) -> bool {
    let offset_secs = tz.utcoffset(dt, false) as i64;
    let utc = dt - TimeDelta::seconds(offset_secs);
    let wall = tz.fromutc(utc);
    wall == dt
}

/// Check if a wall-clock datetime is ambiguous in the given timezone.
/// Returns `true` for times in DST overlaps (fall back).
pub fn datetime_ambiguous(dt: NaiveDateTime, tz: &TimeZone) -> bool {
    tz.is_ambiguous(dt)
}

/// Resolve an imaginary datetime (in a DST gap) by shifting forward.
/// If the datetime already exists, returns it unchanged.
pub fn resolve_imaginary(dt: NaiveDateTime, tz: &TimeZone) -> NaiveDateTime {
    if datetime_exists(dt, tz) {
        return dt;
    }
    // python-dateutil approach: check offsets 24h before and after,
    // then shift by the difference.
    let day = TimeDelta::hours(24);
    let off_before = tz.utcoffset(dt - day, false) as i64;
    let off_after = tz.utcoffset(dt + day, false) as i64;
    // Gap size = off_after - off_before (spring forward: offset increases)
    dt + TimeDelta::seconds(off_after - off_before)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use super::*;

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, mi, s).unwrap()
    }

    // -----------------------------------------------------------------------
    // TimeZone enum dispatch
    // -----------------------------------------------------------------------

    #[test]
    fn test_timezone_utc_dispatch() {
        let tz = TimeZone::utc();
        let d = dt(2024, 6, 15, 12, 0, 0);
        assert_eq!(tz.utcoffset(d, false), 0);
        assert_eq!(tz.dst(d, false), 0);
        assert_eq!(tz.tzname(d, false), "UTC");
        assert!(!tz.is_ambiguous(d));
        assert_eq!(tz.fromutc(d), d);
    }

    #[test]
    fn test_timezone_offset_dispatch() {
        let tz = TimeZone::offset(Some("JST"), 9 * 3600);
        let d = dt(2024, 6, 15, 0, 0, 0);
        assert_eq!(tz.utcoffset(d, false), 32400);
        assert_eq!(tz.dst(d, false), 0);
        assert_eq!(tz.tzname(d, false), "JST");
        assert!(!tz.is_ambiguous(d));
        assert_eq!(tz.fromutc(d), dt(2024, 6, 15, 9, 0, 0));
    }

    #[test]
    fn test_timezone_local_dispatch() {
        let tz = TimeZone::local();
        let d = dt(2024, 6, 15, 12, 0, 0);
        // Just verify it doesn't panic and returns a valid offset
        let off = tz.utcoffset(d, false);
        assert!(off.abs() <= 14 * 3600);
    }

    #[test]
    fn test_utcoffset_delta() {
        let tz = TimeZone::offset(Some("EST"), -5 * 3600);
        let d = dt(2024, 1, 1, 0, 0, 0);
        assert_eq!(tz.utcoffset_delta(d, false), TimeDelta::hours(-5));
    }

    // -----------------------------------------------------------------------
    // gettz()
    // -----------------------------------------------------------------------

    #[test]
    fn test_gettz_iana_name() {
        let tz = gettz(Some("America/New_York")).unwrap();
        // Winter: EST
        assert_eq!(tz.utcoffset(dt(2024, 1, 15, 12, 0, 0), false), -5 * 3600);
        // Summer: EDT
        assert_eq!(tz.utcoffset(dt(2024, 6, 15, 12, 0, 0), false), -4 * 3600);
    }

    #[test]
    fn test_gettz_tokyo() {
        let tz = gettz(Some("Asia/Tokyo")).unwrap();
        assert_eq!(tz.utcoffset(dt(2024, 6, 15, 12, 0, 0), false), 9 * 3600);
        assert_eq!(tz.tzname(dt(2024, 6, 15, 12, 0, 0), false), "JST");
    }

    #[test]
    fn test_gettz_empty_is_local() {
        let tz = gettz(None).unwrap();
        let d = dt(2024, 6, 15, 12, 0, 0);
        let off = tz.utcoffset(d, false);
        assert!(off.abs() <= 14 * 3600);
    }

    #[test]
    fn test_gettz_not_found() {
        let err = gettz(Some("NonExistent/Timezone")).unwrap_err();
        assert!(matches!(err, TzError::NotFound(_)));
    }

    #[test]
    fn test_gettz_caching() {
        // First call resolves from disk
        let tz1 = gettz(Some("Asia/Tokyo")).unwrap();
        // Second call should return from cache
        let tz2 = gettz(Some("Asia/Tokyo")).unwrap();
        let d = dt(2024, 1, 1, 0, 0, 0);
        assert_eq!(tz1.utcoffset(d, false), tz2.utcoffset(d, false));
    }

    #[test]
    fn test_gettz_absolute_path() {
        let tz = gettz(Some("/usr/share/zoneinfo/UTC")).unwrap();
        assert_eq!(tz.utcoffset(dt(2024, 1, 1, 0, 0, 0), false), 0);
    }

    // -----------------------------------------------------------------------
    // Helper functions with TzFile
    // -----------------------------------------------------------------------

    #[test]
    fn test_datetime_exists_gap() {
        let tz = gettz(Some("America/New_York")).unwrap();
        // 2024-03-10 02:30 is in the spring-forward gap
        assert!(!datetime_exists(dt(2024, 3, 10, 2, 30, 0), &tz));
        // 2024-03-10 03:30 exists (EDT)
        assert!(datetime_exists(dt(2024, 3, 10, 3, 30, 0), &tz));
        // 2024-03-10 01:30 exists (EST)
        assert!(datetime_exists(dt(2024, 3, 10, 1, 30, 0), &tz));
    }

    #[test]
    fn test_datetime_ambiguous_overlap() {
        let tz = gettz(Some("America/New_York")).unwrap();
        // 2024-11-03 01:30 is in the fall-back overlap
        assert!(datetime_ambiguous(dt(2024, 11, 3, 1, 30, 0), &tz));
        // 2024-11-03 00:30 is not ambiguous (only EDT)
        assert!(!datetime_ambiguous(dt(2024, 11, 3, 0, 30, 0), &tz));
        // 2024-11-03 02:30 is not ambiguous (only EST)
        assert!(!datetime_ambiguous(dt(2024, 11, 3, 2, 30, 0), &tz));
    }

    #[test]
    fn test_resolve_imaginary() {
        let tz = gettz(Some("America/New_York")).unwrap();
        // 2024-03-10 02:30 EST → spring forward → should become 03:30 EDT
        let resolved = resolve_imaginary(dt(2024, 3, 10, 2, 30, 0), &tz);
        assert_eq!(resolved, dt(2024, 3, 10, 3, 30, 0));
    }

    #[test]
    fn test_resolve_imaginary_existing() {
        let tz = gettz(Some("America/New_York")).unwrap();
        let d = dt(2024, 6, 15, 12, 0, 0);
        assert_eq!(resolve_imaginary(d, &tz), d); // exists, no change
    }

    #[test]
    fn test_datetime_exists_utc() {
        let tz = TimeZone::utc();
        assert!(datetime_exists(dt(2024, 3, 10, 2, 30, 0), &tz));
    }

    #[test]
    fn test_datetime_ambiguous_utc() {
        let tz = TimeZone::utc();
        assert!(!datetime_ambiguous(dt(2024, 11, 3, 1, 30, 0), &tz));
    }

    #[test]
    fn test_resolve_imaginary_utc() {
        let tz = TimeZone::utc();
        let d = dt(2024, 3, 10, 2, 30, 0);
        assert_eq!(resolve_imaginary(d, &tz), d);
    }

    #[test]
    fn test_datetime_exists_fixed_offset() {
        let tz = TimeZone::offset(Some("EST"), -5 * 3600);
        assert!(datetime_exists(dt(2024, 3, 10, 2, 30, 0), &tz));
    }

    // -----------------------------------------------------------------------
    // dst_delta
    // -----------------------------------------------------------------------

    #[test]
    fn test_dst_delta() {
        let tz = TimeZone::offset(Some("EST"), -5 * 3600);
        let d = dt(2024, 1, 1, 0, 0, 0);
        assert_eq!(tz.dst_delta(d, false), TimeDelta::zero());
    }

    #[test]
    fn test_dst_delta_with_dst_timezone() {
        let tz = gettz(Some("America/New_York")).unwrap();
        // Summer: EDT has DST offset of 3600
        let d_summer = dt(2024, 7, 15, 12, 0, 0);
        assert_eq!(tz.dst_delta(d_summer, false), TimeDelta::hours(1));
        // Winter: EST has no DST
        let d_winter = dt(2024, 1, 15, 12, 0, 0);
        assert_eq!(tz.dst_delta(d_winter, false), TimeDelta::zero());
    }

    // -----------------------------------------------------------------------
    // cache_clear
    // -----------------------------------------------------------------------

    #[test]
    fn test_cache_clear() {
        // Populate cache
        let _ = gettz(Some("UTC")).unwrap();
        // Clear should not panic
        cache_clear();
        // Should still work after clearing
        let tz = gettz(Some("UTC")).unwrap();
        assert_eq!(tz.utcoffset(dt(2024, 1, 1, 0, 0, 0), false), 0);
    }

    // -----------------------------------------------------------------------
    // gettz — all UTC aliases
    // -----------------------------------------------------------------------

    #[test]
    fn test_gettz_all_utc_aliases() {
        for name in &["UTC", "utc", "GMT", "gmt", "Z", "z"] {
            let tz = gettz(Some(name)).unwrap();
            assert_eq!(tz.utcoffset(dt(2024, 1, 1, 0, 0, 0), false), 0,
                "failed for alias: {name}");
        }
    }

    #[test]
    fn test_gettz_colon_prefix() {
        // POSIX convention: leading colon
        let tz = gettz(Some(":America/New_York")).unwrap();
        assert_eq!(tz.utcoffset(dt(2024, 1, 15, 12, 0, 0), false), -5 * 3600);
    }

    #[test]
    fn test_gettz_empty_string_is_local() {
        let tz = gettz(Some("")).unwrap();
        let d = dt(2024, 6, 15, 12, 0, 0);
        let off = tz.utcoffset(d, false);
        assert!(off.abs() <= 14 * 3600);
    }

    // -----------------------------------------------------------------------
    // TimeZone::File dispatch
    // -----------------------------------------------------------------------

    #[test]
    fn test_timezone_file_dispatch() {
        let tz = gettz(Some("Asia/Tokyo")).unwrap();
        let d = dt(2024, 6, 15, 12, 0, 0);
        assert_eq!(tz.utcoffset(d, false), 9 * 3600);
        assert_eq!(tz.dst(d, false), 0);
        assert_eq!(tz.tzname(d, false), "JST");
        assert!(!tz.is_ambiguous(d));
        let wall = tz.fromutc(dt(2024, 6, 15, 0, 0, 0));
        assert_eq!(wall, dt(2024, 6, 15, 9, 0, 0));
    }

    // -----------------------------------------------------------------------
    // Overlap/gap boundary precision with helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_datetime_exists_gap_boundaries() {
        let tz = gettz(Some("America/New_York")).unwrap();
        // Just before gap: 01:59:59 exists
        assert!(datetime_exists(dt(2024, 3, 10, 1, 59, 59), &tz));
        // Just after gap: 03:00:00 exists
        assert!(datetime_exists(dt(2024, 3, 10, 3, 0, 0), &tz));
        // In the gap: 02:00:00 does not exist
        assert!(!datetime_exists(dt(2024, 3, 10, 2, 0, 0), &tz));
    }

    #[test]
    fn test_resolve_imaginary_gap_start() {
        let tz = gettz(Some("America/New_York")).unwrap();
        // 02:00 is the start of the gap → resolves to 03:00
        let resolved = resolve_imaginary(dt(2024, 3, 10, 2, 0, 0), &tz);
        assert_eq!(resolved, dt(2024, 3, 10, 3, 0, 0));
    }

    #[test]
    fn test_resolve_imaginary_gap_end() {
        let tz = gettz(Some("America/New_York")).unwrap();
        // 02:59 → 03:59
        let resolved = resolve_imaginary(dt(2024, 3, 10, 2, 59, 0), &tz);
        assert_eq!(resolved, dt(2024, 3, 10, 3, 59, 0));
    }

    #[test]
    fn test_datetime_ambiguous_fixed_offset() {
        // Fixed offsets are never ambiguous
        let tz = TimeZone::offset(Some("EST"), -5 * 3600);
        assert!(!datetime_ambiguous(dt(2024, 11, 3, 1, 30, 0), &tz));
    }

    #[test]
    fn test_resolve_imaginary_fixed_offset() {
        // Fixed offsets never have gaps
        let tz = TimeZone::offset(Some("EST"), -5 * 3600);
        let d = dt(2024, 3, 10, 2, 30, 0);
        assert_eq!(resolve_imaginary(d, &tz), d);
    }

    // -----------------------------------------------------------------------
    // Different geographic zones
    // -----------------------------------------------------------------------

    #[test]
    fn test_gettz_europe() {
        let tz = gettz(Some("Europe/London")).unwrap();
        // Winter: GMT (UTC+0)
        assert_eq!(tz.utcoffset(dt(2024, 1, 15, 12, 0, 0), false), 0);
        // Summer: BST (UTC+1)
        assert_eq!(tz.utcoffset(dt(2024, 7, 15, 12, 0, 0), false), 3600);
    }

    #[test]
    fn test_gettz_southern_hemisphere() {
        let tz = gettz(Some("Australia/Sydney")).unwrap();
        // January is SUMMER in southern hemisphere: AEDT (UTC+11)
        assert_eq!(tz.utcoffset(dt(2024, 1, 15, 12, 0, 0), false), 11 * 3600);
        // July is WINTER: AEST (UTC+10)
        assert_eq!(tz.utcoffset(dt(2024, 7, 15, 12, 0, 0), false), 10 * 3600);
    }

    // ---- Coverage: TimeZone dispatch for Local variant ----

    #[test]
    fn test_timezone_local_dst() {
        let tz = gettz(None).unwrap(); // local timezone
        let d = dt(2024, 7, 15, 12, 0, 0);
        // Just verify it doesn't panic
        let _ = tz.dst(d, false);
    }

    #[test]
    fn test_timezone_local_tzname() {
        let tz = gettz(None).unwrap();
        let d = dt(2024, 1, 15, 12, 0, 0);
        let name = tz.tzname(d, false);
        assert!(!name.is_empty());
    }

    #[test]
    fn test_timezone_local_is_ambiguous() {
        let tz = gettz(None).unwrap();
        let d = dt(2024, 6, 15, 12, 0, 0);
        // Just verify it doesn't panic; midday is never ambiguous
        let _ = tz.is_ambiguous(d);
    }

    #[test]
    fn test_timezone_local_fromutc() {
        let tz = gettz(None).unwrap();
        let d = dt(2024, 1, 15, 12, 0, 0);
        let wall = tz.fromutc(d);
        // fromutc should shift by the local offset
        let offset = tz.utcoffset(wall, false);
        // The difference should match the offset
        let diff = (wall - d).num_seconds() as i32;
        assert_eq!(diff, offset);
    }
}
