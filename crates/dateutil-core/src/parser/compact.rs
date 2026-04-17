//! Compact numeric date/time formats without explicit separators.
//!
//! Handles:
//! - 8 / 10 / 12 / 14 digit `YYYYMMDD[HH[MM[SS]]]`
//! - 6 digit `YYMMDD` with `YYYYMM` fallback
//! - 4 digit `HHMM` and 6 digit `HHMMSS` after a date is already parsed
//!   (used for the portion after `T` in ISO formats)

use std::borrow::Cow;

use super::{fast_parse_decimal, fast_parse_int, ParseState, Ymd};

/// Try to parse compact numeric formats. Returns the number of tokens
/// consumed, or 0 if the token does not match any compact pattern.
#[inline]
pub(super) fn try_parse_compact<'a>(
    tokens: &[Cow<'a, str>],
    i: usize,
    len: usize,
    res: &mut ParseState<'a>,
    ymd: &mut Ymd,
    token: &str,
) -> usize {
    let slen = token.len();
    if !token.as_bytes().iter().all(|b| b.is_ascii_digit()) {
        return 0;
    }

    match slen {
        8 | 12 | 14 if ymd.count == 0 => {
            let Some(year) = fast_parse_int(&token[0..4]) else {
                return 0;
            };
            let Some(month) = fast_parse_int(&token[4..6]) else {
                return 0;
            };
            let Some(day) = fast_parse_int(&token[6..8]) else {
                return 0;
            };

            if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
                return 0;
            }

            ymd.century_specified = true;
            ymd.ystridx = Some(0);
            ymd.push(year);
            ymd.push(month);
            ymd.push(day);

            if slen >= 12 {
                let Some(h) = fast_parse_int(&token[8..10]) else {
                    return 0;
                };
                let Some(m) = fast_parse_int(&token[10..12]) else {
                    return 0;
                };
                res.hour = Some(h as u32);
                res.minute = Some(m as u32);
            }
            if slen == 14 {
                let Some(s) = fast_parse_int(&token[12..14]) else {
                    return 0;
                };
                res.second = Some(s as u32);
            }

            1
        }
        6 if ymd.count == 0 => {
            // Try YYMMDD first; fallback to YYYYMM if month invalid
            let Some(p0) = fast_parse_int(&token[0..2]) else {
                return 0;
            };
            let Some(p1) = fast_parse_int(&token[2..4]) else {
                return 0;
            };
            let Some(p2) = fast_parse_int(&token[4..6]) else {
                return 0;
            };

            if (1..=12).contains(&p1) && (1..=31).contains(&p2) {
                ymd.ystridx = Some(0);
                ymd.push(p0);
                ymd.push(p1);
                ymd.push(p2);
                1
            } else {
                let Some(year) = fast_parse_int(&token[0..4]) else {
                    return 0;
                };
                let Some(month) = fast_parse_int(&token[4..6]) else {
                    return 0;
                };
                if (1..=12).contains(&month) {
                    ymd.century_specified = true;
                    ymd.ystridx = Some(0);
                    ymd.push(year);
                    ymd.push(month);
                    1
                } else {
                    0
                }
            }
        }
        6 if ymd.count == 3 && res.hour.is_none() => {
            // HHMMSS after date is already parsed (e.g., after "T" separator)
            let Some(hour) = fast_parse_int(&token[0..2]) else {
                return 0;
            };
            let Some(minute) = fast_parse_int(&token[2..4]) else {
                return 0;
            };
            let Some(second) = fast_parse_int(&token[4..6]) else {
                return 0;
            };

            if hour <= 23 && minute <= 59 && second <= 59 {
                res.hour = Some(hour as u32);
                res.minute = Some(minute as u32);
                res.second = Some(second as u32);
                1
            } else {
                0
            }
        }
        4 if ymd.count == 3 && res.hour.is_none() => {
            // HHMM after date is already parsed (e.g., "20030925T1049")
            let Some(hour) = fast_parse_int(&token[0..2]) else {
                return 0;
            };
            let Some(minute) = fast_parse_int(&token[2..4]) else {
                return 0;
            };

            if hour <= 23 && minute <= 59 {
                res.hour = Some(hour as u32);
                res.minute = Some(minute as u32);
                1
            } else {
                0
            }
        }
        10 if ymd.count == 0 => {
            // YYYYMMDDHH — optionally followed by :MM(:SS)?
            let Some(year) = fast_parse_int(&token[0..4]) else {
                return 0;
            };
            let Some(month) = fast_parse_int(&token[4..6]) else {
                return 0;
            };
            let Some(day) = fast_parse_int(&token[6..8]) else {
                return 0;
            };
            let Some(hour) = fast_parse_int(&token[8..10]) else {
                return 0;
            };

            if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 {
                return 0;
            }

            ymd.century_specified = true;
            ymd.ystridx = Some(0);
            ymd.push(year);
            ymd.push(month);
            ymd.push(day);
            res.hour = Some(hour as u32);

            let mut consumed = 1;
            if i + 2 < len && tokens[i + 1] == ":" {
                if let Some(min) = fast_parse_int(&tokens[i + 2]) {
                    res.minute = Some(min as u32);
                    consumed = 3;
                    if i + 4 < len && tokens[i + 3] == ":" {
                        if let Some(sec) = fast_parse_int(&tokens[i + 4]) {
                            res.second = Some(sec as u32);
                            consumed = 5;
                        } else if let Some((sec, us)) = fast_parse_decimal(&tokens[i + 4]) {
                            res.second = Some(sec as u32);
                            if us > 0 {
                                res.microsecond = Some(us);
                            }
                            consumed = 5;
                        }
                    }
                }
            }

            consumed
        }
        _ => 0,
    }
}
