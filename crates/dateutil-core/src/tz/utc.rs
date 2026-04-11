//! TzUtc — UTC timezone (zero offset, no DST).

use chrono::NaiveDateTime;

/// UTC timezone. Zero-sized, always returns offset 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TzUtc;

impl TzUtc {
    /// UTC offset in seconds. Always 0.
    #[inline]
    pub fn utcoffset(&self, _dt: NaiveDateTime, _fold: bool) -> i32 {
        0
    }

    /// DST offset in seconds. Always 0.
    #[inline]
    pub fn dst(&self, _dt: NaiveDateTime, _fold: bool) -> i32 {
        0
    }

    /// Timezone abbreviation. Always "UTC".
    #[inline]
    pub fn tzname(&self, _dt: NaiveDateTime, _fold: bool) -> &str {
        "UTC"
    }

    /// UTC is never ambiguous.
    #[inline]
    pub fn is_ambiguous(&self, _dt: NaiveDateTime) -> bool {
        false
    }

    /// Convert UTC datetime to wall time. Identity for UTC.
    #[inline]
    pub fn fromutc(&self, dt: NaiveDateTime) -> NaiveDateTime {
        dt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::dt;

    #[test]
    fn test_utcoffset() {
        let tz = TzUtc;
        assert_eq!(tz.utcoffset(dt(2024, 6, 15, 12, 0, 0), false), 0);
        assert_eq!(tz.utcoffset(dt(2024, 6, 15, 12, 0, 0), true), 0);
    }

    #[test]
    fn test_dst() {
        assert_eq!(TzUtc.dst(dt(2024, 1, 1, 0, 0, 0), false), 0);
    }

    #[test]
    fn test_tzname() {
        assert_eq!(TzUtc.tzname(dt(2024, 1, 1, 0, 0, 0), false), "UTC");
    }

    #[test]
    fn test_is_ambiguous() {
        assert!(!TzUtc.is_ambiguous(dt(2024, 1, 1, 0, 0, 0)));
    }

    #[test]
    fn test_fromutc() {
        let dt_val = dt(2024, 6, 15, 12, 0, 0);
        assert_eq!(TzUtc.fromutc(dt_val), dt_val);
    }
}
