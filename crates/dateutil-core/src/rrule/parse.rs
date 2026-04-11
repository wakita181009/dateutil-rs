//! RFC 5545 RRULE string parsing.

use std::borrow::Cow;

use chrono::{NaiveDate, NaiveDateTime};
use phf::phf_map;

use crate::common::Weekday;
use crate::error::RRuleError;

use super::set::RRuleSet;
use super::{Frequency, RRule, RRuleBuilder, Recurrence};

// ---------------------------------------------------------------------------
// PHF lookup tables
// ---------------------------------------------------------------------------

static FREQ_MAP: phf::Map<&'static str, Frequency> = phf_map! {
    "YEARLY" => Frequency::Yearly,
    "MONTHLY" => Frequency::Monthly,
    "WEEKLY" => Frequency::Weekly,
    "DAILY" => Frequency::Daily,
    "HOURLY" => Frequency::Hourly,
    "MINUTELY" => Frequency::Minutely,
    "SECONDLY" => Frequency::Secondly,
};

static WDAY_MAP: phf::Map<&'static str, u8> = phf_map! {
    "MO" => 0,
    "TU" => 1,
    "WE" => 2,
    "TH" => 3,
    "FR" => 4,
    "SA" => 5,
    "SU" => 6,
};

// ---------------------------------------------------------------------------
// rrulestr — public API
// ---------------------------------------------------------------------------

pub fn rrulestr(
    s: &str,
    dtstart: Option<NaiveDateTime>,
    forceset: bool,
    compatible: bool,
    unfold: bool,
) -> Result<RRuleStrResult, RRuleError> {
    let forceset = forceset || compatible;
    let unfold = unfold || compatible;

    if s.trim().is_empty() {
        return Err(RRuleError::ValueError("empty string".into()));
    }

    let lines: Vec<Cow<'_, str>> = if unfold {
        let mut result: Vec<Cow<'_, str>> = Vec::new();
        for line in s.lines() {
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            if line.starts_with(' ') && !result.is_empty() {
                result.last_mut().unwrap().to_mut().push_str(&line[1..]);
            } else {
                result.push(Cow::Borrowed(line));
            }
        }
        result
    } else {
        s.split_whitespace().map(Cow::Borrowed).collect()
    };

    // Simple case: single RRULE line
    if !forceset
        && lines.len() == 1
        && (!lines[0].contains(':')
            || lines[0]
                .get(..6)
                .is_some_and(|p| p.eq_ignore_ascii_case("RRULE:")))
    {
        let rule = parse_rfc_rrule(&lines[0], dtstart)?;
        return Ok(RRuleStrResult::Single(Box::new(rule)));
    }

    // Complex case: parse as rruleset
    let mut rrulevals: Vec<&str> = Vec::new();
    let mut rdatevals: Vec<&str> = Vec::new();
    let mut exrulevals: Vec<&str> = Vec::new();
    let mut exdatevals: Vec<NaiveDateTime> = Vec::new();
    let mut dtstart = dtstart;

    for line in &lines {
        if line.is_empty() {
            continue;
        }
        let (name_part, value) = if let Some(idx) = line.find(':') {
            (&line[..idx], &line[idx + 1..])
        } else {
            ("RRULE", line.as_ref())
        };

        let prop = name_part.split(';').next().unwrap_or(name_part);

        if prop.eq_ignore_ascii_case("RRULE") {
            rrulevals.push(value);
        } else if prop.eq_ignore_ascii_case("RDATE") {
            rdatevals.push(value);
        } else if prop.eq_ignore_ascii_case("EXRULE") {
            exrulevals.push(value);
        } else if prop.eq_ignore_ascii_case("EXDATE") {
            for datestr in value.split(',') {
                if let Some(dt) = parse_rfc_datetime(datestr.trim()) {
                    exdatevals.push(dt);
                }
            }
        } else if prop.eq_ignore_ascii_case("DTSTART") {
            if value.contains(',') {
                return Err(RRuleError::ValueError(
                    "DTSTART must be a single date-time value".into(),
                ));
            }
            if let Some(dt) = parse_rfc_datetime(value) {
                dtstart = Some(dt);
            }
        } else {
            return Err(RRuleError::ValueError(
                format!("unsupported property: {prop}").into(),
            ));
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
            let rule = parse_rfc_rrule(value, dtstart)?;
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
            let rule = parse_rfc_rrule(value, dtstart)?;
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
        let rule = parse_rfc_rrule(rrulevals[0], dtstart)?;
        Ok(RRuleStrResult::Single(Box::new(rule)))
    } else {
        Err(RRuleError::ValueError("no RRULE found".into()))
    }
}

/// Result of rrulestr parsing.
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

// ---------------------------------------------------------------------------
// Internal parsing
// ---------------------------------------------------------------------------

fn parse_rfc_rrule(line: &str, dtstart: Option<NaiveDateTime>) -> Result<RRule, RRuleError> {
    let value = if let Some((name, val)) = line.split_once(':') {
        if !name.eq_ignore_ascii_case("RRULE") {
            return Err(RRuleError::ValueError(
                format!("unknown parameter name: {name}").into(),
            ));
        }
        val
    } else {
        line
    };

    let mut freq: Option<Frequency> = None;
    let mut builder_interval: u32 = 1;
    let mut builder_wkst: Option<u8> = None;
    let mut builder_count: Option<u32> = None;
    let mut builder_until: Option<NaiveDateTime> = None;
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
        let (raw_name, val) = pair.split_once('=').ok_or_else(|| {
            RRuleError::ValueError(format!("invalid RRULE parameter: {pair}").into())
        })?;
        let name = raw_name.to_ascii_uppercase();

        match name.as_str() {
            "FREQ" => {
                let val_upper = val.to_ascii_uppercase();
                freq = Some(
                    FREQ_MAP
                        .get(val_upper.as_str())
                        .copied()
                        .ok_or_else(|| RRuleError::InvalidFrequency(val.into()))?,
                );
            }
            "INTERVAL" => {
                builder_interval = val.parse().map_err(|_| {
                    RRuleError::ValueError(format!("invalid INTERVAL: {val}").into())
                })?;
            }
            "WKST" => {
                builder_wkst = Some(parse_weekday_name(val)?);
            }
            "COUNT" => {
                builder_count =
                    Some(val.parse().map_err(|_| {
                        RRuleError::ValueError(format!("invalid COUNT: {val}").into())
                    })?);
            }
            "UNTIL" => {
                builder_until = Some(parse_rfc_datetime(val).ok_or_else(|| {
                    RRuleError::ValueError(format!("invalid UNTIL: {val}").into())
                })?);
            }
            "BYSETPOS" => bysetpos = Some(parse_int_list(val)?),
            "BYMONTH" => {
                bymonth = Some(parse_int_list(val)?.into_iter().map(|x| x as u8).collect());
            }
            "BYMONTHDAY" => bymonthday = Some(parse_int_list(val)?),
            "BYYEARDAY" => byyearday = Some(parse_int_list(val)?),
            "BYEASTER" => byeaster = Some(parse_int_list(val)?),
            "BYWEEKNO" => byweekno = Some(parse_int_list(val)?),
            "BYDAY" | "BYWEEKDAY" => byweekday = Some(parse_weekday_list(val)?),
            "BYHOUR" => {
                byhour = Some(parse_int_list(val)?.into_iter().map(|x| x as u8).collect());
            }
            "BYMINUTE" => {
                byminute = Some(parse_int_list(val)?.into_iter().map(|x| x as u8).collect());
            }
            "BYSECOND" => {
                bysecond = Some(parse_int_list(val)?.into_iter().map(|x| x as u8).collect());
            }
            _ => {
                return Err(RRuleError::ValueError(
                    format!("unknown parameter '{name}'").into(),
                ));
            }
        }
    }

    let freq = freq.ok_or(RRuleError::MissingFrequency)?;
    let mut builder = RRuleBuilder::new(freq).interval(builder_interval);

    if let Some(dt) = dtstart {
        builder = builder.dtstart(dt);
    }
    if let Some(v) = builder_wkst {
        builder = builder.wkst(v);
    }
    if let Some(v) = builder_count {
        builder = builder.count(v);
    }
    if let Some(v) = builder_until {
        builder = builder.until(v);
    }
    if let Some(v) = bysetpos {
        builder = builder.bysetpos(v);
    }
    if let Some(v) = bymonth {
        builder = builder.bymonth(v);
    }
    if let Some(v) = bymonthday {
        builder = builder.bymonthday(v);
    }
    if let Some(v) = byyearday {
        builder = builder.byyearday(v);
    }
    if let Some(v) = byeaster {
        builder = builder.byeaster(v);
    }
    if let Some(v) = byweekno {
        builder = builder.byweekno(v);
    }
    if let Some(v) = byweekday {
        builder = builder.byweekday(v);
    }
    if let Some(v) = byhour {
        builder = builder.byhour(v);
    }
    if let Some(v) = byminute {
        builder = builder.byminute(v);
    }
    if let Some(v) = bysecond {
        builder = builder.bysecond(v);
    }

    builder.build()
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_int_list(s: &str) -> Result<Vec<i32>, RRuleError> {
    s.split(',')
        .map(|x| {
            x.trim()
                .parse::<i32>()
                .map_err(|_| RRuleError::ValueError(format!("invalid integer: {x}").into()))
        })
        .collect()
}

fn parse_weekday_name(s: &str) -> Result<u8, RRuleError> {
    let upper = s.to_ascii_uppercase();
    WDAY_MAP
        .get(upper.as_str())
        .copied()
        .ok_or_else(|| RRuleError::ValueError(format!("invalid weekday: {s}").into()))
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

        // Find where the numeric prefix ends and the day name begins
        let mut i = wday_str.len();
        for (pos, ch) in wday_str.char_indices() {
            if !matches!(ch, '+' | '-' | '0'..='9') {
                i = pos;
                break;
            }
        }
        let n_str = &wday_str[..i];
        let w_str = &wday_str[i..];
        if w_str.is_empty() {
            return Err(RRuleError::ValueError(
                format!("invalid BYDAY: missing weekday name in '{wday_str}'").into(),
            ));
        }
        let w = parse_weekday_name(w_str)?;
        let n =
            if n_str.is_empty() {
                None
            } else {
                Some(n_str.parse::<i32>().map_err(|_| {
                    RRuleError::ValueError(format!("invalid BYDAY: {wday_str}").into())
                })?)
            };
        result.push(Weekday::new(w, n).map_err(|e| RRuleError::ValueError(e.to_string().into()))?);
    }
    Ok(result)
}

/// Parse a datetime in RFC 5545 format: YYYYMMDD or YYYYMMDDTHHmmSS
pub fn parse_rfc_datetime(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim().trim_end_matches(['Z', 'z']);
    if s.len() == 15 && s.as_bytes().get(8).is_some_and(|&b| b == b'T' || b == b't') {
        let year = s[0..4].parse::<i32>().ok()?;
        let month = s[4..6].parse::<u32>().ok()?;
        let day = s[6..8].parse::<u32>().ok()?;
        let hour = s[9..11].parse::<u32>().ok()?;
        let min = s[11..13].parse::<u32>().ok()?;
        let sec = s[13..15].parse::<u32>().ok()?;
        NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(hour, min, sec)
    } else if s.len() == 8 {
        let year = s[0..4].parse::<i32>().ok()?;
        let month = s[4..6].parse::<u32>().ok()?;
        let day = s[6..8].parse::<u32>().ok()?;
        NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(0, 0, 0)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::dt;
    use chrono::Datelike;

    #[test]
    fn test_parse_rfc_datetime() {
        assert_eq!(
            parse_rfc_datetime("19970902T090000"),
            Some(dt(1997, 9, 2, 9, 0, 0))
        );
        assert_eq!(
            parse_rfc_datetime("19970902T090000Z"),
            Some(dt(1997, 9, 2, 9, 0, 0))
        );
        assert_eq!(
            parse_rfc_datetime("19970902"),
            Some(dt(1997, 9, 2, 0, 0, 0))
        );
        assert_eq!(parse_rfc_datetime("invalid"), None);
    }

    #[test]
    fn test_rrulestr_basic() {
        let result = rrulestr(
            "FREQ=YEARLY;COUNT=3",
            Some(dt(2020, 1, 1, 0, 0, 0)),
            false,
            false,
            false,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(
            all,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2021, 1, 1, 0, 0, 0),
                dt(2022, 1, 1, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_rrulestr_with_dtstart_line() {
        let result = rrulestr(
            "DTSTART:19970902T090000\nRRULE:FREQ=YEARLY;COUNT=3",
            None,
            false,
            false,
            true,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(
            all,
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1998, 9, 2, 9, 0, 0),
                dt(1999, 9, 2, 9, 0, 0),
            ]
        );
    }

    #[test]
    fn test_rrulestr_forceset() {
        let result = rrulestr(
            "FREQ=DAILY;COUNT=3",
            Some(dt(2020, 1, 1, 0, 0, 0)),
            true,
            false,
            false,
        )
        .unwrap();
        assert!(matches!(result, RRuleStrResult::Set(_)));
    }

    #[test]
    fn test_rrulestr_empty() {
        assert!(rrulestr("", None, false, false, false).is_err());
    }

    #[test]
    fn test_parse_weekday_list_all_numeric() {
        // All-numeric input like "123" should report missing weekday name
        let err = parse_weekday_list("123");
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(
            msg.contains("missing weekday name") && msg.contains("123"),
            "error should report missing weekday name with input, got: {msg}"
        );
    }

    #[test]
    fn test_parse_weekday_list_just_sign() {
        // "+" alone is not a valid BYDAY — no weekday name after sign
        let err = parse_weekday_list("+");
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(msg.contains("missing weekday name"), "got: {msg}");
    }

    #[test]
    fn test_parse_weekday_list_just_number() {
        // "1" alone is not a valid BYDAY
        assert!(parse_weekday_list("1").is_err());
    }

    #[test]
    fn test_parse_weekday_list_valid() {
        let result = parse_weekday_list("MO").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].weekday(), 0);
        assert_eq!(result[0].n(), None);

        let result = parse_weekday_list("+1MO,-1FR").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].weekday(), 0);
        assert_eq!(result[0].n(), Some(1));
        assert_eq!(result[1].weekday(), 4);
        assert_eq!(result[1].n(), Some(-1));
    }

    #[test]
    fn test_rrulestr_with_byday() {
        let result = rrulestr(
            "FREQ=WEEKLY;COUNT=4;BYDAY=TU,TH",
            Some(dt(1997, 9, 2, 9, 0, 0)),
            false,
            false,
            false,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(
            all,
            vec![
                dt(1997, 9, 2, 9, 0, 0),
                dt(1997, 9, 4, 9, 0, 0),
                dt(1997, 9, 9, 9, 0, 0),
                dt(1997, 9, 11, 9, 0, 0),
            ]
        );
    }

    // ===================================================================
    // rrulestr with EXDATE
    // ===================================================================

    #[test]
    fn test_rrulestr_exdate() {
        let result = rrulestr(
            "DTSTART:20200101T000000\nRRULE:FREQ=DAILY;COUNT=5\nEXDATE:20200103T000000",
            None,
            false,
            false,
            true,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(
            all,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 2, 0, 0, 0),
                dt(2020, 1, 4, 0, 0, 0),
                dt(2020, 1, 5, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_rrulestr_exdate_multiple() {
        let result = rrulestr(
            "DTSTART:20200101T000000\nRRULE:FREQ=DAILY;COUNT=5\nEXDATE:20200102T000000,20200104T000000",
            None,
            false,
            false,
            true,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(
            all,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 3, 0, 0, 0),
                dt(2020, 1, 5, 0, 0, 0),
            ]
        );
    }

    // ===================================================================
    // rrulestr with RDATE
    // ===================================================================

    #[test]
    fn test_rrulestr_rdate() {
        let result = rrulestr(
            "DTSTART:20200101T000000\nRRULE:FREQ=DAILY;COUNT=3\nRDATE:20200110T000000",
            None,
            false,
            false,
            true,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(
            all,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 2, 0, 0, 0),
                dt(2020, 1, 3, 0, 0, 0),
                dt(2020, 1, 10, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_rrulestr_rdate_multiple() {
        let result = rrulestr(
            "DTSTART:20200101T000000\nRDATE:20200115T000000,20200105T000000",
            None,
            true,
            false,
            true,
        )
        .unwrap();
        let all = result.all();
        // Should be sorted
        assert_eq!(
            all,
            vec![dt(2020, 1, 5, 0, 0, 0), dt(2020, 1, 15, 0, 0, 0),]
        );
    }

    // ===================================================================
    // rrulestr with EXRULE
    // ===================================================================

    #[test]
    fn test_rrulestr_exrule() {
        let result = rrulestr(
            "DTSTART:20200101T000000\nRRULE:FREQ=DAILY;COUNT=6\nEXRULE:FREQ=DAILY;INTERVAL=2;COUNT=3",
            None,
            false,
            false,
            true,
        )
        .unwrap();
        let all = result.all();
        // Daily: 1,2,3,4,5,6; Exrule excludes 1,3,5
        assert_eq!(
            all,
            vec![
                dt(2020, 1, 2, 0, 0, 0),
                dt(2020, 1, 4, 0, 0, 0),
                dt(2020, 1, 6, 0, 0, 0),
            ]
        );
    }

    // ===================================================================
    // rrulestr compatible mode
    // ===================================================================

    #[test]
    fn test_rrulestr_compatible() {
        // compatible = true means forceset + unfold + dtstart added as rdate
        let result = rrulestr(
            "DTSTART:20200101T000000\nRRULE:FREQ=YEARLY;COUNT=2",
            None,
            false,
            true, // compatible
            false,
        )
        .unwrap();
        assert!(matches!(result, RRuleStrResult::Set(_)));
        let all = result.all();
        // dtstart is added as rdate, but dedup should handle it
        assert!(all.contains(&dt(2020, 1, 1, 0, 0, 0)));
    }

    // ===================================================================
    // rrulestr with multiple RRULE lines
    // ===================================================================

    #[test]
    fn test_rrulestr_multiple_rrules() {
        let result = rrulestr(
            "DTSTART:20200101T000000\nRRULE:FREQ=YEARLY;COUNT=3\nRRULE:FREQ=MONTHLY;COUNT=3;BYMONTHDAY=15",
            None,
            false,
            false,
            true,
        )
        .unwrap();
        assert!(matches!(result, RRuleStrResult::Set(_)));
        let all = result.all();
        // Should have results from both rules, merged and sorted
        assert!(!all.is_empty());
        // Check sorting
        for w in all.windows(2) {
            assert!(w[0] <= w[1], "results should be sorted");
        }
    }

    // ===================================================================
    // rrulestr unfold (line continuation)
    // ===================================================================

    #[test]
    fn test_rrulestr_unfold() {
        // RFC 5545 line folding: continuation lines start with space
        let result = rrulestr(
            "DTSTART:20200101T000000\nRRULE:FREQ=YEARLY\n ;COUNT=3",
            None,
            false,
            false,
            true, // unfold enabled
        )
        .unwrap();
        let all = result.all();
        assert_eq!(all.len(), 3);
    }

    // ===================================================================
    // Case insensitivity
    // ===================================================================

    #[test]
    fn test_rrulestr_case_insensitive() {
        let result = rrulestr(
            "freq=daily;count=3",
            Some(dt(2020, 1, 1, 0, 0, 0)),
            false,
            false,
            false,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(all.len(), 3);
    }

    // ===================================================================
    // Error cases
    // ===================================================================

    #[test]
    fn test_rrulestr_missing_freq() {
        let err = rrulestr(
            "COUNT=3",
            Some(dt(2020, 1, 1, 0, 0, 0)),
            false,
            false,
            false,
        );
        assert!(err.is_err());
    }

    #[test]
    fn test_rrulestr_invalid_freq() {
        let err = rrulestr(
            "FREQ=BIWEEKLY;COUNT=3",
            Some(dt(2020, 1, 1, 0, 0, 0)),
            false,
            false,
            false,
        );
        assert!(err.is_err());
    }

    #[test]
    fn test_rrulestr_invalid_interval() {
        let err = rrulestr(
            "FREQ=DAILY;INTERVAL=abc;COUNT=3",
            Some(dt(2020, 1, 1, 0, 0, 0)),
            false,
            false,
            false,
        );
        assert!(err.is_err());
    }

    #[test]
    fn test_rrulestr_invalid_count() {
        let err = rrulestr(
            "FREQ=DAILY;COUNT=abc",
            Some(dt(2020, 1, 1, 0, 0, 0)),
            false,
            false,
            false,
        );
        assert!(err.is_err());
    }

    #[test]
    fn test_rrulestr_invalid_until() {
        let err = rrulestr(
            "FREQ=DAILY;UNTIL=notadate",
            Some(dt(2020, 1, 1, 0, 0, 0)),
            false,
            false,
            false,
        );
        assert!(err.is_err());
    }

    #[test]
    fn test_rrulestr_unknown_property() {
        let err = rrulestr(
            "VTODO:something\nRRULE:FREQ=DAILY;COUNT=3",
            None,
            false,
            false,
            true,
        );
        assert!(err.is_err());
    }

    #[test]
    fn test_rrulestr_unknown_parameter() {
        let err = rrulestr(
            "FREQ=DAILY;COUNT=3;FOOBAR=123",
            Some(dt(2020, 1, 1, 0, 0, 0)),
            false,
            false,
            false,
        );
        assert!(err.is_err());
    }

    #[test]
    fn test_rrulestr_whitespace_only() {
        assert!(rrulestr("   ", None, false, false, false).is_err());
    }

    // ===================================================================
    // rrulestr with all byxxx parameters
    // ===================================================================

    #[test]
    fn test_rrulestr_with_bymonth() {
        let result = rrulestr(
            "FREQ=YEARLY;COUNT=4;BYMONTH=1,3;BYMONTHDAY=1",
            Some(dt(1997, 9, 2, 9, 0, 0)),
            false,
            false,
            false,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(all.len(), 4);
        for r in &all {
            assert!(r.month() == 1 || r.month() == 3);
            assert_eq!(r.day(), 1);
        }
    }

    #[test]
    fn test_rrulestr_with_bysetpos() {
        let result = rrulestr(
            "FREQ=MONTHLY;COUNT=3;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1",
            Some(dt(1997, 9, 2, 9, 0, 0)),
            false,
            false,
            false,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(all.len(), 3);
        // Last workday of each month
    }

    #[test]
    fn test_rrulestr_with_byhour_byminute_bysecond() {
        let result = rrulestr(
            "FREQ=DAILY;COUNT=2;BYHOUR=9,17;BYMINUTE=0;BYSECOND=0",
            Some(dt(1997, 9, 2, 9, 0, 0)),
            false,
            false,
            false,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(
            all,
            vec![dt(1997, 9, 2, 9, 0, 0), dt(1997, 9, 2, 17, 0, 0),]
        );
    }

    #[test]
    fn test_rrulestr_with_byyearday() {
        let result = rrulestr(
            "FREQ=YEARLY;COUNT=3;BYYEARDAY=1,100,200",
            Some(dt(1997, 9, 2, 9, 0, 0)),
            false,
            false,
            false,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_rrulestr_with_byweekno() {
        let result = rrulestr(
            "FREQ=YEARLY;COUNT=3;BYWEEKNO=20;BYDAY=MO",
            Some(dt(1997, 9, 2, 9, 0, 0)),
            false,
            false,
            false,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_rrulestr_with_wkst() {
        let result = rrulestr(
            "FREQ=WEEKLY;COUNT=3;WKST=SU",
            Some(dt(1997, 9, 2, 9, 0, 0)),
            false,
            false,
            false,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_rrulestr_with_byeaster() {
        let result = rrulestr(
            "FREQ=YEARLY;COUNT=3;BYEASTER=0",
            Some(dt(1997, 1, 1, 0, 0, 0)),
            false,
            false,
            false,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(
            all,
            vec![
                dt(1997, 3, 30, 0, 0, 0),
                dt(1998, 4, 12, 0, 0, 0),
                dt(1999, 4, 4, 0, 0, 0),
            ]
        );
    }

    // ===================================================================
    // parse_rfc_datetime edge cases
    // ===================================================================

    #[test]
    fn test_parse_rfc_datetime_edge_cases() {
        // Too short
        assert_eq!(parse_rfc_datetime("2020"), None);
        // Wrong separator
        assert_eq!(parse_rfc_datetime("20200101X090000"), None);
        // Invalid month
        assert_eq!(parse_rfc_datetime("20201301"), None);
        // Invalid day
        assert_eq!(parse_rfc_datetime("20200132"), None);
    }

    // ===================================================================
    // parse_weekday_list edge cases
    // ===================================================================

    #[test]
    fn test_parse_weekday_list_empty() {
        assert!(parse_weekday_list("").is_err());
    }

    #[test]
    fn test_parse_weekday_list_invalid_day() {
        assert!(parse_weekday_list("XX").is_err());
    }

    #[test]
    fn test_parse_weekday_list_mixed_valid() {
        // Valid weekdays with and without N
        let result = parse_weekday_list("MO,+2TU,-1FR,WE").unwrap();
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].weekday(), 0); // MO
        assert_eq!(result[0].n(), None);
        assert_eq!(result[1].weekday(), 1); // TU
        assert_eq!(result[1].n(), Some(2));
        assert_eq!(result[2].weekday(), 4); // FR
        assert_eq!(result[2].n(), Some(-1));
        assert_eq!(result[3].weekday(), 2); // WE
        assert_eq!(result[3].n(), None);
    }

    // ===================================================================
    // rrulestr with RRULE: prefix
    // ===================================================================

    #[test]
    fn test_rrulestr_with_rrule_prefix() {
        let result = rrulestr(
            "RRULE:FREQ=DAILY;COUNT=3",
            Some(dt(2020, 1, 1, 0, 0, 0)),
            false,
            false,
            false,
        )
        .unwrap();
        let all = result.all();
        assert_eq!(all.len(), 3);
    }

    // ===================================================================
    // RRuleStrResult::all for both variants
    // ===================================================================

    #[test]
    fn test_rrulestr_result_all_single() {
        let result = rrulestr(
            "FREQ=DAILY;COUNT=2",
            Some(dt(2020, 1, 1, 0, 0, 0)),
            false,
            false,
            false,
        )
        .unwrap();
        assert!(matches!(result, RRuleStrResult::Single(_)));
        assert_eq!(result.all().len(), 2);
    }

    #[test]
    fn test_rrulestr_result_all_set() {
        let result = rrulestr(
            "FREQ=DAILY;COUNT=2",
            Some(dt(2020, 1, 1, 0, 0, 0)),
            true, // forceset
            false,
            false,
        )
        .unwrap();
        assert!(matches!(result, RRuleStrResult::Set(_)));
        assert_eq!(result.all().len(), 2);
    }

    // ===================================================================
    // rrulestr no RRULE found
    // ===================================================================

    #[test]
    fn test_rrulestr_no_rrule() {
        let err = rrulestr("DTSTART:20200101T000000", None, false, false, true);
        assert!(err.is_err());
    }

    // ---- Coverage: error paths in parse ----

    #[test]
    fn test_rrulestr_invalid_param_without_equals() {
        // "FREQ" without "=" should error
        let result = rrulestr("FREQ", None, false, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_rrulestr_unknown_param_name() {
        // "XRULE:FREQ=DAILY" — unknown type prefix
        let result = rrulestr("XRULE:FREQ=DAILY;COUNT=3", None, false, false, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_rrulestr_line_unfold() {
        // Test RFC line unfolding (continuation lines starting with space)
        let input = "DTSTART:20200101T000000\nRRULE:FREQ=DAILY;\n COUNT=3";
        let result = rrulestr(input, None, false, false, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().all().len(), 3);
    }
}
