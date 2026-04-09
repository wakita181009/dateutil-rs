use thiserror::Error;

/// Top-level error type for the dateutil-core crate.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    #[error("{0}")]
    Easter(#[from] EasterError),
    #[error("{0}")]
    Weekday(#[from] WeekdayError),
    #[error("{0}")]
    RelativeDelta(#[from] RelativeDeltaError),
    #[error("{0}")]
    Parse(#[from] ParseError),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EasterError {
    #[error("invalid method: {0}")]
    InvalidMethod(i32),
    #[error("invalid year: {0}")]
    InvalidYear(i32),
    #[error("date out of range: {year}-{month}-{day}")]
    DateOutOfRange { year: i32, month: u32, day: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WeekdayError {
    #[error("invalid weekday: {0} (must be 0..=6)")]
    InvalidWeekday(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RelativeDeltaError {
    #[error("invalid year day: {0}")]
    InvalidYearDay(i32),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("unknown string format: {0}")]
    UnknownFormat(Box<str>),
    #[error("string does not contain a date: {0}")]
    NoDate(Box<str>),
    #[error("{0}")]
    ValueError(Box<str>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easter_error_display_all_variants() {
        assert_eq!(EasterError::InvalidMethod(0).to_string(), "invalid method: 0");
        assert_eq!(EasterError::InvalidYear(0).to_string(), "invalid year: 0");
        assert_eq!(
            EasterError::DateOutOfRange { year: 2024, month: 2, day: 30 }.to_string(),
            "date out of range: 2024-2-30"
        );
    }

    #[test]
    fn test_weekday_error_display() {
        assert_eq!(WeekdayError::InvalidWeekday(7).to_string(), "invalid weekday: 7 (must be 0..=6)");
        assert_eq!(WeekdayError::InvalidWeekday(255).to_string(), "invalid weekday: 255 (must be 0..=6)");
    }

    #[test]
    fn test_relativedelta_error_display() {
        assert_eq!(RelativeDeltaError::InvalidYearDay(367).to_string(), "invalid year day: 367");
    }

    #[test]
    fn test_parse_error_display() {
        assert_eq!(ParseError::UnknownFormat("xyz".into()).to_string(), "unknown string format: xyz");
        assert_eq!(ParseError::NoDate("".into()).to_string(), "string does not contain a date: ");
        assert_eq!(ParseError::ValueError("bad value".into()).to_string(), "bad value");
    }

    #[test]
    fn test_top_level_error_from_easter() {
        let e: Error = EasterError::InvalidYear(0).into();
        assert_eq!(e.to_string(), "invalid year: 0");
    }

    #[test]
    fn test_top_level_error_from_weekday() {
        let e: Error = WeekdayError::InvalidWeekday(7).into();
        assert_eq!(e.to_string(), "invalid weekday: 7 (must be 0..=6)");
    }

    #[test]
    fn test_top_level_error_from_relativedelta() {
        let e: Error = RelativeDeltaError::InvalidYearDay(400).into();
        assert_eq!(e.to_string(), "invalid year day: 400");
    }

    #[test]
    fn test_top_level_error_from_parse() {
        let e: Error = ParseError::NoDate("test".into()).into();
        assert_eq!(e.to_string(), "string does not contain a date: test");
    }

    #[test]
    fn test_error_clone_and_eq() {
        let e1 = EasterError::InvalidYear(5);
        let e2 = e1.clone();
        assert_eq!(e1, e2);

        let w1 = WeekdayError::InvalidWeekday(7);
        let w2 = w1.clone();
        assert_eq!(w1, w2);

        let p1 = ParseError::NoDate("x".into());
        let p2 = p1.clone();
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_error_debug_format() {
        let e = EasterError::InvalidMethod(99);
        let debug = format!("{:?}", e);
        assert!(debug.contains("InvalidMethod"));
        assert!(debug.contains("99"));
    }
}
