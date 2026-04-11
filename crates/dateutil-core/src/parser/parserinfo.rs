use std::collections::{HashMap, HashSet};

use super::{
    lookup_ampm, lookup_hms, lookup_jump, lookup_month, lookup_pertain, lookup_utczone,
    lookup_weekday,
};
use super::{lower_str, lowercase_buf};

// ---------------------------------------------------------------------------
// ParserInfo — custom lookup tables for non-default locale support
// ---------------------------------------------------------------------------

/// Custom parser configuration that overrides the default PHF lookup tables.
///
/// All string keys must be stored in **lowercase** for case-insensitive matching.
/// Use [`ParserInfo::default()`] to get the standard English tables as `HashMap`s.
pub struct ParserInfo {
    /// Jump words — ignored during parsing (e.g. "at", "on", ",").
    pub jump: HashSet<String>,
    /// Weekday name → 0-based index (Mon=0 .. Sun=6).
    pub weekdays: HashMap<String, usize>,
    /// Month name → 1-based index (Jan=1 .. Dec=12).
    pub months: HashMap<String, usize>,
    /// HMS indicator → 0=hour, 1=minute, 2=second.
    pub hms: HashMap<String, usize>,
    /// AM/PM → 0=AM, 1=PM.
    pub ampm: HashMap<String, usize>,
    /// UTC-equivalent zone names.
    pub utczone: HashSet<String>,
    /// Pertain words (e.g. "of").
    pub pertain: HashSet<String>,
    /// Known timezone abbreviations → offset in seconds.
    pub tzoffset: HashMap<String, i32>,
}

impl Default for ParserInfo {
    fn default() -> Self {
        let jump: HashSet<String> = [
            " ", ".", ",", ";", "-", "/", "'", "at", "on", "and", "ad", "m", "t", "of", "st", "nd",
            "rd", "th",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let mut weekdays = HashMap::new();
        for (i, names) in [
            &["mon", "monday"][..],
            &["tue", "tuesday"],
            &["wed", "wednesday"],
            &["thu", "thursday"],
            &["fri", "friday"],
            &["sat", "saturday"],
            &["sun", "sunday"],
        ]
        .iter()
        .enumerate()
        {
            for name in *names {
                weekdays.insert(String::from(*name), i);
            }
        }

        let mut months = HashMap::new();
        for (i, names) in [
            &["jan", "january"][..],
            &["feb", "february"],
            &["mar", "march"],
            &["apr", "april"],
            &["may"][..],
            &["jun", "june"],
            &["jul", "july"],
            &["aug", "august"],
            &["sep", "sept", "september"],
            &["oct", "october"],
            &["nov", "november"],
            &["dec", "december"],
        ]
        .iter()
        .enumerate()
        {
            for name in *names {
                months.insert(String::from(*name), i + 1);
            }
        }

        let mut hms = HashMap::new();
        for (i, names) in [
            &["h", "hour", "hours"][..],
            &["m", "minute", "minutes"],
            &["s", "second", "seconds"],
        ]
        .iter()
        .enumerate()
        {
            for name in *names {
                hms.insert(String::from(*name), i);
            }
        }

        let mut ampm = HashMap::new();
        for (i, names) in [&["am", "a"][..], &["pm", "p"]].iter().enumerate() {
            for name in *names {
                ampm.insert(String::from(*name), i);
            }
        }

        let utczone: HashSet<String> = ["utc", "gmt", "z"].into_iter().map(String::from).collect();
        let pertain: HashSet<String> = ["of"].into_iter().map(String::from).collect();

        Self {
            jump,
            weekdays,
            months,
            hms,
            ampm,
            utczone,
            pertain,
            tzoffset: HashMap::new(),
        }
    }
}

impl ParserInfo {
    #[inline]
    pub fn jump(&self, s: &str) -> bool {
        lowercase_buf(s).is_some_and(|buf| self.jump.contains(lower_str(s, &buf)))
    }

    #[inline]
    pub fn weekday(&self, s: &str) -> Option<usize> {
        let buf = lowercase_buf(s)?;
        let low = lower_str(s, &buf);
        if let Some(&v) = self.weekdays.get(low) {
            return Some(v);
        }
        if s.len() >= 4 {
            if let Some(&v) = self.weekdays.get(&low[..3]) {
                return Some(v);
            }
        }
        None
    }

    #[inline]
    pub fn month(&self, s: &str) -> Option<usize> {
        let buf = lowercase_buf(s)?;
        self.months.get(lower_str(s, &buf)).copied()
    }

    #[inline]
    pub fn hms(&self, s: &str) -> Option<usize> {
        let buf = lowercase_buf(s)?;
        self.hms.get(lower_str(s, &buf)).copied()
    }

    #[inline]
    pub fn ampm(&self, s: &str) -> Option<usize> {
        let buf = lowercase_buf(s)?;
        self.ampm.get(lower_str(s, &buf)).copied()
    }

    #[inline]
    pub fn pertain(&self, s: &str) -> bool {
        lowercase_buf(s).is_some_and(|buf| self.pertain.contains(lower_str(s, &buf)))
    }

    #[inline]
    pub fn utczone(&self, s: &str) -> bool {
        lowercase_buf(s).is_some_and(|buf| self.utczone.contains(lower_str(s, &buf)))
    }

    /// Look up a known timezone abbreviation. Returns offset in seconds.
    /// UTC-equivalent zones return `Some(0)`. Matching is case-insensitive.
    /// Single `lowercase_buf` call covers both utczone and tzoffset lookups.
    #[inline]
    pub fn tzoffset(&self, name: &str) -> Option<i32> {
        let buf = lowercase_buf(name)?;
        let low = lower_str(name, &buf);
        if self.utczone.contains(low) {
            return Some(0);
        }
        self.tzoffset.get(low).copied()
    }
}

// ---------------------------------------------------------------------------
// Dispatch helpers — use ParserInfo when provided, PHF otherwise.
// ---------------------------------------------------------------------------

#[inline]
pub(super) fn do_jump(s: &str, info: Option<&ParserInfo>) -> bool {
    match info {
        Some(i) => i.jump(s),
        None => lookup_jump(s),
    }
}

#[inline]
pub(super) fn do_weekday(s: &str, info: Option<&ParserInfo>) -> Option<usize> {
    match info {
        Some(i) => i.weekday(s),
        None => lookup_weekday(s),
    }
}

#[inline]
pub(super) fn do_month(s: &str, info: Option<&ParserInfo>) -> Option<usize> {
    match info {
        Some(i) => i.month(s),
        None => lookup_month(s),
    }
}

#[inline]
pub(super) fn do_hms(s: &str, info: Option<&ParserInfo>) -> Option<usize> {
    match info {
        Some(i) => i.hms(s),
        None => lookup_hms(s),
    }
}

#[inline]
pub(super) fn do_ampm(s: &str, info: Option<&ParserInfo>) -> Option<usize> {
    match info {
        Some(i) => i.ampm(s),
        None => lookup_ampm(s),
    }
}

#[inline]
pub(super) fn do_pertain(s: &str, info: Option<&ParserInfo>) -> bool {
    match info {
        Some(i) => i.pertain(s),
        None => lookup_pertain(s),
    }
}

#[inline]
pub(super) fn do_utczone(s: &str, info: Option<&ParserInfo>) -> bool {
    match info {
        Some(i) => i.utczone(s),
        None => lookup_utczone(s),
    }
}

#[inline]
pub(super) fn do_tzoffset(name: &str, info: Option<&ParserInfo>) -> Option<i32> {
    match info {
        Some(i) => i.tzoffset(name),
        None => None, // default PHF has no tzoffset map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parserinfo_default_months() {
        let info = ParserInfo::default();
        assert_eq!(info.month("January"), Some(1));
        assert_eq!(info.month("jan"), Some(1));
        assert_eq!(info.month("DECEMBER"), Some(12));
        assert_eq!(info.month("sept"), Some(9));
        assert_eq!(info.month("xyz"), None);
    }

    #[test]
    fn test_parserinfo_default_weekdays() {
        let info = ParserInfo::default();
        assert_eq!(info.weekday("Monday"), Some(0));
        assert_eq!(info.weekday("fri"), Some(4));
        assert_eq!(info.weekday("Frid"), Some(4)); // prefix match
        assert_eq!(info.weekday("xyz"), None);
    }

    #[test]
    fn test_parserinfo_default_jump() {
        let info = ParserInfo::default();
        assert!(info.jump("at"));
        assert!(info.jump("on"));
        assert!(info.jump(","));
        assert!(!info.jump("foo"));
    }

    #[test]
    fn test_parserinfo_default_utczone() {
        let info = ParserInfo::default();
        assert!(info.utczone("UTC"));
        assert!(info.utczone("gmt"));
        assert!(info.utczone("Z"));
        assert!(!info.utczone("EST"));
    }

    #[test]
    fn test_parserinfo_tzoffset() {
        let mut info = ParserInfo::default();
        info.tzoffset.insert("est".into(), -18000);
        info.tzoffset.insert("cst".into(), -21600);

        // Case-insensitive lookup
        assert_eq!(info.tzoffset("EST"), Some(-18000));
        assert_eq!(info.tzoffset("est"), Some(-18000));
        assert_eq!(info.tzoffset("Est"), Some(-18000));
        assert_eq!(info.tzoffset("CST"), Some(-21600));
        assert_eq!(info.tzoffset("UTC"), Some(0)); // utczone fallback
        assert_eq!(info.tzoffset("XYZ"), None);
    }

    #[test]
    fn test_parserinfo_custom_months() {
        let mut info = ParserInfo::default();
        // Add German month names
        info.months.insert("januar".into(), 1);
        info.months.insert("februar".into(), 2);
        info.months.insert("maerz".into(), 3);

        assert_eq!(info.month("Januar"), Some(1));
        assert_eq!(info.month("FEBRUAR"), Some(2));
        assert_eq!(info.month("maerz"), Some(3));
        // English still works
        assert_eq!(info.month("January"), Some(1));
    }

    #[test]
    fn test_dispatch_with_none_uses_phf() {
        assert_eq!(do_month("January", None), Some(1));
        assert_eq!(do_weekday("Monday", None), Some(0));
        assert!(do_jump("at", None));
        assert!(do_utczone("UTC", None));
        assert_eq!(do_hms("hour", None), Some(0));
        assert_eq!(do_ampm("AM", None), Some(0));
        assert!(do_pertain("of", None));
        assert_eq!(do_tzoffset("EST", None), None);
    }

    #[test]
    fn test_dispatch_with_info_uses_custom() {
        let mut info = ParserInfo::default();
        info.tzoffset.insert("est".into(), -18000);

        assert_eq!(do_tzoffset("EST", Some(&info)), Some(-18000));
        assert_eq!(do_tzoffset("est", Some(&info)), Some(-18000));
        assert_eq!(do_month("January", Some(&info)), Some(1));
    }

    #[test]
    fn test_parserinfo_hms() {
        let info = ParserInfo::default();
        assert_eq!(info.hms("hour"), Some(0));
        assert_eq!(info.hms("HOURS"), Some(0));
        assert_eq!(info.hms("minute"), Some(1));
        assert_eq!(info.hms("s"), Some(2));
        assert_eq!(info.hms("xyz"), None);
    }

    #[test]
    fn test_parserinfo_ampm() {
        let info = ParserInfo::default();
        assert_eq!(info.ampm("am"), Some(0));
        assert_eq!(info.ampm("AM"), Some(0));
        assert_eq!(info.ampm("p"), Some(1));
        assert_eq!(info.ampm("PM"), Some(1));
        assert_eq!(info.ampm("xyz"), None);
    }

    #[test]
    fn test_parserinfo_pertain() {
        let info = ParserInfo::default();
        assert!(info.pertain("of"));
        assert!(info.pertain("OF"));
        assert!(!info.pertain("xyz"));
    }

    #[test]
    fn test_dispatch_with_some_info() {
        let info = ParserInfo::default();
        assert!(do_jump(",", Some(&info)));
        assert_eq!(do_weekday("Monday", Some(&info)), Some(0));
        assert_eq!(do_month("January", Some(&info)), Some(1));
        assert_eq!(do_hms("hour", Some(&info)), Some(0));
        assert_eq!(do_ampm("AM", Some(&info)), Some(0));
        assert!(do_pertain("of", Some(&info)));
        assert!(do_utczone("UTC", Some(&info)));
    }
}
