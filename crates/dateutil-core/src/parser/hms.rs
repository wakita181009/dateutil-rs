//! HMS (hour/minute/second) label assignment.
//!
//! Used for inputs like "5h", "5.6h" (fractional hour → 5:36), "5m", and
//! bare-number continuation after a labeled field.

use std::borrow::Cow;

use super::{fast_parse_decimal, fast_parse_int, ParseState};

/// Assign a numeric value to the hour, minute, or second slot indicated by
/// `hms_idx` (0 = hour, 1 = minute, 2 = second).
///
/// When a nonzero microsecond component is supplied for hour or minute,
/// the fractional part is carried down to the next smaller unit using
/// pure integer arithmetic (e.g. "5.6h" → hour=5, minute=36).
///
/// Records `last_hms_idx` on the state so a subsequent bare number can be
/// consumed as the next smaller unit ("01h02" → hour=1, minute=2).
#[inline]
pub(super) fn assign_hms(res: &mut ParseState<'_>, hms_idx: usize, int_val: u32, us: u32) {
    match hms_idx {
        0 => {
            res.hour = Some(int_val);
            if us > 0 {
                // Fractional hour → minutes (e.g., "5.6h" → hour=5, minute=36)
                res.minute = Some((us as u64 * 60 / 1_000_000) as u32);
            }
        }
        1 => {
            res.minute = Some(int_val);
            if us > 0 {
                // Fractional minute → seconds (e.g., "5.6m" → minute=5, second=36)
                res.second = Some((us as u64 * 60 / 1_000_000) as u32);
            }
        }
        2 => {
            res.second = Some(int_val);
            if us > 0 {
                res.microsecond = Some(us);
            }
        }
        _ => {}
    }
    if hms_idx <= 2 {
        res.last_hms_idx = Some(hms_idx as u8);
    }
}

/// Consume an optional `:MM[:SS[.ffffff]]` sequence starting at `tokens[i + 1]`,
/// assuming the hour has already been assigned to `tokens[i]`.
#[inline]
pub(super) fn consume_colon_minute_second(
    tokens: &[Cow<'_, str>],
    i: usize,
    len: usize,
    res: &mut ParseState<'_>,
    strict: bool,
) -> usize {
    if i + 2 < len && tokens[i + 1] == ":" {
        if let Some(min) = fast_parse_int(&tokens[i + 2]) {
            res.minute = Some(min as u32);
            let mut consumed = 2; // ":" + MM
            if i + 4 < len && tokens[i + 3] == ":" {
                if let Some(sec) = fast_parse_int(&tokens[i + 4]) {
                    res.second = Some(sec as u32);
                    consumed = 4;
                } else if let Some((sec, us)) = fast_parse_decimal(&tokens[i + 4]) {
                    res.second = Some(sec as u32);
                    if us > 0 {
                        res.microsecond = Some(us);
                    }
                    consumed = 4;
                }
            }
            return consumed;
        }
        if strict {
            res.malformed_time = true;
        }
        return 0;
    }
    if strict && i + 1 < len && tokens[i + 1] == ":" {
        res.malformed_time = true;
    }
    0
}
