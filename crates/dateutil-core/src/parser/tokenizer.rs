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
}
