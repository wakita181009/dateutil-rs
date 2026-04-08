use crate::error::WeekdayError;
use std::fmt;

/// Represents a weekday with an optional N-th occurrence qualifier.
///
/// The `weekday` field is 0-based: 0=Monday, 1=Tuesday, ..., 6=Sunday.
/// The `n` field indicates the N-th occurrence (e.g., 2nd Tuesday = TU(+2)).
/// When `n` is `None` or `Some(0)`, only the day name is displayed.
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
    pub fn with_n(&self, n: Option<i32>) -> Self {
        Self {
            weekday: self.weekday,
            n,
        }
    }
}

impl fmt::Display for Weekday {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = DAY_NAMES[self.weekday as usize];
        match self.n {
            None | Some(0) => write!(f, "{name}"),
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
    fn test_weekday_display_n_zero() {
        let wd = Weekday::new(0, Some(0)).unwrap();
        assert_eq!(wd.to_string(), "MO");
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
    fn test_weekday_negative_one_n() {
        // Last occurrence (e.g., last Friday of month)
        let wd = FR.with_n(Some(-1));
        assert_eq!(wd.n(), Some(-1));
        assert_eq!(wd.to_string(), "FR(-1)");
    }

    #[test]
    fn test_weekday_clone_copy() {
        let wd = MO.with_n(Some(2));
        let cloned = wd;
        assert_eq!(wd, cloned); // Copy semantics — both usable
    }

    #[test]
    fn test_weekday_boundary_values() {
        // Weekday 0 (Monday) and 6 (Sunday) are boundaries
        let mon = Weekday::new(0, Some(1)).unwrap();
        let sun = Weekday::new(6, Some(-1)).unwrap();
        assert_eq!(mon.weekday(), 0);
        assert_eq!(sun.weekday(), 6);
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
    fn test_weekday_eq_none_vs_zero() {
        let a = MO.with_n(None);
        let b = MO.with_n(Some(0));
        // Display is the same but PartialEq differs (n field differs)
        assert_eq!(a.to_string(), b.to_string());
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
}
