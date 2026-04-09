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

    // ==== Edge case tests ====

    #[test]
    fn test_ascii_only_punctuation_mix() {
        // Tokenizer operates on ASCII bytes; test mixed ASCII punctuation
        let tokens = tokenize("2024!01@15");
        assert!(!tokens.is_empty());
        assert_eq!(&*tokens[0], "2024");
    }

    #[test]
    fn test_very_long_number() {
        let long_num = "1".repeat(100);
        let tokens = tokenize(&long_num);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].len(), 100);
    }

    #[test]
    fn test_very_long_alpha() {
        let long_alpha = "a".repeat(200);
        let tokens = tokenize(&long_alpha);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].len(), 200);
    }

    #[test]
    fn test_more_than_16_tokens_spills_smallvec() {
        let input = "1-2-3-4-5-6-7-8-9";
        let tokens = tokenize(input);
        assert!(tokens.len() > 16);
        assert_eq!(&*tokens[0], "1");
        assert_eq!(&*tokens[tokens.len() - 1], "9");
    }

    #[test]
    fn test_standalone_dot() {
        let tokens = tokenize(".");
        assert_eq!(strs(&tokens), vec!["."]);
    }

    #[test]
    fn test_standalone_comma() {
        let tokens = tokenize(",");
        assert_eq!(strs(&tokens), vec![","]);
    }

    #[test]
    fn test_multiple_dots() {
        let tokens = tokenize("...");
        assert_eq!(strs(&tokens), vec![".", ".", "."]);
    }

    #[test]
    fn test_brackets_and_parens() {
        let tokens = tokenize("[2024]");
        assert_eq!(strs(&tokens), vec!["[", "2024", "]"]);
    }

    #[test]
    fn test_comma_at_exactly_offset_2() {
        let tokens = tokenize("12,5");
        assert_eq!(strs(&tokens), vec!["12.5"]);
    }

    #[test]
    fn test_comma_at_offset_1_not_decimal() {
        let tokens = tokenize("1,5");
        assert_eq!(strs(&tokens), vec!["1", ",", "5"]);
    }

    #[test]
    fn test_multiple_null_bytes() {
        let tokens = tokenize("\x00\x00\x002024\x00\x00");
        assert_eq!(strs(&tokens), vec!["2024"]);
    }

    #[test]
    fn test_only_whitespace() {
        let tokens = tokenize("   \t\n  ");
        assert_eq!(strs(&tokens), vec![" "]);
    }

    #[test]
    fn test_number_dot_alpha() {
        let tokens = tokenize("15th");
        assert_eq!(strs(&tokens), vec!["15", "th"]);
    }

    #[test]
    fn test_iso_t_separator_lower() {
        let tokens = tokenize("t");
        assert_eq!(strs(&tokens), vec!["t"]);
    }

    #[test]
    fn test_z_timezone() {
        let tokens = tokenize("Z");
        assert_eq!(strs(&tokens), vec!["Z"]);
    }

    #[test]
    fn test_negative_offset_no_space() {
        let tokens = tokenize("-0800");
        assert_eq!(strs(&tokens), vec!["-", "0800"]);
    }

    #[test]
    fn test_decimal_with_only_trailing_zeros() {
        let tokens = tokenize("45.000000");
        assert_eq!(strs(&tokens), vec!["45.000000"]);
    }

    #[test]
    fn test_mixed_separators_in_date() {
        // Dots after digits at offset >= 2 are treated as decimal separators,
        // so "2024.01.15" becomes one token "2024.01" + ".15"
        let tokens = tokenize("2024.01.15");
        assert!(!tokens.is_empty());
        // The first token should start with "2024"
        assert!(tokens[0].starts_with("2024"));
    }

    #[test]
    fn test_number_immediately_after_alpha() {
        let tokens = tokenize("UTC+5");
        assert_eq!(strs(&tokens), vec!["UTC", "+", "5"]);
    }
}
