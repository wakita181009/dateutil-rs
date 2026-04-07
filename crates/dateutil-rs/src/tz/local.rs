use chrono::{Datelike, Duration, Local, MappedLocalTime, NaiveDate, NaiveDateTime, TimeZone};

/// System local timezone.
///
/// Uses chrono's `Local` timezone to detect the system's timezone offset
/// and DST status for any given datetime.
#[derive(Debug, Clone)]
pub struct TzLocal;

impl TzLocal {
    pub fn new() -> Self {
        TzLocal
    }

    pub fn utcoffset(&self, dt: Option<NaiveDateTime>, fold: bool) -> Option<Duration> {
        let dt = dt?;
        match Local.offset_from_local_datetime(&dt) {
            MappedLocalTime::Single(offset) => {
                Some(Duration::seconds(offset.local_minus_utc() as i64))
            }
            MappedLocalTime::Ambiguous(first, second) => {
                let offset = if fold { second } else { first };
                Some(Duration::seconds(offset.local_minus_utc() as i64))
            }
            MappedLocalTime::None => {
                // In a DST gap — the time doesn't exist.
                // Return the post-transition offset (what the clock jumps to).
                // Shift forward by 1 hour to find a valid time.
                let shifted = dt + Duration::hours(1);
                match Local.offset_from_local_datetime(&shifted) {
                    MappedLocalTime::Single(offset) => {
                        Some(Duration::seconds(offset.local_minus_utc() as i64))
                    }
                    MappedLocalTime::Ambiguous(_, second) => {
                        Some(Duration::seconds(second.local_minus_utc() as i64))
                    }
                    MappedLocalTime::None => {
                        // Very unusual — shift more
                        let now_offset = Local::now().offset().local_minus_utc();
                        Some(Duration::seconds(now_offset as i64))
                    }
                }
            }
        }
    }

    pub fn dst(&self, dt: Option<NaiveDateTime>, fold: bool) -> Option<Duration> {
        // chrono doesn't directly expose DST offset.
        // Compare January and July offsets to find the standard offset.
        // Standard offset = min(jan, jul), which works for both hemispheres:
        //   Northern: Jan=standard(min), Jul=DST
        //   Southern: Jan=DST, Jul=standard(min)
        let dt = dt?;
        let offset = self.utcoffset(Some(dt), fold)?;

        let year = dt.date().year();
        let jan_offset = Self::offset_at(
            NaiveDate::from_ymd_opt(year, 1, 1)?.and_hms_opt(12, 0, 0)?,
        );
        let jul_offset = Self::offset_at(
            NaiveDate::from_ymd_opt(year, 7, 1)?.and_hms_opt(12, 0, 0)?,
        );

        let std_offset = std::cmp::min(jan_offset, jul_offset);
        Some(offset - std_offset)
    }

    /// Get the local UTC offset for a given naive datetime (no fold handling).
    /// Used for probing standard vs DST offsets at reference dates.
    fn offset_at(dt: NaiveDateTime) -> Duration {
        match Local.offset_from_local_datetime(&dt) {
            MappedLocalTime::Single(o) => Duration::seconds(o.local_minus_utc() as i64),
            MappedLocalTime::Ambiguous(o, _) => Duration::seconds(o.local_minus_utc() as i64),
            MappedLocalTime::None => {
                // Shouldn't happen for noon reference times, but handle gracefully
                Duration::seconds(Local::now().offset().local_minus_utc() as i64)
            }
        }
    }

    pub fn tzname(&self, dt: Option<NaiveDateTime>, _fold: bool) -> Option<String> {
        // chrono doesn't provide timezone name directly.
        // Return a generic name based on offset.
        let dt = dt?;
        let offset = self.utcoffset(Some(dt), false)?;
        let total_secs = offset.num_seconds();
        let hours = total_secs / 3600;
        let mins = (total_secs.abs() % 3600) / 60;
        if mins == 0 {
            Some(format!("UTC{hours:+}"))
        } else {
            let sign = if total_secs >= 0 { '+' } else { '-' };
            Some(format!("UTC{sign}{:02}:{:02}", hours.abs(), mins))
        }
    }

    pub fn is_ambiguous(&self, dt: NaiveDateTime) -> bool {
        matches!(
            Local.offset_from_local_datetime(&dt),
            MappedLocalTime::Ambiguous(_, _)
        )
    }

    pub fn fromutc(&self, dt: NaiveDateTime) -> (NaiveDateTime, bool) {
        let offset = Local
            .offset_from_utc_datetime(&dt)
            .local_minus_utc();
        let wall = dt + Duration::seconds(offset as i64);
        let fold = self.is_ambiguous(wall);
        (wall, fold)
    }
}

impl Default for TzLocal {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TzLocal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tzlocal()")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_tzlocal_returns_some() {
        let tz = TzLocal::new();
        let dt = NaiveDate::from_ymd_opt(2020, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert!(tz.utcoffset(Some(dt), false).is_some());
        assert!(tz.dst(Some(dt), false).is_some());
    }

    #[test]
    fn test_tzlocal_none_dt() {
        let tz = TzLocal::new();
        assert_eq!(tz.utcoffset(None, false), None);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", TzLocal::new()), "tzlocal()");
    }

    #[test]
    fn test_default() {
        let tz = TzLocal::default();
        let dt = NaiveDate::from_ymd_opt(2020, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert!(tz.utcoffset(Some(dt), false).is_some());
    }

    #[test]
    fn test_dst_some() {
        let tz = TzLocal::new();
        let dt = NaiveDate::from_ymd_opt(2020, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        // dst() should return Some (may be zero or non-zero depending on locale)
        assert!(tz.dst(Some(dt), false).is_some());
    }

    #[test]
    fn test_dst_none_dt() {
        let tz = TzLocal::new();
        assert_eq!(tz.dst(None, false), None);
    }

    #[test]
    fn test_tzname_some() {
        let tz = TzLocal::new();
        let dt = NaiveDate::from_ymd_opt(2020, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let name = tz.tzname(Some(dt), false);
        assert!(name.is_some());
        assert!(name.unwrap().starts_with("UTC"));
    }

    #[test]
    fn test_tzname_none_dt() {
        let tz = TzLocal::new();
        assert_eq!(tz.tzname(None, false), None);
    }

    #[test]
    fn test_is_ambiguous() {
        let tz = TzLocal::new();
        // Normal time should not be ambiguous
        let dt = NaiveDate::from_ymd_opt(2020, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        // Just test it doesn't panic; result depends on locale
        let _ = tz.is_ambiguous(dt);
    }

    #[test]
    fn test_fromutc() {
        let tz = TzLocal::new();
        let dt = NaiveDate::from_ymd_opt(2020, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let (wall, _fold) = tz.fromutc(dt);
        // wall should be dt + local offset
        let offset = tz.utcoffset(Some(wall), false).unwrap();
        let expected = dt + offset;
        assert_eq!(wall, expected);
    }

    #[test]
    fn test_utcoffset_winter_and_summer() {
        let tz = TzLocal::new();
        let winter = NaiveDate::from_ymd_opt(2020, 1, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let summer = NaiveDate::from_ymd_opt(2020, 7, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        // Both should return Some
        assert!(tz.utcoffset(Some(winter), false).is_some());
        assert!(tz.utcoffset(Some(summer), false).is_some());
    }
}
