//! RRule iteration — IterInfo masks and RRuleIter state machine.

use std::sync::Arc;

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use smallvec::SmallVec;

use super::*;

// ---------------------------------------------------------------------------
// IterInfo — cached year/month masks with reusable buffers
// ---------------------------------------------------------------------------

pub(crate) struct IterInfo {
    pub rule: Arc<RRule>,

    pub yearlen: u16,
    pub nextyearlen: u16,
    pub yearordinal: i32,
    pub yearweekday: u8,

    pub mmask: &'static [u8],
    pub mdaymask: &'static [i32],
    pub nmdaymask: &'static [i32],
    pub wdaymask: &'static [u8],
    pub mrange: &'static [usize; 13],

    wnomask_buf: Vec<u8>,
    nwdaymask_buf: Vec<u8>,
    eastermask_buf: Vec<u8>,
    wnomask_active: bool,
    nwdaymask_active: bool,
    eastermask_active: bool,

    lastyear: Option<i32>,
    lastmonth: Option<u32>,
}

impl IterInfo {
    pub fn new(rule: Arc<RRule>) -> Self {
        Self {
            rule,
            yearlen: 0,
            nextyearlen: 0,
            yearordinal: 0,
            yearweekday: 0,
            mmask: &[],
            mdaymask: &[],
            nmdaymask: &[],
            wdaymask: &[],
            mrange: &M365RANGE,
            wnomask_buf: Vec::new(),
            nwdaymask_buf: Vec::new(),
            eastermask_buf: Vec::new(),
            wnomask_active: false,
            nwdaymask_active: false,
            eastermask_active: false,
            lastyear: None,
            lastmonth: None,
        }
    }

    pub fn rebuild(&mut self, year: i32, month: u32) {
        if self.lastyear != Some(year) {
            let leap = is_leap_year(year);
            self.yearlen = if leap { 366 } else { 365 };
            self.nextyearlen = if is_leap_year(year + 1) { 366 } else { 365 };

            let first_yday = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
            self.yearordinal = first_yday.num_days_from_ce();
            self.yearweekday = first_yday.weekday().num_days_from_monday() as u8;

            let wday = self.yearweekday as usize;
            if leap {
                self.mmask = &M366_MASK;
                self.mdaymask = &MDAY366_MASK;
                self.nmdaymask = &NMDAY366_MASK;
                self.mrange = &M366RANGE;
            } else {
                self.mmask = &M365_MASK;
                self.mdaymask = &MDAY365_MASK;
                self.nmdaymask = &NMDAY365_MASK;
                self.mrange = &M365RANGE;
            }
            self.wdaymask = &WDAY_MASK[wday..];

            self.rebuild_wnomask(year);
            self.rebuild_eastermask(year);
        }

        self.rebuild_nwdaymask(year, month);

        self.lastyear = Some(year);
        self.lastmonth = Some(month);
    }

    fn rebuild_wnomask(&mut self, year: i32) {
        let rr = &self.rule;
        if let Some(ref byweekno) = rr.byweekno {
            let ylen = self.yearlen as usize;
            self.wnomask_buf.clear();
            self.wnomask_buf.resize(ylen + 7, 0);

            let firstwkst = (7 - self.yearweekday + rr.wkst) % 7;
            let mut no1wkst = firstwkst as usize;

            let wyearlen = if no1wkst >= 4 {
                no1wkst = 0;
                ylen + (self.yearweekday as usize + 7 - rr.wkst as usize) % 7
            } else {
                ylen - no1wkst
            };

            let (div, modd) = (wyearlen / 7, wyearlen % 7);
            let numweeks = div + modd / 4;

            for &n in byweekno.iter() {
                let mut n = n;
                if n < 0 {
                    n += numweeks as i32 + 1;
                }
                if !(1..=numweeks as i32).contains(&n) {
                    continue;
                }
                let mut i = if n > 1 {
                    let mut idx = no1wkst + (n as usize - 1) * 7;
                    if no1wkst != firstwkst as usize {
                        idx -= 7 - firstwkst as usize;
                    }
                    idx
                } else {
                    no1wkst
                };
                for _ in 0..7 {
                    if i < self.wnomask_buf.len() {
                        self.wnomask_buf[i] = 1;
                    }
                    i += 1;
                    if i < self.wdaymask.len() && self.wdaymask[i] == rr.wkst {
                        break;
                    }
                }
            }

            // Week 1 of next year
            if byweekno.contains(&1) {
                let mut i = no1wkst + numweeks * 7;
                if no1wkst != firstwkst as usize {
                    i -= 7 - firstwkst as usize;
                }
                if i < ylen {
                    for _ in 0..7 {
                        if i < self.wnomask_buf.len() {
                            self.wnomask_buf[i] = 1;
                        }
                        i += 1;
                        if i < self.wdaymask.len() && self.wdaymask[i] == rr.wkst {
                            break;
                        }
                    }
                }
            }

            // Last week of previous year
            if no1wkst > 0 && !byweekno.contains(&-1) {
                let lyearweekday = NaiveDate::from_ymd_opt(year - 1, 1, 1)
                    .unwrap()
                    .weekday()
                    .num_days_from_monday() as u8;
                let lno1wkst = (7 - lyearweekday + rr.wkst) % 7;
                let lyearlen = if is_leap_year(year - 1) { 366usize } else { 365 };
                let lnumweeks = if lno1wkst >= 4 {
                    52 + (lyearlen + (lyearweekday as usize + 7 - rr.wkst as usize) % 7) % 7 / 4
                } else {
                    52 + (ylen - no1wkst) % 7 / 4
                };

                if byweekno.contains(&(lnumweeks as i32)) {
                    for entry in self.wnomask_buf.iter_mut().take(no1wkst) {
                        *entry = 1;
                    }
                }
            } else if no1wkst > 0 && byweekno.contains(&-1) {
                for entry in self.wnomask_buf.iter_mut().take(no1wkst) {
                    *entry = 1;
                }
            }

            self.wnomask_active = true;
        } else {
            self.wnomask_active = false;
        }
    }

    fn rebuild_nwdaymask(&mut self, year: i32, month: u32) {
        let rr = &self.rule;
        let bynweekday = match rr.bynweekday.as_ref() {
            Some(v) if self.lastmonth != Some(month) || self.lastyear != Some(year) => v,
            _ => {
                if rr.bynweekday.is_none() {
                    self.nwdaymask_active = false;
                }
                return;
            }
        };

        let ylen = self.yearlen as usize;
        let mut ranges: SmallVec<[(usize, usize); 4]> = SmallVec::new();

        if rr.freq == Frequency::Yearly {
            if let Some(ref bymonth) = rr.bymonth {
                for &m in bymonth.iter() {
                    let start = self.mrange[m as usize - 1];
                    let end = self.mrange[m as usize];
                    ranges.push((start, end));
                }
            } else {
                ranges.push((0, ylen));
            }
        } else if rr.freq == Frequency::Monthly {
            let start = self.mrange[month as usize - 1];
            let end = self.mrange[month as usize];
            ranges.push((start, end));
        }

        if !ranges.is_empty() {
            self.nwdaymask_buf.clear();
            self.nwdaymask_buf.resize(ylen, 0);

            for &(first, end) in &ranges {
                let last = end - 1;
                for &(wday, n) in bynweekday.iter() {
                    let i = if n < 0 {
                        let mut idx = last as i64 + (n as i64 + 1) * 7;
                        idx -= (self.wdaymask[idx as usize] as i64 - wday as i64 + 7) % 7;
                        idx as usize
                    } else {
                        let mut idx = first as i64 + (n as i64 - 1) * 7;
                        idx += (7 - self.wdaymask[idx as usize] as i64 + wday as i64) % 7;
                        idx as usize
                    };
                    if first <= i && i <= last {
                        self.nwdaymask_buf[i] = 1;
                    }
                }
            }
            self.nwdaymask_active = true;
        }
    }

    fn rebuild_eastermask(&mut self, year: i32) {
        let rr = &self.rule;
        if let Some(ref byeaster) = rr.byeaster {
            let ylen = self.yearlen as usize;
            self.eastermask_buf.clear();
            self.eastermask_buf.resize(ylen + 7, 0);

            if let Ok(easter_date) =
                crate::easter::easter(year, crate::easter::EasterMethod::Western)
            {
                let eyday = easter_date.num_days_from_ce() as i64 - self.yearordinal as i64;
                for &offset in byeaster.iter() {
                    let idx = (eyday + offset as i64) as usize;
                    if idx < self.eastermask_buf.len() {
                        self.eastermask_buf[idx] = 1;
                    }
                }
            }
            self.eastermask_active = true;
        } else {
            self.eastermask_active = false;
        }
    }

    /// Returns (start, end) range for the current frequency period.
    fn period_range(&self, _year: i32, month: u32, day: u32) -> (usize, usize) {
        match self.rule.freq {
            Frequency::Yearly => (0, self.yearlen as usize),
            Frequency::Monthly => {
                let start = self.mrange[month as usize - 1];
                let end = self.mrange[month as usize];
                (start, end)
            }
            Frequency::Weekly => {
                // Compute day-of-year directly from mrange (avoids NaiveDate construction)
                let i = self.mrange[month as usize - 1] + day as usize - 1;
                let start = i;
                let mut end = i;
                for _ in 0..7 {
                    end += 1;
                    if end < self.wdaymask.len() && self.wdaymask[end] == self.rule.wkst {
                        break;
                    }
                }
                (start, end)
            }
            _ => {
                // Daily / Hourly / Minutely / Secondly
                let i = self.mrange[month as usize - 1] + day as usize - 1;
                (i, i + 1)
            }
        }
    }

    /// Single inline predicate: does day index `i` pass all byxxx filters?
    #[inline]
    fn day_passes_filter(&self, i: usize) -> bool {
        let rr = &self.rule;

        if rr.bymonth_mask != 0 && (rr.bymonth_mask & (1u16 << self.mmask[i])) == 0 {
            return false;
        }
        if rr.byweekno.is_some() && (!self.wnomask_active || self.wnomask_buf[i] == 0) {
            return false;
        }
        if rr.byweekday_mask != 0 && (rr.byweekday_mask & (1u8 << self.wdaymask[i])) == 0 {
            return false;
        }
        if rr.bynweekday.is_some()
            && self.nwdaymask_active
            && (i >= self.nwdaymask_buf.len() || self.nwdaymask_buf[i] == 0)
        {
            return false;
        }
        if rr.byeaster.is_some()
            && (!self.eastermask_active
                || i >= self.eastermask_buf.len()
                || self.eastermask_buf[i] == 0)
        {
            return false;
        }
        if (rr.bymonthday_mask != 0 || rr.bynmonthday_mask != 0)
            && (rr.bymonthday_mask & (1u32 << self.mdaymask[i] as u32)) == 0
            && (rr.bynmonthday_mask & (1u32 << (-self.nmdaymask[i] - 1) as u32)) == 0
        {
            return false;
        }
        if let Some(ref byd) = rr.byyearday {
            let ylen = self.yearlen as usize;
            if i < ylen {
                if !byd.has_pos((i + 1) as u32)
                    && !byd.has_neg((ylen - i) as u32)
                {
                    return false;
                }
            } else {
                let adj = i - ylen;
                if !byd.has_pos((adj + 1) as u32)
                    && !byd.has_neg((self.nextyearlen as usize - adj) as u32)
                {
                    return false;
                }
            }
        }
        true
    }

    /// Collect filtered day-of-year indices into `out`.
    fn collect_days(
        &self,
        year: i32,
        month: u32,
        day: u32,
        out: &mut SmallVec<[u16; 64]>,
    ) -> (usize, usize) {
        out.clear();
        let (start, end) = self.period_range(year, month, day);
        for i in start..end {
            if self.day_passes_filter(i) {
                out.push(i as u16);
            }
        }
        (start, end)
    }
}

// ---------------------------------------------------------------------------
// RRuleIter
// ---------------------------------------------------------------------------

pub struct RRuleIter {
    ii: IterInfo,
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    weekday: u8,
    remaining: Option<u32>,

    day_buf: SmallVec<[u16; 64]>,
    result_buf: SmallVec<[NaiveDateTime; 16]>,
    result_idx: usize,
    timeset_buf: SmallVec<[NaiveTime; 4]>,

    finished: bool,
}

impl RRuleIter {
    /// Create an iterator from a shared `Arc<RRule>`.
    ///
    /// Use this when you already have an `Arc<RRule>` to avoid cloning.
    /// For convenience, `RRule::iter()` wraps this with an automatic clone.
    pub fn new(rule: Arc<RRule>) -> Self {
        let dt = rule.dtstart;
        let remaining = rule.count;
        Self {
            ii: IterInfo::new(rule),
            year: dt.year(),
            month: dt.month(),
            day: dt.day(),
            hour: dt.hour(),
            minute: dt.minute(),
            second: dt.second(),
            weekday: dt.weekday().num_days_from_monday() as u8,
            remaining,
            day_buf: SmallVec::new(),
            result_buf: SmallVec::new(),
            result_idx: 0,
            timeset_buf: SmallVec::new(),
            finished: false,
        }
    }

    fn generate_next_batch(&mut self) -> bool {
        if self.finished {
            return false;
        }

        loop {
            self.ii.rebuild(self.year, self.month);

            // Collect filtered day indices
            self.ii.collect_days(self.year, self.month, self.day, &mut self.day_buf);

            // Compute sub-daily timeset into buffer (no-op for non-sub-daily)
            let is_sub_daily = self.ii.rule.freq.is_sub_daily();
            if is_sub_daily {
                self.compute_sub_daily_timeset();
            }

            // Build results — split field borrows let us pass immutable refs
            // (day_buf, timeset_buf, ii.rule) and mutable refs (remaining,
            // result_buf) into EmitCtx simultaneously without cloning.
            self.result_buf.clear();
            self.result_idx = 0;

            if let Some(bysetpos) = &self.ii.rule.bysetpos {
                let timeset: &[NaiveTime] = if is_sub_daily {
                    &self.timeset_buf
                } else {
                    self.ii.rule.timeset.as_deref().unwrap_or(&[])
                };
                if !timeset.is_empty() {
                    let mut ctx = EmitCtx {
                        day_buf: &self.day_buf,
                        timeset,
                        yearordinal: self.ii.yearordinal,
                        dtstart: self.ii.rule.dtstart,
                        until: self.ii.rule.until,
                        remaining: &mut self.remaining,
                        result_buf: &mut self.result_buf,
                    };
                    self.finished = ctx.emit_bysetpos(bysetpos);
                }
            } else if is_sub_daily {
                // Sub-daily: timeset in self.timeset_buf — split borrow via EmitCtx
                let mut ctx = EmitCtx {
                    day_buf: &self.day_buf,
                    timeset: &self.timeset_buf,
                    yearordinal: self.ii.yearordinal,
                    dtstart: self.ii.rule.dtstart,
                    until: self.ii.rule.until,
                    remaining: &mut self.remaining,
                    result_buf: &mut self.result_buf,
                };
                self.finished = ctx.emit_results();
            } else {
                // Non-sub-daily: timeset in rule (behind Arc) — zero clone
                let ts = self.ii.rule.timeset.as_deref().unwrap_or(&[]);
                let mut ctx = EmitCtx {
                    day_buf: &self.day_buf,
                    timeset: ts,
                    yearordinal: self.ii.yearordinal,
                    dtstart: self.ii.rule.dtstart,
                    until: self.ii.rule.until,
                    remaining: &mut self.remaining,
                    result_buf: &mut self.result_buf,
                };
                self.finished = ctx.emit_results();
            }

            let has_results = !self.result_buf.is_empty();

            if !self.advance_period() {
                self.finished = true;
                return has_results;
            }

            if has_results {
                return true;
            }

            if self.finished {
                return false;
            }
        }
    }

    fn compute_sub_daily_timeset(&mut self) {
        self.timeset_buf.clear();
        let rr = &self.ii.rule;

        // Check validity for sub-daily
        let byhour = rr.byhour.as_deref();
        let byminute = rr.byminute.as_deref();
        let bysecond = rr.bysecond.as_deref();

        let invalid = (rr.freq >= Frequency::Hourly
            && byhour.is_some_and(|bh| !bh.contains(&(self.hour as u8))))
            || (rr.freq >= Frequency::Minutely
                && byminute.is_some_and(|bm| !bm.contains(&(self.minute as u8))))
            || (rr.freq >= Frequency::Secondly
                && bysecond.is_some_and(|bs| !bs.contains(&(self.second as u8))));

        if invalid {
            return;
        }

        match rr.freq {
            Frequency::Hourly => {
                let bm = byminute.unwrap_or(&[]);
                let bs = bysecond.unwrap_or(&[]);
                for &minute in bm {
                    for &second in bs {
                        if let Some(t) =
                            NaiveTime::from_hms_opt(self.hour, minute as u32, second as u32)
                        {
                            self.timeset_buf.push(t);
                        }
                    }
                }
            }
            Frequency::Minutely => {
                let bs = bysecond.unwrap_or(&[]);
                for &second in bs {
                    if let Some(t) =
                        NaiveTime::from_hms_opt(self.hour, self.minute, second as u32)
                    {
                        self.timeset_buf.push(t);
                    }
                }
            }
            Frequency::Secondly => {
                if let Some(t) = NaiveTime::from_hms_opt(self.hour, self.minute, self.second) {
                    self.timeset_buf.push(t);
                }
            }
            _ => {}
        }
        if self.timeset_buf.len() > 1 && !self.timeset_buf.windows(2).all(|w| w[0] <= w[1]) {
            self.timeset_buf.sort();
        }
    }

    fn advance_period(&mut self) -> bool {
        let freq = self.ii.rule.freq;
        let interval = self.ii.rule.interval;
        let mut fixday = false;

        match freq {
            Frequency::Yearly => {
                self.year += interval as i32;
                if self.year > NaiveDate::MAX.year() {
                    return false;
                }
            }
            Frequency::Monthly => {
                self.month += interval;
                if self.month > 12 {
                    let (div, modd) = ((self.month - 1) / 12, (self.month - 1) % 12 + 1);
                    self.month = modd;
                    self.year += div as i32;
                    if self.year > NaiveDate::MAX.year() {
                        return false;
                    }
                }
            }
            Frequency::Weekly => {
                if self.ii.rule.wkst > self.weekday {
                    let adj = -(self.weekday as i32 + 1 + (6 - self.ii.rule.wkst as i32))
                        + interval as i32 * 7;
                    self.day = (self.day as i32 + adj) as u32;
                } else {
                    let adj = -(self.weekday as i32 - self.ii.rule.wkst as i32)
                        + interval as i32 * 7;
                    self.day = (self.day as i32 + adj) as u32;
                }
                self.weekday = self.ii.rule.wkst;
                fixday = true;
            }
            Frequency::Daily => {
                self.day += interval;
                fixday = true;
            }
            Frequency::Hourly => {
                if let Some(ref byhour) = self.ii.rule.byhour {
                    if let Some((ndays, hour)) =
                        mod_distance(self.hour as i64, byhour.as_slice(), 24, interval as i64)
                    {
                        self.hour = hour as u32;
                        if ndays > 0 {
                            self.day += ndays as u32;
                            fixday = true;
                        }
                    }
                } else {
                    let total = self.hour as i64 + interval as i64;
                    self.hour = (total % 24) as u32;
                    let ndays = total / 24;
                    if ndays > 0 {
                        self.day += ndays as u32;
                        fixday = true;
                    }
                }
            }
            Frequency::Minutely => {
                let byhour = self.ii.rule.byhour.as_deref();
                let rep_rate: i64 = 24 * 60;
                let g = gcd(interval as i64, rep_rate);
                let mut valid = false;

                for _ in 0..(rep_rate / g) {
                    if let Some(ref byminute) = self.ii.rule.byminute {
                        if let Some((nhours, minute)) = mod_distance(
                            self.minute as i64,
                            byminute.as_slice(),
                            60,
                            interval as i64,
                        ) {
                            self.minute = minute as u32;
                            let total_h = self.hour as i64 + nhours;
                            self.hour = (total_h % 24) as u32;
                            let ndays = total_h / 24;
                            if ndays > 0 {
                                self.day += ndays as u32;
                                fixday = true;
                            }
                        }
                    } else {
                        let total = self.minute as i64 + interval as i64;
                        self.minute = (total % 60) as u32;
                        let nhours = total / 60;
                        let total_h = self.hour as i64 + nhours;
                        self.hour = (total_h % 24) as u32;
                        let ndays = total_h / 24;
                        if ndays > 0 {
                            self.day += ndays as u32;
                            fixday = true;
                        }
                    }

                    if byhour.is_none_or(|bh| bh.contains(&(self.hour as u8))) {
                        valid = true;
                        break;
                    }
                }

                if !valid {
                    return false;
                }
            }
            Frequency::Secondly => {
                let byhour = self.ii.rule.byhour.as_deref();
                let byminute = self.ii.rule.byminute.as_deref();
                let rep_rate: i64 = 24 * 3600;
                let g = gcd(interval as i64, rep_rate);
                let mut valid = false;

                for _ in 0..(rep_rate / g) {
                    if let Some(ref bysecond) = self.ii.rule.bysecond {
                        if let Some((nminutes, second)) = mod_distance(
                            self.second as i64,
                            bysecond.as_slice(),
                            60,
                            interval as i64,
                        ) {
                            self.second = second as u32;
                            let total_m = self.minute as i64 + nminutes;
                            self.minute = (total_m % 60) as u32;
                            let nhours = total_m / 60;
                            if nhours > 0 {
                                let total_h = self.hour as i64 + nhours;
                                self.hour = (total_h % 24) as u32;
                                let ndays = total_h / 24;
                                if ndays > 0 {
                                    self.day += ndays as u32;
                                    fixday = true;
                                }
                            }
                        }
                    } else {
                        let total = self.second as i64 + interval as i64;
                        self.second = (total % 60) as u32;
                        let nminutes = total / 60;
                        let total_m = self.minute as i64 + nminutes;
                        self.minute = (total_m % 60) as u32;
                        let nhours = total_m / 60;
                        if nhours > 0 {
                            let total_h = self.hour as i64 + nhours;
                            self.hour = (total_h % 24) as u32;
                            let ndays = total_h / 24;
                            if ndays > 0 {
                                self.day += ndays as u32;
                                fixday = true;
                            }
                        }
                    }

                    if byhour.is_none_or(|bh| bh.contains(&(self.hour as u8)))
                        && byminute.is_none_or(|bm| bm.contains(&(self.minute as u8)))
                    {
                        valid = true;
                        break;
                    }
                }

                if !valid {
                    return false;
                }
            }
        }

        // Fix day overflow
        if fixday && self.day > 28 {
            let mut dim = days_in_month(self.year, self.month);
            while self.day > dim {
                self.day -= dim;
                self.month += 1;
                if self.month > 12 {
                    self.month = 1;
                    self.year += 1;
                    if self.year > NaiveDate::MAX.year() {
                        return false;
                    }
                }
                dim = days_in_month(self.year, self.month);
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// Free functions for split-borrow result emission
// ---------------------------------------------------------------------------

/// Shared context for result emission, avoiding repeated parameter passing.
struct EmitCtx<'a> {
    day_buf: &'a [u16],
    timeset: &'a [NaiveTime],
    yearordinal: i32,
    dtstart: NaiveDateTime,
    until: Option<NaiveDateTime>,
    remaining: &'a mut Option<u32>,
    result_buf: &'a mut SmallVec<[NaiveDateTime; 16]>,
}

impl EmitCtx<'_> {
    /// Push a result, checking until/count. Returns true if finished.
    #[inline]
    fn push(&mut self, res: NaiveDateTime) -> bool {
        if let Some(until) = self.until {
            if res > until {
                return true;
            }
        }
        if res >= self.dtstart {
            if let Some(ref mut rem) = self.remaining {
                if *rem == 0 {
                    return true;
                }
                *rem -= 1;
            }
            self.result_buf.push(res);
        }
        false
    }

    /// Emit results from day_buf x timeset. Returns true if finished.
    #[inline]
    fn emit_results(&mut self) -> bool {
        for &day_idx in self.day_buf {
            if let Some(date) =
                NaiveDate::from_num_days_from_ce_opt(self.yearordinal + day_idx as i32)
            {
                for &time in self.timeset {
                    if self.push(NaiveDateTime::new(date, time)) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Emit results with bysetpos filtering. Returns true if finished.
    #[inline]
    fn emit_bysetpos(&mut self, bysetpos: &ByList<i32>) -> bool {
        let mut poslist: SmallVec<[NaiveDateTime; 8]> = SmallVec::new();
        let ts_len = self.timeset.len() as i32;

        for &pos in bysetpos.iter() {
            let (daypos, timepos) = if pos < 0 {
                (pos / ts_len, ((pos % ts_len) + ts_len) % ts_len)
            } else {
                ((pos - 1) / ts_len, (pos - 1) % ts_len)
            };

            let day_idx = if daypos < 0 {
                let len = self.day_buf.len() as i32;
                if daypos + len < 0 {
                    continue;
                }
                (daypos + len) as usize
            } else {
                daypos as usize
            };

            if day_idx >= self.day_buf.len() || timepos as usize >= self.timeset.len() {
                continue;
            }
            let i = self.day_buf[day_idx];
            let time = self.timeset[timepos as usize];
            if let Some(date) =
                NaiveDate::from_num_days_from_ce_opt(self.yearordinal + i as i32)
            {
                poslist.push(NaiveDateTime::new(date, time));
            }
        }
        poslist.sort_unstable();
        poslist.dedup();

        for res in poslist {
            if self.push(res) {
                return true;
            }
        }
        false
    }
}

impl Iterator for RRuleIter {
    type Item = NaiveDateTime;

    fn next(&mut self) -> Option<NaiveDateTime> {
        // Drain buffer first
        if self.result_idx < self.result_buf.len() {
            let result = self.result_buf[self.result_idx];
            self.result_idx += 1;
            return Some(result);
        }

        if self.finished {
            return None;
        }

        if self.generate_next_batch() {
            let result = self.result_buf[self.result_idx];
            self.result_idx += 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let buffered = self.result_buf.len() - self.result_idx;
        if let Some(remaining) = self.remaining {
            let upper = remaining as usize + buffered;
            (buffered, Some(upper))
        } else {
            (buffered, None)
        }
    }
}
