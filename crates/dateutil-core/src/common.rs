use crate::error::WeekdayError;
use std::fmt;

/// Represents a weekday with an optional N-th occurrence qualifier.
///
/// The `weekday` field is 0-based: 0=Monday, 1=Tuesday, ..., 6=Sunday.
/// The `n` field indicates the N-th occurrence (e.g., 2nd Tuesday = TU(+2)).
/// When `n` is `None`, only the day name is displayed.
/// `n = Some(0)` is rejected at construction time (`Weekday::new`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Weekday {
    weekday: u8,
    n: Option<i32>,
}

const DAY_NAMES: [&str; 7] = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];

impl Weekday {
    /// Create a new Weekday.
    ///
    /// Returns `Err` if `weekday` > 6.
    pub fn new(weekday: u8, n: Option<i32>) -> Result<Self, WeekdayError> {
        if weekday > 6 {
            return Err(WeekdayError::InvalidWeekday(weekday));
        }
        if n == Some(0) {
            return Err(WeekdayError::InvalidN);
        }
        Ok(Self { weekday, n })
    }

    #[inline]
    pub fn weekday(&self) -> u8 {
        self.weekday
    }

    #[inline]
    pub fn n(&self) -> Option<i32> {
        self.n
    }

    /// Create a new Weekday with the same day but different `n`.
    /// `Some(0)` is normalized to `None` (n=0 is semantically "any occurrence").
    pub fn with_n(&self, n: Option<i32>) -> Self {
        Self {
            weekday: self.weekday,
            n: if n == Some(0) { None } else { n },
        }
    }
}

impl TryFrom<u8> for Weekday {
    type Error = WeekdayError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value, None)
    }
}

impl fmt::Display for Weekday {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = DAY_NAMES[self.weekday as usize];
        match self.n {
            None => write!(f, "{name}"),
            Some(n) => write!(f, "{name}({n:+})"),
        }
    }
}

// Weekday constants for convenience
pub const MO: Weekday = Weekday {
    weekday: 0,
    n: None,
};
pub const TU: Weekday = Weekday {
    weekday: 1,
    n: None,
};
pub const WE: Weekday = Weekday {
    weekday: 2,
    n: None,
};
pub const TH: Weekday = Weekday {
    weekday: 3,
    n: None,
};
pub const FR: Weekday = Weekday {
    weekday: 4,
    n: None,
};
pub const SA: Weekday = Weekday {
    weekday: 5,
    n: None,
};
pub const SU: Weekday = Weekday {
    weekday: 6,
    n: None,
};

// ---------------------------------------------------------------------------
// Calendar helpers
// ---------------------------------------------------------------------------

/// Returns `true` if `year` is a leap year (Gregorian calendar).
#[inline]
pub(crate) fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Returns the number of days in the given `month` of `year`.
///
/// `month` must be in `1..=12`; out-of-range values return `0`.
#[inline]
pub(crate) fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weekday_new_valid() {
        for i in 0..=6 {
            assert!(Weekday::new(i, None).is_ok());
        }
    }

    #[test]
    fn test_weekday_new_invalid() {
        assert!(matches!(
            Weekday::new(7, None),
            Err(WeekdayError::InvalidWeekday(7))
        ));
        assert!(matches!(
            Weekday::new(255, None),
            Err(WeekdayError::InvalidWeekday(255))
        ));
    }

    #[test]
    fn test_weekday_display() {
        assert_eq!(MO.to_string(), "MO");
        assert_eq!(TU.to_string(), "TU");
        assert_eq!(SU.to_string(), "SU");
    }

    #[test]
    fn test_weekday_display_with_n() {
        let wd = Weekday::new(0, Some(2)).unwrap();
        assert_eq!(wd.to_string(), "MO(+2)");

        let wd = Weekday::new(4, Some(-1)).unwrap();
        assert_eq!(wd.to_string(), "FR(-1)");
    }

    #[test]
    fn test_weekday_n_zero_rejected() {
        assert!(matches!(
            Weekday::new(0, Some(0)),
            Err(WeekdayError::InvalidN)
        ));
    }

    #[test]
    fn test_weekday_with_n() {
        let wd = MO.with_n(Some(3));
        assert_eq!(wd.weekday(), 0);
        assert_eq!(wd.n(), Some(3));
        assert_eq!(wd.to_string(), "MO(+3)");
    }

    #[test]
    fn test_weekday_constants() {
        assert_eq!(MO.weekday(), 0);
        assert_eq!(TU.weekday(), 1);
        assert_eq!(WE.weekday(), 2);
        assert_eq!(TH.weekday(), 3);
        assert_eq!(FR.weekday(), 4);
        assert_eq!(SA.weekday(), 5);
        assert_eq!(SU.weekday(), 6);
    }

    #[test]
    fn test_weekday_eq_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(MO);
        set.insert(TU);
        set.insert(MO);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_weekday_with_n_to_none() {
        let wd = MO.with_n(Some(3));
        let reset = wd.with_n(None);
        assert_eq!(reset.n(), None);
        assert_eq!(reset.to_string(), "MO");
    }

    #[test]
    fn test_weekday_large_n() {
        let wd = Weekday::new(0, Some(53)).unwrap();
        assert_eq!(wd.to_string(), "MO(+53)");
        let wd_neg = Weekday::new(6, Some(-100)).unwrap();
        assert_eq!(wd_neg.to_string(), "SU(-100)");
    }

    #[test]
    fn test_weekday_all_invalid() {
        for i in 7..=255 {
            assert!(Weekday::new(i, None).is_err());
        }
    }

    #[test]
    fn test_weekday_eq_different_n() {
        let a = MO.with_n(Some(1));
        let b = MO.with_n(Some(2));
        assert_ne!(a, b);
    }

    #[test]
    fn test_weekday_hash_with_n() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(MO.with_n(Some(1)));
        set.insert(MO.with_n(Some(2)));
        set.insert(MO.with_n(Some(1)));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_weekday_error_display() {
        let err = Weekday::new(7, None).unwrap_err();
        assert_eq!(err.to_string(), "invalid weekday: 7 (must be 0..=6)");
    }

    #[test]
    fn test_weekday_i32_min_max_n() {
        let wd = Weekday::new(0, Some(i32::MAX)).unwrap();
        assert!(wd.to_string().contains(&format!("{}", i32::MAX)));
        let wd = Weekday::new(0, Some(i32::MIN)).unwrap();
        assert!(wd.to_string().contains(&format!("{}", i32::MIN)));
    }

    #[test]
    fn test_weekday_debug_format() {
        let wd = Weekday::new(3, Some(2)).unwrap();
        let debug = format!("{:?}", wd);
        assert!(debug.contains("weekday: 3"));
        assert!(debug.contains("n: Some(2)"));
    }

    #[test]
    fn test_weekday_with_n_chaining() {
        let wd = MO.with_n(Some(1)).with_n(Some(-2)).with_n(None);
        assert_eq!(wd.n(), None);
        assert_eq!(wd.weekday(), 0);
    }

    #[test]
    fn test_weekday_all_constants_n_is_none() {
        for wd in [MO, TU, WE, TH, FR, SA, SU] {
            assert_eq!(wd.n(), None);
        }
    }

    #[test]
    fn test_weekday_hash_set_none() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(MO.with_n(None));
        set.insert(MO.with_n(Some(1)));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_weekday_try_from_valid() {
        let wd: Weekday = 3u8.try_into().unwrap();
        assert_eq!(wd.weekday(), 3);
        assert_eq!(wd.n(), None);
    }

    #[test]
    fn test_weekday_try_from_invalid() {
        let result: Result<Weekday, _> = 7u8.try_into();
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Calendar helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
        assert!(!is_leap_year(1900)); // century non-leap
        assert!(is_leap_year(2000)); // 400-year leap
        assert!(!is_leap_year(2100)); // century non-leap
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2024, 1), 31);
        assert_eq!(days_in_month(2024, 2), 29); // leap year
        assert_eq!(days_in_month(2023, 2), 28); // non-leap
        assert_eq!(days_in_month(2024, 4), 30);
        assert_eq!(days_in_month(2024, 12), 31);
        assert_eq!(days_in_month(1900, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
    }

    #[test]
    fn test_days_in_month_invalid() {
        assert_eq!(days_in_month(2024, 0), 0);
        assert_eq!(days_in_month(2024, 13), 0);
    }
}
