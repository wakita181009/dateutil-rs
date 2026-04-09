//! RFC 5545 RRULE string parsing.

use chrono::{NaiveDate, NaiveDateTime};
use phf::phf_map;

use crate::common::Weekday;
use crate::error::RRuleError;

use super::set::RRuleSet;
use super::{Frequency, RRule, RRuleBuilder};

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
        let rule = parse_rfc_rrule(&lines[0], dtstart)?;
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
        let (name, value) = if let Some(idx) = line.find(':') {
            (line[..idx].to_string(), line[idx + 1..].to_string())
        } else {
            ("RRULE".to_string(), line.clone())
        };

        let parms: Vec<&str> = name.split(';').collect();
        let name = parms[0];

        match name {
            "RRULE" => rrulevals.push(value),
            "RDATE" => rdatevals.push(value),
            "EXRULE" => exrulevals.push(value),
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
                return Err(RRuleError::ValueError(
                    format!("unsupported property: {name}").into(),
                ));
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
        let rule = parse_rfc_rrule(&rrulevals[0], dtstart)?;
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

fn parse_rfc_rrule(
    line: &str,
    dtstart: Option<NaiveDateTime>,
) -> Result<RRule, RRuleError> {
    let value = if let Some((name, val)) = line.split_once(':') {
        if name != "RRULE" {
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
        let (name, val) = pair.split_once('=').ok_or_else(|| {
            RRuleError::ValueError(format!("invalid RRULE parameter: {pair}").into())
        })?;

        match name {
            "FREQ" => {
                freq = Some(
                    FREQ_MAP
                        .get(val)
                        .copied()
                        .ok_or_else(|| RRuleError::InvalidFrequency(val.into()))?,
                );
            }
            "INTERVAL" => {
                builder_interval = val
                    .parse()
                    .map_err(|_| RRuleError::ValueError(format!("invalid INTERVAL: {val}").into()))?;
            }
            "WKST" => {
                builder_wkst = Some(parse_weekday_name(val)?);
            }
            "COUNT" => {
                builder_count = Some(val.parse().map_err(|_| {
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
                bymonth = Some(
                    parse_int_list(val)?
                        .into_iter()
                        .map(|x| x as u8)
                        .collect(),
                );
            }
            "BYMONTHDAY" => bymonthday = Some(parse_int_list(val)?),
            "BYYEARDAY" => byyearday = Some(parse_int_list(val)?),
            "BYEASTER" => byeaster = Some(parse_int_list(val)?),
            "BYWEEKNO" => byweekno = Some(parse_int_list(val)?),
            "BYDAY" | "BYWEEKDAY" => byweekday = Some(parse_weekday_list(val)?),
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
    WDAY_MAP
        .get(s)
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
        let n = if n_str.is_empty() {
            None
        } else {
            Some(
                n_str
                    .parse::<i32>()
                    .map_err(|_| RRuleError::ValueError(format!("invalid BYDAY: {wday_str}").into()))?,
            )
        };
        result.push(
            Weekday::new(w, n)
                .map_err(|e| RRuleError::ValueError(e.to_string().into()))?,
        );
    }
    Ok(result)
}

/// Parse a datetime in RFC 5545 format: YYYYMMDD or YYYYMMDDTHHmmSS
pub fn parse_rfc_datetime(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim().trim_end_matches('Z');
    if s.len() == 15 && s.as_bytes().get(8) == Some(&b'T') {
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
    use chrono::NaiveDate;

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, mi, s)
            .unwrap()
    }

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
}
