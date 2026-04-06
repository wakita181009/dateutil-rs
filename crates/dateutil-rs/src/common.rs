use std::fmt;

/// Represents a weekday with an optional N-th occurrence qualifier.
///
/// The `weekday` field is 0-based: 0=Monday, 1=Tuesday, ..., 6=Sunday.
/// The `n` field indicates the N-th occurrence (e.g., 2nd Tuesday = TU(+2)).
/// When `n` is `None` or `Some(0)`, only the day name is displayed,
/// matching Python's `if not self.n` behavior where 0 is falsy.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(name = "weekday", frozen, hash, eq, from_py_object)
)]
pub struct Weekday {
    weekday: u8,
    n: Option<i32>,
}

impl Weekday {
    /// Create a new Weekday.
    ///
    /// # Panics
    /// Panics if `weekday` > 6.
    pub fn new(weekday: u8, n: Option<i32>) -> Self {
        assert!(weekday <= 6, "weekday must be 0..=6, got {weekday}");
        Self { weekday, n }
    }

    pub fn weekday(&self) -> u8 {
        self.weekday
    }

    pub fn n(&self) -> Option<i32> {
        self.n
    }

    /// Create a new Weekday with the same day but different `n`.
    pub fn with_n(&self, n: Option<i32>) -> Self {
        Self::new(self.weekday, n)
    }
}

const DAY_NAMES: [&str; 7] = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];

impl fmt::Display for Weekday {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = DAY_NAMES[self.weekday as usize];
        match self.n {
            None | Some(0) => write!(f, "{name}"),
            Some(n) => write!(f, "{name}({n:+})"),
        }
    }
}

#[cfg(feature = "python")]
#[pyo3::pymethods]
impl Weekday {
    #[new]
    #[pyo3(signature = (weekday, n=None))]
    fn py_new(weekday: u8, n: Option<i32>) -> pyo3::PyResult<Self> {
        if weekday > 6 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "weekday must be 0..=6, got {weekday}"
            )));
        }
        Ok(Self { weekday, n })
    }

    fn __call__(&self, n: Option<i32>) -> Self {
        if n == self.n {
            self.clone()
        } else {
            Self { weekday: self.weekday, n }
        }
    }

    fn __repr__(&self) -> String {
        self.to_string()
    }

    #[getter]
    fn get_weekday(&self) -> u8 {
        self.weekday
    }

    #[getter]
    fn get_n(&self) -> Option<i32> {
        self.n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_no_n() {
        assert_eq!(Weekday::new(0, None).to_string(), "MO");
        assert_eq!(Weekday::new(6, None).to_string(), "SU");
    }

    #[test]
    fn test_display_n_zero() {
        assert_eq!(Weekday::new(0, Some(0)).to_string(), "MO");
    }

    #[test]
    fn test_display_positive_n() {
        assert_eq!(Weekday::new(0, Some(1)).to_string(), "MO(+1)");
        assert_eq!(Weekday::new(0, Some(2)).to_string(), "MO(+2)");
    }

    #[test]
    fn test_display_negative_n() {
        assert_eq!(Weekday::new(4, Some(-1)).to_string(), "FR(-1)");
    }

    #[test]
    fn test_all_seven_days() {
        let names = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];
        for (i, name) in names.iter().enumerate() {
            assert_eq!(Weekday::new(i as u8, None).to_string(), *name);
        }
    }

    #[test]
    fn test_equality_same() {
        assert_eq!(Weekday::new(0, None), Weekday::new(0, None));
        assert_eq!(Weekday::new(0, Some(1)), Weekday::new(0, Some(1)));
    }

    #[test]
    fn test_equality_different_weekday() {
        assert_ne!(Weekday::new(0, None), Weekday::new(1, None));
    }

    #[test]
    fn test_equality_different_n() {
        assert_ne!(Weekday::new(0, None), Weekday::new(0, Some(1)));
        assert_ne!(Weekday::new(0, Some(1)), Weekday::new(0, Some(2)));
    }

    #[test]
    fn test_equality_none_vs_zero() {
        // None and Some(0) are NOT equal (Python: weekday(0, None) != weekday(0, 0))
        assert_ne!(Weekday::new(0, None), Weekday::new(0, Some(0)));
    }

    #[test]
    fn test_hash_consistent() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        Weekday::new(0, Some(1)).hash(&mut h1);
        Weekday::new(0, Some(1)).hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_with_n_same() {
        let mo = Weekday::new(0, Some(2));
        let mo2 = mo.with_n(Some(2));
        assert_eq!(mo, mo2);
    }

    #[test]
    fn test_with_n_different() {
        let mo = Weekday::new(0, None);
        let mo2 = mo.with_n(Some(3));
        assert_eq!(mo2.n(), Some(3));
        assert_eq!(mo2.weekday(), 0);
    }

    #[test]
    #[should_panic(expected = "weekday must be 0..=6")]
    fn test_invalid_weekday() {
        Weekday::new(7, None);
    }
}
