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
    UnknownFormat(String),
    #[error("string does not contain a date: {0}")]
    NoDate(String),
    #[error("{0}")]
    ValueError(String),
}
