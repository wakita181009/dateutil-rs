//! RFC 5545 recurrence rules — high-performance v1 implementation.

pub mod iter;
pub mod parse;
pub mod set;

use std::fmt;
use std::sync::Arc;

use chrono::{Datelike, NaiveDateTime, NaiveTime, Timelike};
#[cfg(test)]
use chrono::NaiveDate;
use smallvec::SmallVec;

use crate::common::Weekday;
use crate::error::RRuleError;

// ---------------------------------------------------------------------------
// Frequency enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Frequency {
    Yearly = 0,
    Monthly = 1,
    Weekly = 2,
    Daily = 3,
    Hourly = 4,
    Minutely = 5,
    Secondly = 6,
}

const FREQ_NAMES: [&str; 7] = [
    "YEARLY", "MONTHLY", "WEEKLY", "DAILY", "HOURLY", "MINUTELY", "SECONDLY",
];

impl Frequency {
    pub fn from_name(s: &str) -> Result<Self, RRuleError> {
        match s {
            "YEARLY" => Ok(Self::Yearly),
            "MONTHLY" => Ok(Self::Monthly),
            "WEEKLY" => Ok(Self::Weekly),
            "DAILY" => Ok(Self::Daily),
            "HOURLY" => Ok(Self::Hourly),
            "MINUTELY" => Ok(Self::Minutely),
            "SECONDLY" => Ok(Self::Secondly),
            _ => Err(RRuleError::InvalidFrequency(s.into())),
        }
    }

    #[inline]
    pub fn as_str(self) -> &'static str {
        FREQ_NAMES[self as usize]
    }

    #[inline]
    pub fn is_sub_daily(self) -> bool {
        self >= Self::Hourly
    }
}

impl fmt::Display for Frequency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// SmallVec type alias for byxxx fields
// ---------------------------------------------------------------------------

/// Most rrules have 1-7 values per byxxx field. Inline capacity avoids heap.
pub type ByList<T> = SmallVec<[T; 7]>;

// ---------------------------------------------------------------------------
// Compile-time day/month masks
// ---------------------------------------------------------------------------

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
// Bitflags for tracking explicitly-set fields (for Display)
// ---------------------------------------------------------------------------

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) struct ExplicitFields: u16 {
        const BYSETPOS   = 1 << 0;
        const BYMONTH    = 1 << 1;
        const BYMONTHDAY = 1 << 2;
        const BYYEARDAY  = 1 << 3;
        const BYEASTER   = 1 << 4;
        const BYWEEKNO   = 1 << 5;
        const BYWEEKDAY  = 1 << 6;
        const BYHOUR     = 1 << 7;
        const BYMINUTE   = 1 << 8;
        const BYSECOND   = 1 << 9;
    }
}

// ---------------------------------------------------------------------------
// RRule
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RRule {
    pub(crate) freq: Frequency,
    pub(crate) dtstart: NaiveDateTime,
    pub(crate) interval: u32,
    pub(crate) wkst: u8,
    pub(crate) count: Option<u32>,
    pub(crate) until: Option<NaiveDateTime>,

    pub(crate) bysetpos: Option<ByList<i32>>,
    pub(crate) bymonth: Option<ByList<u8>>,
    pub(crate) bymonthday: ByList<i32>,
    pub(crate) bynmonthday: ByList<i32>,
    pub(crate) byyearday: Option<ByList<i32>>,
    pub(crate) byeaster: Option<ByList<i32>>,
    pub(crate) byweekno: Option<ByList<i32>>,
    pub(crate) byweekday: Option<ByList<u8>>,
    pub(crate) bynweekday: Option<ByList<(u8, i32)>>,
    pub(crate) byhour: Option<ByList<u8>>,
    pub(crate) byminute: Option<ByList<u8>>,
    pub(crate) bysecond: Option<ByList<u8>>,

    pub(crate) timeset: Option<SmallVec<[NaiveTime; 4]>>,

    /// Original weekday values for Display (preserves nth info).
    pub(crate) orig_byweekday: Option<ByList<Weekday>>,
    pub(crate) explicit: ExplicitFields,
}

impl RRule {
    pub fn freq(&self) -> Frequency {
        self.freq
    }

    pub fn dtstart(&self) -> NaiveDateTime {
        self.dtstart
    }

    pub fn is_finite(&self) -> bool {
        self.count.is_some() || self.until.is_some()
    }

    /// Collect all occurrences. Clones self into an Arc internally.
    ///
    /// # Panics
    ///
    /// Panics if the rule is not finite (i.e., neither `count` nor `until` is set).
    pub fn all(&self) -> Vec<NaiveDateTime> {
        assert!(
            self.is_finite(),
            "all() called on infinite RRule (set count or until)"
        );
        self.iter().collect()
    }

    /// Return an iterator. Clones self into an Arc.
    ///
    /// If you plan to create multiple iterators from the same rule,
    /// wrap in `Arc` first and use `RRuleIter::new(arc)` directly.
    pub fn iter(&self) -> iter::RRuleIter {
        iter::RRuleIter::new(Arc::new(self.clone()))
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
        for i in self.iter() {
            let past_end = if inc { i > before } else { i >= before };
            if past_end {
                break;
            }
            let in_range = if inc { i >= after } else { i > after };
            if in_range {
                result.push(i);
            }
        }
        result
    }
}

impl IntoIterator for RRule {
    type Item = NaiveDateTime;
    type IntoIter = iter::RRuleIter;

    /// Consume self and return an iterator without cloning.
    fn into_iter(self) -> iter::RRuleIter {
        iter::RRuleIter::new(Arc::new(self))
    }
}

// ---------------------------------------------------------------------------
// RRuleBuilder
// ---------------------------------------------------------------------------

pub struct RRuleBuilder {
    freq: Frequency,
    dtstart: Option<NaiveDateTime>,
    interval: u32,
    wkst: Option<u8>,
    count: Option<u32>,
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
    pub fn new(freq: Frequency) -> Self {
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
    pub fn interval(mut self, val: u32) -> Self {
        self.interval = val;
        self
    }
    pub fn wkst(mut self, val: u8) -> Self {
        self.wkst = Some(val);
        self
    }
    pub fn count(mut self, val: u32) -> Self {
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
        let dtstart = self.dtstart.unwrap_or_else(|| {
            let now = chrono::Local::now().naive_local();
            now.with_nanosecond(0).unwrap_or(now)
        });
        let dtstart = dtstart.with_nanosecond(0).unwrap_or(dtstart);
        let wkst = self.wkst.unwrap_or(0);
        if wkst > 6 {
            return Err(RRuleError::InvalidWkst(wkst));
        }
        let freq = self.freq;
        let interval = self.interval;
        if interval == 0 {
            return Err(RRuleError::InvalidInterval);
        }

        let mut explicit = ExplicitFields::empty();

        // Validate bysetpos
        let bysetpos = if let Some(pos) = self.bysetpos {
            for &p in &pos {
                if p == 0 || !(-366..=366).contains(&p) {
                    return Err(RRuleError::InvalidBySetPos);
                }
            }
            explicit |= ExplicitFields::BYSETPOS;
            Some(ByList::from_vec(pos))
        } else {
            None
        };

        // Default byxxx when none given
        let mut bymonth = self.bymonth;
        let mut bymonthday = self.bymonthday;
        let mut byweekday = self.byweekday;

        let has_explicit_filter = self.byweekno.is_some()
            || self.byyearday.is_some()
            || bymonthday.is_some()
            || byweekday.is_some()
            || self.byeaster.is_some();

        if !has_explicit_filter {
            match freq {
                Frequency::Yearly => {
                    if bymonth.is_none() {
                        bymonth = Some(vec![dtstart.month() as u8]);
                    } else {
                        explicit |= ExplicitFields::BYMONTH;
                    }
                    bymonthday = Some(vec![dtstart.day() as i32]);
                }
                Frequency::Monthly => {
                    bymonthday = Some(vec![dtstart.day() as i32]);
                }
                Frequency::Weekly => {
                    byweekday = Some(vec![
                        Weekday::new(dtstart.weekday().num_days_from_monday() as u8, None)
                            .expect("weekday from chrono is always valid"),
                    ]);
                }
                _ => {}
            }
        } else {
            if bymonth.is_some() {
                explicit |= ExplicitFields::BYMONTH;
            }
            if bymonthday.is_some() {
                explicit |= ExplicitFields::BYMONTHDAY;
            }
            if byweekday.is_some() {
                explicit |= ExplicitFields::BYWEEKDAY;
            }
        }

        // bymonth
        let bymonth = bymonth.map(|mut v| {
            v.sort();
            v.dedup();
            ByList::from_vec(v)
        });

        // byyearday
        let byyearday = if let Some(mut v) = self.byyearday {
            v.sort();
            v.dedup();
            explicit |= ExplicitFields::BYYEARDAY;
            Some(ByList::from_vec(v))
        } else {
            None
        };

        // byeaster
        let byeaster = if let Some(mut v) = self.byeaster {
            v.sort();
            explicit |= ExplicitFields::BYEASTER;
            Some(ByList::from_vec(v))
        } else {
            None
        };

        // bymonthday -> positive / negative
        let (bymonthday_pos, bynmonthday) = if let Some(bmd) = bymonthday {
            let mut pos: Vec<i32> = bmd.iter().copied().filter(|&x| x > 0).collect();
            let mut neg: Vec<i32> = bmd.iter().copied().filter(|&x| x < 0).collect();
            pos.sort();
            pos.dedup();
            neg.sort();
            neg.dedup();
            (ByList::from_vec(pos), ByList::from_vec(neg))
        } else {
            (ByList::new(), ByList::new())
        };

        // byweekno
        let byweekno = if let Some(mut v) = self.byweekno {
            v.sort();
            v.dedup();
            explicit |= ExplicitFields::BYWEEKNO;
            Some(ByList::from_vec(v))
        } else {
            None
        };

        // byweekday -> plain / nth
        let (byweekday_flat, bynweekday, orig_byweekday) = if let Some(bwd) = byweekday {
            let mut plain: Vec<u8> = Vec::new();
            let mut nth: Vec<(u8, i32)> = Vec::new();

            for wd in &bwd {
                match wd.n() {
                    None | Some(0) => plain.push(wd.weekday()),
                    Some(n) => {
                        if freq > Frequency::Monthly {
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

            let bwd_opt = if plain.is_empty() { None } else { Some(ByList::from_vec(plain)) };
            let bnwd_opt = if nth.is_empty() { None } else { Some(ByList::from_vec(nth)) };
            (bwd_opt, bnwd_opt, Some(ByList::from_vec(bwd)))
        } else {
            (None, None, None)
        };

        // byhour
        let byhour = if let Some(bh) = self.byhour {
            let set = if freq == Frequency::Hourly {
                construct_byset(dtstart.hour() as i64, &bh, 24, interval as i64)?
            } else {
                bh
            };
            let mut v: Vec<u8> = set;
            v.sort();
            v.dedup();
            explicit |= ExplicitFields::BYHOUR;
            Some(ByList::from_vec(v))
        } else if freq < Frequency::Hourly {
            Some(ByList::from_elem(dtstart.hour() as u8, 1))
        } else {
            None
        };

        // byminute
        let byminute = if let Some(bm) = self.byminute {
            let set = if freq == Frequency::Minutely {
                construct_byset(dtstart.minute() as i64, &bm, 60, interval as i64)?
            } else {
                bm
            };
            let mut v: Vec<u8> = set;
            v.sort();
            v.dedup();
            explicit |= ExplicitFields::BYMINUTE;
            Some(ByList::from_vec(v))
        } else if freq < Frequency::Minutely {
            Some(ByList::from_elem(dtstart.minute() as u8, 1))
        } else {
            None
        };

        // bysecond
        let bysecond = if let Some(bs) = self.bysecond {
            let set = if freq == Frequency::Secondly {
                construct_byset(dtstart.second() as i64, &bs, 60, interval as i64)?
            } else {
                bs
            };
            let mut v: Vec<u8> = set;
            v.sort();
            v.dedup();
            explicit |= ExplicitFields::BYSECOND;
            Some(ByList::from_vec(v))
        } else if freq < Frequency::Secondly {
            Some(ByList::from_elem(dtstart.second() as u8, 1))
        } else {
            None
        };

        // Pre-compute timeset for sub-daily frequencies
        let timeset = if freq.is_sub_daily() {
            None
        } else {
            let mut ts = SmallVec::<[NaiveTime; 4]>::new();
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
            count: self.count,
            until: self.until,
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
            orig_byweekday,
            explicit,
        })
    }
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

impl fmt::Display for RRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = Vec::new();
        output.push(self.dtstart.format("DTSTART:%Y%m%dT%H%M%S").to_string());

        let mut parts = vec![format!("FREQ={}", self.freq)];

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

        // BYDAY with nth handling
        if self.explicit.contains(ExplicitFields::BYWEEKDAY) {
            if let Some(ref bwd) = self.orig_byweekday {
                if !bwd.is_empty() {
                    let strs: Vec<String> = bwd
                        .iter()
                        .map(|w| {
                            if let Some(n) = w.n() {
                                if n != 0 {
                                    let name = &["MO", "TU", "WE", "TH", "FR", "SA", "SU"]
                                        [w.weekday() as usize];
                                    return format!("{n:+}{name}");
                                }
                            }
                            w.to_string()
                        })
                        .collect();
                    parts.push(format!("BYDAY={}", strs.join(",")));
                }
            }
        }

        if self.explicit.contains(ExplicitFields::BYSETPOS) {
            if let Some(ref v) = self.bysetpos {
                parts.push(format!("BYSETPOS={}", join_ints(v)));
            }
        }
        if self.explicit.contains(ExplicitFields::BYMONTH) {
            if let Some(ref v) = self.bymonth {
                parts.push(format!("BYMONTH={}", join_ints(v)));
            }
        }
        if self.explicit.contains(ExplicitFields::BYMONTHDAY) {
            let mut combined = SmallVec::<[i32; 14]>::new();
            combined.extend_from_slice(&self.bymonthday);
            combined.extend_from_slice(&self.bynmonthday);
            if !combined.is_empty() {
                parts.push(format!("BYMONTHDAY={}", join_ints(&combined)));
            }
        }
        if self.explicit.contains(ExplicitFields::BYYEARDAY) {
            if let Some(ref v) = self.byyearday {
                parts.push(format!("BYYEARDAY={}", join_ints(v)));
            }
        }
        if self.explicit.contains(ExplicitFields::BYWEEKNO) {
            if let Some(ref v) = self.byweekno {
                parts.push(format!("BYWEEKNO={}", join_ints(v)));
            }
        }
        if self.explicit.contains(ExplicitFields::BYHOUR) {
            if let Some(ref v) = self.byhour {
                parts.push(format!("BYHOUR={}", join_ints(v)));
            }
        }
        if self.explicit.contains(ExplicitFields::BYMINUTE) {
            if let Some(ref v) = self.byminute {
                parts.push(format!("BYMINUTE={}", join_ints(v)));
            }
        }
        if self.explicit.contains(ExplicitFields::BYSECOND) {
            if let Some(ref v) = self.bysecond {
                parts.push(format!("BYSECOND={}", join_ints(v)));
            }
        }
        if self.explicit.contains(ExplicitFields::BYEASTER) {
            if let Some(ref v) = self.byeaster {
                parts.push(format!("BYEASTER={}", join_ints(v)));
            }
        }

        output.push(format!("RRULE:{}", parts.join(";")));
        write!(f, "{}", output.join("\n"))
    }
}

fn join_ints<T: fmt::Display>(v: &[T]) -> String {
    v.iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(crate) fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

pub(crate) fn construct_byset(
    start: i64,
    byxxx: &[u8],
    base: i64,
    interval: i64,
) -> Result<Vec<u8>, RRuleError> {
    let i_gcd = gcd(interval, base);
    let mut set: Vec<u8> = Vec::new();
    for &num in byxxx {
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

pub(crate) fn mod_distance(value: i64, byxxx: &[u8], base: i64, interval: i64) -> Option<(i64, i64)> {
    let mut acc = 0i64;
    let mut val = value;
    for _ in 1..=base {
        let sum = val + interval;
        let d = sum.div_euclid(base);
        let m = sum.rem_euclid(base);
        acc += d;
        val = m;
        if byxxx.contains(&(val as u8)) {
            return Some((acc, val));
        }
    }
    None
}

#[inline]
pub(crate) fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[inline]
pub(crate) fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, mi, s)
            .unwrap()
    }

    #[test]
    fn test_frequency_from_name() {
        assert_eq!(Frequency::from_name("YEARLY").unwrap(), Frequency::Yearly);
        assert_eq!(Frequency::from_name("SECONDLY").unwrap(), Frequency::Secondly);
        assert!(Frequency::from_name("INVALID").is_err());
    }

    #[test]
    fn test_frequency_ordering() {
        assert!(Frequency::Yearly < Frequency::Monthly);
        assert!(Frequency::Daily < Frequency::Hourly);
        assert!(Frequency::Hourly.is_sub_daily());
        assert!(!Frequency::Daily.is_sub_daily());
    }

    #[test]
    fn test_builder_yearly_basic() {
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .build()
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
    fn test_builder_monthly_basic() {
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2020, 1, 31, 10, 0, 0))
            .count(4)
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 31, 10, 0, 0),
                dt(2020, 3, 31, 10, 0, 0),
                dt(2020, 5, 31, 10, 0, 0),
                dt(2020, 7, 31, 10, 0, 0),
            ]
        );
    }

    #[test]
    fn test_builder_weekly_basic() {
        let rule = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(2020, 1, 6, 9, 0, 0)) // Monday
            .count(3)
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 6, 9, 0, 0),
                dt(2020, 1, 13, 9, 0, 0),
                dt(2020, 1, 20, 9, 0, 0),
            ]
        );
    }

    #[test]
    fn test_builder_daily_basic() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(5)
            .build()
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
    fn test_builder_invalid_bysetpos() {
        let err = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .bysetpos(vec![0])
            .build();
        assert!(err.is_err());
    }

    #[test]
    fn test_builder_invalid_wkst() {
        let err = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .wkst(7)
            .count(1)
            .build();
        assert!(err.is_err(), "wkst=7 should be rejected");

        let err = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .wkst(255)
            .count(1)
            .build();
        assert!(err.is_err(), "wkst=255 should be rejected");

        // wkst=6 (Sunday) should be valid
        let ok = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .wkst(6)
            .count(1)
            .build();
        assert!(ok.is_ok(), "wkst=6 should be valid");
    }

    #[test]
    fn test_builder_invalid_interval() {
        let err = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .interval(0)
            .count(1)
            .build();
        assert!(err.is_err(), "interval=0 should be rejected");

        // interval=1 should be valid
        let ok = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .interval(1)
            .count(1)
            .build();
        assert!(ok.is_ok(), "interval=1 should be valid");
    }

    #[test]
    fn test_yearly_bymonth() {
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .bymonth(vec![1, 3])
            .bymonthday(vec![5, 10])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1998, 1, 5, 9, 0, 0),
                dt(1998, 1, 10, 9, 0, 0),
                dt(1998, 3, 5, 9, 0, 0),
            ]
        );
    }

    #[test]
    fn test_daily_bymonth() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .bymonth(vec![1, 3])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1998, 1, 1, 9, 0, 0),
                dt(1998, 1, 2, 9, 0, 0),
                dt(1998, 1, 3, 9, 0, 0),
            ]
        );
    }

    #[test]
    fn test_before_after() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(10)
            .build()
            .unwrap();
        assert_eq!(rule.after(dt(2020, 1, 3, 0, 0, 0), false), Some(dt(2020, 1, 4, 0, 0, 0)));
        assert_eq!(rule.after(dt(2020, 1, 3, 0, 0, 0), true), Some(dt(2020, 1, 3, 0, 0, 0)));
        assert_eq!(rule.before(dt(2020, 1, 3, 0, 0, 0), false), Some(dt(2020, 1, 2, 0, 0, 0)));
        assert_eq!(rule.before(dt(2020, 1, 3, 0, 0, 0), true), Some(dt(2020, 1, 3, 0, 0, 0)));
    }

    #[test]
    fn test_between() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(10)
            .build()
            .unwrap();
        let results = rule.between(dt(2020, 1, 3, 0, 0, 0), dt(2020, 1, 6, 0, 0, 0), true);
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 3, 0, 0, 0),
                dt(2020, 1, 4, 0, 0, 0),
                dt(2020, 1, 5, 0, 0, 0),
                dt(2020, 1, 6, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_until() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .until(dt(2020, 1, 3, 0, 0, 0))
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 2, 0, 0, 0),
                dt(2020, 1, 3, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_interval() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .interval(3)
            .count(4)
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 4, 0, 0, 0),
                dt(2020, 1, 7, 0, 0, 0),
                dt(2020, 1, 10, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_display_basic() {
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .build()
            .unwrap();
        let s = rule.to_string();
        assert!(s.contains("FREQ=YEARLY"));
        assert!(s.contains("COUNT=3"));
        assert!(s.contains("DTSTART:20200101T000000"));
    }

    #[test]
    fn test_is_finite() {
        let finite = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(5)
            .build()
            .unwrap();
        assert!(finite.is_finite());

        let infinite = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .build()
            .unwrap();
        assert!(!infinite.is_finite());
    }

    // -----------------------------------------------------------------------
    // Hourly / Minutely / Secondly
    // -----------------------------------------------------------------------

    #[test]
    fn test_hourly_basic() {
        let rule = RRuleBuilder::new(Frequency::Hourly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 2, 10, 0, 0),
                dt(1997, 9, 2, 11, 0, 0),
            ]
        );
    }

    #[test]
    fn test_hourly_interval() {
        let rule = RRuleBuilder::new(Frequency::Hourly)
            .dtstart(dt(1997, 9, 2, 6, 0, 0))
            .interval(6)
            .count(3)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(1997, 9, 2, 6, 0, 0),
                dt(1997, 9, 2, 12, 0, 0),
                dt(1997, 9, 2, 18, 0, 0),
            ]
        );
    }

    #[test]
    fn test_minutely_basic() {
        let rule = RRuleBuilder::new(Frequency::Minutely)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 2, 9, 1, 0),
                dt(1997, 9, 2, 9, 2, 0),
            ]
        );
    }

    #[test]
    fn test_secondly_basic() {
        let rule = RRuleBuilder::new(Frequency::Secondly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 2, 9, 0, 1),
                dt(1997, 9, 2, 9, 0, 2),
            ]
        );
    }

    // -----------------------------------------------------------------------
    // byweekday with nth occurrence
    // -----------------------------------------------------------------------

    #[test]
    fn test_monthly_bynweekday() {
        use crate::common::{FR, MO};
        // First Friday and last Monday of each month
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .byweekday(vec![FR.with_n(Some(1)), MO.with_n(Some(-1))])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 5, 9, 0, 0),  // first Friday Sep
                dt(1997, 9, 29, 9, 0, 0), // last Monday Sep
                dt(1997, 10, 3, 9, 0, 0), // first Friday Oct
            ]
        );
    }

    // -----------------------------------------------------------------------
    // bysetpos
    // -----------------------------------------------------------------------

    #[test]
    fn test_monthly_bysetpos_last() {
        use crate::common::MO;
        // Last workday of each month (byweekday=MO-FR, bysetpos=-1)
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .byweekday(vec![
                MO.with_n(None),
                Weekday::new(1, None).unwrap(), // TU
                Weekday::new(2, None).unwrap(), // WE
                Weekday::new(3, None).unwrap(), // TH
                Weekday::new(4, None).unwrap(), // FR
            ])
            .bysetpos(vec![-1])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 30, 9, 0, 0),
                dt(1997, 10, 31, 9, 0, 0),
                dt(1997, 11, 28, 9, 0, 0),
            ]
        );
    }

    // -----------------------------------------------------------------------
    // byyearday
    // -----------------------------------------------------------------------

    #[test]
    fn test_yearly_byyearday() {
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .byyearday(vec![1, 100, 200, 365])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 12, 31, 9, 0, 0), // day 365
                dt(1998, 1, 1, 9, 0, 0),   // day 1
                dt(1998, 4, 10, 9, 0, 0),  // day 100
                dt(1998, 7, 19, 9, 0, 0),  // day 200
            ]
        );
    }

    // -----------------------------------------------------------------------
    // byeaster
    // -----------------------------------------------------------------------

    #[test]
    fn test_yearly_byeaster() {
        // Easter Sunday: 1997=Mar 30, 1998=Apr 12, 1999=Apr 4
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 1, 1, 0, 0, 0))
            .count(3)
            .byeaster(vec![0])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 3, 30, 0, 0, 0),
                dt(1998, 4, 12, 0, 0, 0),
                dt(1999, 4, 4, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_yearly_byeaster_offset() {
        // Good Friday = Easter - 2, Easter Monday = Easter + 1
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 1, 1, 0, 0, 0))
            .count(6)
            .byeaster(vec![-2, 1])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 3, 28, 0, 0, 0),  // Good Friday 1997
                dt(1997, 3, 31, 0, 0, 0),  // Easter Monday 1997
                dt(1998, 4, 10, 0, 0, 0),  // Good Friday 1998
                dt(1998, 4, 13, 0, 0, 0),  // Easter Monday 1998
                dt(1999, 4, 2, 0, 0, 0),   // Good Friday 1999
                dt(1999, 4, 5, 0, 0, 0),   // Easter Monday 1999
            ]
        );
    }

    // -----------------------------------------------------------------------
    // weekly byweekday
    // -----------------------------------------------------------------------

    #[test]
    fn test_weekly_byweekday() {
        use crate::common::{TU, TH};
        let rule = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .byweekday(vec![TU, TH])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 9, 0, 0),  // Tue
                dt(1997, 9, 4, 9, 0, 0),  // Thu
                dt(1997, 9, 9, 9, 0, 0),  // Tue
                dt(1997, 9, 11, 9, 0, 0), // Thu
            ]
        );
    }

    // -----------------------------------------------------------------------
    // RRuleSet
    // -----------------------------------------------------------------------

    #[test]
    fn test_rruleset_basic() {
        let mut rset = set::RRuleSet::new();
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(5)
            .build()
            .unwrap();
        rset.rrule(rule);
        let results = rset.all();
        assert_eq!(results.len(), 5);
        assert_eq!(results[0], dt(2020, 1, 1, 0, 0, 0));
        assert_eq!(results[4], dt(2020, 1, 5, 0, 0, 0));
    }

    #[test]
    fn test_rruleset_exdate() {
        let mut rset = set::RRuleSet::new();
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(5)
            .build()
            .unwrap();
        rset.rrule(rule);
        rset.exdate(dt(2020, 1, 3, 0, 0, 0));
        let results = rset.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 2, 0, 0, 0),
                dt(2020, 1, 4, 0, 0, 0),
                dt(2020, 1, 5, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_rruleset_rdate() {
        let mut rset = set::RRuleSet::new();
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .build()
            .unwrap();
        rset.rrule(rule);
        rset.rdate(dt(2020, 1, 10, 0, 0, 0));
        let results = rset.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 2, 0, 0, 0),
                dt(2020, 1, 3, 0, 0, 0),
                dt(2020, 1, 10, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_rruleset_exrule() {
        let mut rset = set::RRuleSet::new();
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(10)
            .build()
            .unwrap();
        rset.rrule(rule);
        // Exclude every other day
        let exrule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 2, 0, 0, 0))
            .interval(2)
            .count(5)
            .build()
            .unwrap();
        rset.exrule(exrule);
        let results = rset.all();
        // exrule excludes Jan 2, 4, 6, 8, 10
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 3, 0, 0, 0),
                dt(2020, 1, 5, 0, 0, 0),
                dt(2020, 1, 7, 0, 0, 0),
                dt(2020, 1, 9, 0, 0, 0),
            ]
        );
    }

    // -----------------------------------------------------------------------
    // Yearly with interval
    // -----------------------------------------------------------------------

    #[test]
    fn test_yearly_interval() {
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(2000, 1, 1, 0, 0, 0))
            .interval(4)
            .count(3)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(2000, 1, 1, 0, 0, 0),
                dt(2004, 1, 1, 0, 0, 0),
                dt(2008, 1, 1, 0, 0, 0),
            ]
        );
    }

    // -----------------------------------------------------------------------
    // Monthly interval
    // -----------------------------------------------------------------------

    #[test]
    fn test_monthly_interval() {
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2020, 1, 15, 0, 0, 0))
            .interval(3)
            .count(4)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(2020, 1, 15, 0, 0, 0),
                dt(2020, 4, 15, 0, 0, 0),
                dt(2020, 7, 15, 0, 0, 0),
                dt(2020, 10, 15, 0, 0, 0),
            ]
        );
    }

    // -----------------------------------------------------------------------
    // Negative bymonthday
    // -----------------------------------------------------------------------

    #[test]
    fn test_monthly_negative_bymonthday() {
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .bymonthday(vec![-1]) // last day of month
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(1997, 9, 30, 9, 0, 0),
                dt(1997, 10, 31, 9, 0, 0),
                dt(1997, 11, 30, 9, 0, 0),
            ]
        );
    }

    // -----------------------------------------------------------------------
    // Cross-year boundary
    // -----------------------------------------------------------------------

    #[test]
    fn test_daily_cross_year() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 12, 30, 0, 0, 0))
            .count(4)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(2020, 12, 30, 0, 0, 0),
                dt(2020, 12, 31, 0, 0, 0),
                dt(2021, 1, 1, 0, 0, 0),
                dt(2021, 1, 2, 0, 0, 0),
            ]
        );
    }

    // -----------------------------------------------------------------------
    // Hourly crossing day boundary
    // -----------------------------------------------------------------------

    #[test]
    fn test_hourly_cross_day() {
        let rule = RRuleBuilder::new(Frequency::Hourly)
            .dtstart(dt(1997, 9, 2, 22, 0, 0))
            .count(4)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(1997, 9, 2, 22, 0, 0),
                dt(1997, 9, 2, 23, 0, 0),
                dt(1997, 9, 3, 0, 0, 0),
                dt(1997, 9, 3, 1, 0, 0),
            ]
        );
    }
}
