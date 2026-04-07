use chrono::{Duration, NaiveDateTime};

/// Fixed-offset timezone (no DST).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TzOffset {
    name: Option<String>,
    offset: Duration,
}

impl TzOffset {
    /// Create a new fixed-offset timezone.
    ///
    /// `offset_seconds` is the UTC offset in seconds (positive = east of UTC).
    pub fn new(name: Option<String>, offset_seconds: i32) -> Self {
        TzOffset {
            name,
            offset: Duration::seconds(offset_seconds as i64),
        }
    }

    /// Create from a `Duration`.
    pub fn from_duration(name: Option<String>, offset: Duration) -> Self {
        TzOffset { name, offset }
    }

    pub fn utcoffset(&self, _dt: Option<NaiveDateTime>) -> Option<Duration> {
        Some(self.offset)
    }

    pub fn dst(&self, _dt: Option<NaiveDateTime>) -> Option<Duration> {
        Some(Duration::zero())
    }

    pub fn tzname(&self, _dt: Option<NaiveDateTime>) -> Option<String> {
        self.name.clone()
    }

    pub fn is_ambiguous(&self, _dt: NaiveDateTime) -> bool {
        false
    }

    pub fn fromutc(&self, dt: NaiveDateTime) -> NaiveDateTime {
        dt + self.offset
    }

    /// Get the offset as total seconds.
    pub fn offset_seconds(&self) -> i64 {
        self.offset.num_seconds()
    }

    /// Get the name.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl std::fmt::Display for TzOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.name {
            Some(name) => write!(f, "tzoffset({}, {})", name, self.offset.num_seconds()),
            None => write!(f, "tzoffset(None, {})", self.offset.num_seconds()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_positive_offset() {
        let tz = TzOffset::new(Some("JST".to_string()), 9 * 3600);
        let dt = NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert_eq!(tz.utcoffset(Some(dt)), Some(Duration::hours(9)));
        assert_eq!(tz.dst(Some(dt)), Some(Duration::zero()));
        assert_eq!(tz.tzname(Some(dt)), Some("JST".to_string()));
    }

    #[test]
    fn test_negative_offset() {
        let tz = TzOffset::new(Some("EST".to_string()), -5 * 3600);
        assert_eq!(
            tz.utcoffset(None),
            Some(Duration::seconds(-5 * 3600))
        );
    }

    #[test]
    fn test_zero_offset() {
        let tz = TzOffset::new(None, 0);
        assert_eq!(tz.utcoffset(None), Some(Duration::zero()));
    }

    #[test]
    fn test_fromutc() {
        let tz = TzOffset::new(Some("JST".to_string()), 9 * 3600);
        let utc = NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let expected = NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap();
        assert_eq!(tz.fromutc(utc), expected);
    }

    #[test]
    fn test_is_not_ambiguous() {
        let tz = TzOffset::new(None, 3600);
        let dt = NaiveDate::from_ymd_opt(2020, 6, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert!(!tz.is_ambiguous(dt));
    }

    #[test]
    fn test_display() {
        let tz = TzOffset::new(Some("EST".to_string()), -18000);
        assert_eq!(format!("{tz}"), "tzoffset(EST, -18000)");

        let tz2 = TzOffset::new(None, 3600);
        assert_eq!(format!("{tz2}"), "tzoffset(None, 3600)");
    }

    #[test]
    fn test_from_duration() {
        let tz = TzOffset::from_duration(Some("CET".to_string()), Duration::hours(1));
        assert_eq!(tz.offset_seconds(), 3600);
    }
}
