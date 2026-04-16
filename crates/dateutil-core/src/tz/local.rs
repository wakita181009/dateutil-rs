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
/// Paired with the TZ env var observed at cache-population time so
/// that subsequent calls automatically re-detect on TZ changes.
static CACHED_LOCAL: RwLock<Option<(Option<String>, TzLocal)>> = RwLock::new(None);

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
        let tz_env = std::env::var("TZ").ok();
        // Fast path: cache already populated with the same TZ env var
        {
            let guard = CACHED_LOCAL.read().unwrap();
            if let Some((cached_env, cached_tz)) = guard.as_ref() {
                if cached_env == &tz_env {
                    return cached_tz.clone();
                }
            }
        }
        // Slow path: resolve and (re-)populate
        let mut guard = CACHED_LOCAL.write().unwrap();
        if let Some((cached_env, cached_tz)) = guard.as_ref() {
            if cached_env == &tz_env {
                return cached_tz.clone();
            }
        }
        let name = Self::detect_name(tz_env.as_deref());
        let inner = Self::resolve_tzfile(&name);
        let tz = Self {
            inner,
            name: name.into(),
        };
        *guard = Some((tz_env, tz.clone()));
        tz
    }

    /// Resolve the IANA timezone name, preferring the `TZ` env var when it
    /// names an IANA zone directly. Falls back to `iana-time-zone` detection.
    fn detect_name(tz_env: Option<&str>) -> String {
        if let Some(name) = tz_env {
            let stripped = name.strip_prefix(':').unwrap_or(name);
            // POSIX TZ strings and empty values are not IANA names; skip them.
            if !stripped.is_empty()
                && !stripped.contains(',')
                && !stripped.chars().next().is_some_and(|c| c.is_ascii_digit())
            {
                for base in super::TZPATHS {
                    if std::path::Path::new(&format!("{}/{}", base, stripped)).exists() {
                        return stripped.to_string();
                    }
                }
            }
        }
        iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".to_string())
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

    /// Whether the local timezone has any DST transitions.
    pub fn has_dst(&self) -> bool {
        self.inner.as_ref().is_some_and(|tz| tz.has_dst())
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

    #[cfg(unix)]
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
