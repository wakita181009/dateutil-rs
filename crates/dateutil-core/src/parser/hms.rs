//! HMS (hour/minute/second) label assignment.
//!
//! Used for inputs like "5h", "5.6h" (fractional hour → 5:36), "5m", and
//! bare-number continuation after a labeled field.

use super::ParseState;

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
