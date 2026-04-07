use chrono::{Duration, NaiveDateTime};

/// UTC timezone — always zero offset, no DST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TzUtc;

impl TzUtc {
    pub fn new() -> Self {
        TzUtc
    }

    pub fn utcoffset(&self, _dt: Option<NaiveDateTime>) -> Option<Duration> {
        Some(Duration::zero())
    }

    pub fn dst(&self, _dt: Option<NaiveDateTime>) -> Option<Duration> {
        Some(Duration::zero())
    }

    pub fn tzname(&self, _dt: Option<NaiveDateTime>) -> Option<String> {
        Some("UTC".to_string())
    }

    pub fn is_ambiguous(&self, _dt: NaiveDateTime) -> bool {
        false
    }

    pub fn fromutc(&self, dt: NaiveDateTime) -> NaiveDateTime {
        dt
    }
}

impl Default for TzUtc {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TzUtc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tzutc()")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_utcoffset() {
        let tz = TzUtc::new();
        let dt = NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tz.utcoffset(Some(dt)), Some(Duration::zero()));
        assert_eq!(tz.utcoffset(None), Some(Duration::zero()));
    }

    #[test]
    fn test_dst() {
        let tz = TzUtc::new();
        assert_eq!(tz.dst(None), Some(Duration::zero()));
    }

    #[test]
    fn test_tzname() {
        let tz = TzUtc::new();
        assert_eq!(tz.tzname(None), Some("UTC".to_string()));
    }

    #[test]
    fn test_is_ambiguous() {
        let tz = TzUtc::new();
        let dt = NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert!(!tz.is_ambiguous(dt));
    }

    #[test]
    fn test_fromutc() {
        let tz = TzUtc::new();
        let dt = NaiveDate::from_ymd_opt(2020, 6, 15)
            .unwrap()
            .and_hms_opt(10, 30, 0)
            .unwrap();
        assert_eq!(tz.fromutc(dt), dt);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", TzUtc::new()), "tzutc()");
    }

    #[test]
    fn test_equality() {
        assert_eq!(TzUtc::new(), TzUtc::new());
    }
}
