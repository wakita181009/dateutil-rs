use crate::error::EasterError;
use chrono::NaiveDate;

pub const EASTER_JULIAN: i32 = 1;
pub const EASTER_ORTHODOX: i32 = 2;
pub const EASTER_WESTERN: i32 = 3;

/// Compute the date of Easter for a given year and method.
///
/// # Methods
/// - `EASTER_JULIAN` (1): Original Julian calendar, valid after 326 AD
/// - `EASTER_ORTHODOX` (2): Julian converted to Gregorian, valid 1583-4099
/// - `EASTER_WESTERN` (3): Revised Gregorian method, valid 1583-4099
///
/// # Errors
/// Returns `Err` if method is not 1-3, year <= 0, or computed date is invalid.
#[inline]
pub fn easter(year: i32, method: i32) -> Result<NaiveDate, EasterError> {
    if !(1..=3).contains(&method) {
        return Err(EasterError::InvalidMethod(method));
    }
    if year <= 0 {
        return Err(EasterError::InvalidYear(year));
    }

    let g = year.rem_euclid(19);
    let mut e = 0;

    let (i, j) = if method < 3 {
        let i = (19 * g + 15).rem_euclid(30);
        let j = (year + year / 4 + i).rem_euclid(7);
        if method == 2 {
            e = 10;
            if year > 1600 {
                e = e + year / 100 - 16 - (year / 100 - 16) / 4;
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

    #[test]
    fn test_western_2024() {
        assert_eq!(
            easter(2024, EASTER_WESTERN).unwrap(),
            NaiveDate::from_ymd_opt(2024, 3, 31).unwrap()
        );
    }

    #[test]
    fn test_orthodox_2024() {
        assert_eq!(
            easter(2024, EASTER_ORTHODOX).unwrap(),
            NaiveDate::from_ymd_opt(2024, 5, 5).unwrap()
        );
    }

    #[test]
    fn test_julian_326() {
        assert_eq!(
            easter(326, EASTER_JULIAN).unwrap(),
            NaiveDate::from_ymd_opt(326, 4, 3).unwrap()
        );
    }

    #[test]
    fn test_invalid_method() {
        assert!(matches!(
            easter(2024, 4),
            Err(EasterError::InvalidMethod(4))
        ));
        assert!(matches!(
            easter(2024, 0),
            Err(EasterError::InvalidMethod(0))
        ));
    }

    #[test]
    fn test_invalid_year() {
        assert!(matches!(
            easter(0, EASTER_WESTERN),
            Err(EasterError::InvalidYear(0))
        ));
        assert!(matches!(
            easter(-1, EASTER_WESTERN),
            Err(EasterError::InvalidYear(-1))
        ));
    }

    #[test]
    fn test_western_range_1990_2050() {
        let expected: Vec<(i32, u32, u32)> = vec![
            (1990, 4, 15), (1991, 3, 31), (1992, 4, 19), (1993, 4, 11),
            (1994, 4, 3),  (1995, 4, 16), (1996, 4, 7),  (1997, 3, 30),
            (1998, 4, 12), (1999, 4, 4),  (2000, 4, 23), (2001, 4, 15),
            (2002, 3, 31), (2003, 4, 20), (2004, 4, 11), (2005, 3, 27),
            (2006, 4, 16), (2007, 4, 8),  (2008, 3, 23), (2009, 4, 12),
            (2010, 4, 4),  (2011, 4, 24), (2012, 4, 8),  (2013, 3, 31),
            (2014, 4, 20), (2015, 4, 5),  (2016, 3, 27), (2017, 4, 16),
            (2018, 4, 1),  (2019, 4, 21), (2020, 4, 12), (2021, 4, 4),
            (2022, 4, 17), (2023, 4, 9),  (2024, 3, 31), (2025, 4, 20),
            (2026, 4, 5),  (2027, 3, 28), (2028, 4, 16), (2029, 4, 1),
            (2030, 4, 21), (2031, 4, 13), (2032, 3, 28), (2033, 4, 17),
            (2034, 4, 9),  (2035, 3, 25), (2036, 4, 13), (2037, 4, 5),
            (2038, 4, 25), (2039, 4, 10), (2040, 4, 1),  (2041, 4, 21),
            (2042, 4, 6),  (2043, 3, 29), (2044, 4, 17), (2045, 4, 9),
            (2046, 3, 25), (2047, 4, 14), (2048, 4, 5),  (2049, 4, 18),
            (2050, 4, 10),
        ];
        for (y, m, d) in expected {
            assert_eq!(
                easter(y, EASTER_WESTERN).unwrap(),
                NaiveDate::from_ymd_opt(y, m, d).unwrap(),
                "Failed for year {y}"
            );
        }
    }

    #[test]
    fn test_orthodox_range_1990_2050() {
        let expected: Vec<(i32, u32, u32)> = vec![
            (1990, 4, 15), (1991, 4, 7),  (1992, 4, 26), (1993, 4, 18),
            (1994, 5, 1),  (1995, 4, 23), (1996, 4, 14), (1997, 4, 27),
            (1998, 4, 19), (1999, 4, 11), (2000, 4, 30), (2001, 4, 15),
            (2002, 5, 5),  (2003, 4, 27), (2004, 4, 11), (2005, 5, 1),
            (2006, 4, 23), (2007, 4, 8),  (2008, 4, 27), (2009, 4, 19),
            (2010, 4, 4),  (2011, 4, 24), (2012, 4, 15), (2013, 5, 5),
            (2014, 4, 20), (2015, 4, 12), (2016, 5, 1),  (2017, 4, 16),
            (2018, 4, 8),  (2019, 4, 28), (2020, 4, 19), (2021, 5, 2),
            (2022, 4, 24), (2023, 4, 16), (2024, 5, 5),  (2025, 4, 20),
            (2026, 4, 12), (2027, 5, 2),  (2028, 4, 16), (2029, 4, 8),
            (2030, 4, 28), (2031, 4, 13), (2032, 5, 2),  (2033, 4, 24),
            (2034, 4, 9),  (2035, 4, 29), (2036, 4, 20), (2037, 4, 5),
            (2038, 4, 25), (2039, 4, 17), (2040, 5, 6),  (2041, 4, 21),
            (2042, 4, 13), (2043, 5, 3),  (2044, 4, 24), (2045, 4, 9),
            (2046, 4, 29), (2047, 4, 21), (2048, 4, 5),  (2049, 4, 25),
            (2050, 4, 17),
        ];
        for (y, m, d) in expected {
            assert_eq!(
                easter(y, EASTER_ORTHODOX).unwrap(),
                NaiveDate::from_ymd_opt(y, m, d).unwrap(),
                "Failed for year {y}"
            );
        }
    }

    #[test]
    fn test_julian_range() {
        let expected: Vec<(i32, u32, u32)> = vec![
            (326, 4, 3),   (375, 4, 5),   (492, 4, 5),   (552, 3, 31),
            (562, 4, 9),   (569, 4, 21),  (597, 4, 14),  (621, 4, 19),
            (636, 3, 31),  (655, 3, 29),  (700, 4, 11),  (725, 4, 8),
            (750, 3, 29),  (782, 4, 7),   (835, 4, 18),  (849, 4, 14),
            (867, 3, 30),  (890, 4, 12),  (922, 4, 21),  (934, 4, 6),
            (1049, 3, 26), (1058, 4, 19), (1113, 4, 6),  (1119, 3, 30),
            (1242, 4, 20), (1255, 3, 28), (1257, 4, 8),  (1258, 3, 24),
            (1261, 4, 24), (1278, 4, 17), (1333, 4, 4),  (1351, 4, 17),
            (1371, 4, 6),  (1391, 3, 26), (1402, 3, 26), (1412, 4, 3),
            (1439, 4, 5),  (1445, 3, 28), (1531, 4, 9),  (1555, 4, 14),
        ];
        for (y, m, d) in expected {
            assert_eq!(
                easter(y, EASTER_JULIAN).unwrap(),
                NaiveDate::from_ymd_opt(y, m, d).unwrap(),
                "Failed for year {y}"
            );
        }
    }
}
