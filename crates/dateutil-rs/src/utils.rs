use chrono::{NaiveDateTime, TimeDelta};

/// Returns true if the absolute difference between dt1 and dt2 is within the
/// given delta (inclusive). The delta is treated as its absolute value,
/// matching Python's `abs(delta)` behavior.
pub fn within_delta(dt1: NaiveDateTime, dt2: NaiveDateTime, delta: TimeDelta) -> bool {
    let delta = if delta < TimeDelta::zero() {
        -delta
    } else {
        delta
    };
    let difference = dt1 - dt2;
    -delta <= difference && difference <= delta
}

/// PyO3 wrapper. With the `chrono` feature enabled on pyo3, NaiveDateTime and
/// TimeDelta are automatically converted from/to Python datetime/timedelta.
/// Timezone-aware datetimes will raise TypeError (NaiveDateTime rejects them).
#[cfg(feature = "python")]
#[pyo3::prelude::pyfunction]
#[pyo3(name = "within_delta")]
pub fn within_delta_py(
    dt1: NaiveDateTime,
    dt2: NaiveDateTime,
    delta: TimeDelta,
) -> bool {
    within_delta(dt1, dt2, delta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn ndt(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32, us: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_micro_opt(h, min, s, us)
            .unwrap()
    }

    #[test]
    fn test_within_delta_true() {
        let d1 = ndt(2016, 1, 1, 12, 14, 1, 9);
        let d2 = ndt(2016, 1, 1, 12, 14, 1, 15);
        assert!(within_delta(d1, d2, TimeDelta::seconds(1)));
    }

    #[test]
    fn test_within_delta_false() {
        let d1 = ndt(2016, 1, 1, 12, 14, 1, 9);
        let d2 = ndt(2016, 1, 1, 12, 14, 1, 15);
        assert!(!within_delta(d1, d2, TimeDelta::microseconds(1)));
    }

    #[test]
    fn test_within_delta_negative_delta() {
        let d1 = ndt(2016, 1, 1, 0, 0, 0, 0);
        let d2 = ndt(2015, 12, 31, 0, 0, 0, 0);
        assert!(within_delta(d2, d1, TimeDelta::try_days(-1).unwrap()));
    }

    #[test]
    fn test_within_delta_exact_boundary() {
        let d1 = ndt(2016, 1, 1, 0, 0, 0, 0);
        let d2 = ndt(2016, 1, 1, 0, 0, 1, 0);
        assert!(within_delta(d1, d2, TimeDelta::seconds(1)));
    }

    #[test]
    fn test_within_delta_zero_difference() {
        let d1 = ndt(2016, 1, 1, 0, 0, 0, 0);
        assert!(within_delta(d1, d1, TimeDelta::zero()));
    }
}
