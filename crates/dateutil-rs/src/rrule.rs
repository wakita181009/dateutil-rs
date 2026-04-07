//! Recurrence rules (RFC 5545) — Rust port of `dateutil.rrule`.

pub mod iter;

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt;

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

use crate::common::Weekday;

// ---------------------------------------------------------------------------
// Frequency constants
// ---------------------------------------------------------------------------

pub const YEARLY: u8 = 0;
pub const MONTHLY: u8 = 1;
pub const WEEKLY: u8 = 2;
pub const DAILY: u8 = 3;
pub const HOURLY: u8 = 4;
pub const MINUTELY: u8 = 5;
pub const SECONDLY: u8 = 6;

pub const FREQNAMES: [&str; 7] = [
    "YEARLY", "MONTHLY", "WEEKLY", "DAILY", "HOURLY", "MINUTELY", "SECONDLY",
];

// ---------------------------------------------------------------------------
// Day / month masks (pre-computed, matching Python's global masks)
// ---------------------------------------------------------------------------

/// Month number for each day of a 366-day year (1-indexed months), plus 7
/// extra days for cross-year weekly periods.
pub(crate) const M366_MASK: [u8; 373] = {
    let days: [usize; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut v = [0u8; 373];
    let mut idx = 0;
    let mut month = 0;
    while month < 12 {
        let mut d = 0;
        while d < days[month] {
            v[idx] = (month + 1) as u8;
            idx += 1;
            d += 1;
        }
        month += 1;
    }
    let mut i = 0;
    while i < 7 {
        v[idx] = 1;
        idx += 1;
        i += 1;
    }
    v
};

pub(crate) const M365_MASK: [u8; 372] = {
    let days: [usize; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut v = [0u8; 372];
    let mut idx = 0;
    let mut month = 0;
    while month < 12 {
        let mut d = 0;
        while d < days[month] {
            v[idx] = (month + 1) as u8;
            idx += 1;
            d += 1;
        }
        month += 1;
    }
    let mut i = 0;
    while i < 7 {
        v[idx] = 1;
        idx += 1;
        i += 1;
    }
    v
};

/// Positive month-day for each day of a 366-day year, plus 7 extra.
pub(crate) const MDAY366_MASK: [i32; 373] = {
    let days: [i32; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut v = [0i32; 373];
    let mut idx = 0;
    let mut month = 0;
    while month < 12 {
        let mut day = 1;
        while day <= days[month] {
            v[idx] = day;
            idx += 1;
            day += 1;
        }
        month += 1;
    }
    let mut day = 1;
    while day <= 7 {
        v[idx] = day;
        idx += 1;
        day += 1;
    }
    v
};

pub(crate) const MDAY365_MASK: [i32; 372] = {
    let days: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut v = [0i32; 372];
    let mut idx = 0;
    let mut month = 0;
    while month < 12 {
        let mut day = 1;
        while day <= days[month] {
            v[idx] = day;
            idx += 1;
            day += 1;
        }
        month += 1;
    }
    let mut day = 1;
    while day <= 7 {
        v[idx] = day;
        idx += 1;
        day += 1;
    }
    v
};

/// Negative month-day for each day of a 366-day year, plus 7 extra.
pub(crate) const NMDAY366_MASK: [i32; 373] = {
    let days: [i32; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut v = [0i32; 373];
    let mut idx = 0;
    let mut month = 0;
    while month < 12 {
        let mut i = 0;
        while i < days[month] {
            v[idx] = i - days[month];
            idx += 1;
            i += 1;
        }
        month += 1;
    }
    let mut i = 0;
    while i < 7 {
        v[idx] = i - 31;
        idx += 1;
        i += 1;
    }
    v
};

pub(crate) const NMDAY365_MASK: [i32; 372] = {
    let days: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut v = [0i32; 372];
    let mut idx = 0;
    let mut month = 0;
    while month < 12 {
        let mut i = 0;
        while i < days[month] {
            v[idx] = i - days[month];
            idx += 1;
            i += 1;
        }
        month += 1;
    }
    let mut i = 0;
    while i < 7 {
        v[idx] = i - 31;
        idx += 1;
        i += 1;
    }
    v
};

pub(crate) const M366RANGE: [usize; 13] =
    [0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335, 366];
pub(crate) const M365RANGE: [usize; 13] =
    [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365];

/// Weekday mask: cycles 0..6 for 55 weeks (385 entries).
pub(crate) const WDAY_MASK: [u8; 385] = {
    let mut v = [0u8; 385];
    let mut i = 0;
    while i < 385 {
        v[i] = (i % 7) as u8;
        i += 1;
    }
    v
};

// ---------------------------------------------------------------------------
// GCD helper
// ---------------------------------------------------------------------------

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

// ---------------------------------------------------------------------------
// RRule
// ---------------------------------------------------------------------------

/// A recurrence rule (RFC 5545 RRULE).
#[derive(Debug, Clone)]
pub struct RRule {
    pub(crate) freq: u8,
    pub(crate) dtstart: NaiveDateTime,
    pub(crate) interval: i64,
    pub(crate) wkst: u8,
    pub(crate) count: Option<i64>,
    pub(crate) until: Option<NaiveDateTime>,
    pub(crate) bysetpos: Option<Vec<i32>>,
    pub(crate) bymonth: Option<Vec<u8>>,
    pub(crate) bymonthday: Vec<i32>,
    pub(crate) bynmonthday: Vec<i32>,
    pub(crate) byyearday: Option<Vec<i32>>,
    pub(crate) byeaster: Option<Vec<i32>>,
    pub(crate) byweekno: Option<Vec<i32>>,
    pub(crate) byweekday: Option<Vec<u8>>,
    pub(crate) bynweekday: Option<Vec<(u8, i32)>>,
    pub(crate) byhour: Option<Vec<u8>>,
    pub(crate) byminute: Option<Vec<u8>>,
    pub(crate) bysecond: Option<Vec<u8>>,
    pub(crate) timeset: Option<Vec<NaiveTime>>,
    // Original rule values for string serialization
    pub(crate) original_rule: OriginalRule,
}

/// Stores the original `byxxx` values as provided by the caller, for use in
/// `__str__` / `to_string()` serialization. `None` means "implicitly derived
/// from dtstart" (Python stores `None` in these cases).
#[derive(Debug, Clone, Default)]
pub(crate) struct OriginalRule {
    pub bysetpos: Option<Vec<i32>>,
    pub bymonth: Option<Option<Vec<u8>>>,
    pub bymonthday: Option<Option<Vec<i32>>>,
    pub byyearday: Option<Vec<i32>>,
    pub byeaster: Option<Vec<i32>>,
    pub byweekno: Option<Vec<i32>>,
    pub byweekday: Option<Option<Vec<Weekday>>>,
    pub byhour: Option<Vec<u8>>,
    pub byminute: Option<Vec<u8>>,
    pub bysecond: Option<Vec<u8>>,
}

/// Builder for constructing an RRule with keyword-style arguments.
#[derive(Debug, Clone)]
pub struct RRuleBuilder {
    freq: u8,
    dtstart: Option<NaiveDateTime>,
    interval: i64,
    wkst: Option<u8>,
    count: Option<i64>,
    until: Option<NaiveDateTime>,
    bysetpos: Option<Vec<i32>>,
    bymonth: Option<Vec<u8>>,
    bymonthday: Option<Vec<i32>>,
    byyearday: Option<Vec<i32>>,
    byeaster: Option<Vec<i32>>,
    byweekno: Option<Vec<i32>>,
    byweekday: Option<Vec<Weekday>>,
    byhour: Option<Vec<u8>>,
    byminute: Option<Vec<u8>>,
    bysecond: Option<Vec<u8>>,
}

impl RRuleBuilder {
    pub fn new(freq: u8) -> Self {
        Self {
            freq,
            dtstart: None,
            interval: 1,
            wkst: None,
            count: None,
            until: None,
            bysetpos: None,
            bymonth: None,
            bymonthday: None,
            byyearday: None,
            byeaster: None,
            byweekno: None,
            byweekday: None,
            byhour: None,
            byminute: None,
            bysecond: None,
        }
    }

    pub fn dtstart(mut self, dt: NaiveDateTime) -> Self {
        self.dtstart = Some(dt);
        self
    }

    pub fn interval(mut self, val: i64) -> Self {
        self.interval = val;
        self
    }

    pub fn wkst(mut self, val: u8) -> Self {
        self.wkst = Some(val);
        self
    }

    pub fn count(mut self, val: i64) -> Self {
        self.count = Some(val);
        self
    }

    pub fn until(mut self, dt: NaiveDateTime) -> Self {
        self.until = Some(dt);
        self
    }

    pub fn bysetpos(mut self, val: Vec<i32>) -> Self {
        self.bysetpos = Some(val);
        self
    }

    pub fn bymonth(mut self, val: Vec<u8>) -> Self {
        self.bymonth = Some(val);
        self
    }

    pub fn bymonthday(mut self, val: Vec<i32>) -> Self {
        self.bymonthday = Some(val);
        self
    }

    pub fn byyearday(mut self, val: Vec<i32>) -> Self {
        self.byyearday = Some(val);
        self
    }

    pub fn byeaster(mut self, val: Vec<i32>) -> Self {
        self.byeaster = Some(val);
        self
    }

    pub fn byweekno(mut self, val: Vec<i32>) -> Self {
        self.byweekno = Some(val);
        self
    }

    pub fn byweekday(mut self, val: Vec<Weekday>) -> Self {
        self.byweekday = Some(val);
        self
    }

    pub fn byhour(mut self, val: Vec<u8>) -> Self {
        self.byhour = Some(val);
        self
    }

    pub fn byminute(mut self, val: Vec<u8>) -> Self {
        self.byminute = Some(val);
        self
    }

    pub fn bysecond(mut self, val: Vec<u8>) -> Self {
        self.bysecond = Some(val);
        self
    }

    pub fn build(self) -> Result<RRule, RRuleError> {
        RRule::new(
            self.freq,
            self.dtstart,
            self.interval,
            self.wkst,
            self.count,
            self.until,
            self.bysetpos,
            self.bymonth,
            self.bymonthday,
            self.byyearday,
            self.byeaster,
            self.byweekno,
            self.byweekday,
            self.byhour,
            self.byminute,
            self.bysecond,
        )
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum RRuleError {
    #[error("bysetpos must be between 1 and 366, or between -366 and -1")]
    InvalidBySetPos,
    #[error("RRULE UNTIL values must be specified in UTC when DTSTART is timezone-aware")]
    UntilTzMismatch,
    #[error("Invalid rrule byxxx generates an empty set.")]
    EmptyBySet,
    #[error("Invalid combination of interval and byhour resulting in empty rule.")]
    EmptyHourRule,
    #[error("Invalid combination of interval, byhour and byminute resulting in empty rule.")]
    EmptyMinuteRule,
    #[error("Can't create weekday with n==0")]
    WeekdayNZero,
    #[error("{0}")]
    ValueError(String),
}

impl RRule {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        freq: u8,
        dtstart: Option<NaiveDateTime>,
        interval: i64,
        wkst: Option<u8>,
        count: Option<i64>,
        until: Option<NaiveDateTime>,
        bysetpos: Option<Vec<i32>>,
        bymonth: Option<Vec<u8>>,
        bymonthday: Option<Vec<i32>>,
        byyearday: Option<Vec<i32>>,
        byeaster: Option<Vec<i32>>,
        byweekno: Option<Vec<i32>>,
        byweekday: Option<Vec<Weekday>>,
        byhour: Option<Vec<u8>>,
        byminute: Option<Vec<u8>>,
        bysecond: Option<Vec<u8>>,
    ) -> Result<Self, RRuleError> {
        let dtstart = dtstart.unwrap_or_else(|| {
            let now = chrono::Local::now().naive_local();
            now.with_nanosecond(0).unwrap_or(now)
        });
        let dtstart = dtstart.with_nanosecond(0).unwrap_or(dtstart);

        if freq > SECONDLY {
            return Err(RRuleError::ValueError(format!(
                "invalid frequency: {freq}"
            )));
        }

        let mut original_rule = OriginalRule::default();

        // wkst
        let wkst = wkst.unwrap_or(0); // Monday by default

        // Validate bysetpos
        let bysetpos = if let Some(ref pos) = bysetpos {
            for &p in pos {
                if p == 0 || !(-366..=366).contains(&p) {
                    return Err(RRuleError::InvalidBySetPos);
                }
            }
            original_rule.bysetpos = Some(pos.clone());
            Some(pos.clone())
        } else {
            None
        };

        // Default byxxx when none of byweekno/byyearday/bymonthday/byweekday/byeaster given
        let mut bymonth = bymonth;
        let mut bymonthday = bymonthday;
        let mut byweekday = byweekday;

        if byweekno.is_none()
            && byyearday.is_none()
            && bymonthday.is_none()
            && byweekday.is_none()
            && byeaster.is_none()
        {
            if freq == YEARLY {
                if bymonth.is_none() {
                    bymonth = Some(vec![dtstart.month() as u8]);
                    original_rule.bymonth = Some(None); // implicit
                }
                bymonthday = Some(vec![dtstart.day() as i32]);
                original_rule.bymonthday = Some(None); // implicit
            } else if freq == MONTHLY {
                bymonthday = Some(vec![dtstart.day() as i32]);
                original_rule.bymonthday = Some(None); // implicit
            } else if freq == WEEKLY {
                byweekday = Some(vec![Weekday::new(
                    dtstart.weekday().num_days_from_monday() as u8,
                    None,
                )]);
                original_rule.byweekday = Some(None); // implicit
            }
        }

        // bymonth
        let bymonth = if let Some(mut bm) = bymonth {
            bm.sort();
            bm.dedup();
            if original_rule.bymonth.is_none() {
                original_rule.bymonth = Some(Some(bm.clone()));
            }
            Some(bm)
        } else {
            None
        };

        // byyearday
        let byyearday = if let Some(mut by) = byyearday {
            by.sort();
            by.dedup();
            original_rule.byyearday = Some(by.clone());
            Some(by)
        } else {
            None
        };

        // byeaster
        let byeaster = if let Some(mut be) = byeaster {
            be.sort();
            original_rule.byeaster = Some(be.clone());
            Some(be)
        } else {
            None
        };

        // bymonthday / bynmonthday
        let (bymonthday_pos, bynmonthday) = if let Some(bmd) = bymonthday {
            let mut pos: Vec<i32> = bmd.iter().copied().filter(|&x| x > 0).collect();
            let mut neg: Vec<i32> = bmd.iter().copied().filter(|&x| x < 0).collect();
            pos.sort();
            pos.dedup();
            neg.sort();
            neg.dedup();
            if original_rule.bymonthday.is_none() {
                let mut combined = pos.clone();
                combined.extend_from_slice(&neg);
                original_rule.bymonthday = Some(Some(combined));
            }
            (pos, neg)
        } else {
            (vec![], vec![])
        };

        // byweekno
        let byweekno = if let Some(mut bwn) = byweekno {
            bwn.sort();
            bwn.dedup();
            original_rule.byweekno = Some(bwn.clone());
            Some(bwn)
        } else {
            None
        };

        // byweekday / bynweekday
        let (byweekday_flat, bynweekday) = if let Some(bwd) = byweekday {
            let mut plain: Vec<u8> = Vec::new();
            let mut nth: Vec<(u8, i32)> = Vec::new();

            for wd in &bwd {
                match wd.n() {
                    None | Some(0) => {
                        plain.push(wd.weekday());
                    }
                    Some(n) => {
                        if freq > MONTHLY {
                            // For frequencies > MONTHLY, nth weekday is treated as plain
                            plain.push(wd.weekday());
                        } else {
                            nth.push((wd.weekday(), n));
                        }
                    }
                }
            }

            plain.sort();
            plain.dedup();
            nth.sort();
            nth.dedup();

            // Build original_rule weekdays
            if original_rule.byweekday.is_none() {
                let mut orig: Vec<Weekday> = plain
                    .iter()
                    .map(|&w| Weekday::new(w, None))
                    .collect();
                orig.extend(nth.iter().map(|&(w, n)| Weekday::new(w, Some(n))));
                original_rule.byweekday = Some(Some(orig));
            }

            let bwd_opt = if plain.is_empty() { None } else { Some(plain) };
            let bnwd_opt = if nth.is_empty() { None } else { Some(nth) };
            (bwd_opt, bnwd_opt)
        } else {
            (None, None)
        };

        // byhour
        let byhour = if let Some(bh) = byhour {
            let set = if freq == HOURLY {
                construct_byset(dtstart.hour() as i64, &bh, 24, interval)?
            } else {
                bh.into_iter().collect()
            };
            let mut v: Vec<u8> = set.into_iter().collect();
            v.sort();
            original_rule.byhour = Some(v.clone());
            Some(v)
        } else if freq < HOURLY {
            Some(vec![dtstart.hour() as u8])
        } else {
            None
        };

        // byminute
        let byminute = if let Some(bm) = byminute {
            let set = if freq == MINUTELY {
                construct_byset(dtstart.minute() as i64, &bm, 60, interval)?
            } else {
                bm.into_iter().collect()
            };
            let mut v: Vec<u8> = set.into_iter().collect();
            v.sort();
            original_rule.byminute = Some(v.clone());
            Some(v)
        } else if freq < MINUTELY {
            Some(vec![dtstart.minute() as u8])
        } else {
            None
        };

        // bysecond
        let bysecond = if let Some(bs) = bysecond {
            let set = if freq == SECONDLY {
                construct_byset(dtstart.second() as i64, &bs, 60, interval)?
            } else {
                bs.into_iter().collect()
            };
            let mut v: Vec<u8> = set.into_iter().collect();
            v.sort();
            original_rule.bysecond = Some(v.clone());
            Some(v)
        } else if freq < SECONDLY {
            Some(vec![dtstart.second() as u8])
        } else {
            None
        };

        // timeset
        let timeset = if freq >= HOURLY {
            None // computed dynamically during iteration
        } else {
            let mut ts: Vec<NaiveTime> = Vec::new();
            let bh = byhour.as_deref().unwrap_or(&[]);
            let bm = byminute.as_deref().unwrap_or(&[]);
            let bs = bysecond.as_deref().unwrap_or(&[]);
            for &hour in bh {
                for &minute in bm {
                    for &second in bs {
                        if let Some(t) =
                            NaiveTime::from_hms_opt(hour as u32, minute as u32, second as u32)
                        {
                            ts.push(t);
                        }
                    }
                }
            }
            ts.sort();
            Some(ts)
        };

        Ok(RRule {
            freq,
            dtstart,
            interval,
            wkst,
            count,
            until,
            bysetpos,
            bymonth,
            bymonthday: bymonthday_pos,
            bynmonthday,
            byyearday,
            byeaster,
            byweekno,
            byweekday: byweekday_flat,
            bynweekday,
            byhour,
            byminute,
            bysecond,
            timeset,
            original_rule,
        })
    }

    /// Whether this rule is guaranteed to produce a finite sequence.
    pub fn is_finite(&self) -> bool {
        self.count.is_some() || self.until.is_some()
    }

    /// Collect all occurrences into a Vec.
    pub fn all(&self) -> Vec<NaiveDateTime> {
        self.iter().collect()
    }

    /// Return an iterator over occurrences.
    pub fn iter(&self) -> iter::RRuleIter {
        iter::RRuleIter::new(self)
    }

    /// Returns the last recurrence before `dt`.
    pub fn before(&self, dt: NaiveDateTime, inc: bool) -> Option<NaiveDateTime> {
        let mut last = None;
        for i in self.iter() {
            if (inc && i > dt) || (!inc && i >= dt) {
                break;
            }
            last = Some(i);
        }
        last
    }

    /// Returns the first recurrence after `dt`.
    pub fn after(&self, dt: NaiveDateTime, inc: bool) -> Option<NaiveDateTime> {
        self.iter().find(|&i| if inc { i >= dt } else { i > dt })
    }

    /// Returns all occurrences between `after` and `before`.
    pub fn between(
        &self,
        after: NaiveDateTime,
        before: NaiveDateTime,
        inc: bool,
    ) -> Vec<NaiveDateTime> {
        let mut result = Vec::new();
        let mut started = false;
        for i in self.iter() {
            if inc {
                if i > before {
                    break;
                }
                if !started {
                    if i >= after {
                        started = true;
                        result.push(i);
                    }
                } else {
                    result.push(i);
                }
            } else {
                if i >= before {
                    break;
                }
                if !started {
                    if i > after {
                        started = true;
                        result.push(i);
                    }
                } else {
                    result.push(i);
                }
            }
        }
        result
    }

    /// Count total occurrences.
    pub fn count_all(&self) -> usize {
        self.iter().count()
    }
}

impl fmt::Display for RRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = Vec::new();

        // DTSTART
        output.push(self.dtstart.format("DTSTART:%Y%m%dT%H%M%S").to_string());

        let mut parts = vec![format!("FREQ={}", FREQNAMES[self.freq as usize])];

        if self.interval != 1 {
            parts.push(format!("INTERVAL={}", self.interval));
        }
        if self.wkst != 0 {
            const WKST_NAMES: [&str; 7] = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];
            parts.push(format!("WKST={}", WKST_NAMES[self.wkst as usize]));
        }
        if let Some(c) = self.count {
            parts.push(format!("COUNT={c}"));
        }
        if let Some(u) = self.until {
            parts.push(u.format("UNTIL=%Y%m%dT%H%M%S").to_string());
        }

        // byweekday with nth handling
        let orig = &self.original_rule;
        if let Some(Some(ref bwd)) = orig.byweekday {
                if !bwd.is_empty() {
                    let strs: Vec<String> = bwd
                        .iter()
                        .map(|w| {
                            if let Some(n) = w.n() {
                                if n != 0 {
                                    let day_name =
                                        &["MO", "TU", "WE", "TH", "FR", "SA", "SU"]
                                            [w.weekday() as usize];
                                    return format!("{n:+}{day_name}");
                                }
                            }
                            w.to_string()
                        })
                        .collect();
                    parts.push(format!("BYDAY={}", strs.join(",")));
                }
        }

        type OrigRuleFn = fn(&OriginalRule) -> Option<String>;
        let named: &[(&str, OrigRuleFn)] = &[
            ("BYSETPOS", |o| {
                o.bysetpos.as_ref().map(|v| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
            }),
            ("BYMONTH", |o| {
                o.bymonth.as_ref().and_then(|v| v.as_ref()).map(|v| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
            }),
            ("BYMONTHDAY", |o| {
                o.bymonthday.as_ref().and_then(|v| v.as_ref()).map(|v| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
            }),
            ("BYYEARDAY", |o| {
                o.byyearday.as_ref().map(|v| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
            }),
            ("BYWEEKNO", |o| {
                o.byweekno.as_ref().map(|v| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
            }),
            ("BYHOUR", |o| {
                o.byhour.as_ref().map(|v| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
            }),
            ("BYMINUTE", |o| {
                o.byminute.as_ref().map(|v| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
            }),
            ("BYSECOND", |o| {
                o.bysecond.as_ref().map(|v| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
            }),
            ("BYEASTER", |o| {
                o.byeaster.as_ref().map(|v| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
            }),
        ];

        for &(name, getter) in named {
            // Skip BYDAY since already handled above
            if let Some(val) = getter(orig) {
                if !val.is_empty() {
                    parts.push(format!("{name}={val}"));
                }
            }
        }

        output.push(format!("RRULE:{}", parts.join(";")));
        write!(f, "{}", output.join("\n"))
    }
}

/// `construct_byset` — filter byxxx values that are reachable from `start`
/// with the given `interval` modulo `base`.
fn construct_byset(
    start: i64,
    byxxx: &[u8],
    base: i64,
    interval: i64,
) -> Result<Vec<u8>, RRuleError> {
    let i_gcd = gcd(interval, base);
    let mut set: Vec<u8> = Vec::new();
    for &num in byxxx {
        // Use divmod rather than % to handle negative nums correctly
        let diff = num as i64 - start;
        let rem = ((diff % i_gcd) + i_gcd) % i_gcd;
        if i_gcd == 1 || rem == 0 {
            set.push(num);
        }
    }
    if set.is_empty() {
        return Err(RRuleError::EmptyBySet);
    }
    Ok(set)
}

/// Calculate the next value in a sequence where FREQ matches BYXXX level.
pub(crate) fn mod_distance(value: i64, byxxx: &[u8], base: i64, interval: i64) -> Option<(i64, i64)> {
    let mut acc = 0i64;
    let mut val = value;
    for _ in 1..=base {
        let (div, new_val) = {
            let sum = val + interval;
            let d = sum.div_euclid(base);
            let m = sum.rem_euclid(base);
            (d, m)
        };
        acc += div;
        val = new_val;
        if byxxx.contains(&(val as u8)) {
            return Some((acc, val));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// RRuleSet
// ---------------------------------------------------------------------------

/// A composite recurrence set (rruleset) combining multiple rrules, rdates,
/// exrules, and exdates.
#[derive(Debug, Clone)]
pub struct RRuleSet {
    rrules: Vec<RRule>,
    rdates: Vec<NaiveDateTime>,
    exrules: Vec<RRule>,
    exdates: Vec<NaiveDateTime>,
}

impl RRuleSet {
    pub fn new() -> Self {
        Self {
            rrules: Vec::new(),
            rdates: Vec::new(),
            exrules: Vec::new(),
            exdates: Vec::new(),
        }
    }

    pub fn rrule(&mut self, rule: RRule) {
        self.rrules.push(rule);
    }

    pub fn rdate(&mut self, dt: NaiveDateTime) {
        self.rdates.push(dt);
    }

    pub fn exrule(&mut self, rule: RRule) {
        self.exrules.push(rule);
    }

    pub fn exdate(&mut self, dt: NaiveDateTime) {
        self.exdates.push(dt);
    }

    /// Whether all rrules in this set are guaranteed to produce finite sequences.
    pub fn is_finite(&self) -> bool {
        self.rrules.iter().all(|r| r.is_finite())
    }

    pub fn all(&self) -> Vec<NaiveDateTime> {
        self.iter().collect()
    }

    pub fn iter(&self) -> RRuleSetIter {
        RRuleSetIter::new(self)
    }

    pub fn before(&self, dt: NaiveDateTime, inc: bool) -> Option<NaiveDateTime> {
        let mut last = None;
        for i in self.iter() {
            if (inc && i > dt) || (!inc && i >= dt) {
                break;
            }
            last = Some(i);
        }
        last
    }

    pub fn after(&self, dt: NaiveDateTime, inc: bool) -> Option<NaiveDateTime> {
        self.iter().find(|&i| if inc { i >= dt } else { i > dt })
    }

    pub fn between(
        &self,
        after: NaiveDateTime,
        before: NaiveDateTime,
        inc: bool,
    ) -> Vec<NaiveDateTime> {
        let mut result = Vec::new();
        let mut started = false;
        for i in self.iter() {
            if inc {
                if i > before {
                    break;
                }
                if !started {
                    if i >= after {
                        started = true;
                        result.push(i);
                    }
                } else {
                    result.push(i);
                }
            } else {
                if i >= before {
                    break;
                }
                if !started {
                    if i > after {
                        started = true;
                        result.push(i);
                    }
                } else {
                    result.push(i);
                }
            }
        }
        result
    }

    pub fn count_all(&self) -> usize {
        self.iter().count()
    }
}

impl Default for RRuleSet {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// RRuleSetIter — heap-merge of multiple iterators
// ---------------------------------------------------------------------------

struct HeapItem {
    dt: NaiveDateTime,
    iter: Box<dyn Iterator<Item = NaiveDateTime> + Send + Sync>,
}

impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.dt == other.dt
    }
}

impl Eq for HeapItem {}

impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap
        other.dt.cmp(&self.dt)
    }
}

pub struct RRuleSetIter {
    rheap: BinaryHeap<HeapItem>,
    exheap: BinaryHeap<HeapItem>,
    last_dt: Option<NaiveDateTime>,
}

impl RRuleSetIter {
    fn new(set: &RRuleSet) -> Self {
        let mut rheap = BinaryHeap::new();
        let mut exheap = BinaryHeap::new();

        // Add rdate iterators
        let mut rdates = set.rdates.clone();
        rdates.sort();
        let mut rdate_iter = rdates.into_iter();
        if let Some(dt) = rdate_iter.next() {
            rheap.push(HeapItem {
                dt,
                iter: Box::new(rdate_iter),
            });
        }

        // Add rrule iterators
        for rule in &set.rrules {
            let mut rule_iter = rule.iter();
            if let Some(dt) = rule_iter.next() {
                rheap.push(HeapItem {
                    dt,
                    iter: Box::new(rule_iter),
                });
            }
        }

        // Add exdate iterators
        let mut exdates = set.exdates.clone();
        exdates.sort();
        let mut exdate_iter = exdates.into_iter();
        if let Some(dt) = exdate_iter.next() {
            exheap.push(HeapItem {
                dt,
                iter: Box::new(exdate_iter),
            });
        }

        // Add exrule iterators
        for rule in &set.exrules {
            let mut rule_iter = rule.iter();
            if let Some(dt) = rule_iter.next() {
                exheap.push(HeapItem {
                    dt,
                    iter: Box::new(rule_iter),
                });
            }
        }

        RRuleSetIter {
            rheap,
            exheap,
            last_dt: None,
        }
    }
}

impl Iterator for RRuleSetIter {
    type Item = NaiveDateTime;

    fn next(&mut self) -> Option<NaiveDateTime> {
        while let Some(mut ritem) = self.rheap.pop() {
            let dt = ritem.dt;

            // Advance this iterator
            if let Some(next_dt) = ritem.iter.next() {
                ritem.dt = next_dt;
                self.rheap.push(ritem);
            }

            // Skip duplicates
            if self.last_dt == Some(dt) {
                continue;
            }

            // Advance exclusion heap past dt
            while let Some(exitem) = self.exheap.peek() {
                if exitem.dt < dt {
                    let mut exitem = self.exheap.pop().unwrap();
                    if let Some(next_dt) = exitem.iter.next() {
                        exitem.dt = next_dt;
                        self.exheap.push(exitem);
                    }
                } else {
                    break;
                }
            }

            // Check if excluded — consume ALL exclusion sources matching dt
            let excluded = self.exheap.peek().is_some_and(|ex| ex.dt == dt);
            if excluded {
                while self.exheap.peek().is_some_and(|ex| ex.dt == dt) {
                    let mut exitem = self.exheap.pop().unwrap();
                    if let Some(next_dt) = exitem.iter.next() {
                        exitem.dt = next_dt;
                        self.exheap.push(exitem);
                    }
                }
                self.last_dt = Some(dt);
                continue;
            }

            self.last_dt = Some(dt);
            return Some(dt);
        }
        None
    }
}

// ---------------------------------------------------------------------------
// rrulestr — parse RFC 5545 RRULE strings
// ---------------------------------------------------------------------------

pub fn rrulestr(
    s: &str,
    dtstart: Option<NaiveDateTime>,
    forceset: bool,
    compatible: bool,
    unfold: bool,
) -> Result<RRuleStrResult, RRuleError> {
    let parser = RRuleStrParser;
    parser.parse(s, dtstart, forceset, compatible, unfold)
}

/// Result of rrulestr parsing — can be either a single RRule or an RRuleSet.
pub enum RRuleStrResult {
    Single(Box<RRule>),
    Set(RRuleSet),
}

impl RRuleStrResult {
    pub fn all(&self) -> Vec<NaiveDateTime> {
        match self {
            RRuleStrResult::Single(r) => r.all(),
            RRuleStrResult::Set(s) => s.all(),
        }
    }
}

struct RRuleStrParser;

impl RRuleStrParser {
    fn parse(
        &self,
        s: &str,
        dtstart: Option<NaiveDateTime>,
        forceset: bool,
        compatible: bool,
        unfold: bool,
    ) -> Result<RRuleStrResult, RRuleError> {
        let forceset = forceset || compatible;
        let unfold = unfold || compatible;

        let s_upper = s.to_uppercase();
        if s_upper.trim().is_empty() {
            return Err(RRuleError::ValueError("empty string".into()));
        }

        let lines: Vec<String> = if unfold {
            let raw_lines: Vec<&str> = s_upper.lines().collect();
            let mut result: Vec<String> = Vec::new();
            for line in raw_lines {
                let line = line.trim_end();
                if line.is_empty() {
                    continue;
                }
                if line.starts_with(' ') && !result.is_empty() {
                    let last = result.last_mut().unwrap();
                    last.push_str(&line[1..]);
                } else {
                    result.push(line.to_string());
                }
            }
            result
        } else {
            s_upper.split_whitespace().map(|s| s.to_string()).collect()
        };

        // Simple case: single RRULE line
        if !forceset
            && lines.len() == 1
            && (!lines[0].contains(':') || lines[0].starts_with("RRULE:"))
        {
            let rule = self.parse_rfc_rrule(&lines[0], dtstart)?;
            return Ok(RRuleStrResult::Single(Box::new(rule)));
        }

        // Complex case: parse as rruleset
        let mut rrulevals: Vec<String> = Vec::new();
        let mut rdatevals: Vec<String> = Vec::new();
        let mut exrulevals: Vec<String> = Vec::new();
        let mut exdatevals: Vec<NaiveDateTime> = Vec::new();
        let mut dtstart = dtstart;

        for line in &lines {
            if line.is_empty() {
                continue;
            }
            let (name, value) = if line.contains(':') {
                let idx = line.find(':').unwrap();
                (line[..idx].to_string(), line[idx + 1..].to_string())
            } else {
                ("RRULE".to_string(), line.clone())
            };

            let parms: Vec<&str> = name.split(';').collect();
            let name = parms[0];
            let _parms = &parms[1..];

            match name {
                "RRULE" => {
                    rrulevals.push(value);
                }
                "RDATE" => {
                    rdatevals.push(value);
                }
                "EXRULE" => {
                    exrulevals.push(value);
                }
                "EXDATE" => {
                    for datestr in value.split(',') {
                        if let Some(dt) = parse_rfc_datetime(datestr.trim()) {
                            exdatevals.push(dt);
                        }
                    }
                }
                "DTSTART" => {
                    if let Some(dt) = parse_rfc_datetime(&value) {
                        dtstart = Some(dt);
                    }
                }
                _ => {
                    return Err(RRuleError::ValueError(format!(
                        "unsupported property: {name}"
                    )));
                }
            }
        }

        if forceset
            || rrulevals.len() > 1
            || !rdatevals.is_empty()
            || !exrulevals.is_empty()
            || !exdatevals.is_empty()
        {
            let mut rset = RRuleSet::new();
            for value in &rrulevals {
                let rule = self.parse_rfc_rrule(value, dtstart)?;
                rset.rrule(rule);
            }
            for value in &rdatevals {
                for datestr in value.split(',') {
                    if let Some(dt) = parse_rfc_datetime(datestr.trim()) {
                        rset.rdate(dt);
                    }
                }
            }
            for value in &exrulevals {
                let rule = self.parse_rfc_rrule(value, dtstart)?;
                rset.exrule(rule);
            }
            for dt in exdatevals {
                rset.exdate(dt);
            }
            if compatible {
                if let Some(dt) = dtstart {
                    rset.rdate(dt);
                }
            }
            Ok(RRuleStrResult::Set(rset))
        } else if !rrulevals.is_empty() {
            let rule = self.parse_rfc_rrule(&rrulevals[0], dtstart)?;
            Ok(RRuleStrResult::Single(Box::new(rule)))
        } else {
            Err(RRuleError::ValueError("no RRULE found".into()))
        }
    }

    fn parse_rfc_rrule(
        &self,
        line: &str,
        dtstart: Option<NaiveDateTime>,
    ) -> Result<RRule, RRuleError> {
        let value = if line.contains(':') {
            let (name, val) = line.split_once(':').unwrap();
            if name != "RRULE" {
                return Err(RRuleError::ValueError(format!(
                    "unknown parameter name: {name}"
                )));
            }
            val
        } else {
            line
        };

        let mut freq: Option<u8> = None;
        let mut interval: i64 = 1;
        let mut wkst: Option<u8> = None;
        let mut count: Option<i64> = None;
        let mut until: Option<NaiveDateTime> = None;
        let mut bysetpos: Option<Vec<i32>> = None;
        let mut bymonth: Option<Vec<u8>> = None;
        let mut bymonthday: Option<Vec<i32>> = None;
        let mut byyearday: Option<Vec<i32>> = None;
        let mut byeaster: Option<Vec<i32>> = None;
        let mut byweekno: Option<Vec<i32>> = None;
        let mut byweekday: Option<Vec<Weekday>> = None;
        let mut byhour: Option<Vec<u8>> = None;
        let mut byminute: Option<Vec<u8>> = None;
        let mut bysecond: Option<Vec<u8>> = None;

        for pair in value.split(';') {
            let (name, val) = pair.split_once('=').ok_or_else(|| {
                RRuleError::ValueError(format!("invalid RRULE parameter: {pair}"))
            })?;

            match name {
                "FREQ" => {
                    freq = Some(match val {
                        "YEARLY" => YEARLY,
                        "MONTHLY" => MONTHLY,
                        "WEEKLY" => WEEKLY,
                        "DAILY" => DAILY,
                        "HOURLY" => HOURLY,
                        "MINUTELY" => MINUTELY,
                        "SECONDLY" => SECONDLY,
                        _ => {
                            return Err(RRuleError::ValueError(format!(
                                "invalid FREQ: {val}"
                            )));
                        }
                    });
                }
                "INTERVAL" => {
                    interval = val
                        .parse()
                        .map_err(|_| RRuleError::ValueError(format!("invalid INTERVAL: {val}")))?;
                }
                "WKST" => {
                    wkst = Some(parse_weekday_name(val)?);
                }
                "COUNT" => {
                    count = Some(val.parse().map_err(|_| {
                        RRuleError::ValueError(format!("invalid COUNT: {val}"))
                    })?);
                }
                "UNTIL" => {
                    until = Some(parse_rfc_datetime(val).ok_or_else(|| {
                        RRuleError::ValueError(format!("invalid UNTIL: {val}"))
                    })?);
                }
                "BYSETPOS" => {
                    bysetpos = Some(parse_int_list(val)?);
                }
                "BYMONTH" => {
                    bymonth = Some(
                        parse_int_list(val)?
                            .into_iter()
                            .map(|x| x as u8)
                            .collect(),
                    );
                }
                "BYMONTHDAY" => {
                    bymonthday = Some(parse_int_list(val)?);
                }
                "BYYEARDAY" => {
                    byyearday = Some(parse_int_list(val)?);
                }
                "BYEASTER" => {
                    byeaster = Some(parse_int_list(val)?);
                }
                "BYWEEKNO" => {
                    byweekno = Some(parse_int_list(val)?);
                }
                "BYDAY" | "BYWEEKDAY" => {
                    byweekday = Some(parse_weekday_list(val)?);
                }
                "BYHOUR" => {
                    byhour = Some(
                        parse_int_list(val)?
                            .into_iter()
                            .map(|x| x as u8)
                            .collect(),
                    );
                }
                "BYMINUTE" => {
                    byminute = Some(
                        parse_int_list(val)?
                            .into_iter()
                            .map(|x| x as u8)
                            .collect(),
                    );
                }
                "BYSECOND" => {
                    bysecond = Some(
                        parse_int_list(val)?
                            .into_iter()
                            .map(|x| x as u8)
                            .collect(),
                    );
                }
                _ => {
                    return Err(RRuleError::ValueError(format!(
                        "unknown parameter '{name}'"
                    )));
                }
            }
        }

        let freq = freq.ok_or_else(|| RRuleError::ValueError("FREQ is required".into()))?;

        RRule::new(
            freq, dtstart, interval, wkst, count, until, bysetpos, bymonth, bymonthday,
            byyearday, byeaster, byweekno, byweekday, byhour, byminute, bysecond,
        )
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_int_list(s: &str) -> Result<Vec<i32>, RRuleError> {
    s.split(',')
        .map(|x| {
            x.trim()
                .parse::<i32>()
                .map_err(|_| RRuleError::ValueError(format!("invalid integer: {x}")))
        })
        .collect()
}

fn parse_weekday_name(s: &str) -> Result<u8, RRuleError> {
    match s {
        "MO" => Ok(0),
        "TU" => Ok(1),
        "WE" => Ok(2),
        "TH" => Ok(3),
        "FR" => Ok(4),
        "SA" => Ok(5),
        "SU" => Ok(6),
        _ => Err(RRuleError::ValueError(format!("invalid weekday: {s}"))),
    }
}

fn parse_weekday_list(s: &str) -> Result<Vec<Weekday>, RRuleError> {
    let mut result = Vec::new();
    for wday_str in s.split(',') {
        let wday_str = wday_str.trim();
        if wday_str.is_empty() {
            return Err(RRuleError::ValueError(
                "Invalid (empty) BYDAY specification.".into(),
            ));
        }

        if wday_str.contains('(') {
            // TH(+1) format
            let parts: Vec<&str> = wday_str.split('(').collect();
            let w = parse_weekday_name(parts[0])?;
            let n: i32 = parts[1]
                .trim_end_matches(')')
                .parse()
                .map_err(|_| RRuleError::ValueError(format!("invalid BYDAY: {wday_str}")))?;
            result.push(Weekday::new(w, Some(n)));
        } else {
            // +1MO or just MO format
            let mut i = 0;
            for (pos, ch) in wday_str.char_indices() {
                if !matches!(ch, '+' | '-' | '0'..='9') {
                    i = pos;
                    break;
                }
            }
            let n_str = &wday_str[..i];
            let w_str = &wday_str[i..];
            let w = parse_weekday_name(w_str)?;
            let n = if n_str.is_empty() {
                None
            } else {
                Some(
                    n_str
                        .parse::<i32>()
                        .map_err(|_| RRuleError::ValueError(format!("invalid BYDAY: {wday_str}")))?,
                )
            };
            result.push(Weekday::new(w, n));
        }
    }
    Ok(result)
}

/// Parse a datetime in RFC 5545 format: YYYYMMDD or YYYYMMDDTHHmmSS
fn parse_rfc_datetime(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim().trim_end_matches('Z');
    if s.len() == 15 && s.as_bytes().get(8) == Some(&b'T') {
        // YYYYMMDDTHHmmSS
        let year = s[0..4].parse::<i32>().ok()?;
        let month = s[4..6].parse::<u32>().ok()?;
        let day = s[6..8].parse::<u32>().ok()?;
        let hour = s[9..11].parse::<u32>().ok()?;
        let min = s[11..13].parse::<u32>().ok()?;
        let sec = s[13..15].parse::<u32>().ok()?;
        NaiveDate::from_ymd_opt(year, month, day)?
            .and_hms_opt(hour, min, sec)
    } else if s.len() == 8 {
        // YYYYMMDD
        let year = s[0..4].parse::<i32>().ok()?;
        let month = s[4..6].parse::<u32>().ok()?;
        let day = s[6..8].parse::<u32>().ok()?;
        NaiveDate::from_ymd_opt(year, month, day)?
            .and_hms_opt(0, 0, 0)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, mi, s)
            .unwrap()
    }

    #[test]
    fn test_yearly_basic() {
        let rule = RRule::new(
            YEARLY,
            Some(dt(2020, 1, 1, 0, 0, 0)),
            1,
            None,
            Some(3),
            None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2021, 1, 1, 0, 0, 0),
                dt(2022, 1, 1, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_monthly_basic() {
        let rule = RRule::new(
            MONTHLY,
            Some(dt(2020, 1, 31, 0, 0, 0)),
            1,
            None,
            Some(4),
            None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 31, 0, 0, 0),
                dt(2020, 3, 31, 0, 0, 0),
                dt(2020, 5, 31, 0, 0, 0),
                dt(2020, 7, 31, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_weekly_basic() {
        let rule = RRule::new(
            WEEKLY,
            Some(dt(2020, 1, 6, 0, 0, 0)), // Monday
            1,
            None,
            Some(3),
            None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 6, 0, 0, 0),
                dt(2020, 1, 13, 0, 0, 0),
                dt(2020, 1, 20, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_daily_basic() {
        let rule = RRule::new(
            DAILY,
            Some(dt(2020, 1, 1, 0, 0, 0)),
            1,
            None,
            Some(5),
            None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 2, 0, 0, 0),
                dt(2020, 1, 3, 0, 0, 0),
                dt(2020, 1, 4, 0, 0, 0),
                dt(2020, 1, 5, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_daily_until() {
        let rule = RRule::new(
            DAILY,
            Some(dt(2020, 1, 1, 0, 0, 0)),
            1,
            None,
            None,
            Some(dt(2020, 1, 5, 0, 0, 0)),
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 2, 0, 0, 0),
                dt(2020, 1, 3, 0, 0, 0),
                dt(2020, 1, 4, 0, 0, 0),
                dt(2020, 1, 5, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_hourly_basic() {
        let rule = RRule::new(
            HOURLY,
            Some(dt(2020, 1, 1, 0, 0, 0)),
            1,
            None,
            Some(3),
            None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 1, 1, 0, 0),
                dt(2020, 1, 1, 2, 0, 0),
            ]
        );
    }

    #[test]
    fn test_rruleset_basic() {
        let r1 = RRule::new(
            DAILY,
            Some(dt(2020, 1, 1, 0, 0, 0)),
            1,
            None,
            Some(3),
            None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        let mut rset = RRuleSet::new();
        rset.rrule(r1);
        rset.exdate(dt(2020, 1, 2, 0, 0, 0));
        let results = rset.all();
        assert_eq!(
            results,
            vec![dt(2020, 1, 1, 0, 0, 0), dt(2020, 1, 3, 0, 0, 0)]
        );
    }

    #[test]
    fn test_rrulestr_basic() {
        let result = rrulestr(
            "DTSTART:20200101T000000\nRRULE:FREQ=DAILY;COUNT=3",
            None,
            false,
            false,
            false,
        )
        .unwrap();
        let dates = result.all();
        assert_eq!(
            dates,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 2, 0, 0, 0),
                dt(2020, 1, 3, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_bysetpos_invalid() {
        let result = RRule::new(
            MONTHLY,
            Some(dt(2020, 1, 1, 0, 0, 0)),
            1,
            None,
            None,
            None,
            Some(vec![0]),
            None, None, None, None, None, None, None, None, None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_rfc_datetime_basic() {
        assert_eq!(
            parse_rfc_datetime("20200101T120000"),
            Some(dt(2020, 1, 1, 12, 0, 0))
        );
        assert_eq!(
            parse_rfc_datetime("20200101"),
            Some(dt(2020, 1, 1, 0, 0, 0))
        );
    }

    // -----------------------------------------------------------------------
    // Bug fix regression tests
    // -----------------------------------------------------------------------

    /// Bug #1: byeaster filter condition was inverted — days outside the
    /// easter mask were NOT being filtered out.
    #[test]
    fn test_byeaster_filters_non_easter_days() {
        // Easter 2020 = April 12 (day-of-year 103, 0-indexed 102)
        // byeaster=[0] means exactly Easter Sunday.
        // With YEARLY + count=3 we expect 3 consecutive Easter Sundays.
        let rule = RRule::new(
            YEARLY,
            Some(dt(2020, 1, 1, 0, 0, 0)),
            1,
            None,
            Some(3),
            None,
            None,
            None,
            None,
            None,
            Some(vec![0]), // byeaster
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let results = rule.all();
        assert_eq!(results.len(), 3);
        // Easter 2020 = Apr 12, 2021 = Apr 4, 2022 = Apr 17
        assert_eq!(results[0], dt(2020, 4, 12, 0, 0, 0));
        assert_eq!(results[1], dt(2021, 4, 4, 0, 0, 0));
        assert_eq!(results[2], dt(2022, 4, 17, 0, 0, 0));
    }

    /// Bug #1 variant: byeaster with offset (e.g. Good Friday = easter - 2).
    #[test]
    fn test_byeaster_with_offset() {
        let rule = RRule::new(
            YEARLY,
            Some(dt(2020, 1, 1, 0, 0, 0)),
            1,
            None,
            Some(2),
            None,
            None,
            None,
            None,
            None,
            Some(vec![-2]), // Good Friday
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let results = rule.all();
        assert_eq!(results.len(), 2);
        // Good Friday 2020 = Apr 10, 2021 = Apr 2
        assert_eq!(results[0], dt(2020, 4, 10, 0, 0, 0));
        assert_eq!(results[1], dt(2021, 4, 2, 0, 0, 0));
    }

    /// Bug #2: RRuleSetIter only consumed one exclusion source per dt.
    /// When both exdate AND exrule match the same dt, both must be consumed.
    #[test]
    fn test_rruleset_multiple_exclusion_sources() {
        // Daily rule: Jan 1-5
        let r1 = RRule::new(
            DAILY,
            Some(dt(2020, 1, 1, 0, 0, 0)),
            1, None, Some(5), None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();

        // Exrule: daily starting Jan 2 with count=1 (excludes Jan 2)
        let exr = RRule::new(
            DAILY,
            Some(dt(2020, 1, 2, 0, 0, 0)),
            1, None, Some(1), None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();

        let mut rset = RRuleSet::new();
        rset.rrule(r1);
        rset.exrule(exr);
        // Also add Jan 2 as an exdate — same dt excluded by two sources
        rset.exdate(dt(2020, 1, 2, 0, 0, 0));
        // Also exclude Jan 4 via exdate only
        rset.exdate(dt(2020, 1, 4, 0, 0, 0));

        let results = rset.all();
        // Jan 2 excluded (by both exrule + exdate), Jan 4 excluded (exdate)
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 3, 0, 0, 0),
                dt(2020, 1, 5, 0, 0, 0),
            ]
        );
    }

    /// Bug #2 variant: verify subsequent dates after multi-source exclusion
    /// are still generated correctly.
    #[test]
    fn test_rruleset_exclusion_doesnt_skip_subsequent() {
        let r1 = RRule::new(
            DAILY,
            Some(dt(2020, 1, 1, 0, 0, 0)),
            1, None, Some(3), None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();

        // Two exrules both excluding Jan 1
        let exr1 = RRule::new(
            DAILY,
            Some(dt(2020, 1, 1, 0, 0, 0)),
            1, None, Some(1), None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        let exr2 = RRule::new(
            DAILY,
            Some(dt(2020, 1, 1, 0, 0, 0)),
            1, None, Some(1), None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();

        let mut rset = RRuleSet::new();
        rset.rrule(r1);
        rset.exrule(exr1);
        rset.exrule(exr2);

        let results = rset.all();
        // Only Jan 1 excluded, Jan 2 and Jan 3 still present
        assert_eq!(
            results,
            vec![dt(2020, 1, 2, 0, 0, 0), dt(2020, 1, 3, 0, 0, 0)]
        );
    }

    /// Bug #3: is_finite guard for unbounded rules.
    #[test]
    fn test_is_finite() {
        // With count → finite
        let finite_rule = RRule::new(
            DAILY, Some(dt(2020, 1, 1, 0, 0, 0)), 1, None,
            Some(5), None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        assert!(finite_rule.is_finite());

        // With until → finite
        let finite_until = RRule::new(
            DAILY, Some(dt(2020, 1, 1, 0, 0, 0)), 1, None,
            None, Some(dt(2020, 1, 10, 0, 0, 0)),
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        assert!(finite_until.is_finite());

        // No count or until → infinite
        let infinite_rule = RRule::new(
            DAILY, Some(dt(2020, 1, 1, 0, 0, 0)), 1, None,
            None, None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        assert!(!infinite_rule.is_finite());
    }

    /// Bug #3: RRuleSet.is_finite depends on all contained rrules.
    #[test]
    fn test_rruleset_is_finite() {
        let finite_rule = RRule::new(
            DAILY, Some(dt(2020, 1, 1, 0, 0, 0)), 1, None,
            Some(3), None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        let infinite_rule = RRule::new(
            DAILY, Some(dt(2020, 1, 1, 0, 0, 0)), 1, None,
            None, None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();

        let mut set_finite = RRuleSet::new();
        set_finite.rrule(finite_rule.clone());
        assert!(set_finite.is_finite());

        let mut set_infinite = RRuleSet::new();
        set_infinite.rrule(finite_rule);
        set_infinite.rrule(infinite_rule);
        assert!(!set_infinite.is_finite());
    }

    /// Bug #3: Lazy positive indexing on infinite rrule — iter().nth() works.
    #[test]
    fn test_infinite_rrule_lazy_nth() {
        // No count, no until → infinite rule
        let rule = RRule::new(
            DAILY, Some(dt(2020, 1, 1, 0, 0, 0)), 1, None,
            None, None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        assert!(!rule.is_finite());

        // Lazy positive indexing via iterator should work
        let mut it = rule.iter();
        assert_eq!(it.next(), Some(dt(2020, 1, 1, 0, 0, 0)));
        assert_eq!(it.next(), Some(dt(2020, 1, 2, 0, 0, 0)));
        assert_eq!(it.next(), Some(dt(2020, 1, 3, 0, 0, 0)));
    }

    /// Bug #3: Lazy positive indexing on infinite rruleset.
    #[test]
    fn test_infinite_rruleset_lazy_iter() {
        let rule = RRule::new(
            DAILY, Some(dt(2020, 1, 1, 0, 0, 0)), 1, None,
            None, None,
            None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();

        let mut rset = RRuleSet::new();
        rset.rrule(rule);
        assert!(!rset.is_finite());

        // Lazy iteration should work
        let mut it = rset.iter();
        assert_eq!(it.next(), Some(dt(2020, 1, 1, 0, 0, 0)));
        assert_eq!(it.next(), Some(dt(2020, 1, 2, 0, 0, 0)));
        assert_eq!(it.next(), Some(dt(2020, 1, 3, 0, 0, 0)));
    }
}

// ===========================================================================
// PyO3 bindings
// ===========================================================================

#[cfg(feature = "python")]
pub mod python {
    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyList};

    use super::*;
    use crate::common::Weekday;

    // Helper: extract NaiveDateTime from Python datetime
    fn py_datetime_to_naive(dt: &Bound<'_, PyAny>) -> PyResult<NaiveDateTime> {
        let year: i32 = dt.getattr("year")?.extract()?;
        let month: u32 = dt.getattr("month")?.extract()?;
        let day: u32 = dt.getattr("day")?.extract()?;
        let hour: u32 = dt.getattr("hour")?.extract()?;
        let minute: u32 = dt.getattr("minute")?.extract()?;
        let second: u32 = dt.getattr("second")?.extract()?;
        NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|d| d.and_hms_opt(hour, minute, second))
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid datetime: {year}-{month}-{day} {hour}:{minute}:{second}"
                ))
            })
    }

    // Helper: convert NaiveDateTime to Python datetime
    fn naive_to_py_datetime<'py>(
        py: Python<'py>,
        dt: NaiveDateTime,
        tzinfo: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let datetime_mod = py.import("datetime")?;
        let datetime_cls = datetime_mod.getattr("datetime")?;
        let args = (
            dt.year(),
            dt.month(),
            dt.day(),
            dt.hour(),
            dt.minute(),
            dt.second(),
            0i32, // microsecond
            tzinfo,
        );
        datetime_cls.call1(args)
    }

    // Helper: extract optional Vec<Weekday> from Python byweekday param
    fn extract_byweekday(obj: &Bound<'_, PyAny>) -> PyResult<Option<Vec<Weekday>>> {
        if obj.is_none() {
            return Ok(None);
        }
        let mut result = Vec::new();
        if let Ok(val) = obj.extract::<i32>() {
            result.push(Weekday::new(val as u8, None));
        } else if let Ok(wd) = obj.extract::<Weekday>() {
            result.push(wd);
        } else {
            for item in obj.try_iter()? {
                let item: Bound<'_, PyAny> = item?;
                if let Ok(val) = item.extract::<i32>() {
                    result.push(Weekday::new(val as u8, None));
                } else if let Ok(wd) = item.extract::<Weekday>() {
                    result.push(wd);
                } else {
                    return Err(pyo3::exceptions::PyTypeError::new_err(
                        "byweekday must be an int or weekday",
                    ));
                }
            }
        }
        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    fn extract_int_or_seq_i32(obj: &Bound<'_, PyAny>) -> PyResult<Option<Vec<i32>>> {
        if obj.is_none() {
            return Ok(None);
        }
        if let Ok(val) = obj.extract::<i32>() {
            return Ok(Some(vec![val]));
        }
        let mut result = Vec::new();
        for item in obj.try_iter()? {
            let item: Bound<'_, PyAny> = item?;
            result.push(item.extract::<i32>()?);
        }
        if result.is_empty() { Ok(None) } else { Ok(Some(result)) }
    }

    fn extract_int_or_seq_u8(obj: &Bound<'_, PyAny>) -> PyResult<Option<Vec<u8>>> {
        if obj.is_none() {
            return Ok(None);
        }
        if let Ok(val) = obj.extract::<u8>() {
            return Ok(Some(vec![val]));
        }
        let mut result = Vec::new();
        for item in obj.try_iter()? {
            let item: Bound<'_, PyAny> = item?;
            result.push(item.extract::<u8>()?);
        }
        if result.is_empty() { Ok(None) } else { Ok(Some(result)) }
    }

    /// Lazy Python iterator over rrule occurrences.
    #[pyclass]
    pub struct PyRRuleIter {
        inner: iter::RRuleIter,
        tzinfo: Option<Py<PyAny>>,
    }

    #[pymethods]
    impl PyRRuleIter {
        fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
            slf
        }

        fn __next__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
            match self.inner.next() {
                Some(dt) => {
                    let tzinfo = self.tzinfo.as_ref().map(|t| t.bind(py));
                    Ok(Some(naive_to_py_datetime(py, dt, tzinfo)?))
                }
                None => Ok(None),
            }
        }
    }

    /// Lazy Python iterator over rruleset occurrences.
    #[pyclass]
    pub struct PyRRuleSetIter {
        inner: std::sync::Mutex<RRuleSetIter>,
        tzinfo: Option<Py<PyAny>>,
    }

    #[pymethods]
    impl PyRRuleSetIter {
        fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
            slf
        }

        fn __next__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
            let mut inner = self.inner.lock().expect("PyRRuleSetIter mutex poisoned");
            match inner.next() {
                Some(dt) => {
                    let tzinfo = self.tzinfo.as_ref().map(|t| t.bind(py));
                    Ok(Some(naive_to_py_datetime(py, dt, tzinfo)?))
                }
                None => Ok(None),
            }
        }
    }

    /// Python-exposed rrule class.
    #[pyclass(name = "rrule")]
    pub struct PyRRule {
        inner: RRule,
        tzinfo: Option<Py<PyAny>>,
    }

    #[pymethods]
    impl PyRRule {
        #[new]
        #[pyo3(signature = (
            freq,
            dtstart=None,
            interval=1,
            wkst=None,
            count=None,
            until=None,
            bysetpos=None,
            bymonth=None,
            bymonthday=None,
            byyearday=None,
            byeaster=None,
            byweekno=None,
            byweekday=None,
            byhour=None,
            byminute=None,
            bysecond=None,
            cache=false,
        ))]
        #[allow(clippy::too_many_arguments)]
        fn new(
            py: Python<'_>,
            freq: u8,
            dtstart: Option<&Bound<'_, PyAny>>,
            interval: i64,
            wkst: Option<&Bound<'_, PyAny>>,
            count: Option<i64>,
            until: Option<&Bound<'_, PyAny>>,
            bysetpos: Option<&Bound<'_, PyAny>>,
            bymonth: Option<&Bound<'_, PyAny>>,
            bymonthday: Option<&Bound<'_, PyAny>>,
            byyearday: Option<&Bound<'_, PyAny>>,
            byeaster: Option<&Bound<'_, PyAny>>,
            byweekno: Option<&Bound<'_, PyAny>>,
            byweekday: Option<&Bound<'_, PyAny>>,
            byhour: Option<&Bound<'_, PyAny>>,
            byminute: Option<&Bound<'_, PyAny>>,
            bysecond: Option<&Bound<'_, PyAny>>,
            cache: bool,
        ) -> PyResult<Self> {
            let _ = cache;

            let (naive_dtstart, tzinfo) = if let Some(dt_obj) = dtstart {
                let naive = py_datetime_to_naive(dt_obj)?;
                let tz = dt_obj.getattr("tzinfo")?;
                let tz_obj: Option<Py<PyAny>> = if tz.is_none() {
                    None
                } else {
                    Some(tz.unbind())
                };
                (Some(naive), tz_obj)
            } else if let Some(until_obj) = until {
                let tz = until_obj.getattr("tzinfo")?;
                if !tz.is_none() {
                    let datetime_mod = py.import("datetime")?;
                    let datetime_cls = datetime_mod.getattr("datetime")?;
                    let now = datetime_cls.call_method1("now", (&tz,))?;
                    let naive = py_datetime_to_naive(&now)?;
                    (Some(naive), Some(tz.unbind()))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            let wkst_val = if let Some(w) = wkst {
                if let Ok(val) = w.extract::<u8>() {
                    Some(val)
                } else if let Ok(wd) = w.extract::<Weekday>() {
                    Some(wd.weekday())
                } else {
                    None
                }
            } else {
                None
            };

            let naive_until = until.map(py_datetime_to_naive).transpose()?;

            let bysetpos_v = bysetpos.map(extract_int_or_seq_i32).transpose()?.flatten();
            let bymonth_v = bymonth.map(extract_int_or_seq_u8).transpose()?.flatten();
            let bymonthday_v = bymonthday.map(extract_int_or_seq_i32).transpose()?.flatten();
            let byyearday_v = byyearday.map(extract_int_or_seq_i32).transpose()?.flatten();
            let byeaster_v = byeaster.map(extract_int_or_seq_i32).transpose()?.flatten();
            let byweekno_v = byweekno.map(extract_int_or_seq_i32).transpose()?.flatten();
            let byweekday_v = byweekday.map(extract_byweekday).transpose()?.flatten();
            let byhour_v = byhour.map(extract_int_or_seq_u8).transpose()?.flatten();
            let byminute_v = byminute.map(extract_int_or_seq_u8).transpose()?.flatten();
            let bysecond_v = bysecond.map(extract_int_or_seq_u8).transpose()?.flatten();

            let inner = RRule::new(
                freq, naive_dtstart, interval, wkst_val, count, naive_until,
                bysetpos_v, bymonth_v, bymonthday_v, byyearday_v, byeaster_v,
                byweekno_v, byweekday_v, byhour_v, byminute_v, bysecond_v,
            )
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

            Ok(PyRRule { inner, tzinfo })
        }

        fn __iter__(slf: PyRef<'_, Self>) -> PyRRuleIter {
            PyRRuleIter {
                inner: slf.inner.iter(),
                tzinfo: slf.tzinfo.as_ref().map(|t| t.clone_ref(slf.py())),
            }
        }

        fn __getitem__<'py>(&self, py: Python<'py>, item: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            let tzinfo = self.tzinfo.as_ref().map(|t| t.bind(py));

            if let Ok(idx) = item.extract::<isize>() {
                if idx >= 0 {
                    // Positive index: lazy iteration (matches Python behavior)
                    let mut it = self.inner.iter();
                    for _ in 0..idx {
                        if it.next().is_none() {
                            return Err(pyo3::exceptions::PyIndexError::new_err("index out of range"));
                        }
                    }
                    match it.next() {
                        Some(dt) => naive_to_py_datetime(py, dt, tzinfo),
                        None => Err(pyo3::exceptions::PyIndexError::new_err("index out of range")),
                    }
                } else {
                    // Negative index: must materialize all results
                    if !self.inner.is_finite() {
                        return Err(pyo3::exceptions::PyTypeError::new_err(
                            "Negative indexing on an infinite rrule is not supported.",
                        ));
                    }
                    let results = self.inner.all();
                    let actual_idx = (results.len() as isize + idx) as usize;
                    if actual_idx >= results.len() {
                        return Err(pyo3::exceptions::PyIndexError::new_err("index out of range"));
                    }
                    naive_to_py_datetime(py, results[actual_idx], tzinfo)
                }
            } else {
                // Slice access
                let slice = item.cast::<pyo3::types::PySlice>()?;
                let step: isize = slice.getattr("step")?.extract::<Option<isize>>()?.unwrap_or(1);
                let start: Option<isize> = slice.getattr("start")?.extract()?;
                let stop: Option<isize> = slice.getattr("stop")?.extract()?;

                if step > 0 && start.unwrap_or(0) >= 0 && stop.is_some() && stop.unwrap() >= 0 {
                    // Positive slice with explicit stop: lazy islice
                    let s = start.unwrap_or(0) as usize;
                    let e = stop.unwrap() as usize;
                    let step = step as usize;
                    let py_list = PyList::empty(py);
                    let mut it = self.inner.iter();
                    let mut pos = 0usize;
                    while pos < e {
                        match it.next() {
                            Some(dt) => {
                                if pos >= s && (pos - s) % step == 0 {
                                    py_list.append(naive_to_py_datetime(py, dt, tzinfo)?)?;
                                }
                            }
                            None => break,
                        }
                        pos += 1;
                    }
                    Ok(py_list.into_any())
                } else {
                    // Negative step/start/stop or open-ended: materialize
                    if !self.inner.is_finite() {
                        return Err(pyo3::exceptions::PyTypeError::new_err(
                            "Slicing an infinite rrule requires explicit positive start, stop, and step.",
                        ));
                    }
                    let results = self.inner.all();
                    let indices = slice.indices(results.len() as isize)?;
                    let py_list = PyList::empty(py);
                    let mut i = indices.start;
                    while (indices.step > 0 && i < indices.stop)
                        || (indices.step < 0 && i > indices.stop)
                    {
                        py_list.append(naive_to_py_datetime(py, results[i as usize], tzinfo)?)?;
                        i += indices.step;
                    }
                    Ok(py_list.into_any())
                }
            }
        }

        fn __contains__(&self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
            let dt = py_datetime_to_naive(item)?;
            for i in self.inner.iter() {
                if i == dt { return Ok(true); }
                if i > dt { return Ok(false); }
            }
            Ok(false)
        }

        fn __len__(&self) -> PyResult<usize> {
            if !self.inner.is_finite() {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "length of unsized rrule",
                ));
            }
            Ok(self.inner.count_all())
        }

        fn __str__(&self) -> String {
            self.inner.to_string()
        }

        fn __repr__(&self) -> String {
            self.inner.to_string()
        }

        fn count(&self) -> PyResult<usize> {
            if !self.inner.is_finite() {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "length of unsized rrule",
                ));
            }
            Ok(self.inner.count_all())
        }

        #[pyo3(signature = (dt, inc=false))]
        fn before<'py>(&self, py: Python<'py>, dt: &Bound<'py, PyAny>, inc: bool) -> PyResult<Bound<'py, PyAny>> {
            let naive_dt = py_datetime_to_naive(dt)?;
            let tzinfo = self.tzinfo.as_ref().map(|t| t.bind(py));
            match self.inner.before(naive_dt, inc) {
                Some(result) => naive_to_py_datetime(py, result, tzinfo),
                None => Ok(py.None().into_bound(py)),
            }
        }

        #[pyo3(signature = (dt, inc=false))]
        fn after<'py>(&self, py: Python<'py>, dt: &Bound<'py, PyAny>, inc: bool) -> PyResult<Bound<'py, PyAny>> {
            let naive_dt = py_datetime_to_naive(dt)?;
            let tzinfo = self.tzinfo.as_ref().map(|t| t.bind(py));
            match self.inner.after(naive_dt, inc) {
                Some(result) => naive_to_py_datetime(py, result, tzinfo),
                None => Ok(py.None().into_bound(py)),
            }
        }

        #[pyo3(signature = (dt, count=None, inc=false))]
        fn xafter<'py>(
            &self,
            py: Python<'py>,
            dt: &Bound<'py, PyAny>,
            count: Option<usize>,
            inc: bool,
        ) -> PyResult<Bound<'py, PyAny>> {
            let naive_dt = py_datetime_to_naive(dt)?;
            let tzinfo = self.tzinfo.as_ref().map(|t| t.bind(py));
            let py_list = PyList::empty(py);
            let mut n = 0;
            for i in self.inner.iter() {
                let ok = if inc { i >= naive_dt } else { i > naive_dt };
                if ok {
                    if let Some(c) = count {
                        n += 1;
                        if n > c { break; }
                    }
                    py_list.append(naive_to_py_datetime(py, i, tzinfo)?)?;
                }
            }
            py_list.call_method0("__iter__")
        }

        #[pyo3(signature = (after, before, inc=false, count=None))]
        fn between<'py>(
            &self,
            py: Python<'py>,
            after: &Bound<'py, PyAny>,
            before: &Bound<'py, PyAny>,
            inc: bool,
            count: Option<usize>,
        ) -> PyResult<Bound<'py, PyList>> {
            let naive_after = py_datetime_to_naive(after)?;
            let naive_before = py_datetime_to_naive(before)?;
            let tzinfo = self.tzinfo.as_ref().map(|t| t.bind(py));
            let results = self.inner.between(naive_after, naive_before, inc);
            let max_count = count.unwrap_or(usize::MAX);
            let py_list = PyList::empty(py);
            for (i, dt) in results.iter().enumerate() {
                if i >= max_count {
                    break;
                }
                py_list.append(naive_to_py_datetime(py, *dt, tzinfo)?)?;
            }
            Ok(py_list)
        }

        #[getter]
        fn _dtstart<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
            let tzinfo = self.tzinfo.as_ref().map(|t| t.bind(py));
            naive_to_py_datetime(py, self.inner.dtstart, tzinfo)
        }

        #[getter]
        fn _freq(&self) -> u8 { self.inner.freq }

        #[getter]
        fn _interval(&self) -> i64 { self.inner.interval }

        #[getter]
        fn _count(&self) -> Option<i64> { self.inner.count }

        #[getter]
        fn _until<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
            match self.inner.until {
                Some(dt) => {
                    let tzinfo = self.tzinfo.as_ref().map(|t| t.bind(py));
                    naive_to_py_datetime(py, dt, tzinfo)
                }
                None => Ok(py.None().into_bound(py)),
            }
        }

        #[getter]
        fn _wkst(&self) -> u8 { self.inner.wkst }

        #[pyo3(signature = (**kwargs))]
        fn replace<'py>(&self, py: Python<'py>, kwargs: Option<&Bound<'py, PyDict>>) -> PyResult<Bound<'py, PyAny>> {
            let rrule_cls = py.import("dateutil_rs._native")?.getattr("rrule")?;
            let new_kwargs = PyDict::new(py);
            new_kwargs.set_item("freq", self.inner.freq)?;
            new_kwargs.set_item("interval", self.inner.interval)?;
            new_kwargs.set_item("wkst", self.inner.wkst)?;
            let tzinfo = self.tzinfo.as_ref().map(|t| t.bind(py));
            new_kwargs.set_item("dtstart", naive_to_py_datetime(py, self.inner.dtstart, tzinfo)?)?;
            if let Some(c) = self.inner.count {
                new_kwargs.set_item("count", c)?;
            }
            if let Some(u) = self.inner.until {
                new_kwargs.set_item("until", naive_to_py_datetime(py, u, tzinfo)?)?;
            }
            // byxxx parameters from original_rule
            let orig = &self.inner.original_rule;
            if let Some(ref v) = orig.bysetpos {
                new_kwargs.set_item("bysetpos", v.clone())?;
            }
            if let Some(Some(ref v)) = orig.bymonth {
                new_kwargs.set_item("bymonth", v.clone())?;
            }
            if let Some(Some(ref v)) = orig.bymonthday {
                new_kwargs.set_item("bymonthday", v.clone())?;
            }
            if let Some(ref v) = orig.byyearday {
                new_kwargs.set_item("byyearday", v.clone())?;
            }
            if let Some(ref v) = orig.byeaster {
                new_kwargs.set_item("byeaster", v.clone())?;
            }
            if let Some(ref v) = orig.byweekno {
                new_kwargs.set_item("byweekno", v.clone())?;
            }
            if let Some(Some(ref v)) = orig.byweekday {
                let py_list = PyList::empty(py);
                for wd in v {
                    py_list.append(wd.into_pyobject(py)?)?;
                }
                new_kwargs.set_item("byweekday", py_list)?;
            }
            if let Some(ref v) = orig.byhour {
                new_kwargs.set_item("byhour", v.clone())?;
            }
            if let Some(ref v) = orig.byminute {
                new_kwargs.set_item("byminute", v.clone())?;
            }
            if let Some(ref v) = orig.bysecond {
                new_kwargs.set_item("bysecond", v.clone())?;
            }
            // Override with user-provided kwargs
            if let Some(kw) = kwargs {
                new_kwargs.update(kw.as_mapping())?;
            }
            rrule_cls.call((), Some(&new_kwargs))
        }
    }

    /// Python-exposed rruleset class, backed by Rust `RRuleSet`.
    #[pyclass(name = "rruleset")]
    pub struct PyRRuleSet {
        inner: RRuleSet,
        tzinfo: Option<Py<PyAny>>,
    }

    #[pymethods]
    impl PyRRuleSet {
        #[new]
        #[pyo3(signature = (cache=false))]
        fn new(cache: bool) -> Self {
            let _ = cache;
            PyRRuleSet {
                inner: RRuleSet::new(),
                tzinfo: None,
            }
        }

        fn rrule(&mut self, rrule_obj: &Bound<'_, PyAny>) -> PyResult<()> {
            let py_rrule: PyRef<'_, PyRRule> = rrule_obj.extract()?;
            self.inner.rrule(py_rrule.inner.clone());
            if self.tzinfo.is_none() {
                if let Some(ref tz) = py_rrule.tzinfo {
                    self.tzinfo = Some(tz.clone_ref(rrule_obj.py()));
                }
            }
            Ok(())
        }

        fn rdate(&mut self, rdate: &Bound<'_, PyAny>) -> PyResult<()> {
            let naive = py_datetime_to_naive(rdate)?;
            self.inner.rdate(naive);
            if self.tzinfo.is_none() {
                let tz = rdate.getattr("tzinfo")?;
                if !tz.is_none() {
                    self.tzinfo = Some(tz.unbind());
                }
            }
            Ok(())
        }

        fn exrule(&mut self, exrule_obj: &Bound<'_, PyAny>) -> PyResult<()> {
            let py_rrule: PyRef<'_, PyRRule> = exrule_obj.extract()?;
            self.inner.exrule(py_rrule.inner.clone());
            Ok(())
        }

        fn exdate(&mut self, exdate: &Bound<'_, PyAny>) -> PyResult<()> {
            let naive = py_datetime_to_naive(exdate)?;
            self.inner.exdate(naive);
            Ok(())
        }

        fn __iter__(slf: PyRef<'_, Self>) -> PyRRuleSetIter {
            PyRRuleSetIter {
                inner: std::sync::Mutex::new(slf.inner.iter()),
                tzinfo: slf.tzinfo.as_ref().map(|t| t.clone_ref(slf.py())),
            }
        }

        fn __getitem__<'py>(&self, py: Python<'py>, item: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            let tzinfo = self.tzinfo.as_ref().map(|t| t.bind(py));

            if let Ok(idx) = item.extract::<isize>() {
                if idx >= 0 {
                    // Positive index: lazy iteration
                    let mut it = self.inner.iter();
                    for _ in 0..idx {
                        if it.next().is_none() {
                            return Err(pyo3::exceptions::PyIndexError::new_err("index out of range"));
                        }
                    }
                    match it.next() {
                        Some(dt) => naive_to_py_datetime(py, dt, tzinfo),
                        None => Err(pyo3::exceptions::PyIndexError::new_err("index out of range")),
                    }
                } else {
                    // Negative index: must materialize
                    if !self.inner.is_finite() {
                        return Err(pyo3::exceptions::PyTypeError::new_err(
                            "Negative indexing on an infinite rruleset is not supported.",
                        ));
                    }
                    let results = self.inner.all();
                    let actual_idx = (results.len() as isize + idx) as usize;
                    if actual_idx >= results.len() {
                        return Err(pyo3::exceptions::PyIndexError::new_err("index out of range"));
                    }
                    naive_to_py_datetime(py, results[actual_idx], tzinfo)
                }
            } else {
                // Slice access
                let slice = item.cast::<pyo3::types::PySlice>()?;
                let step: isize = slice.getattr("step")?.extract::<Option<isize>>()?.unwrap_or(1);
                let start: Option<isize> = slice.getattr("start")?.extract()?;
                let stop: Option<isize> = slice.getattr("stop")?.extract()?;

                if step > 0 && start.unwrap_or(0) >= 0 && stop.is_some() && stop.unwrap() >= 0 {
                    // Positive slice with explicit stop: lazy islice
                    let s = start.unwrap_or(0) as usize;
                    let e = stop.unwrap() as usize;
                    let step = step as usize;
                    let py_list = PyList::empty(py);
                    let mut it = self.inner.iter();
                    let mut pos = 0usize;
                    while pos < e {
                        match it.next() {
                            Some(dt) => {
                                if pos >= s && (pos - s) % step == 0 {
                                    py_list.append(naive_to_py_datetime(py, dt, tzinfo)?)?;
                                }
                            }
                            None => break,
                        }
                        pos += 1;
                    }
                    Ok(py_list.into_any())
                } else {
                    // Negative step/start/stop or open-ended: materialize
                    if !self.inner.is_finite() {
                        return Err(pyo3::exceptions::PyTypeError::new_err(
                            "Slicing an infinite rruleset requires explicit positive start, stop, and step.",
                        ));
                    }
                    let results = self.inner.all();
                    let indices = slice.indices(results.len() as isize)?;
                    let py_list = PyList::empty(py);
                    let mut i = indices.start;
                    while (indices.step > 0 && i < indices.stop)
                        || (indices.step < 0 && i > indices.stop)
                    {
                        py_list.append(naive_to_py_datetime(py, results[i as usize], tzinfo)?)?;
                        i += indices.step;
                    }
                    Ok(py_list.into_any())
                }
            }
        }

        fn __contains__(&self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
            let dt = py_datetime_to_naive(item)?;
            for i in self.inner.iter() {
                if i == dt { return Ok(true); }
                if i > dt { return Ok(false); }
            }
            Ok(false)
        }

        fn __len__(&self) -> PyResult<usize> {
            if !self.inner.is_finite() {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "length of unsized rruleset",
                ));
            }
            Ok(self.inner.count_all())
        }

        fn count(&self) -> PyResult<usize> {
            if !self.inner.is_finite() {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "length of unsized rruleset",
                ));
            }
            Ok(self.inner.count_all())
        }

        #[pyo3(signature = (dt, inc=false))]
        fn before<'py>(&self, py: Python<'py>, dt: &Bound<'py, PyAny>, inc: bool) -> PyResult<Bound<'py, PyAny>> {
            let naive_dt = py_datetime_to_naive(dt)?;
            let tzinfo = self.tzinfo.as_ref().map(|t| t.bind(py));
            match self.inner.before(naive_dt, inc) {
                Some(result) => naive_to_py_datetime(py, result, tzinfo),
                None => Ok(py.None().into_bound(py)),
            }
        }

        #[pyo3(signature = (dt, inc=false))]
        fn after<'py>(&self, py: Python<'py>, dt: &Bound<'py, PyAny>, inc: bool) -> PyResult<Bound<'py, PyAny>> {
            let naive_dt = py_datetime_to_naive(dt)?;
            let tzinfo = self.tzinfo.as_ref().map(|t| t.bind(py));
            match self.inner.after(naive_dt, inc) {
                Some(result) => naive_to_py_datetime(py, result, tzinfo),
                None => Ok(py.None().into_bound(py)),
            }
        }

        #[pyo3(signature = (after, before, inc=false, count=None))]
        fn between<'py>(
            &self,
            py: Python<'py>,
            after: &Bound<'py, PyAny>,
            before: &Bound<'py, PyAny>,
            inc: bool,
            count: Option<usize>,
        ) -> PyResult<Bound<'py, PyList>> {
            let naive_after = py_datetime_to_naive(after)?;
            let naive_before = py_datetime_to_naive(before)?;
            let tzinfo = self.tzinfo.as_ref().map(|t| t.bind(py));
            let results = self.inner.between(naive_after, naive_before, inc);
            let max_count = count.unwrap_or(usize::MAX);
            let py_list = PyList::empty(py);
            for (i, dt) in results.iter().enumerate() {
                if i >= max_count { break; }
                py_list.append(naive_to_py_datetime(py, *dt, tzinfo)?)?;
            }
            Ok(py_list)
        }
    }

    /// Python-exposed rrulestr function.
    #[pyfunction]
    #[pyo3(name = "rrulestr", signature = (s, dtstart=None, cache=false, unfold=false, forceset=false, compatible=false, ignoretz=false, tzids=None, tzinfos=None))]
    pub fn rrulestr_py<'py>(
        py: Python<'py>,
        s: &str,
        dtstart: Option<&Bound<'py, PyAny>>,
        cache: bool,
        unfold: bool,
        forceset: bool,
        compatible: bool,
        ignoretz: bool,
        tzids: Option<&Bound<'py, PyAny>>,
        tzinfos: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let _ = (cache, ignoretz, tzids, tzinfos);

        let naive_dtstart = dtstart.map(py_datetime_to_naive).transpose()?;

        let result = rrulestr(s, naive_dtstart, forceset, compatible, unfold)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let tzinfo: Option<Py<PyAny>> = dtstart.and_then(|dt| {
            dt.getattr("tzinfo")
                .ok()
                .and_then(|tz| if tz.is_none() { None } else { Some(tz.unbind()) })
        });

        match result {
            RRuleStrResult::Single(rule) => {
                let py_rule = PyRRule { inner: *rule, tzinfo };
                Ok(py_rule.into_pyobject(py)?.into_any())
            }
            RRuleStrResult::Set(set) => {
                let mut py_set = PyRRuleSet::new(false);
                py_set.inner = set;
                py_set.tzinfo = tzinfo;
                Ok(py_set.into_pyobject(py)?.into_any())
            }
        }
    }

    /// Register rrule types and functions with the parent module.
    pub fn register(m: &Bound<'_, pyo3::types::PyModule>) -> PyResult<()> {
        m.add_class::<PyRRule>()?;
        m.add_class::<PyRRuleSet>()?;
        m.add_function(pyo3::wrap_pyfunction!(rrulestr_py, m)?)?;
        m.add("YEARLY", YEARLY)?;
        m.add("MONTHLY", MONTHLY)?;
        m.add("WEEKLY", WEEKLY)?;
        m.add("DAILY", DAILY)?;
        m.add("HOURLY", HOURLY)?;
        m.add("MINUTELY", MINUTELY)?;
        m.add("SECONDLY", SECONDLY)?;
        Ok(())
    }
}
