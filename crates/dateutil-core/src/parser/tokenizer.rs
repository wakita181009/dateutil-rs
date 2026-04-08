/// Zero-copy tokenizer for date/time strings.
///
/// Returns `Vec<String>` for now (some tokens need mutation like comma→dot),
/// but the tokenizer avoids intermediate VecDeque and char-by-char String::push.
/// Instead it tracks byte ranges in the input and only allocates final tokens.
pub fn tokenize(s: &str) -> Vec<String> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut tokens: Vec<String> = Vec::with_capacity(16);
    let mut pos = 0;

    while pos < len {
        let b = bytes[pos];

        if b == 0 {
            pos += 1;
            continue;
        }

        if b.is_ascii_whitespace() {
            tokens.push(" ".into());
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
            let mut token = s[start..pos].to_string();
            // If it ends with '.' or ',' and has letters after, split
            if has_dot && token.ends_with(['.', ',']) {
                token.pop();
                pos -= 1;
            }
            // Replace comma with dot for decimal numbers
            if has_dot && !token.contains('.') {
                token = token.replace(',', ".");
            }
            tokens.push(token);
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
                let _is_abbrev = true;
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
                            tokens.push(part.to_string());
                        }
                        tokens.push(".".into());
                    }
                    // Remove trailing dot if we added one extra
                    if full.ends_with('.') {
                        // Already correct
                    } else {
                        tokens.pop(); // remove extra trailing "."
                    }
                    continue;
                } else {
                    pos = dot_start; // backtrack, not an abbreviation
                }
            }
            tokens.push(s[start..pos].to_string());
            continue;
        }

        // Single character token (punctuation)
        tokens.push(s[pos..pos + 1].to_string());
        pos += 1;
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_date() {
        let tokens = tokenize("2024-01-15");
        assert_eq!(tokens, vec!["2024", "-", "01", "-", "15"]);
    }

    #[test]
    fn test_datetime() {
        let tokens = tokenize("2024-01-15 10:30:45");
        assert_eq!(
            tokens,
            vec!["2024", "-", "01", "-", "15", " ", "10", ":", "30", ":", "45"]
        );
    }

    #[test]
    fn test_month_name() {
        let tokens = tokenize("January 15, 2024");
        assert_eq!(tokens, vec!["January", " ", "15", ",", " ", "2024"]);
    }

    #[test]
    fn test_decimal_seconds() {
        let tokens = tokenize("10:30:45.123");
        assert_eq!(tokens, vec!["10", ":", "30", ":", "45.123"]);
    }

    #[test]
    fn test_tz_offset() {
        let tokens = tokenize("2024-01-15T10:30:45+05:30");
        assert_eq!(
            tokens,
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
        assert_eq!(tokens, vec!["Jan", " ", "15", " ", "2024"]);
    }
}
