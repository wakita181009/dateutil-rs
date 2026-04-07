pub mod isoparser;

pub use isoparser::{IsoDateTime, IsoParser};

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Timelike};
use std::collections::{HashMap, VecDeque};

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ParserError {
    UnknownFormat(String),
    NoDate(String),
    ValueError(String),
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownFormat(s) => write!(f, "Unknown string format: {}", s),
            Self::NoDate(s) => write!(f, "String does not contain a date: {}", s),
            Self::ValueError(s) => write!(f, "{}", s),
        }
    }
}
impl std::error::Error for ParserError {}

// ---------------------------------------------------------------------------
// Tokenizer — port of _timelex
// ---------------------------------------------------------------------------

/// Split a date/time string into lexical tokens.
/// Port of `dateutil.parser._parser._timelex.split()`.
pub fn tokenize(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut tokens: Vec<String> = Vec::new();
    let mut charstack: VecDeque<char> = VecDeque::new();
    let mut tokenstack: VecDeque<String> = VecDeque::new();
    let mut pos: usize = 0;

    loop {
        // Drain token stack first
        if let Some(tok) = tokenstack.pop_front() {
            tokens.push(tok);
            continue;
        }

        let mut token = String::new();
        #[derive(PartialEq, Clone, Copy)]
        enum State {
            Init,
            Alpha,
            Num,
            AlphaDot,
            NumDot,
        }
        let mut state = State::Init;
        let mut seen_letters = false;
        let mut eof = false;

        loop {
            let nextchar = if let Some(ch) = charstack.pop_front() {
                ch
            } else if pos < len {
                let ch = chars[pos];
                pos += 1;
                if ch == '\0' {
                    continue;
                }
                ch
            } else {
                eof = true;
                break;
            };

            match state {
                State::Init => {
                    token.push(nextchar);
                    if nextchar.is_alphabetic() {
                        state = State::Alpha;
                    } else if nextchar.is_ascii_digit() {
                        state = State::Num;
                    } else if nextchar.is_whitespace() {
                        token = " ".into();
                        break;
                    } else {
                        break; // single char token
                    }
                }
                State::Alpha => {
                    seen_letters = true;
                    if nextchar.is_alphabetic() {
                        token.push(nextchar);
                    } else if nextchar == '.' {
                        token.push(nextchar);
                        state = State::AlphaDot;
                    } else {
                        charstack.push_back(nextchar);
                        break;
                    }
                }
                State::Num => {
                    if nextchar.is_ascii_digit() {
                        token.push(nextchar);
                    } else if nextchar == '.' || (nextchar == ',' && token.len() >= 2) {
                        token.push(nextchar);
                        state = State::NumDot;
                    } else {
                        charstack.push_back(nextchar);
                        break;
                    }
                }
                State::AlphaDot => {
                    seen_letters = true;
                    if nextchar == '.' || nextchar.is_alphabetic() {
                        token.push(nextchar);
                    } else if nextchar.is_ascii_digit() && token.ends_with('.') {
                        token.push(nextchar);
                        state = State::NumDot;
                    } else {
                        charstack.push_back(nextchar);
                        break;
                    }
                }
                State::NumDot => {
                    if nextchar == '.' || nextchar.is_ascii_digit() {
                        token.push(nextchar);
                    } else if nextchar.is_alphabetic() && token.ends_with('.') {
                        token.push(nextchar);
                        state = State::AlphaDot;
                    } else {
                        charstack.push_back(nextchar);
                        break;
                    }
                }
            }
        }

        // Post-process: split composite dot-separated tokens
        if matches!(state, State::AlphaDot | State::NumDot)
            && (seen_letters
                || token.matches('.').count() > 1
                || token.ends_with('.')
                || token.ends_with(','))
        {
            let parts = split_on_decimal(&token);
            token = parts[0].clone();
            for p in &parts[1..] {
                if !p.is_empty() {
                    tokenstack.push_back(p.clone());
                }
            }
        }

        // Replace comma with dot in pure-numeric decimals
        if state == State::NumDot && !token.contains('.') {
            token = token.replace(',', ".");
        }

        if token.is_empty() && eof && tokenstack.is_empty() {
            break;
        }
        if !token.is_empty() {
            tokens.push(token);
        }
        if eof && tokenstack.is_empty() {
            break;
        }
    }

    tokens
}

/// Equivalent of `re.compile("([.,])").split(s)`.
fn split_on_decimal(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    for ch in s.chars() {
        if ch == '.' || ch == ',' {
            result.push(std::mem::take(&mut current));
            result.push(ch.to_string());
        } else {
            current.push(ch);
        }
    }
    result.push(current);
    result
}

// ---------------------------------------------------------------------------
// ParserInfo — port of dateutil.parser.parserinfo
// ---------------------------------------------------------------------------

/// Lookup tables for the date/time parser.
pub struct ParserInfo {
    pub dayfirst: bool,
    pub yearfirst: bool,
    jump: HashMap<String, usize>,
    weekdays: HashMap<String, usize>,
    months: HashMap<String, usize>,
    hms: HashMap<String, usize>,
    ampm: HashMap<String, usize>,
    utczone: HashMap<String, usize>,
    pertain: HashMap<String, usize>,
    pub tzoffset_map: HashMap<String, i32>,
    year: i32,
    century: i32,
}

impl Default for ParserInfo {
    fn default() -> Self {
        Self::new(false, false)
    }
}

impl ParserInfo {
    pub fn new(dayfirst: bool, yearfirst: bool) -> Self {
        let now_year = chrono::Local::now().year();

        Self {
            dayfirst,
            yearfirst,
            jump: Self::convert_list(&[
                &[" "],
                &["."],
                &[","],
                &[";"],
                &["-"],
                &["/"],
                &["'"],
                &["at"],
                &["on"],
                &["and"],
                &["ad"],
                &["m"],
                &["t"],
                &["of"],
                &["st"],
                &["nd"],
                &["rd"],
                &["th"],
            ]),
            weekdays: Self::convert_list(&[
                &["Mon", "Monday"],
                &["Tue", "Tuesday"],
                &["Wed", "Wednesday"],
                &["Thu", "Thursday"],
                &["Fri", "Friday"],
                &["Sat", "Saturday"],
                &["Sun", "Sunday"],
            ]),
            months: Self::convert_list(&[
                &["Jan", "January"],
                &["Feb", "February"],
                &["Mar", "March"],
                &["Apr", "April"],
                &["May"],
                &["Jun", "June"],
                &["Jul", "July"],
                &["Aug", "August"],
                &["Sep", "Sept", "September"],
                &["Oct", "October"],
                &["Nov", "November"],
                &["Dec", "December"],
            ]),
            hms: Self::convert_list(&[
                &["h", "hour", "hours"],
                &["m", "minute", "minutes"],
                &["s", "second", "seconds"],
            ]),
            ampm: Self::convert_list(&[&["am", "a"], &["pm", "p"]]),
            utczone: Self::convert_list(&[&["UTC"], &["GMT"], &["Z"], &["z"]]),
            pertain: Self::convert_list(&[&["of"]]),
            tzoffset_map: HashMap::new(),
            year: now_year,
            century: now_year / 100 * 100,
        }
    }

    /// Build a `ParserInfo` from pre-built lookup tables (received from Python).
    #[allow(clippy::too_many_arguments)]
    pub fn from_config(
        dayfirst: bool,
        yearfirst: bool,
        jump: HashMap<String, usize>,
        weekdays: HashMap<String, usize>,
        months: HashMap<String, usize>,
        hms: HashMap<String, usize>,
        ampm: HashMap<String, usize>,
        utczone: HashMap<String, usize>,
        pertain: HashMap<String, usize>,
        tzoffset_map: HashMap<String, i32>,
    ) -> Self {
        let now_year = chrono::Local::now().year();
        Self {
            dayfirst,
            yearfirst,
            jump,
            weekdays,
            months,
            hms,
            ampm,
            utczone,
            pertain,
            tzoffset_map,
            year: now_year,
            century: now_year / 100 * 100,
        }
    }

    fn convert_list(lst: &[&[&str]]) -> HashMap<String, usize> {
        let mut m = HashMap::new();
        for (i, variants) in lst.iter().enumerate() {
            for v in *variants {
                m.insert(v.to_lowercase(), i);
            }
        }
        m
    }

    pub fn jump(&self, name: &str) -> bool {
        self.jump.contains_key(&name.to_lowercase())
    }

    pub fn weekday(&self, name: &str) -> Option<usize> {
        let lower = name.to_lowercase();
        if let Some(&v) = self.weekdays.get(&lower) {
            return Some(v);
        }
        // Prefix match for 4+ letter abbreviations: "Frid" → "fri" matches Friday.
        // Safe: all 3-letter weekday abbreviations (mon–sun) are unique prefixes.
        if lower.len() >= 4 {
            for (key, &val) in &self.weekdays {
                if key.len() == 3 && lower.starts_with(key) {
                    return Some(val);
                }
            }
        }
        None
    }

    /// Returns 1-based month number.
    pub fn month(&self, name: &str) -> Option<usize> {
        self.months.get(&name.to_lowercase()).map(|v| v + 1)
    }

    pub fn hms(&self, name: &str) -> Option<usize> {
        self.hms.get(&name.to_lowercase()).copied()
    }

    pub fn ampm(&self, name: &str) -> Option<usize> {
        self.ampm.get(&name.to_lowercase()).copied()
    }

    pub fn pertain(&self, name: &str) -> bool {
        self.pertain.contains_key(&name.to_lowercase())
    }

    pub fn utczone(&self, name: &str) -> bool {
        self.utczone.contains_key(&name.to_lowercase())
    }

    pub fn tzoffset(&self, name: &str) -> Option<i32> {
        if self.utczone(name) {
            return Some(0);
        }
        self.tzoffset_map.get(name).copied()
    }

    pub fn convertyear(&self, year: i32, century_specified: bool) -> i32 {
        debug_assert!(year >= 0);
        if year < 100 && !century_specified {
            let mut y = year + self.century;
            if y >= self.year + 50 {
                y -= 100;
            } else if y < self.year - 50 {
                y += 100;
            }
            y
        } else {
            year
        }
    }

    fn validate(&self, res: &mut ParseResult) -> bool {
        if let Some(y) = res.year {
            res.year = Some(self.convertyear(y, res.century_specified));
        }
        if (res.tzoffset == Some(0) && res.tzname.is_none())
            || res.tzname.as_deref() == Some("Z")
            || res.tzname.as_deref() == Some("z")
        {
            res.tzname = Some("UTC".into());
            res.tzoffset = Some(0);
        } else if res.tzoffset.is_some()
            && res.tzoffset != Some(0)
            && res.tzname.is_some()
            && self.utczone(res.tzname.as_deref().unwrap_or(""))
        {
            res.tzoffset = Some(0);
        }
        true
    }
}

// ---------------------------------------------------------------------------
// ParseResult
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct ParseResult {
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub day: Option<u32>,
    pub weekday: Option<usize>,
    pub hour: Option<u32>,
    pub minute: Option<u32>,
    pub second: Option<u32>,
    pub microsecond: Option<u32>,
    pub ampm: Option<usize>,
    pub tzname: Option<String>,
    pub tzoffset: Option<i32>,
    pub century_specified: bool,
}

impl ParseResult {
    fn len(&self) -> usize {
        let mut n = 0;
        if self.year.is_some() {
            n += 1;
        }
        if self.month.is_some() {
            n += 1;
        }
        if self.day.is_some() {
            n += 1;
        }
        if self.weekday.is_some() {
            n += 1;
        }
        if self.hour.is_some() {
            n += 1;
        }
        if self.minute.is_some() {
            n += 1;
        }
        if self.second.is_some() {
            n += 1;
        }
        if self.microsecond.is_some() {
            n += 1;
        }
        if self.ampm.is_some() {
            n += 1;
        }
        if self.tzname.is_some() {
            n += 1;
        }
        if self.tzoffset.is_some() {
            n += 1;
        }
        n
    }
}

// ---------------------------------------------------------------------------
// YMD resolver
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct Ymd {
    values: Vec<i32>,
    century_specified: bool,
    ystridx: Option<usize>,
    mstridx: Option<usize>,
    dstridx: Option<usize>,
}

impl Ymd {
    fn len(&self) -> usize {
        self.values.len()
    }

    fn could_be_day(&self, value: i32) -> bool {
        if self.dstridx.is_some() {
            return false;
        }
        if let Some(mi) = self.mstridx {
            if let Some(yi) = self.ystridx {
                let month = self.values[mi] as u32;
                let year = self.values[yi];
                let max = days_in_month(year, month);
                (1..=max as i32).contains(&value)
            } else {
                let month = self.values[mi] as u32;
                let max = days_in_month(2000, month); // permissive: assume leap year
                (1..=max as i32).contains(&value)
            }
        } else {
            (1..=31).contains(&value)
        }
    }

    fn append_val(&mut self, val: i32, label: Option<char>) -> Result<(), String> {
        let mut lbl = label;
        if val > 100 {
            self.century_specified = true;
            if lbl.is_some() && lbl != Some('Y') {
                return Err("Value > 100 must be year".into());
            }
            lbl = Some('Y');
        }
        self.values.push(val);
        match lbl {
            Some('M') => {
                if self.mstridx.is_some() {
                    return Err("Month is already set".into());
                }
                self.mstridx = Some(self.values.len() - 1);
            }
            Some('D') => {
                if self.dstridx.is_some() {
                    return Err("Day is already set".into());
                }
                self.dstridx = Some(self.values.len() - 1);
            }
            Some('Y') => {
                if self.ystridx.is_some() {
                    return Err("Year is already set".into());
                }
                self.ystridx = Some(self.values.len() - 1);
            }
            _ => {}
        }
        Ok(())
    }

    fn append_str(&mut self, s: &str, label: Option<char>) -> Result<(), String> {
        let mut lbl = label;
        if s.chars().all(|c| c.is_ascii_digit()) && s.len() > 2 {
            self.century_specified = true;
            if lbl.is_some() && lbl != Some('Y') {
                return Err("Long digit string must be year".into());
            }
            lbl = Some('Y');
        }
        let val: i32 = s.parse().map_err(|_| format!("Not a number: {}", s))?;
        if val > 100 && lbl.is_none() {
            self.century_specified = true;
            lbl = Some('Y');
        }
        self.append_val(val, lbl)
    }

    fn resolve_ymd(
        &self,
        yearfirst: bool,
        dayfirst: bool,
    ) -> (Option<i32>, Option<u32>, Option<u32>) {
        let n = self.values.len();
        if n == 0 {
            return (None, None, None);
        }

        // Collect known string indices
        let mut strids: HashMap<char, usize> = HashMap::new();
        if let Some(i) = self.ystridx {
            strids.insert('y', i);
        }
        if let Some(i) = self.mstridx {
            strids.insert('m', i);
        }
        if let Some(i) = self.dstridx {
            strids.insert('d', i);
        }

        // If we have enough resolved indices, use them
        if (n == strids.len() && n > 0) || (n == 3 && strids.len() == 2) {
            return self.resolve_from_stridxs(&strids);
        }

        let mstridx = self.mstridx;

        if n > 3 {
            return (None, None, None); // too many
        }

        if n == 1 || (mstridx.is_some() && n == 2) {
            let (month, other) = if let Some(mi) = mstridx {
                let other_idx = if mi == 0 { 1 } else { 0 };
                (Some(self.values[mi] as u32), if n > 1 { Some(self.values[other_idx]) } else { None })
            } else {
                (None, Some(self.values[0]))
            };

            if let Some(o) = other {
                if n > 1 || mstridx.is_none() {
                    if o > 31 {
                        return (Some(o), month, None);
                    } else {
                        return (None, month, Some(o as u32));
                    }
                }
            }
            return (None, month, None);
        }

        if n == 2 {
            let (a, b) = (self.values[0], self.values[1]);
            if a > 31 {
                return (Some(a), Some(b as u32), None);
            } else if b > 31 {
                return (Some(b), Some(a as u32), None);
            } else if dayfirst && b <= 12 {
                return (None, Some(b as u32), Some(a as u32));
            } else {
                return (None, Some(a as u32), Some(b as u32));
            }
        }

        // n == 3
        let (a, b, c) = (self.values[0], self.values[1], self.values[2]);

        if let Some(mi) = mstridx {
            if mi == 0 {
                if b > 31 {
                    return (Some(b), Some(a as u32), Some(c as u32));
                } else {
                    return (Some(c), Some(a as u32), Some(b as u32));
                }
            } else if mi == 1 {
                if a > 31 || (yearfirst && c <= 31) {
                    return (Some(a), Some(b as u32), Some(c as u32));
                } else {
                    return (Some(c), Some(b as u32), Some(a as u32));
                }
            } else if mi == 2 {
                if b > 31 {
                    return (Some(b), Some(c as u32), Some(a as u32));
                } else {
                    return (Some(a), Some(c as u32), Some(b as u32));
                }
            }
        }

        // No month string index
        if a > 31 || self.ystridx == Some(0) || (yearfirst && b <= 12 && c <= 31) {
            if dayfirst && c <= 12 {
                (Some(a), Some(c as u32), Some(b as u32))
            } else {
                (Some(a), Some(b as u32), Some(c as u32))
            }
        } else if a > 12 || (dayfirst && b <= 12) {
            (Some(c), Some(b as u32), Some(a as u32))
        } else {
            (Some(c), Some(a as u32), Some(b as u32))
        }
    }

    fn resolve_from_stridxs(
        &self,
        strids: &HashMap<char, usize>,
    ) -> (Option<i32>, Option<u32>, Option<u32>) {
        let mut ids = strids.clone();
        if self.values.len() == 3 && ids.len() == 2 {
            let used: Vec<usize> = ids.values().copied().collect();
            let missing_idx = (0..3).find(|i| !used.contains(i)).unwrap();
            let missing_key = ['y', 'm', 'd']
                .iter()
                .find(|k| !ids.contains_key(k))
                .unwrap();
            ids.insert(*missing_key, missing_idx);
        }
        let y = ids.get(&'y').map(|&i| self.values[i]);
        let m = ids.get(&'m').map(|&i| self.values[i] as u32);
        let d = ids.get(&'d').map(|&i| self.values[i] as u32);
        (y, m, d)
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Parser {
    info: ParserInfo,
}

impl Parser {
    pub fn new(info: ParserInfo) -> Self {
        Self { info }
    }

    /// Parse a date/time string and build a `NaiveDateTime`.
    /// Returns `(datetime, weekday, tzname, tzoffset, skipped_tokens)`.
    pub fn parse(
        &self,
        timestr: &str,
        default: Option<NaiveDateTime>,
        dayfirst: Option<bool>,
        yearfirst: Option<bool>,
        fuzzy: bool,
        fuzzy_with_tokens: bool,
    ) -> Result<ParseOutput, ParserError> {
        let default = default.unwrap_or_else(|| {
            let now = chrono::Local::now().naive_local();
            NaiveDateTime::new(
                now.date(),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            )
        });

        let (res, skipped) = self._parse(
            timestr,
            dayfirst,
            yearfirst,
            fuzzy || fuzzy_with_tokens,
            fuzzy_with_tokens,
        )?;

        let naive = self.build_naive(&res, &default)?;

        Ok(ParseOutput {
            naive,
            weekday: res.weekday,
            tzname: res.tzname,
            tzoffset: res.tzoffset,
            skipped_tokens: skipped,
        })
    }

    fn _parse(
        &self,
        timestr: &str,
        dayfirst: Option<bool>,
        yearfirst: Option<bool>,
        fuzzy: bool,
        fuzzy_with_tokens: bool,
    ) -> Result<(ParseResult, Option<Vec<String>>), ParserError> {
        let info = &self.info;
        let dayfirst = dayfirst.unwrap_or(info.dayfirst);
        let yearfirst = yearfirst.unwrap_or(info.yearfirst);

        let mut res = ParseResult::default();
        let mut tokens = tokenize(timestr);
        let len_l = tokens.len();

        let mut ymd = Ymd::default();
        let mut skipped_idxs: Vec<usize> = Vec::new();
        let mut i: usize = 0;

        let parse_result = (|| -> Result<(), ()> {
            while i < len_l {
                let token = tokens[i].clone();
                let value_f: Option<f64> = token.parse().ok();

                if let Some(_vf) = value_f {
                    // Numeric token
                    i = self
                        .parse_numeric_token(&tokens, i, &mut ymd, &mut res, fuzzy)
                        .map_err(|_| ())?;
                } else if info.weekday(&token).is_some() {
                    res.weekday = info.weekday(&token);
                } else if info.month(&token).is_some() {
                    let month_val = info.month(&token).unwrap();
                    ymd.append_val(month_val as i32, Some('M'))
                        .map_err(|_| ())?;

                    if i + 1 < len_l {
                        if tokens[i + 1] == "-" || tokens[i + 1] == "/" {
                            // Jan-01[-99]
                            // Python accesses tokens[i+2] unconditionally here;
                            // IndexError is caught by the outer try/except.
                            if i + 2 >= len_l {
                                return Err(());
                            }
                            let sep = tokens[i + 1].clone();
                            ymd.append_str(&tokens[i + 2], None).map_err(|_| ())?;

                            if i + 3 < len_l && tokens[i + 3] == sep {
                                if i + 4 >= len_l {
                                    return Err(());
                                }
                                ymd.append_str(&tokens[i + 4], None).map_err(|_| ())?;
                                i += 2;
                            }
                            i += 2;
                        } else if i + 4 < len_l
                            && tokens[i + 1] == " "
                            && info.pertain(&tokens[i + 2])
                            && tokens[i + 3] == " "
                        {
                            // Jan of 01
                            if tokens[i + 4].chars().all(|c| c.is_ascii_digit()) {
                                let v: i32 = tokens[i + 4].parse().unwrap_or(0);
                                let year = info.convertyear(v, false);
                                ymd.append_val(year, Some('Y')).map_err(|_| ())?;
                            }
                            i += 4;
                        }
                    }
                } else if let Some(skip) = era_pattern_len(&tokens, i) {
                    // Era marker (AD, A.D., BC, B.C.) — skip
                    i += skip - 1; // -1 because the loop adds 1
                } else if info.ampm(&token).is_some() {
                    let ampm_val = info.ampm(&token).unwrap();
                    let valid = ampm_valid(res.hour, res.ampm, fuzzy)?;

                    if valid {
                        res.hour = Some(adjust_ampm(
                            res.hour.unwrap_or(0),
                            ampm_val,
                        ));
                        res.ampm = Some(ampm_val);
                    } else if fuzzy {
                        skipped_idxs.push(i);
                    }
                } else if could_be_tzname(res.hour, &res.tzname, &res.tzoffset, &token, info)
                {
                    res.tzname = Some(token.clone());
                    res.tzoffset = info.tzoffset(&token);

                    // Handle GMT+3 pattern: "my time +3 is GMT" → reverse sign.
                    // Python mutates l[i+1] in place to flip the sign.
                    if i + 1 < len_l && (tokens[i + 1] == "+" || tokens[i + 1] == "-") {
                        tokens[i + 1] = if tokens[i + 1] == "+" {
                            "-".into()
                        } else {
                            "+".into()
                        };
                        res.tzoffset = None;
                        if info.utczone(&token) {
                            res.tzname = None;
                        }
                    }
                } else if res.hour.is_some() && (token == "+" || token == "-") {
                    let signal: i32 = if token == "+" { 1 } else { -1 };
                    let len_next = tokens.get(i + 1).map(|t| t.len()).unwrap_or(0);

                    let (hour_offset, min_offset, extra_skip) =
                        if len_next == 4 {
                            // -0300
                            let t = &tokens[i + 1];
                            (
                                t[..2].parse::<i32>().unwrap_or(0),
                                t[2..].parse::<i32>().unwrap_or(0),
                                0usize,
                            )
                        } else if i + 2 < len_l && tokens[i + 2] == ":" {
                            // -03:00
                            (
                                tokens[i + 1].parse::<i32>().unwrap_or(0),
                                tokens.get(i + 3).and_then(|t| t.parse::<i32>().ok()).unwrap_or(0),
                                2,
                            )
                        } else if len_next <= 2 {
                            // -03
                            let t = &tokens[i + 1];
                            (t[..len_next.min(2)].parse::<i32>().unwrap_or(0), 0, 0)
                        } else {
                            return Err(());
                        };

                    res.tzoffset = Some(signal * (hour_offset * 3600 + min_offset * 60));

                    // Look for timezone name in parentheses: -0300 (BRST)
                    if i + 5 + extra_skip < len_l
                        && info.jump(&tokens[i + 2 + extra_skip])
                        && tokens[i + 3 + extra_skip] == "("
                        && tokens[i + 5 + extra_skip] == ")"
                        && tokens[i + 4 + extra_skip].len() >= 3
                        && could_be_tzname(
                            res.hour,
                            &res.tzname,
                            &None,
                            &tokens[i + 4 + extra_skip],
                            info,
                        )
                    {
                        res.tzname = Some(tokens[i + 4 + extra_skip].clone());
                        i += 4 + extra_skip;
                    }

                    i += 1 + extra_skip;
                } else if !(info.jump(&token) || fuzzy) {
                    return Err(());
                } else {
                    skipped_idxs.push(i);
                }

                i += 1;
            }

            // Resolve year/month/day
            let (year, month, day) = ymd.resolve_ymd(yearfirst, dayfirst);
            res.century_specified = ymd.century_specified;
            res.year = year;
            res.month = month;
            res.day = day;
            Ok(())
        })();

        if parse_result.is_err() {
            return Err(ParserError::UnknownFormat(timestr.into()));
        }

        if !info.validate(&mut res) {
            return Err(ParserError::UnknownFormat(timestr.into()));
        }

        if res.len() == 0 {
            return Err(ParserError::NoDate(timestr.into()));
        }

        let skipped = if fuzzy_with_tokens {
            Some(recombine_skipped(&tokens, &skipped_idxs))
        } else {
            None
        };

        Ok((res, skipped))
    }

    fn parse_numeric_token(
        &self,
        tokens: &[String],
        idx: usize,
        ymd: &mut Ymd,
        res: &mut ParseResult,
        fuzzy: bool,
    ) -> Result<usize, ()> {
        let info = &self.info;
        let value_repr = &tokens[idx];
        let value_f: f64 = value_repr.parse().map_err(|_| ())?;
        let len_li = value_repr.len();
        let len_l = tokens.len();
        let mut idx = idx;

        if ymd.len() == 3
            && (len_li == 2 || len_li == 4)
            && res.hour.is_none()
            && (idx + 1 >= len_l
                || (tokens[idx + 1] != ":" && info.hms(&tokens[idx + 1]).is_none()))
        {
            // 19990101T23[59]
            let s = &tokens[idx];
            res.hour = Some(s[..2].parse::<u32>().unwrap_or(0));
            if len_li == 4 {
                res.minute = Some(s[2..].parse::<u32>().unwrap_or(0));
            }
        } else if len_li == 6 || (len_li > 6 && tokens[idx].find('.') == Some(6)) {
            // YYMMDD or HHMMSS[.ss], with YYYYMM fallback
            let s = &tokens[idx];
            if ymd.len() == 0 && !s.contains('.') {
                // Try YYMMDD first; if month/day would be invalid, fall back to YYYYMM
                let mm: u32 = s[2..4].parse().unwrap_or(0);
                let dd: u32 = s[4..6].parse().unwrap_or(0);
                if (1..=12).contains(&mm) && (1..=31).contains(&dd) {
                    // Valid YYMMDD
                    ymd.append_str(&s[..2], None).map_err(|_| ())?;
                    ymd.append_str(&s[2..4], None).map_err(|_| ())?;
                    ymd.append_str(&s[4..], None).map_err(|_| ())?;
                } else {
                    // Fallback: try YYYYMM (e.g. "201712" → 2017-12)
                    let mm2: u32 = s[4..6].parse().unwrap_or(0);
                    if (1..=12).contains(&mm2) {
                        ymd.append_str(&s[..4], Some('Y')).map_err(|_| ())?;
                        ymd.append_str(&s[4..6], None).map_err(|_| ())?;
                    } else {
                        return Err(());
                    }
                }
            } else {
                // Python uses int() which raises ValueError on non-digit slices.
                res.hour = Some(s[..2].parse::<u32>().map_err(|_| ())?);
                res.minute = Some(s[2..4].parse::<u32>().map_err(|_| ())?);
                let (sec, us) = parsems(&s[4..]);
                res.second = Some(sec);
                res.microsecond = Some(us);
            }
        } else if len_li == 10 && tokens[idx].chars().all(|c| c.is_ascii_digit()) {
            // YYYYMMDDHH — e.g. "1991041310"
            let s = &tokens[idx];
            ymd.append_str(&s[..4], Some('Y')).map_err(|_| ())?;
            ymd.append_str(&s[4..6], None).map_err(|_| ())?;
            ymd.append_str(&s[6..8], None).map_err(|_| ())?;
            res.hour = Some(s[8..10].parse::<u32>().unwrap_or(0));
            // Check for trailing :MM[:SS]
            if idx + 2 < len_l && tokens[idx + 1] == ":" {
                res.minute = Some(tokens[idx + 2].parse::<u32>().unwrap_or(0));
                idx += 2;
                if idx + 2 < len_l && tokens[idx + 1] == ":" {
                    let (s, us) = parsems(&tokens[idx + 2]);
                    res.second = Some(s);
                    res.microsecond = Some(us);
                    idx += 2;
                }
            }
        } else if matches!(len_li, 8 | 12 | 14) {
            // YYYYMMDD[HHMM[SS]]
            let s = &tokens[idx];
            ymd.append_str(&s[..4], Some('Y')).map_err(|_| ())?;
            ymd.append_str(&s[4..6], None).map_err(|_| ())?;
            ymd.append_str(&s[6..8], None).map_err(|_| ())?;
            if len_li > 8 {
                res.hour = Some(s[8..10].parse::<u32>().unwrap_or(0));
                res.minute = Some(s[10..12].parse::<u32>().unwrap_or(0));
                if len_li > 12 {
                    res.second = Some(s[12..].parse::<u32>().unwrap_or(0));
                }
            }
        } else if find_hms_idx(idx, tokens, info, true).is_some() {
            // HH[ ]h or MM[ ]m or SS[.ss][ ]s
            let hms_idx = find_hms_idx(idx, tokens, info, true).unwrap();
            let (new_idx, hms) = parse_hms(idx, tokens, info, Some(hms_idx));
            idx = new_idx;
            if let Some(hms) = hms {
                assign_hms(res, value_repr, hms);
            }
        } else if idx + 2 < len_l && tokens[idx + 1] == ":" {
            // HH:MM[:SS[.ss]]
            res.hour = Some(value_f as u32);
            let (min, sec) = parse_min_sec(&tokens[idx + 2]);
            res.minute = Some(min);
            if sec.is_some() {
                res.second = sec;
            }

            if idx + 4 < len_l && tokens[idx + 3] == ":" {
                let (s, us) = parsems(&tokens[idx + 4]);
                res.second = Some(s);
                res.microsecond = Some(us);
                idx += 2;
            }
            idx += 2;
        } else if idx + 1 < len_l
            && (tokens[idx + 1] == "-" || tokens[idx + 1] == "/" || tokens[idx + 1] == ".")
        {
            let sep = tokens[idx + 1].clone();
            ymd.append_str(value_repr, None).map_err(|_| ())?;

            if idx + 2 < len_l && !info.jump(&tokens[idx + 2]) {
                if tokens[idx + 2].chars().all(|c| c.is_ascii_digit()) {
                    ymd.append_str(&tokens[idx + 2], None).map_err(|_| ())?;
                } else {
                    let m = info.month(&tokens[idx + 2]);
                    if let Some(m) = m {
                        ymd.append_val(m as i32, Some('M')).map_err(|_| ())?;
                    } else {
                        return Err(());
                    }
                }

                if idx + 3 < len_l && tokens[idx + 3] == sep {
                    // Python accesses tokens[idx+4] here; IndexError
                    // is caught by the outer try/except.
                    if idx + 4 >= len_l {
                        return Err(());
                    }
                    let m = info.month(&tokens[idx + 4]);
                    if let Some(m) = m {
                        ymd.append_val(m as i32, Some('M')).map_err(|_| ())?;
                    } else {
                        ymd.append_str(&tokens[idx + 4], None).map_err(|_| ())?;
                    }
                    idx += 2;
                }
                idx += 1;
            }
            idx += 1;
        } else if idx + 1 >= len_l || info.jump(&tokens[idx + 1]) {
            // Check if the jump word is an era marker (AD, BC, etc.)
            let is_era_jump = idx + 1 < len_l && is_era(&tokens[idx + 1]);

            if is_era_jump {
                // Year + era suffix: "6AD", "1973 AD", etc.
                ymd.append_val(value_f as i32, Some('Y')).map_err(|_| ())?;
                ymd.century_specified = true;
            } else if idx + 2 < len_l
                && info.ampm(&tokens[idx + 2]).is_some()
                && era_pattern_len(tokens, idx + 2).is_none()
            {
                // "12 am" — but not "1973 A.D."
                let hour = value_f as u32;
                res.hour = Some(adjust_ampm(hour, info.ampm(&tokens[idx + 2]).unwrap()));
                idx += 1;
            } else {
                // Use append_str to preserve string length info (e.g. "0031" → 4 digits → year)
                ymd.append_str(value_repr, None).map_err(|_| ())?;
            }
            idx += 1;
        } else if info.ampm(&tokens[idx + 1]).is_some()
            && era_pattern_len(tokens, idx + 1).is_none()
            && (0.0..24.0).contains(&value_f)
        {
            // "12am" — but not "6A.D."
            let hour = value_f as u32;
            res.hour = Some(adjust_ampm(hour, info.ampm(&tokens[idx + 1]).unwrap()));
            idx += 1;
        } else if ymd.could_be_day(value_f as i32) {
            ymd.append_val(value_f as i32, None).map_err(|_| ())?;
        } else if !fuzzy {
            return Err(());
        }

        Ok(idx)
    }

    fn build_naive(
        &self,
        res: &ParseResult,
        default: &NaiveDateTime,
    ) -> Result<NaiveDateTime, ParserError> {
        let year = res.year.unwrap_or(default.year());
        let month = res.month.unwrap_or(default.month());
        let mut day = res.day.unwrap_or(default.day());
        let hour = res.hour.unwrap_or(default.hour());
        let minute = res.minute.unwrap_or(default.minute());
        let second = res.second.unwrap_or(default.second());
        let microsecond = res.microsecond.unwrap_or(default.nanosecond() / 1000);

        // If day not explicitly set, clamp to month's max
        if res.day.is_none() {
            let max = days_in_month(year, month);
            if day > max {
                day = max;
            }
        }

        let date = NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
            ParserError::ValueError(format!(
                "day is out of range for month: {}-{}-{}",
                year, month, day
            ))
        })?;
        let time = NaiveTime::from_hms_micro_opt(hour, minute, second, microsecond)
            .ok_or_else(|| {
                ParserError::ValueError(format!(
                    "Invalid time: {}:{}:{}.{}",
                    hour, minute, second, microsecond
                ))
            })?;
        let mut naive = NaiveDateTime::new(date, time);

        // Weekday resolution
        if let Some(wd) = res.weekday {
            if res.day.is_none() {
                let target = weekday_from_num(wd);
                // Advance to next matching weekday (or stay if already matching)
                let mut d = naive.date();
                while d.weekday() != target {
                    d += TimeDelta::days(1);
                }
                naive = NaiveDateTime::new(d, naive.time());
            }
        }

        Ok(naive)
    }
}

/// Output from `Parser::parse`.
pub struct ParseOutput {
    pub naive: NaiveDateTime,
    pub weekday: Option<usize>,
    pub tzname: Option<String>,
    pub tzoffset: Option<i32>,
    pub skipped_tokens: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn weekday_from_num(n: usize) -> chrono::Weekday {
    use chrono::Weekday::*;
    match n {
        0 => Mon,
        1 => Tue,
        2 => Wed,
        3 => Thu,
        4 => Fri,
        5 => Sat,
        6 => Sun,
        _ => Mon,
    }
}

fn could_be_tzname(
    hour: Option<u32>,
    tzname: &Option<String>,
    tzoffset: &Option<i32>,
    token: &str,
    _info: &ParserInfo,
) -> bool {
    // Python checks `token in self.info.UTCZONE` (case-sensitive list).
    // The UTCZONE class attribute is ["UTC", "GMT", "Z", "z"].
    hour.is_some()
        && tzname.is_none()
        && tzoffset.is_none()
        && token.len() <= 5
        && (token.chars().all(|c| c.is_ascii_uppercase())
            || matches!(token, "UTC" | "GMT" | "Z" | "z"))
}

/// Returns Ok(true) if valid AM/PM, Ok(false) if should skip/ignore,
/// Err(()) if should raise ValueError (non-fuzzy invalid state).
fn ampm_valid(hour: Option<u32>, ampm: Option<usize>, fuzzy: bool) -> Result<bool, ()> {
    // If there's already an AM/PM flag, this one isn't one.
    if fuzzy && ampm.is_some() {
        return Ok(false);
    }
    match hour {
        None => {
            if fuzzy {
                Ok(false)
            } else {
                Err(()) // Python: ValueError("No hour specified with AM or PM flag.")
            }
        }
        Some(h) if !(0..=12).contains(&h) => {
            if fuzzy {
                Ok(false)
            } else {
                Err(()) // Python: ValueError("Invalid hour specified for 12-hour clock.")
            }
        }
        _ => Ok(true),
    }
}

fn adjust_ampm(hour: u32, ampm: usize) -> u32 {
    if hour < 12 && ampm == 1 {
        hour + 12
    } else if hour == 12 && ampm == 0 {
        0
    } else {
        hour
    }
}

fn find_hms_idx(
    idx: usize,
    tokens: &[String],
    info: &ParserInfo,
    allow_jump: bool,
) -> Option<usize> {
    let len_l = tokens.len();

    if idx + 1 < len_l && info.hms(&tokens[idx + 1]).is_some() {
        return Some(idx + 1);
    }

    if allow_jump
        && idx + 2 < len_l
        && tokens[idx + 1] == " "
        && info.hms(&tokens[idx + 2]).is_some()
    {
        return Some(idx + 2);
    }

    if idx > 0 && info.hms(&tokens[idx - 1]).is_some() {
        return Some(idx - 1);
    }

    if idx > 1
        && idx == len_l - 1
        && tokens[idx - 1] == " "
        && info.hms(&tokens[idx - 2]).is_some()
    {
        return Some(idx - 2);
    }

    None
}

fn parse_hms(
    idx: usize,
    tokens: &[String],
    info: &ParserInfo,
    hms_idx: Option<usize>,
) -> (usize, Option<usize>) {
    match hms_idx {
        None => (idx, None),
        Some(hi) if hi > idx => (hi, info.hms(&tokens[hi])),
        Some(hi) => (idx, info.hms(&tokens[hi]).map(|v| v + 1)),
    }
}

fn assign_hms(res: &mut ParseResult, value_repr: &str, hms: usize) {
    let (int_part, frac_str) = parse_decimal(value_repr);
    match hms {
        0 => {
            // Hour
            res.hour = Some(int_part as u32);
            if !frac_str.is_empty() {
                res.minute = Some(frac_mul(frac_str, 60));
            }
        }
        1 => {
            // Minute
            let (m, s) = parse_min_sec(value_repr);
            res.minute = Some(m);
            if s.is_some() {
                res.second = s;
            }
        }
        2 => {
            // Second
            let (s, us) = parsems(value_repr);
            res.second = Some(s);
            res.microsecond = Some(us);
        }
        _ => {}
    }
}

/// Parse "I[.F]" into (integer_part, fractional_digits_str).
/// Uses string representation to avoid f64 rounding issues
/// (matching Python's Decimal-based `_to_decimal`).
fn parse_decimal(s: &str) -> (i64, &str) {
    if let Some(dot) = s.find('.') {
        let int_part: i64 = s[..dot].parse().unwrap_or(0);
        (int_part, &s[dot + 1..])
    } else {
        (s.parse().unwrap_or(0), "")
    }
}

/// Integer-accurate `int(multiplier * 0.<frac_str>)`, matching Python's
/// `int(Decimal)` truncation semantics.
fn frac_mul(frac_str: &str, multiplier: u32) -> u32 {
    if frac_str.is_empty() {
        return 0;
    }
    let frac_num: u64 = frac_str.parse().unwrap_or(0);
    let divisor = 10u64.pow(frac_str.len() as u32);
    (u64::from(multiplier) * frac_num / divisor) as u32
}

/// Parse a minute value with optional fractional seconds.
fn parse_min_sec(s: &str) -> (u32, Option<u32>) {
    let (int_part, frac_str) = parse_decimal(s);
    let minute = int_part as u32;
    if frac_str.is_empty() {
        (minute, None)
    } else {
        (minute, Some(frac_mul(frac_str, 60)))
    }
}

/// Parse "I[.F]" seconds into (seconds, microseconds).
fn parsems(s: &str) -> (u32, u32) {
    if let Some(dot) = s.find('.') {
        let sec: u32 = s[..dot].parse().unwrap_or(0);
        let frac_str = &s[dot + 1..];
        let n = frac_str.len().min(6);
        let us_raw: u32 = frac_str[..n].parse().unwrap_or(0);
        let us = us_raw * 10u32.pow(6 - n as u32);
        (sec, us)
    } else {
        (s.parse().unwrap_or(0), 0)
    }
}

/// Check if a token is an era marker (AD, BC, CE, BCE).
fn is_era(token: &str) -> bool {
    token.eq_ignore_ascii_case("ad")
        || token.eq_ignore_ascii_case("bc")
        || token.eq_ignore_ascii_case("ce")
        || token.eq_ignore_ascii_case("bce")
}

/// Check if tokens starting at `i` form an era dot-pattern like "A.D." or "B.C.".
/// Returns the number of tokens consumed, or `None` if no era pattern.
fn era_pattern_len(tokens: &[String], i: usize) -> Option<usize> {
    if i >= tokens.len() {
        return None;
    }
    // Single-token era: "AD", "BC", etc.
    if is_era(&tokens[i]) {
        return Some(1);
    }
    // Dot-separated era: "A.D." → ["A", ".", "D", "."] or "B.C." → ["B", ".", "C", "."]
    let t = &tokens[i];
    if t.len() == 1 && i + 2 < tokens.len() && tokens[i + 1] == "." {
        let first = t.as_bytes()[0] | 0x20; // ASCII lowercase
        let next = &tokens[i + 2];
        if next.len() == 1 {
            let second = next.as_bytes()[0] | 0x20;
            if (first == b'a' && second == b'd') || (first == b'b' && second == b'c') {
                if i + 3 < tokens.len() && tokens[i + 3] == "." {
                    return Some(4); // "A.D." or "B.C." with trailing dot
                }
                return Some(3); // "A.D" or "B.C" without trailing dot
            }
        }
    }
    None
}

fn recombine_skipped(tokens: &[String], skipped_idxs: &[usize]) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut sorted = skipped_idxs.to_vec();
    sorted.sort_unstable();
    for (i, &idx) in sorted.iter().enumerate() {
        if i > 0 && idx == sorted[i - 1] + 1 {
            if let Some(last) = result.last_mut() {
                last.push_str(&tokens[idx]);
            }
        } else {
            result.push(tokens[idx].clone());
        }
    }
    result
}

/// Convenience wrapper matching Python's `dateutil.parser.parse()` API.
pub fn parse(
    timestr: &str,
    dayfirst: Option<bool>,
    yearfirst: Option<bool>,
    default: Option<NaiveDateTime>,
    fuzzy: bool,
    fuzzy_with_tokens: bool,
) -> Result<ParseOutput, ParserError> {
    let parser = Parser::default();
    parser.parse(timestr, default, dayfirst, yearfirst, fuzzy, fuzzy_with_tokens)
}

// ---------------------------------------------------------------------------
// PyO3 bindings
// ---------------------------------------------------------------------------

#[cfg(feature = "python")]
pub mod python {
    use super::*;
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;
    use pyo3::types::{PyDateAccess, PyDateTime, PyDict, PyTimeAccess, PyTuple, PyTzInfo};

    pyo3::create_exception!(_native, ParserErrorPy, pyo3::exceptions::PyValueError);

    // ---- helpers: parserinfo config extraction ----

    fn extract_str_usize_map(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<HashMap<String, usize>> {
        let obj = dict
            .get_item(key)?
            .ok_or_else(|| PyValueError::new_err(format!("parserinfo_config missing key: {key}")))?;
        obj.extract::<HashMap<String, usize>>()
    }

    fn extract_str_i32_map(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<HashMap<String, i32>> {
        let obj = dict
            .get_item(key)?
            .ok_or_else(|| PyValueError::new_err(format!("parserinfo_config missing key: {key}")))?;
        obj.extract::<HashMap<String, i32>>()
    }

    fn parser_info_from_py_config(config: &Bound<'_, PyDict>) -> PyResult<ParserInfo> {
        let dayfirst = config
            .get_item("dayfirst")?
            .ok_or_else(|| PyValueError::new_err("parserinfo_config missing key: dayfirst"))?
            .extract::<bool>()?;
        let yearfirst = config
            .get_item("yearfirst")?
            .ok_or_else(|| PyValueError::new_err("parserinfo_config missing key: yearfirst"))?
            .extract::<bool>()?;

        Ok(ParserInfo::from_config(
            dayfirst,
            yearfirst,
            extract_str_usize_map(config, "jump")?,
            extract_str_usize_map(config, "weekdays")?,
            extract_str_usize_map(config, "months")?,
            extract_str_usize_map(config, "hms")?,
            extract_str_usize_map(config, "ampm")?,
            extract_str_usize_map(config, "utczone")?,
            extract_str_usize_map(config, "pertain")?,
            extract_str_i32_map(config, "tzoffset")?,
        ))
    }

    // ---- helpers ----

    fn make_py_tz<'py>(py: Python<'py>, offset_seconds: i32) -> PyResult<Bound<'py, PyTzInfo>> {
        let datetime_mod = py.import("datetime")?;
        let td = datetime_mod
            .getattr("timedelta")?
            .call1((0, offset_seconds))?;
        let tz = datetime_mod.getattr("timezone")?.call1((&td,))?;
        tz.cast_into::<PyTzInfo>()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn make_py_utc<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyTzInfo>> {
        let datetime_mod = py.import("datetime")?;
        let utc = datetime_mod.getattr("timezone")?.getattr("utc")?;
        utc.cast_into::<PyTzInfo>()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn ndt_to_pydt<'py>(
        py: Python<'py>,
        ndt: &NaiveDateTime,
        tz: Option<&Bound<'py, PyTzInfo>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let obj = PyDateTime::new(
            py,
            ndt.year(),
            ndt.month() as u8,
            ndt.day() as u8,
            ndt.hour() as u8,
            ndt.minute() as u8,
            ndt.second() as u8,
            ndt.nanosecond() / 1000,
            tz,
        )?;
        Ok(obj.into_any())
    }

    fn iso_to_pydt<'py>(
        py: Python<'py>,
        res: &IsoDateTime,
    ) -> PyResult<Bound<'py, PyAny>> {
        let tzinfo = match res.tz_offset_seconds {
            None => None,
            Some(0) => Some(make_py_utc(py)?),
            Some(offset) => Some(make_py_tz(py, offset)?),
        };
        let obj = PyDateTime::new(
            py,
            res.year,
            res.month as u8,
            res.day as u8,
            res.hour as u8,
            res.minute as u8,
            res.second as u8,
            res.microsecond,
            tzinfo.as_ref(),
        )?;
        Ok(obj.into_any())
    }

    // ---- parse() ----

    #[pyfunction]
    #[pyo3(name = "parse", signature = (
        timestr,
        *,
        parserinfo_config = None,
        default = None,
        ignoretz = false,
        tzinfos = None,
        dayfirst = None,
        yearfirst = None,
        fuzzy = false,
        fuzzy_with_tokens = false,
    ))]
    pub fn parse_py<'py>(
        py: Python<'py>,
        timestr: &str,
        parserinfo_config: Option<&Bound<'_, PyDict>>,
        default: Option<&Bound<'_, PyDateTime>>,
        ignoretz: bool,
        tzinfos: Option<Bound<'py, pyo3::PyAny>>,
        dayfirst: Option<bool>,
        yearfirst: Option<bool>,
        fuzzy: bool,
        fuzzy_with_tokens: bool,
    ) -> PyResult<Bound<'py, pyo3::PyAny>> {
        // Build default NaiveDateTime
        let default_ndt = if let Some(d) = default {
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(
                    d.get_year(),
                    d.get_month() as u32,
                    d.get_day() as u32,
                )
                .ok_or_else(|| PyValueError::new_err("Invalid default date"))?,
                NaiveTime::from_hms_micro_opt(
                    d.get_hour() as u32,
                    d.get_minute() as u32,
                    d.get_second() as u32,
                    d.get_microsecond(),
                )
                .ok_or_else(|| PyValueError::new_err("Invalid default time"))?,
            )
        } else {
            let now = chrono::Local::now().naive_local();
            NaiveDateTime::new(now.date(), NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        };

        let parser = if let Some(config) = parserinfo_config {
            Parser::new(parser_info_from_py_config(config)?)
        } else {
            Parser::default()
        };
        let output = parser
            .parse(
                timestr,
                Some(default_ndt),
                dayfirst,
                yearfirst,
                fuzzy,
                fuzzy_with_tokens,
            )
            .map_err(|e| ParserErrorPy::new_err(e.to_string()))?;

        // Build Python datetime
        let ndt = &output.naive;

        let dt_obj: Bound<'py, pyo3::PyAny> = if ignoretz || output.tzoffset.is_none() {
            ndt_to_pydt(py, ndt, None)?
        } else if let Some(ref tzinfos_obj) = tzinfos {
            if let Some(ref tzname) = output.tzname {
                // Call tzinfos (callable or mapping)
                let tzdata = if tzinfos_obj.is_callable() {
                    let offset_arg: Bound<'py, pyo3::PyAny> =
                        match output.tzoffset {
                            Some(o) => o.into_pyobject(py)?.into_any(),
                            None => py.None().into_bound(py),
                        };
                    tzinfos_obj.call1((tzname.as_str(), offset_arg))?
                } else {
                    tzinfos_obj.get_item(tzname.as_str())?
                };

                if tzdata.is_none() {
                    ndt_to_pydt(py, ndt, None)?
                } else if let Ok(offset_secs) = tzdata.extract::<i32>() {
                    let tz = make_py_tz(py, offset_secs)?;
                    ndt_to_pydt(py, ndt, Some(&tz))?
                } else {
                    // Assume it's a tzinfo
                    let tz = tzdata.cast::<PyTzInfo>()?;
                    ndt_to_pydt(py, ndt, Some(tz))?
                }
            } else {
                // No tzname but we have tzoffset
                match output.tzoffset {
                    Some(0) => {
                        let tz = make_py_utc(py)?;
                        ndt_to_pydt(py, ndt, Some(&tz))?
                    }
                    Some(offset) => {
                        let tz = make_py_tz(py, offset)?;
                        ndt_to_pydt(py, ndt, Some(&tz))?
                    }
                    None => ndt_to_pydt(py, ndt, None)?,
                }
            }
        } else {
            match output.tzoffset {
                Some(0) => {
                    let tz = make_py_utc(py)?;
                    ndt_to_pydt(py, ndt, Some(&tz))?
                }
                Some(offset) => {
                    let tz = make_py_tz(py, offset)?;
                    ndt_to_pydt(py, ndt, Some(&tz))?
                }
                None => ndt_to_pydt(py, ndt, None)?,
            }
        };

        if fuzzy_with_tokens {
            let skipped = output.skipped_tokens.unwrap_or_default();
            let skipped_tuple =
                PyTuple::new(py, skipped.iter().map(|s| s.as_str()))?;
            Ok(PyTuple::new(py, [&dt_obj, &skipped_tuple.into_any()])?.into_any())
        } else {
            Ok(dt_obj)
        }
    }

    // ---- isoparse() ----

    #[pyfunction]
    #[pyo3(name = "isoparse")]
    pub fn isoparse_py<'py>(
        py: Python<'py>,
        dt_str: &str,
    ) -> PyResult<Bound<'py, pyo3::PyAny>> {
        let parser = IsoParser::default();
        let res = parser
            .isoparse(dt_str)
            .map_err(|e| PyValueError::new_err(e))?;
        iso_to_pydt(py, &res)
    }

    // ---- isoparser class ----

    #[pyclass(name = "isoparser")]
    pub struct IsoParserPy {
        inner: IsoParser,
    }

    #[pymethods]
    impl IsoParserPy {
        #[new]
        #[pyo3(signature = (sep = None))]
        fn new(sep: Option<&str>) -> PyResult<Self> {
            let sep_byte = if let Some(s) = sep {
                if s.len() != 1 || !s.is_ascii() {
                    return Err(PyValueError::new_err(
                        "Separator must be a single, non-numeric ASCII character",
                    ));
                }
                Some(s.as_bytes()[0])
            } else {
                None
            };
            let inner =
                IsoParser::new(sep_byte).map_err(|e| PyValueError::new_err(e))?;
            Ok(Self { inner })
        }

        fn isoparse<'py>(
            &self,
            py: Python<'py>,
            dt_str: &str,
        ) -> PyResult<Bound<'py, pyo3::PyAny>> {
            let res = self
                .inner
                .isoparse(dt_str)
                .map_err(|e| PyValueError::new_err(e))?;
            iso_to_pydt(py, &res)
        }
    }

    pub fn register(m: &Bound<'_, pyo3::types::PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(parse_py, m)?)?;
        m.add_function(wrap_pyfunction!(isoparse_py, m)?)?;
        m.add_class::<IsoParserPy>()?;
        m.add("ParserError", m.py().get_type::<ParserErrorPy>())?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_dt() -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2003, 9, 25).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        )
    }

    fn p(s: &str) -> NaiveDateTime {
        let parser = Parser::default();
        let out = parser
            .parse(s, Some(default_dt()), None, None, false, false)
            .unwrap();
        out.naive
    }

    fn pd(s: &str) -> NaiveDateTime {
        p(s) // with default 2003-09-25
    }

    fn ndt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(y, m, d).unwrap(),
            NaiveTime::from_hms_opt(h, mi, s).unwrap(),
        )
    }

    fn ndt_us(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32, us: u32) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(y, m, d).unwrap(),
            NaiveTime::from_hms_micro_opt(h, mi, s, us).unwrap(),
        )
    }

    // --- tokenizer tests ---

    #[test]
    fn tokenize_iso() {
        let t = tokenize("2003-09-25T10:49:41");
        assert_eq!(
            t,
            vec!["2003", "-", "09", "-", "25", "T", "10", ":", "49", ":", "41"]
        );
    }

    #[test]
    fn tokenize_hms() {
        let t = tokenize("10h36m28.5s");
        assert_eq!(t, vec!["10", "h", "36", "m", "28.5", "s"]);
    }

    #[test]
    fn tokenize_date_with_dots() {
        let t = tokenize("1996.July.10");
        assert_eq!(t, vec!["1996", ".", "July", ".", "10"]);
    }

    #[test]
    fn tokenize_comma_decimal() {
        let t = tokenize("10:49:41,502");
        assert_eq!(t, vec!["10", ":", "49", ":", "41.502"]);
    }

    #[test]
    fn tokenize_spaces() {
        let t = tokenize("  July   4 ,  1976  ");
        // Spaces are normalized to " "
        assert!(t.contains(&"July".to_string()));
        assert!(t.contains(&"4".to_string()));
        assert!(t.contains(&"1976".to_string()));
    }

    // --- parser tests ---

    #[test]
    fn parse_iso() {
        assert_eq!(pd("2003-09-25T10:49:41"), ndt(2003, 9, 25, 10, 49, 41));
    }

    #[test]
    fn parse_iso_stripped() {
        assert_eq!(pd("20030925T104941"), ndt(2003, 9, 25, 10, 49, 41));
    }

    #[test]
    fn parse_date_with_dash() {
        assert_eq!(pd("09-25-2003"), ndt(2003, 9, 25, 0, 0, 0));
    }

    #[test]
    fn parse_date_with_slash() {
        assert_eq!(pd("09/25/2003"), ndt(2003, 9, 25, 0, 0, 0));
    }

    #[test]
    fn parse_date_with_dot() {
        assert_eq!(pd("2003.09.25"), ndt(2003, 9, 25, 0, 0, 0));
    }

    #[test]
    fn parse_date_with_space() {
        assert_eq!(pd("2003 09 25"), ndt(2003, 9, 25, 0, 0, 0));
    }

    #[test]
    fn parse_month_name() {
        assert_eq!(pd("Jan 1 1999 11:23:34"), ndt(1999, 1, 1, 11, 23, 34));
    }

    #[test]
    fn parse_month_name_microseconds() {
        assert_eq!(
            pd("Jan 1 1999 11:23:34.578"),
            ndt_us(1999, 1, 1, 11, 23, 34, 578_000)
        );
    }

    #[test]
    fn parse_date_command() {
        assert_eq!(
            pd("Thu Sep 25 10:36:28 2003"),
            ndt(2003, 9, 25, 10, 36, 28)
        );
    }

    #[test]
    fn parse_time_only() {
        assert_eq!(pd("10:36:28"), ndt(2003, 9, 25, 10, 36, 28));
    }

    #[test]
    fn parse_hms_letters() {
        assert_eq!(pd("10h36m28s"), ndt(2003, 9, 25, 10, 36, 28));
    }

    #[test]
    fn parse_am_pm() {
        assert_eq!(pd("10pm"), ndt(2003, 9, 25, 22, 0, 0));
    }

    #[test]
    fn parse_12am() {
        assert_eq!(pd("12:08 PM"), ndt(2003, 9, 25, 12, 8, 0));
    }

    #[test]
    fn parse_weekday_alone() {
        // "Wed" from default 2003-09-25 (Thu) should advance to next Wed = 2003-10-01
        assert_eq!(pd("Wed"), ndt(2003, 10, 1, 0, 0, 0));
    }

    #[test]
    fn parse_month_alone() {
        assert_eq!(pd("October"), ndt(2003, 10, 25, 0, 0, 0));
    }

    #[test]
    fn parse_year_alone() {
        assert_eq!(pd("2003"), ndt(2003, 9, 25, 0, 0, 0));
    }

    #[test]
    fn parse_logger_format() {
        assert_eq!(
            pd("2003-09-25 10:49:41,502"),
            ndt_us(2003, 9, 25, 10, 49, 41, 502_000)
        );
    }

    #[test]
    fn parse_july_4_1976() {
        assert_eq!(pd("July 4, 1976"), ndt(1976, 7, 4, 0, 0, 0));
    }

    #[test]
    fn parse_random_format_1() {
        assert_eq!(pd("4 jul 1976"), ndt(1976, 7, 4, 0, 0, 0));
    }

    #[test]
    fn parse_random_format_2() {
        assert_eq!(pd("19760704"), ndt(1976, 7, 4, 0, 0, 0));
    }

    #[test]
    fn parse_two_digit_year() {
        assert_eq!(pd("10-09-03"), ndt(2003, 10, 9, 0, 0, 0));
    }

    #[test]
    fn parse_ad() {
        assert_eq!(
            pd("1996.July.10 AD 12:08 PM"),
            ndt(1996, 7, 10, 12, 8, 0)
        );
    }

    #[test]
    fn parse_no_sep_12digit() {
        assert_eq!(pd("199709020908"), ndt(1997, 9, 2, 9, 8, 0));
    }

    #[test]
    fn parse_no_sep_14digit() {
        assert_eq!(pd("19970902090807"), ndt(1997, 9, 2, 9, 8, 7));
    }

    #[test]
    fn parse_3rd_of_may() {
        assert_eq!(pd("3rd of May 2001"), ndt(2001, 5, 3, 0, 0, 0));
    }

    #[test]
    fn parse_unknown_format() {
        let parser = Parser::default();
        let result = parser.parse("xyz", Some(default_dt()), None, None, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn parse_tz_offset() {
        let parser = Parser::default();
        let out = parser
            .parse(
                "2003-09-25 10:49:41 -0300",
                Some(default_dt()),
                None,
                None,
                false,
                false,
            )
            .unwrap();
        assert_eq!(out.tzoffset, Some(-3 * 3600));
    }

    #[test]
    fn parse_tz_utc() {
        let parser = Parser::default();
        let out = parser
            .parse(
                "2003-09-25 10:49:41 UTC",
                Some(default_dt()),
                None,
                None,
                false,
                false,
            )
            .unwrap();
        assert_eq!(out.tzname, Some("UTC".into()));
        assert_eq!(out.tzoffset, Some(0));
    }

    #[test]
    fn parse_fuzzy() {
        let parser = Parser::default();
        let out = parser
            .parse(
                "Today is January 1, 2047 at 8:21:00AM",
                Some(default_dt()),
                None,
                None,
                true,
                true,
            )
            .unwrap();
        assert_eq!(out.naive, ndt(2047, 1, 1, 8, 21, 0));
        assert!(out.skipped_tokens.is_some());
    }

    #[test]
    fn parse_hms_spaces() {
        assert_eq!(pd("10 h 36"), ndt(2003, 9, 25, 10, 36, 0));
    }

    #[test]
    fn parse_fractional_hours() {
        assert_eq!(pd("2016-12-21 04.2h"), ndt(2016, 12, 21, 4, 12, 0));
    }

    #[test]
    fn parse_zero_year() {
        assert_eq!(pd("31-Dec-00"), ndt(2000, 12, 31, 0, 0, 0));
    }

    #[test]
    fn parse_compact_hhmm_after_date() {
        assert_eq!(pd("20030925T1049"), ndt(2003, 9, 25, 10, 49, 0));
    }

    #[test]
    fn parse_high_precision() {
        assert_eq!(
            pd("20080227T21:26:01.123456789"),
            ndt_us(2008, 2, 27, 21, 26, 1, 123_456)
        );
    }

    // --- timezone offset patterns ---

    #[test]
    fn parse_tz_offset_colon() {
        let parser = Parser::default();
        let out = parser
            .parse(
                "2003-09-25 10:49:41 -03:00",
                Some(default_dt()),
                None,
                None,
                false,
                false,
            )
            .unwrap();
        assert_eq!(out.tzoffset, Some(-3 * 3600));
    }

    #[test]
    fn parse_tz_offset_positive() {
        let parser = Parser::default();
        let out = parser
            .parse(
                "2003-09-25 10:49:41 +0530",
                Some(default_dt()),
                None,
                None,
                false,
                false,
            )
            .unwrap();
        assert_eq!(out.tzoffset, Some(5 * 3600 + 30 * 60));
    }

    #[test]
    fn parse_tz_offset_short() {
        let parser = Parser::default();
        let out = parser
            .parse(
                "2003-09-25 10:49:41 +05",
                Some(default_dt()),
                None,
                None,
                false,
                false,
            )
            .unwrap();
        assert_eq!(out.tzoffset, Some(5 * 3600));
    }

    // --- timezone name patterns ---

    #[test]
    fn parse_tz_gmt() {
        let parser = Parser::default();
        let out = parser
            .parse(
                "2003-09-25 10:49:41 GMT",
                Some(default_dt()),
                None,
                None,
                false,
                false,
            )
            .unwrap();
        assert_eq!(out.tzname, Some("GMT".into()));
    }

    #[test]
    fn parse_tz_est() {
        let parser = Parser::default();
        let out = parser
            .parse(
                "2003-09-25 10:49:41 EST",
                Some(default_dt()),
                None,
                None,
                false,
                false,
            )
            .unwrap();
        assert_eq!(out.tzname, Some("EST".into()));
    }

    // --- month with separators ---

    #[test]
    fn parse_month_name_dash() {
        // Jan-01-99
        assert_eq!(pd("Jan-01-99"), ndt(1999, 1, 1, 0, 0, 0));
    }

    #[test]
    fn parse_month_name_slash() {
        assert_eq!(pd("Jan/01/99"), ndt(1999, 1, 1, 0, 0, 0));
    }

    #[test]
    fn parse_month_name_dash_year() {
        assert_eq!(pd("Jan-01-2003"), ndt(2003, 1, 1, 0, 0, 0));
    }

    // --- AM/PM edge cases ---

    #[test]
    fn parse_12am_midnight() {
        assert_eq!(pd("12:00 AM"), ndt(2003, 9, 25, 0, 0, 0));
    }

    #[test]
    fn parse_12pm_noon() {
        assert_eq!(pd("12:00 PM"), ndt(2003, 9, 25, 12, 0, 0));
    }

    #[test]
    fn parse_am_morning() {
        assert_eq!(pd("6am"), ndt(2003, 9, 25, 6, 0, 0));
    }

    // --- dayfirst / yearfirst ---

    #[test]
    fn parse_dayfirst() {
        let parser = Parser::default();
        let out = parser
            .parse("10/09/03", Some(default_dt()), Some(true), None, false, false)
            .unwrap();
        assert_eq!(out.naive, ndt(2003, 9, 10, 0, 0, 0));
    }

    #[test]
    fn parse_yearfirst() {
        let parser = Parser::default();
        let out = parser
            .parse("03/09/25", Some(default_dt()), None, Some(true), false, false)
            .unwrap();
        assert_eq!(out.naive, ndt(2003, 9, 25, 0, 0, 0));
    }

    // --- HHMMSS.ss format ---

    #[test]
    fn parse_hhmmss_after_date() {
        assert_eq!(pd("20030925T104928"), ndt(2003, 9, 25, 10, 49, 28));
    }

    // --- fuzzy parsing ---

    #[test]
    fn parse_fuzzy_no_tokens() {
        let parser = Parser::default();
        let out = parser
            .parse(
                "I]ll meet you at 2003-09-25 10:49",
                Some(default_dt()),
                None,
                None,
                true,
                false,
            )
            .unwrap();
        assert_eq!(out.naive, ndt(2003, 9, 25, 10, 49, 0));
        assert!(out.skipped_tokens.is_none());
    }

    // --- single date values ---

    #[test]
    fn parse_month_and_year_value() {
        // "2003 10" with default -> year=2003, month=10
        assert_eq!(pd("October 2003"), ndt(2003, 10, 25, 0, 0, 0));
    }

    #[test]
    fn parse_two_values_day_month() {
        // "15 03" — two values, both <= 12: first=month, second=day
        assert_eq!(pd("10 03"), ndt(2003, 10, 3, 0, 0, 0));
    }

    // --- ParserError Display ---

    #[test]
    fn test_parser_error_display() {
        let e1 = ParserError::UnknownFormat("xyz".into());
        assert!(format!("{}", e1).contains("Unknown string format"));

        let e2 = ParserError::NoDate("".into());
        assert!(format!("{}", e2).contains("does not contain a date"));

        let e3 = ParserError::ValueError("bad".into());
        assert_eq!(format!("{}", e3), "bad");
    }

    // --- Pertain pattern: "Jan of 2001" ---

    #[test]
    fn parse_pertain_of() {
        assert_eq!(pd("Jan of 2003"), ndt(2003, 1, 25, 0, 0, 0));
    }

    // --- No separator date (YYMMDD) ---

    #[test]
    fn parse_yymmdd_8digit() {
        // 8-digit compact: YYYYMMDD
        assert_eq!(pd("20030925"), ndt(2003, 9, 25, 0, 0, 0));
    }

    // --- convenience parse() function ---

    #[test]
    fn test_parse_convenience() {
        let out = parse(
            "2003-09-25 10:49:41",
            None,
            None,
            Some(default_dt()),
            false,
            false,
        )
        .unwrap();
        assert_eq!(out.naive, ndt(2003, 9, 25, 10, 49, 41));
    }

    // --- GMT+3 sign flip ---

    #[test]
    fn parse_gmt_plus_offset() {
        let parser = Parser::default();
        let out = parser
            .parse(
                "2003-09-25 10:49:41 UTC+03:00",
                Some(default_dt()),
                None,
                None,
                false,
                false,
            )
            .unwrap();
        // UTC+03:00 → sign is flipped to -3*3600 per Python's behavior
        assert_eq!(out.tzoffset, Some(-3 * 3600));
    }

    // --- HHMMSS with fractional seconds ---

    #[test]
    fn parse_hhmmss_fractional() {
        // After a date, HHMMSS.ss
        assert_eq!(
            pd("2003-09-25 104941.5"),
            ndt_us(2003, 9, 25, 10, 49, 41, 500_000)
        );
    }

    // --- 12am boundary ---

    #[test]
    fn parse_jump_then_ampm() {
        assert_eq!(pd("12 am"), ndt(2003, 9, 25, 0, 0, 0));
    }

    // --- 6-digit YYMMDD ---

    #[test]
    fn parse_yymmdd_6digit() {
        assert_eq!(pd("030925"), ndt(2003, 9, 25, 0, 0, 0));
    }

    // --- 6-digit YYYYMM fallback (invalid day) ---

    #[test]
    fn parse_yyyymm_fallback() {
        // "201712" — mm=17 invalid, so fallback to YYYYMM (2017-12)
        assert_eq!(
            p("201712"),
            ndt(2017, 12, 25, 0, 0, 0) // day from default
        );
    }

    // --- HHMMSS 6-digit after date ---

    #[test]
    fn parse_hhmmss_6digit_after_date() {
        assert_eq!(pd("2003-09-25 104941"), ndt_us(2003, 9, 25, 10, 49, 41, 0));
    }

    // --- 10-digit YYYYMMDDHH ---

    #[test]
    fn parse_10digit_yyyymmddhh() {
        assert_eq!(pd("1991041310"), ndt(1991, 4, 13, 10, 0, 0));
    }

    // --- 10-digit YYYYMMDDHH with :MM ---

    #[test]
    fn parse_10digit_with_minutes() {
        assert_eq!(pd("1991041310:30"), ndt(1991, 4, 13, 10, 30, 0));
    }

    // --- 10-digit YYYYMMDDHH with :MM:SS ---

    #[test]
    fn parse_10digit_with_seconds() {
        assert_eq!(pd("1991041310:30:45"), ndt(1991, 4, 13, 10, 30, 45));
    }

    // --- 12-digit YYYYMMDDHHMM ---

    #[test]
    fn parse_12digit_yyyymmddhhmm() {
        assert_eq!(pd("200309251049"), ndt(2003, 9, 25, 10, 49, 0));
    }

    // --- 14-digit YYYYMMDDHHMMSS ---

    #[test]
    fn parse_14digit_yyyymmddhhmmss() {
        assert_eq!(pd("20030925104928"), ndt(2003, 9, 25, 10, 49, 28));
    }

    // --- fuzzy_with_tokens returning skipped tokens ---

    #[test]
    fn parse_fuzzy_with_tokens() {
        let parser = Parser::default();
        let out = parser
            .parse(
                "Today is January 1, 2047 at 8:21:00AM",
                Some(default_dt()),
                None,
                None,
                true,
                true,
            )
            .unwrap();
        assert_eq!(out.naive, ndt(2047, 1, 1, 8, 21, 0));
        let tokens = out.skipped_tokens.unwrap();
        assert!(!tokens.is_empty());
        assert!(tokens.iter().any(|t| t.contains("Today")));
    }

    // --- weekday resolution (day not explicitly set) ---

    #[test]
    fn parse_weekday_resolution() {
        // "Monday" from default 2003-09-25 (Thu) → next Mon = 2003-09-29
        assert_eq!(pd("Monday"), ndt(2003, 9, 29, 0, 0, 0));
    }

    // --- two values with first > 31 ---

    #[test]
    fn parse_two_values_year_month() {
        assert_eq!(pd("2003 10"), ndt(2003, 10, 25, 0, 0, 0));
    }

    // --- two values with second > 31 ---

    #[test]
    fn parse_two_values_month_year() {
        assert_eq!(pd("10 2003"), ndt(2003, 10, 25, 0, 0, 0));
    }

    // --- dayfirst with two values ---

    #[test]
    fn parse_dayfirst_two_values() {
        let parser = Parser::default();
        let out = parser
            .parse("10/09", Some(default_dt()), Some(true), None, false, false)
            .unwrap();
        assert_eq!(out.naive, ndt(2003, 9, 10, 0, 0, 0));
    }

    // --- yearfirst with three values ---

    #[test]
    fn parse_yearfirst_three_values() {
        let parser = Parser::default();
        let out = parser
            .parse("03/09/25", Some(default_dt()), None, Some(true), false, false)
            .unwrap();
        assert_eq!(out.naive, ndt(2003, 9, 25, 0, 0, 0));
    }

    // --- month with separator and third value as month name ---

    #[test]
    fn parse_day_sep_month_name_sep_year() {
        assert_eq!(pd("25-Sep-2003"), ndt(2003, 9, 25, 0, 0, 0));
    }

    // --- date with month name in middle of separator chain ---

    #[test]
    fn parse_year_month_name_day() {
        assert_eq!(pd("2003/Sep/25"), ndt(2003, 9, 25, 0, 0, 0));
    }

    // --- could_be_day: with known month and year ---

    #[test]
    fn parse_explicit_month_then_day() {
        assert_eq!(pd("Feb 28 2003"), ndt(2003, 2, 28, 0, 0, 0));
    }

    // --- HH:MM where minutes contain a period (sub-seconds) ---

    #[test]
    fn parse_min_sec_combined() {
        // "10:36:28.5" — seconds with fractional
        assert_eq!(
            pd("2003-09-25 10:36:28.5"),
            ndt_us(2003, 9, 25, 10, 36, 28, 500_000)
        );
    }

    // --- pertain: month "of" year ---

    #[test]
    fn parse_pertain_of_long_year() {
        assert_eq!(pd("September of 2003"), ndt(2003, 9, 25, 0, 0, 0));
    }

    // --- century_specified flag (4-digit string) ---

    #[test]
    fn parse_four_digit_year_string() {
        assert_eq!(pd("0031-01-01"), ndt(31, 1, 1, 0, 0, 0));
    }

    // --- default day clamping ---

    #[test]
    fn parse_month_clamps_day() {
        // Default day is 31, Feb only has 28 days in 2003
        let parser = Parser::default();
        let default = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2003, 1, 31).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        let out = parser
            .parse("Feb 2003", Some(default), None, None, false, false)
            .unwrap();
        assert_eq!(out.naive, ndt(2003, 2, 28, 0, 0, 0));
    }

    // --- ParserInfo custom config ---

    #[test]
    fn parse_custom_parserinfo() {
        let info = ParserInfo::new(false, false);
        let parser = Parser::new(info);
        let out = parser
            .parse("2003-09-25", Some(default_dt()), None, None, false, false)
            .unwrap();
        assert_eq!(out.naive, ndt(2003, 9, 25, 0, 0, 0));
    }
}
