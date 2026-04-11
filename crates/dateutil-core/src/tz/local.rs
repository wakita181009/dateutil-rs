//! TzLocal — System local timezone.
//!
//! Detects the system's IANA timezone name via `iana-time-zone`,
//! then loads the corresponding TZif file for full DST support.

use std::sync::RwLock;

use chrono::{Local, NaiveDateTime, TimeZone as ChronoTz};

use super::file::TzFile;

/// Cached singleton for the system local timezone.
/// Uses `RwLock` (instead of `OnceLock`) so that the cache can be
/// invalidated when the `TZ` environment variable changes.
static CACHED_LOCAL: RwLock<Option<TzLocal>> = RwLock::new(None);

/// System local timezone.
///
/// Uses `iana-time-zone` to detect the system timezone name,
/// then loads the corresponding TZif file for accurate DST handling.
/// Falls back to chrono's `Local` if TZif resolution fails.
///
/// The result is cached via `RwLock` so that only the first call
/// performs OS detection and TZif file I/O.  Call
/// [`TzLocal::invalidate_cache`] to force re-detection (e.g. after
/// changing `TZ`).
#[derive(Debug, Clone)]
pub struct TzLocal {
    inner: Option<TzFile>,
    name: Box<str>,
}

impl Default for TzLocal {
    fn default() -> Self {
        Self::new()
    }
}

impl TzLocal {
    /// Create a new TzLocal by detecting the system timezone.
    ///
    /// The first call resolves the timezone from the OS and loads the
    /// TZif file.  Subsequent calls return a cheap `clone()` of the
    /// cached result (`TzFile` is `Arc`-backed).
    pub fn new() -> Self {
        // Fast path: cache already populated
        {
            let guard = CACHED_LOCAL.read().unwrap();
            if let Some(cached) = guard.as_ref() {
                return cached.clone();
            }
        }
        // Slow path: resolve and populate
        let mut guard = CACHED_LOCAL.write().unwrap();
        // Double-check after acquiring write lock
        if let Some(cached) = guard.as_ref() {
            return cached.clone();
        }
        let name = iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".to_string());
        let inner = Self::resolve_tzfile(&name);
        let tz = Self {
            inner,
            name: name.into(),
        };
        *guard = Some(tz.clone());
        tz
    }

    /// Clear the cached timezone so the next [`TzLocal::new`] call
    /// re-detects the system timezone.  Useful after changing `TZ`.
    pub fn invalidate_cache() {
        let mut guard = CACHED_LOCAL.write().unwrap();
        *guard = None;
    }

    /// Try to load a TZif file for the given IANA timezone name.
    fn resolve_tzfile(name: &str) -> Option<TzFile> {
        for base in super::TZPATHS {
            let path = format!("{}/{}", base, name);
            if let Ok(tz) = TzFile::from_path(&path) {
                return Some(tz);
            }
        }
        None
    }

    /// UTC offset in seconds.
    pub fn utcoffset(&self, dt: NaiveDateTime, fold: bool) -> i32 {
        if let Some(ref tz) = self.inner {
            return tz.utcoffset(dt, fold);
        }
        chrono_local_offset(dt)
    }

    /// DST offset in seconds.
    pub fn dst(&self, dt: NaiveDateTime, fold: bool) -> i32 {
        if let Some(ref tz) = self.inner {
            return tz.dst(dt, fold);
        }
        0 // chrono doesn't expose DST component
    }

    /// Timezone abbreviation.
    pub fn tzname(&self, dt: NaiveDateTime, fold: bool) -> &str {
        if let Some(ref tz) = self.inner {
            return tz.tzname(dt, fold);
        }
        &self.name
    }

    /// Whether the given wall time is ambiguous.
    pub fn is_ambiguous(&self, dt: NaiveDateTime) -> bool {
        if let Some(ref tz) = self.inner {
            return tz.is_ambiguous(dt);
        }
        false
    }

    /// Convert UTC to wall time.
    pub fn fromutc(&self, dt: NaiveDateTime) -> NaiveDateTime {
        if let Some(ref tz) = self.inner {
            return tz.fromutc(dt);
        }
        dt + chrono::TimeDelta::seconds(chrono_local_offset(dt) as i64)
    }

    /// The detected IANA timezone name.
    pub fn iana_name(&self) -> &str {
        &self.name
    }
}

/// Get UTC offset from chrono's Local timezone (fallback).
fn chrono_local_offset(dt: NaiveDateTime) -> i32 {
    match Local.from_local_datetime(&dt) {
        chrono::LocalResult::Single(aware) => aware.offset().local_minus_utc(),
        chrono::LocalResult::Ambiguous(a, _) => a.offset().local_minus_utc(),
        chrono::LocalResult::None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::dt;

    #[test]
    fn test_tzlocal_creates() {
        let tz = TzLocal::new();
        // Should have detected a timezone
        assert!(!tz.iana_name().is_empty());
    }

    #[test]
    fn test_tzlocal_offset_nonzero_or_utc() {
        let tz = TzLocal::new();
        let d = dt(2024, 6, 15, 12, 0, 0);
        let off = tz.utcoffset(d, false);
        // On any system, the offset should be a valid value
        assert!(off.abs() <= 14 * 3600);
    }

    #[test]
    fn test_tzlocal_has_tzfile() {
        let tz = TzLocal::new();
        // On macOS/Linux with zoneinfo, TzFile should be available
        assert!(
            tz.inner.is_some(),
            "TzLocal should resolve to a TzFile on this system"
        );
    }

    #[test]
    fn test_tzlocal_fromutc_roundtrip() {
        let tz = TzLocal::new();
        let utc = dt(2024, 6, 15, 0, 0, 0);
        let wall = tz.fromutc(utc);
        let off = tz.utcoffset(wall, false);
        let back = wall - chrono::TimeDelta::seconds(off as i64);
        assert_eq!(back, utc);
    }

    #[test]
    fn test_tzlocal_dst() {
        let tz = TzLocal::new();
        let d = dt(2024, 6, 15, 12, 0, 0);
        let dst = tz.dst(d, false);
        // DST offset should be 0 or positive (typically 3600)
        assert!(dst >= 0 && dst <= 7200);
    }

    #[test]
    fn test_tzlocal_tzname() {
        let tz = TzLocal::new();
        let d = dt(2024, 6, 15, 12, 0, 0);
        let name = tz.tzname(d, false);
        // Should return a non-empty timezone abbreviation
        assert!(!name.is_empty());
    }

    #[test]
    fn test_tzlocal_is_ambiguous() {
        let tz = TzLocal::new();
        // A normal mid-day time should not be ambiguous
        let d = dt(2024, 6, 15, 12, 0, 0);
        assert!(!tz.is_ambiguous(d));
    }

    #[test]
    fn test_tzlocal_fold_parameter() {
        let tz = TzLocal::new();
        let d = dt(2024, 6, 15, 12, 0, 0);
        // fold=true/false should give same result for non-ambiguous times
        assert_eq!(tz.utcoffset(d, false), tz.utcoffset(d, true));
    }

    #[test]
    fn test_tzlocal_winter_vs_summer() {
        let tz = TzLocal::new();
        let summer = dt(2024, 7, 15, 12, 0, 0);
        let winter = dt(2024, 1, 15, 12, 0, 0);
        let off_summer = tz.utcoffset(summer, false);
        let off_winter = tz.utcoffset(winter, false);
        // Both offsets should be valid
        assert!(off_summer.abs() <= 14 * 3600);
        assert!(off_winter.abs() <= 14 * 3600);
        // On DST-aware systems the offsets may differ; on non-DST systems they're equal
    }

    #[test]
    fn test_tzlocal_default() {
        let tz = TzLocal::default();
        assert!(!tz.iana_name().is_empty());
    }
}
