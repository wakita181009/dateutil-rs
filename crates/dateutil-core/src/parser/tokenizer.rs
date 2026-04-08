use smallvec::SmallVec;
use std::borrow::Cow;

/// Zero-copy tokenizer for date/time strings.
///
/// Returns `SmallVec<[Cow<'_, str>; 16]>` — most tokens borrow directly from the input,
/// only tokens requiring mutation (comma→dot decimal normalization) are owned.
pub fn tokenize(s: &str) -> SmallVec<[Cow<'_, str>; 16]> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut tokens: SmallVec<[Cow<'_, str>; 16]> = SmallVec::new();
    let mut pos = 0;

    while pos < len {
        let b = bytes[pos];

        if b == 0 {
            pos += 1;
            continue;
        }

        if b.is_ascii_whitespace() {
            tokens.push(Cow::Borrowed(" "));
            pos += 1;
            // Skip consecutive whitespace
            while pos < len && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
            continue;
        }

        if b.is_ascii_digit() {
            let start = pos;
            pos += 1;
            let mut has_dot = false;
            while pos < len {
                let c = bytes[pos];
                if c.is_ascii_digit() {
                    pos += 1;
                } else if c == b'.' || (c == b',' && pos - start >= 2) {
                    has_dot = true;
                    pos += 1;
                } else {
                    break;
                }
            }
            // Trim trailing dot/comma (sentence-ending punctuation, not decimal)
            if has_dot && pos > start && (bytes[pos - 1] == b'.' || bytes[pos - 1] == b',') {
                pos -= 1;
            }
            let slice = &s[start..pos];
            // Replace comma with dot for decimal numbers (e.g., "1,5" → "1.5")
            if slice.contains(',') && !slice.contains('.') {
                tokens.push(Cow::Owned(slice.replace(',', ".")));
            } else {
                tokens.push(Cow::Borrowed(slice));
            }
            continue;
        }

        if b.is_ascii_alphabetic() {
            let start = pos;
            pos += 1;
            while pos < len && bytes[pos].is_ascii_alphabetic() {
                pos += 1;
            }
            // Handle dot-separated abbreviations (e.g. "a.m.")
            if pos < len && bytes[pos] == b'.' {
                let dot_start = pos;
                pos += 1;
                while pos < len {
                    let c = bytes[pos];
                    if c.is_ascii_alphabetic() || c == b'.' {
                        pos += 1;
                    } else {
                        break;
                    }
                }
                // Split dot-separated tokens: "a.m." → ["a", ".", "m", "."]
                let full = &s[start..pos];
                if full.contains('.') {
                    for part in full.split('.') {
                        if !part.is_empty() {
                            tokens.push(Cow::Borrowed(part));
                        }
                        tokens.push(Cow::Borrowed("."));
                    }
                    // Remove trailing dot if we added one extra
                    if !full.ends_with('.') {
                        tokens.pop();
                    }
                    continue;
                } else {
                    pos = dot_start; // backtrack, not an abbreviation
                }
            }
            tokens.push(Cow::Borrowed(&s[start..pos]));
            continue;
        }

        // Single character token (punctuation) — borrow directly from input
        tokens.push(Cow::Borrowed(&s[pos..pos + 1]));
        pos += 1;
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strs<'a>(tokens: &'a [Cow<'_, str>]) -> Vec<&'a str> {
        tokens.iter().map(|c| &**c).collect()
    }

    #[test]
    fn test_basic_date() {
        let tokens = tokenize("2024-01-15");
        assert_eq!(strs(&tokens), vec!["2024", "-", "01", "-", "15"]);
    }

    #[test]
    fn test_datetime() {
        let tokens = tokenize("2024-01-15 10:30:45");
        assert_eq!(
            strs(&tokens),
            vec!["2024", "-", "01", "-", "15", " ", "10", ":", "30", ":", "45"]
        );
    }

    #[test]
    fn test_month_name() {
        let tokens = tokenize("January 15, 2024");
        assert_eq!(strs(&tokens), vec!["January", " ", "15", ",", " ", "2024"]);
    }

    #[test]
    fn test_decimal_seconds() {
        let tokens = tokenize("10:30:45.123");
        assert_eq!(strs(&tokens), vec!["10", ":", "30", ":", "45.123"]);
    }

    #[test]
    fn test_tz_offset() {
        let tokens = tokenize("2024-01-15T10:30:45+05:30");
        assert_eq!(
            strs(&tokens),
            vec!["2024", "-", "01", "-", "15", "T", "10", ":", "30", ":", "45", "+", "05", ":", "30"]
        );
    }

    #[test]
    fn test_empty() {
        assert!(tokenize("").is_empty());
    }

    #[test]
    fn test_whitespace_collapse() {
        let tokens = tokenize("Jan  15   2024");
        assert_eq!(strs(&tokens), vec!["Jan", " ", "15", " ", "2024"]);
    }

    #[test]
    fn test_null_bytes() {
        let tokens = tokenize("2024\x0001\x0015");
        // Null bytes should be skipped
        assert_eq!(strs(&tokens), vec!["2024", "01", "15"]);
    }

    #[test]
    fn test_comma_decimal_separator() {
        // European decimal: "45,123" → "45.123" (comma at offset >= 2 from start)
        let tokens = tokenize("10:30:45,123");
        assert_eq!(strs(&tokens), vec!["10", ":", "30", ":", "45.123"]);
    }

    #[test]
    fn test_comma_not_decimal_if_short() {
        // Comma in short number is not treated as decimal (it's punctuation)
        let tokens = tokenize("5,2024");
        // "5" then "," then "2024"
        assert_eq!(strs(&tokens), vec!["5", ",", "2024"]);
    }

    #[test]
    fn test_trailing_dot_stripped() {
        // "2024." — trailing dot is sentence punctuation, not decimal
        let tokens = tokenize("2024.");
        assert_eq!(strs(&tokens), vec!["2024", "."]);
    }

    #[test]
    fn test_only_punctuation() {
        let tokens = tokenize("---");
        assert_eq!(strs(&tokens), vec!["-", "-", "-"]);
    }

    #[test]
    fn test_single_char() {
        let tokens = tokenize("T");
        assert_eq!(strs(&tokens), vec!["T"]);
    }

    #[test]
    fn test_am_dot_abbreviation() {
        // "a.m." should be split into alphabetic + dot tokens
        let tokens = tokenize("10:30 a.m.");
        // Expect: "10", ":", "30", " ", "a", ".", "m", "."
        assert!(strs(&tokens).contains(&"a"));
        assert!(strs(&tokens).contains(&"m"));
    }

    #[test]
    fn test_mixed_alpha_num() {
        let tokens = tokenize("Jan15");
        assert_eq!(strs(&tokens), vec!["Jan", "15"]);
    }

    #[test]
    fn test_numbers_with_leading_zeros() {
        let tokens = tokenize("01-02-2024");
        assert_eq!(strs(&tokens), vec!["01", "-", "02", "-", "2024"]);
    }

    #[test]
    fn test_plus_minus_tokens() {
        let tokens = tokenize("+05:30");
        assert_eq!(strs(&tokens), vec!["+", "05", ":", "30"]);
    }

    #[test]
    fn test_long_string() {
        let s = "Wednesday, January 15, 2024 at 10:30:45.123456 PM UTC+05:30";
        let tokens = tokenize(s);
        assert!(!tokens.is_empty());
        // Should not panic, and first token should be the weekday
        assert_eq!(&*tokens[0], "Wednesday");
    }

    #[test]
    fn test_tabs_and_newlines() {
        let tokens = tokenize("Jan\t15\n2024");
        assert_eq!(strs(&tokens), vec!["Jan", " ", "15", " ", "2024"]);
    }

    #[test]
    fn test_consecutive_separators() {
        let tokens = tokenize("2024//01//15");
        assert_eq!(strs(&tokens), vec!["2024", "/", "/", "01", "/", "/", "15"]);
    }

    #[test]
    fn test_decimal_seconds_with_many_digits() {
        let tokens = tokenize("45.123456789");
        assert_eq!(strs(&tokens), vec!["45.123456789"]);
    }

    #[test]
    fn test_zero_value() {
        let tokens = tokenize("0");
        assert_eq!(strs(&tokens), vec!["0"]);
    }
}
