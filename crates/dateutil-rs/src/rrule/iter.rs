//! RRule iteration logic — port of `_iterinfo` and `rrule._iter()`.

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};

use super::*;

// ---------------------------------------------------------------------------
// IterInfo — cached year/month masks (port of Python's _iterinfo)
// ---------------------------------------------------------------------------

pub(crate) struct IterInfo {
    rule: RRule,

    pub yearlen: usize,
    pub nextyearlen: usize,
    pub yearordinal: i64,
    pub yearweekday: u8,

    pub mmask: &'static [u8],
    pub mdaymask: &'static [i32],
    pub nmdaymask: &'static [i32],
    pub wdaymask: &'static [u8],
    pub mrange: &'static [usize; 13],

    pub wnomask: Option<Vec<u8>>,
    pub nwdaymask: Option<Vec<u8>>,
    pub eastermask: Option<Vec<u8>>,

    lastyear: Option<i32>,
    lastmonth: Option<u32>,
}

impl IterInfo {
    fn new(rule: &RRule) -> Self {
        Self {
            rule: rule.clone(),
            yearlen: 0,
            nextyearlen: 0,
            yearordinal: 0,
            yearweekday: 0,
            mmask: &[],
            mdaymask: &[],
            nmdaymask: &[],
            wdaymask: &[],
            mrange: &M365RANGE,
            wnomask: None,
            nwdaymask: None,
            eastermask: None,
            lastyear: None,
            lastmonth: None,
        }
    }

    fn rebuild(&mut self, year: i32, month: u32) {
        let rr = &self.rule;

        if self.lastyear != Some(year) {
            let is_leap = is_leap_year(year);
            self.yearlen = if is_leap { 366 } else { 365 };
            self.nextyearlen = if is_leap_year(year + 1) { 366 } else { 365 };

            let first_yday = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
            self.yearordinal = first_yday.num_days_from_ce() as i64;
            self.yearweekday = first_yday.weekday().num_days_from_monday() as u8;

            let wday = self.yearweekday as usize;
            if is_leap {
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

            // byweekno
            if let Some(ref byweekno) = rr.byweekno {
                self.wnomask = Some(vec![0; self.yearlen + 7]);
                let wnomask = self.wnomask.as_mut().unwrap();

                let firstwkst = (7 - self.yearweekday + rr.wkst) % 7;
                let mut no1wkst = firstwkst as usize;

                let wyearlen;
                if no1wkst >= 4 {
                    no1wkst = 0;
                    wyearlen =
                        self.yearlen + (self.yearweekday as usize + 7 - rr.wkst as usize) % 7;
                } else {
                    wyearlen = self.yearlen - no1wkst;
                }

                let (div, modd) = (wyearlen / 7, wyearlen % 7);
                let numweeks = div + modd / 4;

                for &n in byweekno {
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
                        if i < wnomask.len() {
                            wnomask[i] = 1;
                        }
                        i += 1;
                        if i < self.wdaymask.len() && self.wdaymask[i] == rr.wkst {
                            break;
                        }
                    }
                }

                // Check week 1 of next year
                if byweekno.contains(&1) {
                    let mut i = no1wkst + numweeks * 7;
                    if no1wkst != firstwkst as usize {
                        i -= 7 - firstwkst as usize;
                    }
                    if i < self.yearlen {
                        for _ in 0..7 {
                            if i < wnomask.len() {
                                wnomask[i] = 1;
                            }
                            i += 1;
                            if i < self.wdaymask.len() && self.wdaymask[i] == rr.wkst {
                                break;
                            }
                        }
                    }
                }

                // Check last week of previous year
                if no1wkst > 0 && !byweekno.contains(&-1) {
                    let lyearweekday =
                        NaiveDate::from_ymd_opt(year - 1, 1, 1)
                            .unwrap()
                            .weekday()
                            .num_days_from_monday() as u8;
                    let lno1wkst = (7 - lyearweekday + rr.wkst) % 7;
                    let lyearlen = if is_leap_year(year - 1) { 366usize } else { 365 };
                    let lnumweeks = if lno1wkst >= 4 {
                        52 + (lyearlen + (lyearweekday as usize + 7 - rr.wkst as usize) % 7) % 7
                            / 4
                    } else {
                        52 + (self.yearlen - no1wkst) % 7 / 4
                    };

                    if byweekno.contains(&(lnumweeks as i32)) {
                        for entry in wnomask.iter_mut().take(no1wkst) {
                            *entry = 1;
                        }
                    }
                } else if no1wkst > 0 && byweekno.contains(&-1) {
                    for entry in wnomask.iter_mut().take(no1wkst) {
                        *entry = 1;
                    }
                }
            } else {
                self.wnomask = None;
            }
        }

        // nwdaymask — nth weekday within month/year ranges
        if let Some(bynweekday) = rr.bynweekday.as_ref().filter(|_|
            self.lastmonth != Some(month) || self.lastyear != Some(year))
        {
            let mut ranges: Vec<(usize, usize)> = Vec::new();

            if rr.freq == YEARLY {
                if let Some(ref bymonth) = rr.bymonth {
                    for &m in bymonth {
                        let start = self.mrange[m as usize - 1];
                        let end = self.mrange[m as usize];
                        ranges.push((start, end));
                    }
                } else {
                    ranges.push((0, self.yearlen));
                }
            } else if rr.freq == MONTHLY {
                let start = self.mrange[month as usize - 1];
                let end = self.mrange[month as usize];
                ranges.push((start, end));
            }

            if !ranges.is_empty() {
                let mut nwdaymask = vec![0u8; self.yearlen];
                for (first, last) in &ranges {
                    let last = last - 1; // inclusive end
                    for &(wday, n) in bynweekday {
                        let i = if n < 0 {
                            let mut idx = last as i64 + (n as i64 + 1) * 7;
                            idx -= (self.wdaymask[idx as usize] as i64 - wday as i64 + 7) % 7;
                            idx as usize
                        } else {
                            let mut idx = *first as i64 + (n as i64 - 1) * 7;
                            idx += (7 - self.wdaymask[idx as usize] as i64 + wday as i64) % 7;
                            idx as usize
                        };
                        if *first <= i && i <= last {
                            nwdaymask[i] = 1;
                        }
                    }
                }
                self.nwdaymask = Some(nwdaymask);
            }
        }

        // easter mask
        if let Some(ref byeaster) = rr.byeaster {
            let mut eastermask = vec![0u8; self.yearlen + 7];
            if let Ok(easter_date) = crate::easter::easter(year, crate::easter::EASTER_WESTERN) {
                let eyday = easter_date.num_days_from_ce() as i64 - self.yearordinal;
                for &offset in byeaster {
                    let idx = (eyday + offset as i64) as usize;
                    if idx < eastermask.len() {
                        eastermask[idx] = 1;
                    }
                }
            }
            self.eastermask = Some(eastermask);
        }

        self.lastyear = Some(year);
        self.lastmonth = Some(month);
    }

    // Day sets for different frequencies
    fn ydayset(&self, _year: i32, _month: u32, _day: u32) -> (Vec<Option<usize>>, usize, usize) {
        let v: Vec<Option<usize>> = (0..self.yearlen).map(Some).collect();
        (v, 0, self.yearlen)
    }

    fn mdayset(&self, _year: i32, month: u32, _day: u32) -> (Vec<Option<usize>>, usize, usize) {
        let mut dset: Vec<Option<usize>> = vec![None; self.yearlen];
        let start = self.mrange[month as usize - 1];
        let end = self.mrange[month as usize];
        for (i, slot) in dset.iter_mut().enumerate().take(end).skip(start) {
            *slot = Some(i);
        }
        (dset, start, end)
    }

    fn wdayset(&self, year: i32, month: u32, day: u32) -> (Vec<Option<usize>>, usize, usize) {
        let mut dset: Vec<Option<usize>> = vec![None; self.yearlen + 7];
        let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        let mut i = (date.num_days_from_ce() as i64 - self.yearordinal) as usize;
        let start = i;
        for _ in 0..7 {
            dset[i] = Some(i);
            i += 1;
            if i < self.wdaymask.len() && self.wdaymask[i] == self.rule.wkst {
                break;
            }
        }
        (dset, start, i)
    }

    fn ddayset(&self, year: i32, month: u32, day: u32) -> (Vec<Option<usize>>, usize, usize) {
        let mut dset: Vec<Option<usize>> = vec![None; self.yearlen];
        let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        let i = (date.num_days_from_ce() as i64 - self.yearordinal) as usize;
        dset[i] = Some(i);
        (dset, i, i + 1)
    }

    // Time sets for high-frequency rules
    fn htimeset(&self, hour: u32, _minute: u32, _second: u32) -> Vec<NaiveTime> {
        let rr = &self.rule;
        let mut tset = Vec::new();
        let byminute = rr.byminute.as_deref().unwrap_or(&[]);
        let bysecond = rr.bysecond.as_deref().unwrap_or(&[]);
        for &minute in byminute {
            for &second in bysecond {
                if let Some(t) = NaiveTime::from_hms_opt(hour, minute as u32, second as u32) {
                    tset.push(t);
                }
            }
        }
        tset.sort();
        tset
    }

    fn mtimeset(&self, hour: u32, minute: u32, _second: u32) -> Vec<NaiveTime> {
        let rr = &self.rule;
        let mut tset = Vec::new();
        let bysecond = rr.bysecond.as_deref().unwrap_or(&[]);
        for &second in bysecond {
            if let Some(t) = NaiveTime::from_hms_opt(hour, minute, second as u32) {
                tset.push(t);
            }
        }
        tset.sort();
        tset
    }

    fn stimeset(&self, hour: u32, minute: u32, second: u32) -> Vec<NaiveTime> {
        if let Some(t) = NaiveTime::from_hms_opt(hour, minute, second) {
            vec![t]
        } else {
            vec![]
        }
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Number of days in a given month.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// RRuleIter — the main iteration state machine
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
    total: i64,
    count: Option<i64>,
    // Buffered results for the current period
    buffer: Vec<NaiveDateTime>,
    buffer_idx: usize,
    finished: bool,
}

impl RRuleIter {
    pub(crate) fn new(rule: &RRule) -> Self {
        let dt = rule.dtstart;
        Self {
            ii: IterInfo::new(rule),
            year: dt.year(),
            month: dt.month(),
            day: dt.day(),
            hour: dt.hour(),
            minute: dt.minute(),
            second: dt.second(),
            weekday: dt.weekday().num_days_from_monday() as u8,
            total: 0,
            count: rule.count,
            buffer: Vec::new(),
            buffer_idx: 0,
            finished: false,
        }
    }

    /// Generate the next batch of results for the current period.
    fn generate_next_batch(&mut self) -> bool {
        if self.finished {
            return false;
        }

        loop {
            // Rebuild masks
            self.ii.rebuild(self.year, self.month);

            // Get dayset for current frequency
            let (mut dayset, start, end) = match self.ii.rule.freq {
                YEARLY => self.ii.ydayset(self.year, self.month, self.day),
                MONTHLY => self.ii.mdayset(self.year, self.month, self.day),
                WEEKLY => self.ii.wdayset(self.year, self.month, self.day),
                _ => self.ii.ddayset(self.year, self.month, self.day),
            };

            // Determine timeset
            let timeset = if self.ii.rule.freq < HOURLY {
                self.ii.rule.timeset.clone().unwrap_or_default()
            } else {
                // Check if current time is valid
                let byhour = self.ii.rule.byhour.as_deref();
                let byminute = self.ii.rule.byminute.as_deref();
                let bysecond = self.ii.rule.bysecond.as_deref();

                let invalid = (self.ii.rule.freq >= HOURLY
                    && byhour.is_some()
                    && !byhour.unwrap().contains(&(self.hour as u8)))
                    || (self.ii.rule.freq >= MINUTELY
                        && byminute.is_some()
                        && !byminute.unwrap().contains(&(self.minute as u8)))
                    || (self.ii.rule.freq >= SECONDLY
                        && bysecond.is_some()
                        && !bysecond.unwrap().contains(&(self.second as u8)));

                if invalid {
                    vec![]
                } else {
                    match self.ii.rule.freq {
                        HOURLY => self.ii.htimeset(self.hour, self.minute, self.second),
                        MINUTELY => self.ii.mtimeset(self.hour, self.minute, self.second),
                        SECONDLY => self.ii.stimeset(self.hour, self.minute, self.second),
                        _ => vec![],
                    }
                }
            };

            // Filter dayset
            let mut filtered = false;
            let bymonth = self.ii.rule.bymonth.as_deref();
            let byweekno = self.ii.rule.byweekno.as_deref();
            let byweekday = self.ii.rule.byweekday.as_deref();
            let byeaster = self.ii.rule.byeaster.as_deref();
            let bymonthday = &self.ii.rule.bymonthday;
            let bynmonthday = &self.ii.rule.bynmonthday;
            let byyearday = self.ii.rule.byyearday.as_deref();

            for slot in dayset[start..end].iter_mut() {
                let Some(i) = *slot else { continue };

                let should_filter = bymonth.is_some_and(|bm| !bm.contains(&self.ii.mmask[i]))
                    || (byweekno.is_some()
                        && self.ii.wnomask.as_ref().is_none_or(|m| m[i] == 0))
                    || byweekday.is_some_and(|bw| !bw.contains(&self.ii.wdaymask[i]))
                    || (self.ii.rule.bynweekday.is_some()
                        && self.ii.nwdaymask.as_ref().is_some_and(|m| m[i] == 0))
                    || (byeaster.is_some()
                        && self.ii.eastermask.as_ref().is_none_or(|m| i >= m.len() || m[i] == 0))
                    || ((!bymonthday.is_empty() || !bynmonthday.is_empty())
                        && !bymonthday.contains(&self.ii.mdaymask[i])
                        && !bynmonthday.contains(&self.ii.nmdaymask[i]))
                    || byyearday.is_some_and(|byd| {
                        if i < self.ii.yearlen {
                            !byd.contains(&(i as i32 + 1))
                                && !byd.contains(&(i as i32 - self.ii.yearlen as i32))
                        } else {
                            let adj = i - self.ii.yearlen;
                            !byd.contains(&(adj as i32 + 1))
                                && !byd.contains(&(adj as i32 - self.ii.nextyearlen as i32))
                        }
                    });

                if should_filter {
                    *slot = None;
                    filtered = true;
                }
            }

            // Build results
            let mut results: Vec<NaiveDateTime> = Vec::new();

            if let Some(bysetpos) = self.ii.rule.bysetpos.as_ref().filter(|_| !timeset.is_empty()) {
                let mut poslist: Vec<NaiveDateTime> = Vec::new();
                for &pos in bysetpos {
                    let (daypos, timepos) = if pos < 0 {
                        let ts_len = timeset.len() as i32;
                        (pos / ts_len, ((pos % ts_len) + ts_len) % ts_len)
                    } else {
                        let ts_len = timeset.len() as i32;
                        ((pos - 1) / ts_len, (pos - 1) % ts_len)
                    };
                    let valid_days: Vec<usize> = dayset[start..end]
                        .iter()
                        .filter_map(|&x| x)
                        .collect();
                    let day_idx = if daypos < 0 {
                        let len = valid_days.len() as i32;
                        if daypos + len < 0 {
                            continue;
                        }
                        (daypos + len) as usize
                    } else {
                        daypos as usize
                    };
                    if day_idx >= valid_days.len() || timepos as usize >= timeset.len() {
                        continue;
                    }
                    let i = valid_days[day_idx];
                    let time = timeset[timepos as usize];
                    let date = NaiveDate::from_num_days_from_ce_opt(
                        (self.ii.yearordinal + i as i64) as i32,
                    );
                    if let Some(date) = date {
                        let res = NaiveDateTime::new(date, time);
                        if !poslist.contains(&res) {
                            poslist.push(res);
                        }
                    }
                }
                poslist.sort();

                for res in poslist {
                    if let Some(until) = self.ii.rule.until {
                        if res > until {
                            self.finished = true;
                            break;
                        }
                    }
                    if res >= self.ii.rule.dtstart {
                        if let Some(ref mut count) = self.count {
                            if *count <= 0 {
                                self.finished = true;
                                break;
                            }
                            *count -= 1;
                        }
                        self.total += 1;
                        results.push(res);
                    }
                }
            } else {
                for slot in &dayset[start..end] {
                    if let Some(i) = *slot {
                        let date = NaiveDate::from_num_days_from_ce_opt(
                            (self.ii.yearordinal + i as i64) as i32,
                        );
                        if let Some(date) = date {
                            for &time in &timeset {
                                let res = NaiveDateTime::new(date, time);
                                if let Some(until) = self.ii.rule.until {
                                    if res > until {
                                        self.finished = true;
                                        break;
                                    }
                                }
                                if res >= self.ii.rule.dtstart {
                                    if let Some(ref mut count) = self.count {
                                        if *count <= 0 {
                                            self.finished = true;
                                            break;
                                        }
                                        *count -= 1;
                                    }
                                    self.total += 1;
                                    results.push(res);
                                }
                            }
                        }
                        if self.finished {
                            break;
                        }
                    }
                }
            }

            if !results.is_empty() {
                self.buffer = results;
                self.buffer_idx = 0;
            }

            // Advance to next period
            if !self.advance_period(filtered) {
                self.finished = true;
                return !self.buffer.is_empty() && self.buffer_idx < self.buffer.len();
            }

            if !self.buffer.is_empty() && self.buffer_idx < self.buffer.len() {
                return true;
            }

            if self.finished {
                return false;
            }
        }
    }

    fn advance_period(&mut self, filtered: bool) -> bool {
        let freq = self.ii.rule.freq;
        let interval = self.ii.rule.interval;

        let mut fixday = false;

        match freq {
            YEARLY => {
                self.year += interval as i32;
                if self.year > chrono::NaiveDate::MAX.year() {
                    return false;
                }
                self.ii.rebuild(self.year, self.month);
            }
            MONTHLY => {
                self.month += interval as u32;
                if self.month > 12 {
                    let (div, modd) = ((self.month - 1) / 12, (self.month - 1) % 12 + 1);
                    self.month = modd;
                    self.year += div as i32;
                    if self.year > chrono::NaiveDate::MAX.year() {
                        return false;
                    }
                }
                self.ii.rebuild(self.year, self.month);
            }
            WEEKLY => {
                if self.ii.rule.wkst > self.weekday {
                    let adj =
                        -(self.weekday as i32 + 1 + (6 - self.ii.rule.wkst as i32)) + interval as i32 * 7;
                    self.day = (self.day as i32 + adj) as u32;
                } else {
                    let adj =
                        -(self.weekday as i32 - self.ii.rule.wkst as i32) + interval as i32 * 7;
                    self.day = (self.day as i32 + adj) as u32;
                }
                self.weekday = self.ii.rule.wkst;
                fixday = true;
            }
            DAILY => {
                self.day += interval as u32;
                fixday = true;
            }
            HOURLY => {
                if filtered {
                    self.hour += ((23 - self.hour) / interval as u32) * interval as u32;
                }

                if let Some(ref byhour) = self.ii.rule.byhour {
                    if let Some((ndays, hour)) =
                        mod_distance(self.hour as i64, byhour.as_slice(), 24, interval)
                    {
                        self.hour = hour as u32;
                        if ndays > 0 {
                            self.day += ndays as u32;
                            fixday = true;
                        }
                    }
                } else {
                    let (ndays, hour) = (
                        (self.hour as i64 + interval) / 24,
                        ((self.hour as i64 + interval) % 24) as u32,
                    );
                    self.hour = hour;
                    if ndays > 0 {
                        self.day += ndays as u32;
                        fixday = true;
                    }
                }
            }
            MINUTELY => {
                if filtered {
                    let total_min = self.hour * 60 + self.minute;
                    self.minute += ((1439 - total_min) / interval as u32) * interval as u32;
                }

                let byhour = self.ii.rule.byhour.as_deref();
                let mut valid = false;
                let rep_rate = 24 * 60;
                let g = gcd(interval, rep_rate as i64);

                for _ in 0..(rep_rate as i64 / g) {
                    if let Some(ref byminute) = self.ii.rule.byminute {
                        if let Some((nhours, minute)) =
                            mod_distance(self.minute as i64, byminute.as_slice(), 60, interval)
                        {
                            self.minute = minute as u32;
                            let (div, hour) =
                                ((self.hour as i64 + nhours) / 24, ((self.hour as i64 + nhours) % 24) as u32);
                            self.hour = hour;
                            if div > 0 {
                                self.day += div as u32;
                                fixday = true;
                            }
                        }
                    } else {
                        let (nhours, minute) = (
                            (self.minute as i64 + interval) / 60,
                            ((self.minute as i64 + interval) % 60) as u32,
                        );
                        self.minute = minute;
                        let (div, hour) =
                            ((self.hour as i64 + nhours) / 24, ((self.hour as i64 + nhours) % 24) as u32);
                        self.hour = hour;
                        if div > 0 {
                            self.day += div as u32;
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
            SECONDLY => {
                if filtered {
                    let total_sec =
                        self.hour as i64 * 3600 + self.minute as i64 * 60 + self.second as i64;
                    self.second +=
                        ((86399 - total_sec) / interval) as u32 * interval as u32;
                }

                let byhour = self.ii.rule.byhour.as_deref();
                let byminute = self.ii.rule.byminute.as_deref();
                let rep_rate: i64 = 24 * 3600;
                let g = gcd(interval, rep_rate);
                let mut valid = false;

                for _ in 0..(rep_rate / g) {
                    if let Some(ref bysecond) = self.ii.rule.bysecond {
                        if let Some((nminutes, second)) =
                            mod_distance(self.second as i64, bysecond.as_slice(), 60, interval)
                        {
                            self.second = second as u32;
                            let (div, minute) = (
                                (self.minute as i64 + nminutes) / 60,
                                ((self.minute as i64 + nminutes) % 60) as u32,
                            );
                            self.minute = minute;
                            if div > 0 {
                                self.hour += div as u32;
                                let (div2, hour) = (self.hour / 24, self.hour % 24);
                                self.hour = hour;
                                if div2 > 0 {
                                    self.day += div2;
                                    fixday = true;
                                }
                            }
                        }
                    } else {
                        let (nminutes, second) = (
                            (self.second as i64 + interval) / 60,
                            ((self.second as i64 + interval) % 60) as u32,
                        );
                        self.second = second;
                        let (div, minute) = (
                            (self.minute as i64 + nminutes) / 60,
                            ((self.minute as i64 + nminutes) % 60) as u32,
                        );
                        self.minute = minute;
                        if div > 0 {
                            self.hour += div as u32;
                            let (div2, hour) = (self.hour / 24, self.hour % 24);
                            self.hour = hour;
                            if div2 > 0 {
                                self.day += div2;
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
            _ => return false,
        }

        // Fix day overflow
        if fixday && self.day > 28 {
            let mut daysinmonth = days_in_month(self.year, self.month);
            while self.day > daysinmonth {
                self.day -= daysinmonth;
                self.month += 1;
                if self.month > 12 {
                    self.month = 1;
                    self.year += 1;
                    if self.year > chrono::NaiveDate::MAX.year() {
                        return false;
                    }
                }
                daysinmonth = days_in_month(self.year, self.month);
            }
            self.ii.rebuild(self.year, self.month);
        }

        true
    }
}

impl Iterator for RRuleIter {
    type Item = NaiveDateTime;

    fn next(&mut self) -> Option<NaiveDateTime> {
        // Return buffered result if available
        if self.buffer_idx < self.buffer.len() {
            let result = self.buffer[self.buffer_idx];
            self.buffer_idx += 1;
            return Some(result);
        }

        if self.finished {
            return None;
        }

        // Clear buffer and generate next batch
        self.buffer.clear();
        self.buffer_idx = 0;

        if self.generate_next_batch() {
            let result = self.buffer[self.buffer_idx];
            self.buffer_idx += 1;
            Some(result)
        } else {
            None
        }
    }
}

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
    fn test_yearly_bymonth() {
        let rule = RRule::new(
            YEARLY,
            Some(dt(1997, 9, 2, 9, 0, 0)),
            1,
            None,
            Some(3),
            None,
            None,
            Some(vec![1, 3]),
            Some(vec![5, 10]),
            None, None, None, None, None, None, None,
        )
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
        let rule = RRule::new(
            DAILY,
            Some(dt(1997, 9, 2, 9, 0, 0)),
            1,
            None,
            Some(3),
            None,
            None,
            Some(vec![1, 3]),
            None, None, None, None, None, None, None, None,
        )
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
    fn test_weekly_interval2() {
        let rule = RRule::new(
            WEEKLY,
            Some(dt(1997, 9, 2, 9, 0, 0)),
            2,
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
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 16, 9, 0, 0),
                dt(1997, 9, 30, 9, 0, 0),
            ]
        );
    }

    #[test]
    fn test_monthly_bynweekday() {
        // First Friday of each month
        let rule = RRule::new(
            MONTHLY,
            Some(dt(1997, 9, 2, 9, 0, 0)),
            1,
            None,
            Some(3),
            None,
            None,
            None,
            None, None, None, None,
            Some(vec![Weekday::new(4, Some(1))]), // FR(+1)
            None, None, None,
        )
        .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 5, 9, 0, 0),
                dt(1997, 10, 3, 9, 0, 0),
                dt(1997, 11, 7, 9, 0, 0),
            ]
        );
    }

    #[test]
    fn test_daily_byweekday() {
        let rule = RRule::new(
            DAILY,
            Some(dt(1997, 9, 2, 9, 0, 0)),
            1,
            None,
            Some(3),
            None,
            None, None, None, None, None, None,
            Some(vec![
                Weekday::new(1, None), // TU
                Weekday::new(3, None), // TH
            ]),
            None, None, None,
        )
        .unwrap();
        let results = rule.all();
        assert_eq!(
            results,
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 4, 9, 0, 0),
                dt(1997, 9, 9, 9, 0, 0),
            ]
        );
    }

    #[test]
    fn test_bysetpos_last_weekday_of_month() {
        // Last weekday (MO-FR) of each month
        let rule = RRule::new(
            MONTHLY,
            Some(dt(1997, 9, 2, 9, 0, 0)),
            1,
            None,
            Some(3),
            None,
            Some(vec![-1]),  // last
            None,
            None, None, None, None,
            Some(vec![
                Weekday::new(0, None), // MO
                Weekday::new(1, None), // TU
                Weekday::new(2, None), // WE
                Weekday::new(3, None), // TH
                Weekday::new(4, None), // FR
            ]),
            None, None, None,
        )
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

    #[test]
    fn test_minutely_basic() {
        let rule = RRule::new(
            MINUTELY,
            Some(dt(1997, 9, 2, 9, 0, 0)),
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
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 2, 9, 1, 0),
                dt(1997, 9, 2, 9, 2, 0),
            ]
        );
    }

    #[test]
    fn test_secondly_basic() {
        let rule = RRule::new(
            SECONDLY,
            Some(dt(1997, 9, 2, 9, 0, 0)),
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
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 2, 9, 0, 1),
                dt(1997, 9, 2, 9, 0, 2),
            ]
        );
    }
}
