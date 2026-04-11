//! TzOffset — Fixed UTC offset timezone (no DST).

use chrono::{NaiveDateTime, TimeDelta};

/// Fixed-offset timezone with an optional name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TzOffset {
    name: Option<Box<str>>,
    /// Pre-computed display name for `tzname()`: user-provided name or "UTC+HH:MM".
    display_name: Box<str>,
    offset_secs: i32,
}

// Hash only on `name` and `offset_secs` — `display_name` is derived from them.
impl std::hash::Hash for TzOffset {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.offset_secs.hash(state);
    }
}

impl TzOffset {
    /// Create a new fixed-offset timezone.
    ///
    /// `name` — timezone abbreviation (e.g. "EST", "JST").
    /// `offset_secs` — total UTC offset in seconds (positive = east).
    pub fn new(name: Option<&str>, offset_secs: i32) -> Self {
        let display_name: Box<str> = match name {
            Some(n) => n.into(),
            None => format_utc_offset(offset_secs).into(),
        };
        Self {
            name: name.map(|s| s.into()),
            display_name,
            offset_secs,
        }
    }

    /// UTC offset in seconds.
    #[inline]
    pub fn utcoffset(&self, _dt: NaiveDateTime, _fold: bool) -> i32 {
        self.offset_secs
    }

    /// DST offset in seconds. Always 0 for fixed offsets.
    #[inline]
    pub fn dst(&self, _dt: NaiveDateTime, _fold: bool) -> i32 {
        0
    }

    /// Timezone abbreviation, or a formatted offset string (e.g. "UTC+05:30") if no name.
    #[inline]
    pub fn tzname(&self, _dt: NaiveDateTime, _fold: bool) -> &str {
        &self.display_name
    }

    /// Fixed offsets are never ambiguous.
    #[inline]
    pub fn is_ambiguous(&self, _dt: NaiveDateTime) -> bool {
        false
    }

    /// Convert a UTC datetime to wall time.
    #[inline]
    pub fn fromutc(&self, dt: NaiveDateTime) -> NaiveDateTime {
        dt + TimeDelta::seconds(self.offset_secs as i64)
    }

    /// Raw offset in seconds.
    #[inline]
    pub fn offset_seconds(&self) -> i32 {
        self.offset_secs
    }

    /// Timezone name, if set.
    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Display name: user-provided name or formatted offset (e.g. "UTC+05:30").
    #[inline]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

/// Format a UTC offset as "UTC", "UTC+HH:MM", or "UTC-HH:MM".
fn format_utc_offset(offset_secs: i32) -> String {
    if offset_secs == 0 {
        return "UTC".to_string();
    }
    let sign = if offset_secs >= 0 { '+' } else { '-' };
    let abs = offset_secs.unsigned_abs();
    let h = abs / 3600;
    let m = (abs % 3600) / 60;
    if m == 0 {
        format!("UTC{sign}{h:02}")
    } else {
        format!("UTC{sign}{h:02}:{m:02}")
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use super::*;

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, mi, s).unwrap()
    }

    #[test]
    fn test_positive_offset() {
        let jst = TzOffset::new(Some("JST"), 9 * 3600);
        let utc_dt = dt(2024, 6, 15, 0, 0, 0);
        assert_eq!(jst.utcoffset(utc_dt, false), 32400);
        assert_eq!(jst.dst(utc_dt, false), 0);
        assert_eq!(jst.tzname(utc_dt, false), "JST");
        assert!(!jst.is_ambiguous(utc_dt));
        assert_eq!(jst.fromutc(utc_dt), dt(2024, 6, 15, 9, 0, 0));
    }

    #[test]
    fn test_negative_offset() {
        let est = TzOffset::new(Some("EST"), -5 * 3600);
        let utc_dt = dt(2024, 6, 15, 12, 0, 0);
        assert_eq!(est.utcoffset(utc_dt, false), -18000);
        assert_eq!(est.fromutc(utc_dt), dt(2024, 6, 15, 7, 0, 0));
    }

    #[test]
    fn test_zero_offset() {
        let utc = TzOffset::new(None, 0);
        let d = dt(2024, 1, 1, 0, 0, 0);
        assert_eq!(utc.utcoffset(d, false), 0);
        assert_eq!(utc.tzname(d, false), "UTC");
        assert_eq!(utc.fromutc(d), d);
    }

    #[test]
    fn test_half_hour_offset() {
        let ist = TzOffset::new(Some("IST"), 5 * 3600 + 1800); // +05:30
        let utc_dt = dt(2024, 6, 15, 0, 0, 0);
        assert_eq!(ist.utcoffset(utc_dt, false), 19800);
        assert_eq!(ist.fromutc(utc_dt), dt(2024, 6, 15, 5, 30, 0));
    }

    #[test]
    fn test_fold_ignored() {
        let tz = TzOffset::new(Some("X"), 3600);
        let d = dt(2024, 1, 1, 0, 0, 0);
        assert_eq!(tz.utcoffset(d, false), tz.utcoffset(d, true));
        assert_eq!(tz.dst(d, false), tz.dst(d, true));
    }

    #[test]
    fn test_equality() {
        let a = TzOffset::new(Some("EST"), -18000);
        let b = TzOffset::new(Some("EST"), -18000);
        let c = TzOffset::new(Some("CDT"), -18000);
        assert_eq!(a, b);
        assert_ne!(a, c); // different name
    }

    #[test]
    fn test_accessors() {
        let tz = TzOffset::new(Some("CET"), 3600);
        assert_eq!(tz.offset_seconds(), 3600);
        assert_eq!(tz.name(), Some("CET"));

        let unnamed = TzOffset::new(None, 0);
        assert_eq!(unnamed.name(), None);
    }

    // -----------------------------------------------------------------------
    // format_utc_offset / display_name
    // -----------------------------------------------------------------------

    #[test]
    fn test_unnamed_positive_offset_display() {
        let tz = TzOffset::new(None, 5 * 3600 + 1800); // +05:30
        let d = dt(2024, 1, 1, 0, 0, 0);
        assert_eq!(tz.tzname(d, false), "UTC+05:30");
        assert_eq!(tz.name(), None);
    }

    #[test]
    fn test_unnamed_negative_offset_display() {
        let tz = TzOffset::new(None, -5 * 3600);
        let d = dt(2024, 1, 1, 0, 0, 0);
        assert_eq!(tz.tzname(d, false), "UTC-05");
    }

    #[test]
    fn test_unnamed_quarter_hour_offset() {
        let tz = TzOffset::new(None, 5 * 3600 + 2700); // +05:45 (Nepal)
        let d = dt(2024, 1, 1, 0, 0, 0);
        assert_eq!(tz.tzname(d, false), "UTC+05:45");
    }

    #[test]
    fn test_unnamed_large_offset() {
        let tz = TzOffset::new(None, 13 * 3600); // +13:00 (Samoa)
        let d = dt(2024, 1, 1, 0, 0, 0);
        assert_eq!(tz.tzname(d, false), "UTC+13");
    }

    #[test]
    fn test_named_offset_display_uses_name() {
        let tz = TzOffset::new(Some("IST"), 5 * 3600 + 1800);
        let d = dt(2024, 1, 1, 0, 0, 0);
        assert_eq!(tz.tzname(d, false), "IST"); // name overrides format
    }

    #[test]
    fn test_clone_preserves_display_name() {
        let tz = TzOffset::new(None, -9 * 3600 - 1800);
        let cloned = tz.clone();
        let d = dt(2024, 1, 1, 0, 0, 0);
        assert_eq!(cloned.tzname(d, false), tz.tzname(d, false));
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::HashSet;
        let a = TzOffset::new(None, 3600);
        let b = TzOffset::new(None, 3600);
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }

    #[test]
    fn test_display_name_method() {
        let named = TzOffset::new(Some("EST"), -5 * 3600);
        assert_eq!(named.display_name(), "EST");

        let unnamed = TzOffset::new(None, 5 * 3600 + 1800);
        assert_eq!(unnamed.display_name(), "UTC+05:30");

        let utc = TzOffset::new(None, 0);
        assert_eq!(utc.display_name(), "UTC");
    }
}
