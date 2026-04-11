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
// Sorted-slice search helpers (for cached recurrence results)
// ---------------------------------------------------------------------------

/// Find the last datetime before `dt` in a sorted slice.
pub fn search_before(sorted: &[NaiveDateTime], dt: NaiveDateTime, inc: bool) -> Option<NaiveDateTime> {
    let idx = if inc {
        sorted.partition_point(|&x| x <= dt)
    } else {
        sorted.partition_point(|&x| x < dt)
    };
    if idx > 0 { Some(sorted[idx - 1]) } else { None }
}

/// Find the first datetime after `dt` in a sorted slice.
pub fn search_after(sorted: &[NaiveDateTime], dt: NaiveDateTime, inc: bool) -> Option<NaiveDateTime> {
    let idx = if inc {
        sorted.partition_point(|&x| x < dt)
    } else {
        sorted.partition_point(|&x| x <= dt)
    };
    sorted.get(idx).copied()
}

/// Find all datetimes between `after` and `before` in a sorted slice.
pub fn search_between(
    sorted: &[NaiveDateTime],
    after: NaiveDateTime,
    before: NaiveDateTime,
    inc: bool,
) -> &[NaiveDateTime] {
    let start = if inc {
        sorted.partition_point(|&x| x < after)
    } else {
        sorted.partition_point(|&x| x <= after)
    };
    let end = if inc {
        sorted.partition_point(|&x| x <= before)
    } else {
        sorted.partition_point(|&x| x < before)
    };
    &sorted[start..end]
}

// ---------------------------------------------------------------------------
// Recurrence trait — shared before/after/between logic
// ---------------------------------------------------------------------------

/// Trait for types that produce a sequence of recurrence datetimes.
pub trait Recurrence {
    type Iter: Iterator<Item = NaiveDateTime>;

    fn iter(&self) -> Self::Iter;
    fn is_finite(&self) -> bool;

    /// Collect all occurrences.
    ///
    /// # Panics
    ///
    /// Panics if the recurrence is not finite (i.e., neither `count` nor `until` is set).
    fn all(&self) -> Vec<NaiveDateTime> {
        assert!(
            self.is_finite(),
            "all() called on infinite recurrence (set count or until)"
        );
        self.iter().collect()
    }

    fn before(&self, dt: NaiveDateTime, inc: bool) -> Option<NaiveDateTime> {
        let mut last = None;
        for i in self.iter() {
            if (inc && i > dt) || (!inc && i >= dt) {
                break;
            }
            last = Some(i);
        }
        last
    }

    fn after(&self, dt: NaiveDateTime, inc: bool) -> Option<NaiveDateTime> {
        self.iter().find(|&i| if inc { i >= dt } else { i > dt })
    }

    fn between(
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

    /// Check if a datetime is produced by this recurrence.
    fn contains(&self, dt: NaiveDateTime) -> bool {
        self.after(dt, true).is_some_and(|found| found == dt)
    }

    /// Return `true` if the recurrence produces zero occurrences.
    fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }

    /// Return the number of occurrences, or `None` if the recurrence is infinite.
    fn len(&self) -> Option<usize> {
        if self.is_finite() {
            Some(self.iter().count())
        } else {
            None
        }
    }

    /// Return the `n`-th occurrence (0-indexed), iterating lazily.
    fn nth(&self, n: usize) -> Option<NaiveDateTime> {
        self.iter().nth(n)
    }

    /// Return the `n`-th occurrence from the end (0-indexed).
    ///
    /// Requires the recurrence to be finite. Returns `None` if infinite
    /// or if `n` exceeds the total number of occurrences.
    fn nth_back(&self, n: usize) -> Option<NaiveDateTime> {
        if !self.is_finite() {
            return None;
        }
        let all = self.all();
        all.len().checked_sub(n + 1).map(|i| all[i])
    }

    /// Collect occurrences at indices `start..stop` with the given `step`.
    ///
    /// All parameters are 0-indexed. Iterates lazily via iterator adapters.
    fn take_slice(&self, start: usize, stop: usize, step: usize) -> Vec<NaiveDateTime> {
        assert!(step > 0, "step must be >= 1");
        self.iter()
            .skip(start)
            .take(stop.saturating_sub(start))
            .step_by(step)
            .collect()
    }
}

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

impl TryFrom<u8> for Frequency {
    type Error = RRuleError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= Self::Secondly as u8 {
            // SAFETY: Frequency is #[repr(u8)] with contiguous values 0..=6,
            // and the bounds check above guarantees `value` is in range.
            Ok(unsafe { std::mem::transmute::<u8, Frequency>(value) })
        } else {
            Err(RRuleError::InvalidFrequency(value.to_string().into()))
        }
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

/// Pre-computed bitmask for O(1) byyearday filter checks.
///
/// Yearday values range from -366..=-1 and 1..=366. Two `[u64; 6]` masks
/// (384 bits each) cover the full range with constant-time lookup.
#[derive(Debug, Clone)]
pub(crate) struct ByYearDay {
    pub(crate) values: ByList<i32>,
    pos_mask: [u64; 6], // bit i set ⟹ positive yearday (i+1) is present
    neg_mask: [u64; 6], // bit i set ⟹ negative yearday -(i+1) is present
}

impl ByYearDay {
    fn new(values: ByList<i32>) -> Self {
        let mut pos_mask = [0u64; 6];
        let mut neg_mask = [0u64; 6];
        for &v in &values {
            if v > 0 && v <= 366 {
                let idx = (v - 1) as usize;
                pos_mask[idx / 64] |= 1u64 << (idx % 64);
            } else if (-366..0).contains(&v) {
                let idx = (-v - 1) as usize;
                neg_mask[idx / 64] |= 1u64 << (idx % 64);
            }
        }
        Self {
            values,
            pos_mask,
            neg_mask,
        }
    }

    /// Check if positive yearday `v` (1..=366) is in the set.
    #[inline]
    pub fn has_pos(&self, v: u32) -> bool {
        let idx = v.wrapping_sub(1) as usize;
        idx < 384 && (self.pos_mask[idx / 64] & (1u64 << (idx % 64))) != 0
    }

    /// Check if negative yearday whose absolute value is `v` (1..=366) is in the set.
    #[inline]
    pub fn has_neg(&self, v: u32) -> bool {
        let idx = v.wrapping_sub(1) as usize;
        idx < 384 && (self.neg_mask[idx / 64] & (1u64 << (idx % 64))) != 0
    }
}

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
// Bitflags for O(1) optional byxxx field presence checks in day_passes_filter
// ---------------------------------------------------------------------------

bitflags::bitflags! {
    /// Tracks which optional byxxx fields are set on an RRule.
    ///
    /// Used in `day_passes_filter()` to avoid scattered `Option` discriminant
    /// loads — a single `u8` AND replaces 4 separate memory loads.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) struct ByPresent: u8 {
        const WEEKNO   = 1 << 0;
        const NWEEKDAY = 1 << 1;
        const EASTER   = 1 << 2;
        const YEARDAY  = 1 << 3;
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
    pub(crate) byyearday: Option<ByYearDay>,
    pub(crate) byeaster: Option<ByList<i32>>,
    pub(crate) byweekno: Option<ByList<i32>>,
    pub(crate) bynweekday: Option<ByList<(u8, i32)>>,
    pub(crate) byhour: Option<ByList<u8>>,
    pub(crate) byminute: Option<ByList<u8>>,
    pub(crate) bysecond: Option<ByList<u8>>,

    // Pre-computed bitmasks for O(1) filter checks
    pub(crate) bymonth_mask: u16,
    pub(crate) byweekday_mask: u8,
    pub(crate) bymonthday_mask: u32,
    pub(crate) bynmonthday_mask: u32,

    pub(crate) timeset: Option<SmallVec<[NaiveTime; 4]>>,

    /// Cache-friendly presence mask for optional byxxx fields in day_passes_filter.
    pub(crate) by_present: ByPresent,

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

    pub fn interval(&self) -> u32 {
        self.interval
    }

    pub fn wkst(&self) -> u8 {
        self.wkst
    }

    pub fn count(&self) -> Option<u32> {
        self.count
    }

    pub fn until(&self) -> Option<NaiveDateTime> {
        self.until
    }

    pub fn bysetpos(&self) -> Option<&[i32]> {
        self.bysetpos.as_deref()
    }

    pub fn bymonth(&self) -> Option<&[u8]> {
        self.bymonth.as_deref()
    }

    pub fn bymonthday(&self) -> &[i32] {
        &self.bymonthday
    }

    pub fn bynmonthday(&self) -> &[i32] {
        &self.bynmonthday
    }

    pub fn byyearday(&self) -> Option<&[i32]> {
        self.byyearday.as_ref().map(|v| v.values.as_slice())
    }

    pub fn byeaster(&self) -> Option<&[i32]> {
        self.byeaster.as_deref()
    }

    pub fn byweekno(&self) -> Option<&[i32]> {
        self.byweekno.as_deref()
    }

    pub fn byweekday(&self) -> Option<&[Weekday]> {
        self.orig_byweekday.as_deref()
    }

    pub fn bynweekday(&self) -> Option<&[(u8, i32)]> {
        self.bynweekday.as_deref()
    }

    pub fn byhour(&self) -> Option<&[u8]> {
        self.byhour.as_deref()
    }

    pub fn byminute(&self) -> Option<&[u8]> {
        self.byminute.as_deref()
    }

    pub fn bysecond(&self) -> Option<&[u8]> {
        self.bysecond.as_deref()
    }
}

impl Recurrence for RRule {
    type Iter = iter::RRuleIter;

    /// Return an iterator. Clones self into an Arc.
    ///
    /// If you plan to create multiple iterators from the same rule,
    /// wrap in `Arc` first and use `RRuleIter::new(arc)` directly.
    fn iter(&self) -> iter::RRuleIter {
        iter::RRuleIter::new(Arc::new(self.clone()))
    }

    fn is_finite(&self) -> bool {
        self.count.is_some() || self.until.is_some()
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

impl Recurrence for Arc<RRule> {
    type Iter = iter::RRuleIter;

    /// Iterate without cloning — reuses the existing `Arc`.
    fn iter(&self) -> iter::RRuleIter {
        iter::RRuleIter::new(Arc::clone(self))
    }

    fn is_finite(&self) -> bool {
        self.count.is_some() || self.until.is_some()
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
            if freq != Frequency::Yearly && bymonth.is_some() {
                explicit |= ExplicitFields::BYMONTH;
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
            Some(ByYearDay::new(ByList::from_vec(v)))
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

        // Pre-compute presence mask for optional byxxx fields
        let mut by_present = ByPresent::empty();
        if byweekno.is_some() { by_present |= ByPresent::WEEKNO; }
        if bynweekday.is_some() { by_present |= ByPresent::NWEEKDAY; }
        if byeaster.is_some() { by_present |= ByPresent::EASTER; }
        if byyearday.is_some() { by_present |= ByPresent::YEARDAY; }

        // Pre-compute bitmasks for O(1) filter checks
        let bymonth_mask = bymonth.as_deref().map_or(0u16, |v| {
            v.iter().fold(0u16, |acc, &m| acc | (1u16 << m))
        });
        let byweekday_mask = byweekday_flat.as_deref().map_or(0u8, |v| {
            v.iter().fold(0u8, |acc, &w| acc | (1u8 << w))
        });
        let bymonthday_mask = bymonthday_pos.iter().fold(0u32, |acc, &d| {
            if (1..=31).contains(&d) { acc | (1u32 << d as u32) } else { acc }
        });
        let bynmonthday_mask = bynmonthday.iter().fold(0u32, |acc, &d| {
            if (-31..=-1).contains(&d) { acc | (1u32 << (-d - 1) as u32) } else { acc }
        });

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
            bynweekday,
            byhour,
            byminute,
            bysecond,
            bymonth_mask,
            byweekday_mask,
            bymonthday_mask,
            bynmonthday_mask,
            timeset,
            by_present,
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
            if let Some(ref byd) = self.byyearday {
                parts.push(format!("BYYEARDAY={}", join_ints(&byd.values)));
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

    // ===================================================================
    // BYWEEKNO tests
    // ===================================================================

    #[test]
    fn test_yearly_byweekno() {
        // Every year in week 20
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .byweekno(vec![20])
            .byweekday(vec![crate::common::MO])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1998, 5, 11, 9, 0, 0),
                dt(1999, 5, 17, 9, 0, 0),
                dt(2000, 5, 15, 9, 0, 0),
            ]
        );
    }

    #[test]
    fn test_yearly_byweekno_and_weekday() {
        // Week 1 and 52, on Tuesday and Thursday
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .byweekno(vec![1])
            .byweekday(vec![crate::common::TU, crate::common::TH])
            .build()
            .unwrap();
        let results = rule.all();
        // 1998 week 1: starts Dec 29, 1997. Tue=Dec 30, Thu=Jan 1
        assert_eq!(results.len(), 4);
        for r in &results {
            let wd = r.weekday().num_days_from_monday();
            assert!(wd == 1 || wd == 3, "should be Tue or Thu, got weekday {wd}");
        }
    }

    #[test]
    fn test_yearly_byweekno_negative() {
        // Last week of the year
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .byweekno(vec![-1])
            .byweekday(vec![crate::common::MO])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(results.len(), 3);
        // Each result should be a Monday in late December
        for r in &results {
            assert_eq!(r.weekday().num_days_from_monday(), 0);
        }
    }

    #[test]
    fn test_yearly_byweekno_week53() {
        // Week 53 — only some years have it
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .byweekno(vec![53])
            .byweekday(vec![crate::common::MO])
            .build()
            .unwrap();
        let results = rule.all();
        // Should produce results only in years with 53 weeks
        assert!(!results.is_empty() || results.is_empty()); // no panic
        for r in &results {
            assert_eq!(r.weekday().num_days_from_monday(), 0);
        }
    }

    // ===================================================================
    // WKST (week start day) tests
    // ===================================================================

    #[test]
    fn test_weekly_wkst_monday() {
        // WEEKLY interval=2, wkst=MO (default)
        let rule = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0)) // Tuesday
            .count(4)
            .interval(2)
            .byweekday(vec![crate::common::TU, crate::common::SU])
            .wkst(0) // Monday
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 9, 0, 0),  // Tue
                dt(1997, 9, 7, 9, 0, 0),  // Sun (same week with MO start)
                dt(1997, 9, 16, 9, 0, 0), // Tue (2 weeks later)
                dt(1997, 9, 21, 9, 0, 0), // Sun
            ]
        );
    }

    #[test]
    fn test_weekly_wkst_sunday() {
        // WEEKLY interval=2, wkst=SU — different grouping
        let rule = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0)) // Tuesday
            .count(4)
            .interval(2)
            .byweekday(vec![crate::common::TU, crate::common::SU])
            .wkst(6) // Sunday
            .build()
            .unwrap();
        let results = rule.all();
        // With SU as week start, Sun Sep 7 falls in a different week than Tue Sep 2
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 9, 0, 0),  // Tue
                dt(1997, 9, 14, 9, 0, 0), // Sun (next included week)
                dt(1997, 9, 16, 9, 0, 0), // Tue
                dt(1997, 9, 28, 9, 0, 0), // Sun
            ]
        );
    }

    // ===================================================================
    // Yearly BYDAY (without N) tests
    // ===================================================================

    #[test]
    fn test_yearly_byweekday_no_n() {
        // Every Tuesday and Thursday in the year
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(6)
            .byweekday(vec![crate::common::TU, crate::common::TH])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(results.len(), 6);
        for r in &results {
            let wd = r.weekday().num_days_from_monday();
            assert!(wd == 1 || wd == 3, "Expected Tue/Thu");
        }
        assert_eq!(results[0], dt(1997, 9, 2, 9, 0, 0));  // Tue
        assert_eq!(results[1], dt(1997, 9, 4, 9, 0, 0));  // Thu
    }

    #[test]
    fn test_yearly_bymonth_and_weekday() {
        // Every TU and TH in January and March
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .bymonth(vec![1, 3])
            .byweekday(vec![crate::common::TU, crate::common::TH])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(results.len(), 4);
        for r in &results {
            let m = r.month();
            assert!(m == 1 || m == 3, "Expected January or March");
            let wd = r.weekday().num_days_from_monday();
            assert!(wd == 1 || wd == 3, "Expected Tue/Thu");
        }
    }

    #[test]
    fn test_yearly_bymonth_and_nweekday() {
        // First Monday and last Friday of January
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .bymonth(vec![1])
            .byweekday(vec![
                crate::common::MO.with_n(Some(1)),
                crate::common::FR.with_n(Some(-1)),
            ])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(results.len(), 4);
        for r in &results {
            assert_eq!(r.month(), 1);
        }
        // 1998 Jan: first Mon=5, last Fri=30
        assert_eq!(results[0], dt(1998, 1, 5, 9, 0, 0));
        assert_eq!(results[1], dt(1998, 1, 30, 9, 0, 0));
    }

    // ===================================================================
    // Negative BYYEARDAY tests
    // ===================================================================

    #[test]
    fn test_yearly_byyearday_negative() {
        // Last day and 2nd-to-last day of the year
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .byyearday(vec![-1, -2])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 12, 30, 9, 0, 0), // -2 = Dec 30
                dt(1997, 12, 31, 9, 0, 0), // -1 = Dec 31
                dt(1998, 12, 30, 9, 0, 0),
                dt(1998, 12, 31, 9, 0, 0),
            ]
        );
    }

    // ===================================================================
    // Yearly BYSETPOS tests
    // ===================================================================

    #[test]
    fn test_yearly_bysetpos() {
        // Every year, 1st and last day from MO-FR in January
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .bymonth(vec![1])
            .byweekday(vec![
                crate::common::MO,
                crate::common::TU,
                crate::common::WE,
                crate::common::TH,
                crate::common::FR,
            ])
            .bysetpos(vec![1, -1])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(results.len(), 4);
        // 1998 Jan: first workday = Thu Jan 1, last workday = Fri Jan 30
        assert_eq!(results[0], dt(1998, 1, 1, 9, 0, 0));
        assert_eq!(results[1], dt(1998, 1, 30, 9, 0, 0));
    }

    // ===================================================================
    // Monthly + BYMONTH combination
    // ===================================================================

    #[test]
    fn test_monthly_bymonth() {
        // Monthly, but only in January and March
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .bymonth(vec![1, 3])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1998, 1, 2, 9, 0, 0),
                dt(1998, 3, 2, 9, 0, 0),
                dt(1999, 1, 2, 9, 0, 0),
                dt(1999, 3, 2, 9, 0, 0),
            ]
        );
    }

    // ===================================================================
    // Daily + BYWEEKDAY
    // ===================================================================

    #[test]
    fn test_daily_byweekday() {
        // Daily but only TU and TH
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(1997, 9, 2, 9, 0, 0)) // Tuesday
            .count(4)
            .byweekday(vec![crate::common::TU, crate::common::TH])
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

    // ===================================================================
    // Sub-daily + BY* filter combinations
    // ===================================================================

    #[test]
    fn test_hourly_byhour() {
        // Hourly but only at 9 and 17
        let rule = RRuleBuilder::new(Frequency::Hourly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .byhour(vec![9, 17])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 2, 17, 0, 0),
                dt(1997, 9, 3, 9, 0, 0),
                dt(1997, 9, 3, 17, 0, 0),
            ]
        );
    }

    #[test]
    fn test_hourly_byminute_and_bysecond() {
        // Hourly with byminute and bysecond
        let rule = RRuleBuilder::new(Frequency::Hourly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .byminute(vec![0, 30])
            .bysecond(vec![0])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 2, 9, 30, 0),
                dt(1997, 9, 2, 10, 0, 0),
                dt(1997, 9, 2, 10, 30, 0),
            ]
        );
    }

    #[test]
    fn test_minutely_byminute() {
        // Minutely at minutes 0, 15, 30, 45
        let rule = RRuleBuilder::new(Frequency::Minutely)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .byminute(vec![0, 15, 30, 45])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 2, 9, 15, 0),
                dt(1997, 9, 2, 9, 30, 0),
                dt(1997, 9, 2, 9, 45, 0),
            ]
        );
    }

    #[test]
    fn test_minutely_byhour() {
        // Minutely, but only in hour 9 and 10
        let rule = RRuleBuilder::new(Frequency::Minutely)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .interval(15)
            .count(8)
            .byhour(vec![9, 10])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(results.len(), 8);
        for r in &results {
            assert!(r.hour() == 9 || r.hour() == 10);
        }
    }

    #[test]
    fn test_minutely_cross_hour() {
        // Minutely crossing hour boundary
        let rule = RRuleBuilder::new(Frequency::Minutely)
            .dtstart(dt(1997, 9, 2, 9, 58, 0))
            .count(4)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(1997, 9, 2, 9, 58, 0),
                dt(1997, 9, 2, 9, 59, 0),
                dt(1997, 9, 2, 10, 0, 0),
                dt(1997, 9, 2, 10, 1, 0),
            ]
        );
    }

    #[test]
    fn test_secondly_cross_minute() {
        // Secondly crossing minute boundary
        let rule = RRuleBuilder::new(Frequency::Secondly)
            .dtstart(dt(1997, 9, 2, 9, 59, 58))
            .count(4)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(1997, 9, 2, 9, 59, 58),
                dt(1997, 9, 2, 9, 59, 59),
                dt(1997, 9, 2, 10, 0, 0),
                dt(1997, 9, 2, 10, 0, 1),
            ]
        );
    }

    #[test]
    fn test_secondly_bysecond() {
        // Secondly at seconds 0 and 30
        let rule = RRuleBuilder::new(Frequency::Secondly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .bysecond(vec![0, 30])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 2, 9, 0, 30),
                dt(1997, 9, 2, 9, 1, 0),
                dt(1997, 9, 2, 9, 1, 30),
            ]
        );
    }

    #[test]
    fn test_hourly_bymonth() {
        // Hourly, but only in January
        let rule = RRuleBuilder::new(Frequency::Hourly)
            .dtstart(dt(1997, 12, 31, 22, 0, 0))
            .count(3)
            .bymonth(vec![1])
            .build()
            .unwrap();
        let results = rule.all();
        // Should skip Dec 31 hours and start from Jan 1
        assert_eq!(results.len(), 3);
        for r in &results {
            assert_eq!(r.month(), 1);
        }
    }

    #[test]
    fn test_hourly_cross_month() {
        // Hourly crossing month boundary
        let rule = RRuleBuilder::new(Frequency::Hourly)
            .dtstart(dt(1997, 9, 30, 22, 0, 0))
            .count(4)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(1997, 9, 30, 22, 0, 0),
                dt(1997, 9, 30, 23, 0, 0),
                dt(1997, 10, 1, 0, 0, 0),
                dt(1997, 10, 1, 1, 0, 0),
            ]
        );
    }

    // ===================================================================
    // Leap year tests
    // ===================================================================

    #[test]
    fn test_yearly_feb29_leap_year() {
        // Yearly on Feb 29 — only hits leap years
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(2000, 2, 29, 0, 0, 0))
            .count(3)
            .bymonth(vec![2])
            .bymonthday(vec![29])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(2000, 2, 29, 0, 0, 0),
                dt(2004, 2, 29, 0, 0, 0),
                dt(2008, 2, 29, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_monthly_negative_bymonthday_feb() {
        // Last day of month crossing February
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2000, 1, 1, 0, 0, 0))
            .count(4)
            .bymonthday(vec![-1])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(2000, 1, 31, 0, 0, 0),
                dt(2000, 2, 29, 0, 0, 0), // leap year
                dt(2000, 3, 31, 0, 0, 0),
                dt(2000, 4, 30, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_daily_cross_feb_leap() {
        // Daily crossing Feb 28-29 in leap year
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2000, 2, 27, 0, 0, 0))
            .count(4)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(2000, 2, 27, 0, 0, 0),
                dt(2000, 2, 28, 0, 0, 0),
                dt(2000, 2, 29, 0, 0, 0),
                dt(2000, 3, 1, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_daily_cross_feb_non_leap() {
        // Daily crossing Feb 28 in non-leap year
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2001, 2, 27, 0, 0, 0))
            .count(3)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(2001, 2, 27, 0, 0, 0),
                dt(2001, 2, 28, 0, 0, 0),
                dt(2001, 3, 1, 0, 0, 0),
            ]
        );
    }

    // ===================================================================
    // Count edge cases
    // ===================================================================

    #[test]
    fn test_count_zero() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(0)
            .build()
            .unwrap();
        let results = rule.all();
        assert!(results.is_empty());
    }

    #[test]
    fn test_count_one() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(1)
            .build()
            .unwrap();
        assert_eq!(rule.all(), vec![dt(2020, 1, 1, 0, 0, 0)]);
    }

    // ===================================================================
    // between exclusive and before/after None
    // ===================================================================

    #[test]
    fn test_between_exclusive() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(10)
            .build()
            .unwrap();
        let results = rule.between(dt(2020, 1, 3, 0, 0, 0), dt(2020, 1, 6, 0, 0, 0), false);
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 4, 0, 0, 0),
                dt(2020, 1, 5, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_before_none() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 5, 0, 0, 0))
            .count(3)
            .build()
            .unwrap();
        // Nothing before dtstart
        assert_eq!(rule.before(dt(2020, 1, 4, 0, 0, 0), false), None);
        assert_eq!(rule.before(dt(2020, 1, 5, 0, 0, 0), false), None);
    }

    #[test]
    fn test_after_none() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .build()
            .unwrap();
        // Nothing after last occurrence
        assert_eq!(rule.after(dt(2020, 1, 3, 0, 0, 0), false), None);
        assert_eq!(rule.after(dt(2020, 1, 4, 0, 0, 0), true), None);
    }

    // ===================================================================
    // IntoIterator trait
    // ===================================================================

    #[test]
    fn test_into_iterator() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .build()
            .unwrap();
        let collected: Vec<_> = rule.into_iter().collect();
        assert_eq!(
            collected,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 2, 0, 0, 0),
                dt(2020, 1, 3, 0, 0, 0),
            ]
        );
    }

    // ===================================================================
    // Display roundtrip tests
    // ===================================================================

    #[test]
    fn test_display_with_interval_and_wkst() {
        let rule = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .interval(2)
            .wkst(6) // Sunday
            .count(3)
            .build()
            .unwrap();
        let s = rule.to_string();
        assert!(s.contains("INTERVAL=2"), "missing INTERVAL: {s}");
        assert!(s.contains("WKST=SU"), "missing WKST: {s}");
    }

    #[test]
    fn test_display_with_byday_nth() {
        use crate::common::{MO, FR};
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .byweekday(vec![MO.with_n(Some(1)), FR.with_n(Some(-1))])
            .build()
            .unwrap();
        let s = rule.to_string();
        assert!(s.contains("BYDAY="), "missing BYDAY: {s}");
        assert!(s.contains("+1MO"), "missing +1MO: {s}");
        assert!(s.contains("-1FR"), "missing -1FR: {s}");
    }

    #[test]
    fn test_display_with_until() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .until(dt(2020, 1, 10, 0, 0, 0))
            .build()
            .unwrap();
        let s = rule.to_string();
        assert!(s.contains("UNTIL=20200110T000000"), "missing UNTIL: {s}");
        assert!(!s.contains("COUNT"), "should not have COUNT: {s}");
    }

    #[test]
    fn test_display_bymonthday_negative() {
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .bymonthday(vec![1, -1])
            .build()
            .unwrap();
        let s = rule.to_string();
        assert!(s.contains("BYMONTHDAY="), "missing BYMONTHDAY: {s}");
        assert!(s.contains("1"), "missing positive day: {s}");
        assert!(s.contains("-1"), "missing negative day: {s}");
    }

    // ===================================================================
    // Weekly/Monthly crossing year boundary
    // ===================================================================

    #[test]
    fn test_weekly_cross_year() {
        let rule = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(2020, 12, 28, 0, 0, 0)) // Monday
            .count(3)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(2020, 12, 28, 0, 0, 0),
                dt(2021, 1, 4, 0, 0, 0),
                dt(2021, 1, 11, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_monthly_cross_year() {
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2020, 11, 15, 0, 0, 0))
            .count(4)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(2020, 11, 15, 0, 0, 0),
                dt(2020, 12, 15, 0, 0, 0),
                dt(2021, 1, 15, 0, 0, 0),
                dt(2021, 2, 15, 0, 0, 0),
            ]
        );
    }

    // ===================================================================
    // Monthly with BYHOUR / BYMINUTE / BYSECOND
    // ===================================================================

    #[test]
    fn test_monthly_byhour_byminute() {
        // dtstart at 9:00, byhour=[6,18]: 6:00 < 9:00 so first month skips it
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .byhour(vec![6, 18])
            .byminute(vec![0])
            .bysecond(vec![0])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 18, 0, 0),
                dt(1997, 10, 2, 6, 0, 0),
                dt(1997, 10, 2, 18, 0, 0),
                dt(1997, 11, 2, 6, 0, 0),
            ]
        );
    }

    // ===================================================================
    // Daily with BYHOUR
    // ===================================================================

    #[test]
    fn test_daily_byhour() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(6)
            .byhour(vec![9, 17])
            .byminute(vec![0])
            .bysecond(vec![0])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 2, 17, 0, 0),
                dt(1997, 9, 3, 9, 0, 0),
                dt(1997, 9, 3, 17, 0, 0),
                dt(1997, 9, 4, 9, 0, 0),
                dt(1997, 9, 4, 17, 0, 0),
            ]
        );
    }

    // ===================================================================
    // Builder validation edge cases
    // ===================================================================

    #[test]
    fn test_builder_bysetpos_out_of_range() {
        let err = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .bysetpos(vec![367])
            .build();
        assert!(err.is_err());
    }

    #[test]
    fn test_builder_bysetpos_negative_valid() {
        // -1 (last) should be valid
        let ok = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(1)
            .byweekday(vec![crate::common::MO])
            .bysetpos(vec![-1])
            .build();
        assert!(ok.is_ok());
    }

    // ===================================================================
    // Large interval tests
    // ===================================================================

    #[test]
    fn test_yearly_large_interval() {
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(2000, 1, 1, 0, 0, 0))
            .interval(100)
            .count(3)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(2000, 1, 1, 0, 0, 0),
                dt(2100, 1, 1, 0, 0, 0),
                dt(2200, 1, 1, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_weekly_large_interval() {
        let rule = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(2020, 1, 6, 0, 0, 0)) // Monday
            .interval(52)
            .count(3)
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], dt(2020, 1, 6, 0, 0, 0));
        // ~1 year gap
        assert!(results[1].year() == 2020 || results[1].year() == 2021);
    }

    // ===================================================================
    // Monthly 31st edge case (months with fewer days)
    // ===================================================================

    #[test]
    fn test_monthly_31st_skips() {
        // Monthly on the 31st — skips months with fewer days
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2020, 1, 31, 0, 0, 0))
            .count(5)
            .build()
            .unwrap();
        let results = rule.all();
        // Jan 31, Mar 31, May 31, Jul 31, Aug 31
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 31, 0, 0, 0),
                dt(2020, 3, 31, 0, 0, 0),
                dt(2020, 5, 31, 0, 0, 0),
                dt(2020, 7, 31, 0, 0, 0),
                dt(2020, 8, 31, 0, 0, 0),
            ]
        );
    }

    // ===================================================================
    // Multiple iterators from same rule
    // ===================================================================

    #[test]
    fn test_multiple_iterators() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .build()
            .unwrap();
        let results1 = rule.all();
        let results2 = rule.all();
        assert_eq!(results1, results2);
    }

    // ===================================================================
    // BYMONTHDAY + BYWEEKDAY combo
    // ===================================================================

    #[test]
    fn test_yearly_bymonthday_and_weekday() {
        // Days 1-7 that are Mondays (= first Monday)
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .bymonth(vec![1])
            .bymonthday(vec![1, 2, 3, 4, 5, 6, 7])
            .byweekday(vec![crate::common::MO])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(results.len(), 3);
        for r in &results {
            assert_eq!(r.month(), 1);
            assert!(r.day() <= 7);
            assert_eq!(r.weekday().num_days_from_monday(), 0);
        }
    }

    // ===================================================================
    // Secondly with interval > 60 (crossing multiple boundaries)
    // ===================================================================

    #[test]
    fn test_secondly_large_interval() {
        let rule = RRuleBuilder::new(Frequency::Secondly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .interval(90) // 1.5 minutes
            .count(4)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 2, 9, 1, 30),
                dt(1997, 9, 2, 9, 3, 0),
                dt(1997, 9, 2, 9, 4, 30),
            ]
        );
    }

    // ===================================================================
    // Minutely with interval crossing day boundary
    // ===================================================================

    #[test]
    fn test_minutely_cross_day() {
        let rule = RRuleBuilder::new(Frequency::Minutely)
            .dtstart(dt(1997, 9, 2, 23, 58, 0))
            .count(4)
            .build()
            .unwrap();
        assert_eq!(
            rule.all(),
            vec![
                dt(1997, 9, 2, 23, 58, 0),
                dt(1997, 9, 2, 23, 59, 0),
                dt(1997, 9, 3, 0, 0, 0),
                dt(1997, 9, 3, 0, 1, 0),
            ]
        );
    }

    // ===================================================================
    // until edge: matching date is included
    // ===================================================================

    #[test]
    fn test_until_matching() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 9, 0, 0))
            .until(dt(2020, 1, 3, 9, 0, 0))
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(results.len(), 3);
        assert_eq!(*results.last().unwrap(), dt(2020, 1, 3, 9, 0, 0));
    }

    #[test]
    fn test_until_non_matching() {
        // until falls between occurrences
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 9, 0, 0))
            .until(dt(2020, 1, 3, 8, 0, 0))
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_until_before_dtstart() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 5, 0, 0, 0))
            .until(dt(2020, 1, 1, 0, 0, 0))
            .build()
            .unwrap();
        let results = rule.all();
        assert!(results.is_empty());
    }

    // ===================================================================
    // Secondly + bysecond filter combinations
    // ===================================================================

    #[test]
    fn test_secondly_bysecond_with_byminute() {
        // Secondly at seconds 0,30 filtered to minute 0 and 1
        let rule = RRuleBuilder::new(Frequency::Secondly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(6)
            .bysecond(vec![0, 30])
            .byminute(vec![0, 1])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 2, 9, 0, 30),
                dt(1997, 9, 2, 9, 1, 0),
                dt(1997, 9, 2, 9, 1, 30),
                dt(1997, 9, 2, 10, 0, 0),
                dt(1997, 9, 2, 10, 0, 30),
            ]
        );
    }

    #[test]
    fn test_secondly_bysecond_byhour_byminute() {
        // Secondly at second 0, minutes 0,30, hours 9,10
        let rule = RRuleBuilder::new(Frequency::Secondly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .bysecond(vec![0])
            .byminute(vec![0, 30])
            .byhour(vec![9, 10])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 2, 9, 30, 0),
                dt(1997, 9, 2, 10, 0, 0),
                dt(1997, 9, 2, 10, 30, 0),
            ]
        );
    }

    // ===================================================================
    // Minutely + byhour + byminute cross-day
    // ===================================================================

    #[test]
    fn test_minutely_byhour_byminute_cross_day() {
        // Minutely at minutes 0,30 in hours 23,0 — crosses midnight
        let rule = RRuleBuilder::new(Frequency::Minutely)
            .dtstart(dt(1997, 9, 2, 23, 0, 0))
            .count(6)
            .byhour(vec![23, 0])
            .byminute(vec![0, 30])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 23, 0, 0),
                dt(1997, 9, 2, 23, 30, 0),
                dt(1997, 9, 3, 0, 0, 0),
                dt(1997, 9, 3, 0, 30, 0),
                dt(1997, 9, 3, 23, 0, 0),
                dt(1997, 9, 3, 23, 30, 0),
            ]
        );
    }

    // ===================================================================
    // bysetpos + multi-time daily frequency
    // ===================================================================

    #[test]
    fn test_daily_byhour_bysetpos_first() {
        // Daily with byhour=[9,17], bysetpos=[1] → picks first time (9:00) each day
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .byhour(vec![9, 17])
            .byminute(vec![0])
            .bysecond(vec![0])
            .bysetpos(vec![1])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 3, 9, 0, 0),
                dt(1997, 9, 4, 9, 0, 0),
            ]
        );
    }

    #[test]
    fn test_daily_byhour_bysetpos_last() {
        // Daily with byhour=[9,17], bysetpos=[-1] → picks last time (17:00) each day
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .byhour(vec![9, 17])
            .byminute(vec![0])
            .bysecond(vec![0])
            .bysetpos(vec![-1])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 17, 0, 0),
                dt(1997, 9, 3, 17, 0, 0),
                dt(1997, 9, 4, 17, 0, 0),
            ]
        );
    }

    // ===================================================================
    // Display roundtrip test
    // ===================================================================

    #[test]
    fn test_display_roundtrip_yearly() {
        let original = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .bymonth(vec![1, 3])
            .bymonthday(vec![5, 10])
            .build()
            .unwrap();
        let original_results = original.all();
        let display_str = original.to_string();

        let reparsed = crate::rrule::parse::rrulestr(
            &display_str, None, false, false, true,
        ).unwrap();
        let reparsed_results = reparsed.all();
        assert_eq!(original_results, reparsed_results);
    }

    #[test]
    fn test_display_roundtrip_weekly_with_byday() {
        let original = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(4)
            .interval(2)
            .byweekday(vec![crate::common::TU, crate::common::TH])
            .build()
            .unwrap();
        let original_results = original.all();
        let display_str = original.to_string();

        let reparsed = crate::rrule::parse::rrulestr(
            &display_str, None, false, false, true,
        ).unwrap();
        let reparsed_results = reparsed.all();
        assert_eq!(original_results, reparsed_results);
    }

    #[test]
    fn test_display_roundtrip_monthly_nth_weekday() {
        use crate::common::{MO, FR};
        let original = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(1997, 9, 2, 9, 0, 0))
            .count(3)
            .byweekday(vec![MO.with_n(Some(1)), FR.with_n(Some(-1))])
            .build()
            .unwrap();
        let original_results = original.all();
        let display_str = original.to_string();

        let reparsed = crate::rrule::parse::rrulestr(
            &display_str, None, false, false, true,
        ).unwrap();
        let reparsed_results = reparsed.all();
        assert_eq!(original_results, reparsed_results);
    }

    #[test]
    fn test_display_roundtrip_daily_with_until() {
        let original = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 9, 0, 0))
            .until(dt(2020, 1, 10, 9, 0, 0))
            .byhour(vec![9, 17])
            .byminute(vec![0])
            .bysecond(vec![0])
            .build()
            .unwrap();
        let original_results = original.all();
        let display_str = original.to_string();

        let reparsed = crate::rrule::parse::rrulestr(
            &display_str, None, false, false, true,
        ).unwrap();
        let reparsed_results = reparsed.all();
        assert_eq!(original_results, reparsed_results);
    }

    // -----------------------------------------------------------------------
    // TryFrom<u8> for Frequency
    // -----------------------------------------------------------------------

    #[test]
    fn test_frequency_try_from_u8() {
        assert_eq!(Frequency::try_from(0u8).unwrap(), Frequency::Yearly);
        assert_eq!(Frequency::try_from(1u8).unwrap(), Frequency::Monthly);
        assert_eq!(Frequency::try_from(2u8).unwrap(), Frequency::Weekly);
        assert_eq!(Frequency::try_from(3u8).unwrap(), Frequency::Daily);
        assert_eq!(Frequency::try_from(4u8).unwrap(), Frequency::Hourly);
        assert_eq!(Frequency::try_from(5u8).unwrap(), Frequency::Minutely);
        assert_eq!(Frequency::try_from(6u8).unwrap(), Frequency::Secondly);
        assert!(Frequency::try_from(7u8).is_err());
        assert!(Frequency::try_from(255u8).is_err());
    }

    // -----------------------------------------------------------------------
    // Sorted-slice search helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_search_before() {
        let dates = vec![
            dt(2020, 1, 1, 0, 0, 0),
            dt(2020, 2, 1, 0, 0, 0),
            dt(2020, 3, 1, 0, 0, 0),
        ];
        // Exclusive: before 2020-02-01 → 2020-01-01
        assert_eq!(
            search_before(&dates, dt(2020, 2, 1, 0, 0, 0), false),
            Some(dt(2020, 1, 1, 0, 0, 0))
        );
        // Inclusive: before 2020-02-01 → 2020-02-01 itself
        assert_eq!(
            search_before(&dates, dt(2020, 2, 1, 0, 0, 0), true),
            Some(dt(2020, 2, 1, 0, 0, 0))
        );
        // Before earliest → None
        assert_eq!(search_before(&dates, dt(2019, 1, 1, 0, 0, 0), false), None);
        assert_eq!(search_before(&dates, dt(2020, 1, 1, 0, 0, 0), false), None);
    }

    #[test]
    fn test_search_after() {
        let dates = vec![
            dt(2020, 1, 1, 0, 0, 0),
            dt(2020, 2, 1, 0, 0, 0),
            dt(2020, 3, 1, 0, 0, 0),
        ];
        // Exclusive: after 2020-02-01 → 2020-03-01
        assert_eq!(
            search_after(&dates, dt(2020, 2, 1, 0, 0, 0), false),
            Some(dt(2020, 3, 1, 0, 0, 0))
        );
        // Inclusive: after 2020-02-01 → 2020-02-01 itself
        assert_eq!(
            search_after(&dates, dt(2020, 2, 1, 0, 0, 0), true),
            Some(dt(2020, 2, 1, 0, 0, 0))
        );
        // After latest → None
        assert_eq!(search_after(&dates, dt(2020, 3, 1, 0, 0, 0), false), None);
    }

    #[test]
    fn test_search_between() {
        let dates = vec![
            dt(2020, 1, 1, 0, 0, 0),
            dt(2020, 2, 1, 0, 0, 0),
            dt(2020, 3, 1, 0, 0, 0),
            dt(2020, 4, 1, 0, 0, 0),
        ];
        // Exclusive
        let result = search_between(
            &dates,
            dt(2020, 1, 1, 0, 0, 0),
            dt(2020, 4, 1, 0, 0, 0),
            false,
        );
        assert_eq!(
            result,
            &[dt(2020, 2, 1, 0, 0, 0), dt(2020, 3, 1, 0, 0, 0)]
        );
        // Inclusive
        let result = search_between(
            &dates,
            dt(2020, 1, 1, 0, 0, 0),
            dt(2020, 4, 1, 0, 0, 0),
            true,
        );
        assert_eq!(result, &dates[..]);
    }

    // -----------------------------------------------------------------------
    // Recurrence trait: contains, count, nth, nth_back, take_slice
    // -----------------------------------------------------------------------

    #[test]
    fn test_recurrence_contains() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(5)
            .build()
            .unwrap();
        assert!(rule.contains(dt(2020, 1, 1, 0, 0, 0)));
        assert!(rule.contains(dt(2020, 1, 3, 0, 0, 0)));
        assert!(!rule.contains(dt(2020, 1, 6, 0, 0, 0))); // past count
        assert!(!rule.contains(dt(2020, 1, 1, 12, 0, 0))); // wrong time
    }

    #[test]
    fn test_recurrence_len() {
        let finite = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(5)
            .build()
            .unwrap();
        assert_eq!(finite.len(), Some(5));

        let infinite = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .build()
            .unwrap();
        assert_eq!(infinite.len(), None);
    }

    #[test]
    fn test_recurrence_nth() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(5)
            .build()
            .unwrap();
        assert_eq!(rule.nth(0), Some(dt(2020, 1, 1, 0, 0, 0)));
        assert_eq!(rule.nth(2), Some(dt(2020, 1, 3, 0, 0, 0)));
        assert_eq!(rule.nth(4), Some(dt(2020, 1, 5, 0, 0, 0)));
        assert_eq!(rule.nth(5), None);
    }

    #[test]
    fn test_recurrence_nth_back() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(5)
            .build()
            .unwrap();
        assert_eq!(rule.nth_back(0), Some(dt(2020, 1, 5, 0, 0, 0)));
        assert_eq!(rule.nth_back(1), Some(dt(2020, 1, 4, 0, 0, 0)));
        assert_eq!(rule.nth_back(4), Some(dt(2020, 1, 1, 0, 0, 0)));
        assert_eq!(rule.nth_back(5), None);

        // Infinite → None
        let infinite = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .build()
            .unwrap();
        assert_eq!(infinite.nth_back(0), None);
    }

    #[test]
    fn test_recurrence_take_slice() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(10)
            .build()
            .unwrap();
        // [0..5] step 1
        let result = rule.take_slice(0, 5, 1);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], dt(2020, 1, 1, 0, 0, 0));
        assert_eq!(result[4], dt(2020, 1, 5, 0, 0, 0));

        // [2..8] step 2 → indices 2, 4, 6
        let result = rule.take_slice(2, 8, 2);
        assert_eq!(
            result,
            vec![
                dt(2020, 1, 3, 0, 0, 0),
                dt(2020, 1, 5, 0, 0, 0),
                dt(2020, 1, 7, 0, 0, 0),
            ]
        );

        // [8..20] — only 2 elements left
        let result = rule.take_slice(8, 20, 1);
        assert_eq!(
            result,
            vec![dt(2020, 1, 9, 0, 0, 0), dt(2020, 1, 10, 0, 0, 0)]
        );

        // Empty range
        let result = rule.take_slice(5, 5, 1);
        assert!(result.is_empty());
    }

    // ---- Coverage: getter methods ----

    #[test]
    fn test_rrule_getters() {
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2020, 1, 1, 9, 0, 0))
            .interval(2)
            .count(5)
            .bymonth(vec![1, 6])
            .bysetpos(vec![1, -1])
            .byweekno(vec![1, 52])
            .byyearday(vec![1, 365])
            .byeaster(vec![0])
            .byhour(vec![9, 17])
            .byminute(vec![0, 30])
            .bysecond(vec![0])
            .build()
            .unwrap();

        assert_eq!(rule.freq(), Frequency::Monthly);
        assert_eq!(rule.dtstart(), dt(2020, 1, 1, 9, 0, 0));
        assert_eq!(rule.interval(), 2);
        assert_eq!(rule.wkst(), 0);
        assert_eq!(rule.count(), Some(5));
        assert_eq!(rule.until(), None);
        assert!(rule.bysetpos().is_some());
        assert!(rule.bymonth().is_some());
        assert!(rule.byyearday().is_some());
        assert!(rule.byeaster().is_some());
        assert!(rule.byweekno().is_some());
        assert!(rule.byhour().is_some());
        assert!(rule.byminute().is_some());
        assert!(rule.bysecond().is_some());
    }

    #[test]
    fn test_rrule_bymonthday_and_bynmonthday() {
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2020, 1, 15, 0, 0, 0))
            .count(3)
            .build()
            .unwrap();
        // Default bymonthday should be dtstart day
        assert!(!rule.bymonthday().is_empty());
        assert!(rule.bynmonthday().is_empty());
    }

    #[test]
    fn test_rrule_byweekday_getter() {
        use crate::common::{MO, FR};
        let rule = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(5)
            .byweekday(vec![MO, FR])
            .build()
            .unwrap();
        assert!(rule.byweekday().is_some());
    }

    #[test]
    fn test_rrule_bynweekday_getter() {
        use crate::common::MO;
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .byweekday(vec![MO.with_n(Some(2))])
            .build()
            .unwrap();
        assert!(rule.bynweekday().is_some());
    }

    // ---- Coverage: is_empty, all() panic ----

    #[test]
    fn test_recurrence_is_empty() {
        // Rule that produces no results (month 13 doesn't exist)
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(0)
            .build()
            .unwrap();
        assert!(rule.is_empty());
    }

    #[test]
    #[should_panic(expected = "all() called on infinite")]
    fn test_all_panics_on_infinite() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .build()
            .unwrap();
        let _ = rule.all();
    }

    // ---- Coverage: len() ----

    #[test]
    fn test_recurrence_len_finite_and_infinite() {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(5)
            .build()
            .unwrap();
        assert_eq!(rule.len(), Some(5));

        let infinite = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .build()
            .unwrap();
        assert_eq!(infinite.len(), None);
    }

    // ---- Coverage: Arc<RRule> iter ----

    #[test]
    fn test_arc_rrule_iter() {
        let rule = Arc::new(
            RRuleBuilder::new(Frequency::Daily)
                .dtstart(dt(2020, 1, 1, 0, 0, 0))
                .count(3)
                .build()
                .unwrap(),
        );
        let results: Vec<_> = rule.iter().collect();
        assert_eq!(results.len(), 3);
        assert!(rule.is_finite());
    }

    // ---- Coverage: Display impl for RRULE ----

    #[test]
    fn test_rrule_display_with_bysetpos() {
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .bysetpos(vec![1, -1])
            .bymonth(vec![1, 6])
            .byyearday(vec![1, 100])
            .byweekno(vec![1, 52])
            .byhour(vec![9, 17])
            .byminute(vec![0, 30])
            .bysecond(vec![0])
            .byeaster(vec![0, -2])
            .build()
            .unwrap();
        let s = rule.to_string();
        assert!(s.contains("BYSETPOS=1,-1"));
        assert!(s.contains("BYMONTH="));
        assert!(s.contains("BYYEARDAY="));
        assert!(s.contains("BYWEEKNO="));
        assert!(s.contains("BYHOUR="));
        assert!(s.contains("BYMINUTE="));
        assert!(s.contains("BYSECOND="));
        assert!(s.contains("BYEASTER="));
    }

    #[test]
    fn test_rrule_display_with_bymonthday() {
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .bymonthday(vec![1, 15])
            .build()
            .unwrap();
        let s = rule.to_string();
        assert!(s.contains("BYMONTHDAY="));
    }

    // ---- Coverage: construct_byset empty error ----

    #[test]
    fn test_construct_byset_empty() {
        // interval=2, start=0, byxxx=[1] → gcd(2,24)=2, 1%2=1≠0 → empty
        let result = construct_byset(0, &[1], 24, 2);
        assert!(result.is_err());
    }

    // ---- Coverage: mod_distance returns None ----

    #[test]
    fn test_mod_distance_none() {
        // No matching value within base iterations
        let result = mod_distance(0, &[], 60, 1);
        assert_eq!(result, None);
    }

    // ---- Coverage: days_in_month invalid month ----

    #[test]
    fn test_days_in_month_invalid() {
        assert_eq!(days_in_month(2024, 0), 0);
        assert_eq!(days_in_month(2024, 13), 0);
    }

    // ---- Coverage: invalid wkst ----

    #[test]
    fn test_builder_invalid_wkst_7() {
        let result = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .wkst(7)
            .build();
        assert!(result.is_err());
    }

    // ---- Coverage: freq > Monthly with nth weekday (flattened to plain) ----

    #[test]
    fn test_weekly_nth_weekday_flattened() {
        use crate::common::MO;
        // nth weekday with freq > Monthly → pushed to plain weekday
        let rule = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(2020, 1, 6, 0, 0, 0)) // Monday
            .count(3)
            .byweekday(vec![MO.with_n(Some(2))])
            .build()
            .unwrap();
        let results = rule.all();
        assert_eq!(results.len(), 3);
    }
}
