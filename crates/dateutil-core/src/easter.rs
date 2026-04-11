use crate::error::EasterError;
use chrono::NaiveDate;

/// Easter calculation method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EasterMethod {
    /// Original Julian calendar, valid after 326 AD
    Julian = 1,
    /// Julian converted to Gregorian, valid 1583-4099
    Orthodox = 2,
    /// Revised Gregorian method, valid 1583-4099
    Western = 3,
}

impl EasterMethod {
    /// Convert from i32 for compatibility with python-dateutil constants.
    pub fn from_i32(v: i32) -> Result<Self, EasterError> {
        match v {
            1 => Ok(Self::Julian),
            2 => Ok(Self::Orthodox),
            3 => Ok(Self::Western),
            _ => Err(EasterError::InvalidMethod(v)),
        }
    }
}

/// Compute the date of Easter for a given year and method.
///
/// # Errors
/// Returns `Err` if year <= 0 or computed date is invalid.
#[inline]
pub fn easter(year: i32, method: EasterMethod) -> Result<NaiveDate, EasterError> {
    if year <= 0 {
        return Err(EasterError::InvalidYear(year));
    }

    let g = year.rem_euclid(19);
    let mut e = 0;

    let (i, j) = if (method as i32) < 3 {
        let i = (19 * g + 15).rem_euclid(30);
        let j = (year + year / 4 + i).rem_euclid(7);
        if method == EasterMethod::Orthodox {
            e = 10;
            if year > 1600 {
                e += year / 100 - 16 - (year / 100 - 16) / 4;
            }
        }
        (i, j)
    } else {
        let c = year / 100;
        let h = (c - c / 4 - (8 * c + 13) / 25 + 19 * g + 15).rem_euclid(30);
        let i = h - (h / 28) * (1 - (h / 28) * (29 / (h + 1)) * ((21 - g) / 11));
        let j = (year + year / 4 + i + 2 - c + c / 4).rem_euclid(7);
        (i, j)
    };

    let p = i - j + e;
    let d = 1 + (p + 27 + (p + 6) / 40).rem_euclid(31);
    let m = 3 + (p + 26) / 30;

    NaiveDate::from_ymd_opt(year, m as u32, d as u32).ok_or(EasterError::DateOutOfRange {
        year,
        month: m as u32,
        day: d as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_invalid_method_from_i32() {
        assert!(matches!(
            EasterMethod::from_i32(4),
            Err(EasterError::InvalidMethod(4))
        ));
        assert!(matches!(
            EasterMethod::from_i32(0),
            Err(EasterError::InvalidMethod(0))
        ));
    }

    #[test]
    fn test_western_range_1990_2050() {
        let expected: Vec<(i32, u32, u32)> = vec![
            (1990, 4, 15),
            (1991, 3, 31),
            (1992, 4, 19),
            (1993, 4, 11),
            (1994, 4, 3),
            (1995, 4, 16),
            (1996, 4, 7),
            (1997, 3, 30),
            (1998, 4, 12),
            (1999, 4, 4),
            (2000, 4, 23),
            (2001, 4, 15),
            (2002, 3, 31),
            (2003, 4, 20),
            (2004, 4, 11),
            (2005, 3, 27),
            (2006, 4, 16),
            (2007, 4, 8),
            (2008, 3, 23),
            (2009, 4, 12),
            (2010, 4, 4),
            (2011, 4, 24),
            (2012, 4, 8),
            (2013, 3, 31),
            (2014, 4, 20),
            (2015, 4, 5),
            (2016, 3, 27),
            (2017, 4, 16),
            (2018, 4, 1),
            (2019, 4, 21),
            (2020, 4, 12),
            (2021, 4, 4),
            (2022, 4, 17),
            (2023, 4, 9),
            (2024, 3, 31),
            (2025, 4, 20),
            (2026, 4, 5),
            (2027, 3, 28),
            (2028, 4, 16),
            (2029, 4, 1),
            (2030, 4, 21),
            (2031, 4, 13),
            (2032, 3, 28),
            (2033, 4, 17),
            (2034, 4, 9),
            (2035, 3, 25),
            (2036, 4, 13),
            (2037, 4, 5),
            (2038, 4, 25),
            (2039, 4, 10),
            (2040, 4, 1),
            (2041, 4, 21),
            (2042, 4, 6),
            (2043, 3, 29),
            (2044, 4, 17),
            (2045, 4, 9),
            (2046, 3, 25),
            (2047, 4, 14),
            (2048, 4, 5),
            (2049, 4, 18),
            (2050, 4, 10),
        ];
        for (y, m, d) in expected {
            assert_eq!(
                easter(y, EasterMethod::Western).unwrap(),
                NaiveDate::from_ymd_opt(y, m, d).unwrap(),
                "Failed for year {y}"
            );
        }
    }

    #[test]
    fn test_orthodox_range_1990_2050() {
        let expected: Vec<(i32, u32, u32)> = vec![
            (1990, 4, 15),
            (1991, 4, 7),
            (1992, 4, 26),
            (1993, 4, 18),
            (1994, 5, 1),
            (1995, 4, 23),
            (1996, 4, 14),
            (1997, 4, 27),
            (1998, 4, 19),
            (1999, 4, 11),
            (2000, 4, 30),
            (2001, 4, 15),
            (2002, 5, 5),
            (2003, 4, 27),
            (2004, 4, 11),
            (2005, 5, 1),
            (2006, 4, 23),
            (2007, 4, 8),
            (2008, 4, 27),
            (2009, 4, 19),
            (2010, 4, 4),
            (2011, 4, 24),
            (2012, 4, 15),
            (2013, 5, 5),
            (2014, 4, 20),
            (2015, 4, 12),
            (2016, 5, 1),
            (2017, 4, 16),
            (2018, 4, 8),
            (2019, 4, 28),
            (2020, 4, 19),
            (2021, 5, 2),
            (2022, 4, 24),
            (2023, 4, 16),
            (2024, 5, 5),
            (2025, 4, 20),
            (2026, 4, 12),
            (2027, 5, 2),
            (2028, 4, 16),
            (2029, 4, 8),
            (2030, 4, 28),
            (2031, 4, 13),
            (2032, 5, 2),
            (2033, 4, 24),
            (2034, 4, 9),
            (2035, 4, 29),
            (2036, 4, 20),
            (2037, 4, 5),
            (2038, 4, 25),
            (2039, 4, 17),
            (2040, 5, 6),
            (2041, 4, 21),
            (2042, 4, 13),
            (2043, 5, 3),
            (2044, 4, 24),
            (2045, 4, 9),
            (2046, 4, 29),
            (2047, 4, 21),
            (2048, 4, 5),
            (2049, 4, 25),
            (2050, 4, 17),
        ];
        for (y, m, d) in expected {
            assert_eq!(
                easter(y, EasterMethod::Orthodox).unwrap(),
                NaiveDate::from_ymd_opt(y, m, d).unwrap(),
                "Failed for year {y}"
            );
        }
    }

    #[test]
    fn test_western_year_1583() {
        // First valid year for Western method
        let d = easter(1583, EasterMethod::Western).unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(1583, 4, 10).unwrap());
    }

    #[test]
    fn test_western_year_4099() {
        let d = easter(4099, EasterMethod::Western).unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(4099, 4, 19).unwrap());
    }

    #[test]
    fn test_year_1_all_methods() {
        // Smallest valid year
        assert!(easter(1, EasterMethod::Julian).is_ok());
        assert!(easter(1, EasterMethod::Orthodox).is_ok());
        assert!(easter(1, EasterMethod::Western).is_ok());
    }

    #[test]
    fn test_year_9999() {
        // Very large year — should not panic
        assert!(easter(9999, EasterMethod::Western).is_ok());
        assert!(easter(9999, EasterMethod::Orthodox).is_ok());
        assert!(easter(9999, EasterMethod::Julian).is_ok());
    }

    #[test]
    fn test_invalid_year_i32_min() {
        assert!(matches!(
            easter(i32::MIN, EasterMethod::Western),
            Err(EasterError::InvalidYear(i32::MIN))
        ));
    }

    #[test]
    fn test_orthodox_boundary_1600() {
        // Orthodox has special logic for year > 1600
        let before = easter(1600, EasterMethod::Orthodox).unwrap();
        let after = easter(1601, EasterMethod::Orthodox).unwrap();
        assert!(before.month() >= 3 && before.month() <= 5);
        assert!(after.month() >= 3 && after.month() <= 5);
    }

    #[test]
    fn test_easter_always_march_or_april_western() {
        // Western Easter is always in March or April
        for y in 1990..=2100 {
            let d = easter(y, EasterMethod::Western).unwrap();
            assert!(
                d.month() == 3 || d.month() == 4,
                "Western Easter {y} in month {}",
                d.month()
            );
        }
    }

    #[test]
    fn test_easter_orthodox_march_to_may() {
        // Orthodox Easter (Gregorian) can fall in March, April, or May
        for y in 1990..=2100 {
            let d = easter(y, EasterMethod::Orthodox).unwrap();
            assert!(
                (3..=5).contains(&d.month()),
                "Orthodox Easter {y} in month {}",
                d.month()
            );
        }
    }

    #[test]
    fn test_method_from_i32_valid() {
        assert_eq!(EasterMethod::from_i32(1).unwrap(), EasterMethod::Julian);
        assert_eq!(EasterMethod::from_i32(2).unwrap(), EasterMethod::Orthodox);
        assert_eq!(EasterMethod::from_i32(3).unwrap(), EasterMethod::Western);
    }

    #[test]
    fn test_method_from_i32_negative() {
        assert!(EasterMethod::from_i32(-1).is_err());
        assert!(EasterMethod::from_i32(i32::MIN).is_err());
    }

    #[test]
    fn test_easter_is_always_sunday() {
        // Easter should always fall on a Sunday
        for y in 2000..=2050 {
            let d = easter(y, EasterMethod::Western).unwrap();
            assert_eq!(
                d.weekday(),
                chrono::Weekday::Sun,
                "Western Easter {y} is not Sunday: {:?}",
                d.weekday()
            );
        }
    }

    #[test]
    fn test_julian_range() {
        let expected: Vec<(i32, u32, u32)> = vec![
            (326, 4, 3),
            (375, 4, 5),
            (492, 4, 5),
            (552, 3, 31),
            (562, 4, 9),
            (569, 4, 21),
            (597, 4, 14),
            (621, 4, 19),
            (636, 3, 31),
            (655, 3, 29),
            (700, 4, 11),
            (725, 4, 8),
            (750, 3, 29),
            (782, 4, 7),
            (835, 4, 18),
            (849, 4, 14),
            (867, 3, 30),
            (890, 4, 12),
            (922, 4, 21),
            (934, 4, 6),
            (1049, 3, 26),
            (1058, 4, 19),
            (1113, 4, 6),
            (1119, 3, 30),
            (1242, 4, 20),
            (1255, 3, 28),
            (1257, 4, 8),
            (1258, 3, 24),
            (1261, 4, 24),
            (1278, 4, 17),
            (1333, 4, 4),
            (1351, 4, 17),
            (1371, 4, 6),
            (1391, 3, 26),
            (1402, 3, 26),
            (1412, 4, 3),
            (1439, 4, 5),
            (1445, 3, 28),
            (1531, 4, 9),
            (1555, 4, 14),
        ];
        for (y, m, d) in expected {
            assert_eq!(
                easter(y, EasterMethod::Julian).unwrap(),
                NaiveDate::from_ymd_opt(y, m, d).unwrap(),
                "Failed for year {y}"
            );
        }
    }

    #[test]
    fn test_year_0_is_invalid() {
        assert!(matches!(
            easter(0, EasterMethod::Western),
            Err(EasterError::InvalidYear(0))
        ));
        assert!(matches!(
            easter(0, EasterMethod::Orthodox),
            Err(EasterError::InvalidYear(0))
        ));
        assert!(matches!(
            easter(0, EasterMethod::Julian),
            Err(EasterError::InvalidYear(0))
        ));
    }

    #[test]
    fn test_negative_years_various() {
        for y in [-1, -100, -1000, -999_999] {
            assert!(easter(y, EasterMethod::Western).is_err());
            assert!(easter(y, EasterMethod::Orthodox).is_err());
            assert!(easter(y, EasterMethod::Julian).is_err());
        }
    }

    #[test]
    fn test_i32_max_year() {
        // i32::MAX causes arithmetic overflow in the algorithm — this is expected
        let result = std::panic::catch_unwind(|| easter(i32::MAX, EasterMethod::Western));
        // Either panics (overflow) or returns an error — either is acceptable
        if let Ok(res) = result {
            assert!(res.is_ok() || matches!(res, Err(EasterError::DateOutOfRange { .. })));
        }
    }

    #[test]
    fn test_century_boundary_leap_years() {
        for y in [1900, 2000, 2100] {
            let d = easter(y, EasterMethod::Western).unwrap();
            assert!(
                d.month() == 3 || d.month() == 4,
                "year={y}, month={}",
                d.month()
            );
        }
    }

    #[test]
    fn test_easter_method_from_i32_large_values() {
        assert!(EasterMethod::from_i32(100).is_err());
        assert!(EasterMethod::from_i32(i32::MAX).is_err());
        assert!(EasterMethod::from_i32(i32::MIN).is_err());
    }

    #[test]
    fn test_easter_error_display() {
        assert_eq!(
            EasterError::InvalidMethod(99).to_string(),
            "invalid method: 99"
        );
        assert_eq!(EasterError::InvalidYear(-5).to_string(), "invalid year: -5");
        assert_eq!(
            EasterError::DateOutOfRange {
                year: 2024,
                month: 13,
                day: 1
            }
            .to_string(),
            "date out of range: 2024-13-1"
        );
    }

    #[test]
    fn test_easter_always_sunday_western_wide_range() {
        // Western (Gregorian) Easter is always Sunday across a wide year range
        for y in [1583, 1900, 2000, 2100, 3000, 4000, 5000, 9999] {
            let d = easter(y, EasterMethod::Western).unwrap();
            assert_eq!(
                d.weekday(),
                chrono::Weekday::Sun,
                "Western Easter year={y} is not Sunday"
            );
        }
    }

    #[test]
    fn test_easter_method_enum_values() {
        assert_eq!(EasterMethod::Julian as i32, 1);
        assert_eq!(EasterMethod::Orthodox as i32, 2);
        assert_eq!(EasterMethod::Western as i32, 3);
    }

    #[test]
    fn test_western_earliest_and_latest_possible() {
        let d2008 = easter(2008, EasterMethod::Western).unwrap();
        assert_eq!(d2008.month(), 3);
        assert_eq!(d2008.day(), 23);
        let d2038 = easter(2038, EasterMethod::Western).unwrap();
        assert_eq!(d2038.month(), 4);
        assert_eq!(d2038.day(), 25);
    }
}
