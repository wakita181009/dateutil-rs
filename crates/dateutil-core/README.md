# dateutil

[![Crates.io](https://img.shields.io/crates/v/dateutil.svg?style=flat-square)](https://crates.io/crates/dateutil)
[![docs.rs](https://img.shields.io/docsrs/dateutil?style=flat-square)](https://docs.rs/dateutil)
[![License](https://img.shields.io/crates/l/dateutil.svg?style=flat-square)](https://crates.io/crates/dateutil)
[![CI](https://github.com/wakita181009/dateutil-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/wakita181009/dateutil-rs/actions/workflows/ci.yml)

Fast date utility library for Rust — parser, relativedelta, rrule, timezone.

A performance-focused Rust reimplementation of [python-dateutil](https://github.com/dateutil/dateutil), designed for native Rust usage. Also available as a Python package ([python-dateutil-rs](https://pypi.org/project/python-dateutil-rs/)) via PyO3.

## Features

- **Parser** — Parse human-readable date strings with a zero-copy tokenizer and PHF lookup tables
- **ISO 8601** — Strict ISO-8601 parsing via `isoparse()`
- **RelativeDelta** — Relative date arithmetic (months, years, weekdays, etc.)
- **RRule / RRuleSet** — RFC 5545 recurrence rules with bitflag filters and buffer-reusing iteration
- **Timezone** — `gettz()` with TZif file support, DST handling, and process-lifetime caching
- **Easter** — Easter date calculation (Julian, Orthodox, Western)
- **Weekday** — `MO`–`SU` constants with N-th occurrence support

## Installation

```toml
[dependencies]
dateutil = "0.1"
```

## Quick Start

### Parsing date strings

```rust
use chrono::NaiveDate;
use dateutil::parser;

// Parse a human-readable date string
let dt = parser::parse("January 15, 2026 10:30 AM", None, None, false)
    .unwrap();
assert_eq!(dt.date(), NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());

// ISO-8601 strict parsing
use dateutil::parser::isoparser::isoparse;
let dt = isoparse("2026-01-15T10:30:00").unwrap();
```

### Relative deltas

```rust
use chrono::{NaiveDate, NaiveDateTime};
use dateutil::relativedelta::RelativeDelta;

let dt = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap().and_hms_opt(0, 0, 0).unwrap();
let rd = RelativeDelta::new().months(1).build();

// Jan 31 + 1 month = Feb 28 (clamped)
let result = rd.add_to_datetime(dt);
assert_eq!(result.date(), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
```

### Recurrence rules

```rust
use chrono::NaiveDate;
use dateutil::rrule::{Frequency, Recurrence, RRuleBuilder};

let dtstart = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
    .and_hms_opt(0, 0, 0).unwrap();

let rule = RRuleBuilder::new(Frequency::Monthly)
    .dtstart(dtstart)
    .count(3)
    .build()
    .unwrap();

let dates = rule.all();
assert_eq!(dates.len(), 3);

// Also works as an iterator
for dt in rule.iter().take(3) {
    println!("{}", dt);
}
```

### Parsing RFC 5545 RRULE strings

```rust
use dateutil::rrule::parse::rrulestr;

let result = rrulestr(
    "DTSTART:20260101T000000\nRRULE:FREQ=WEEKLY;COUNT=4;BYDAY=MO,WE,FR",
    false,
).unwrap();
```

### Timezones

```rust
use chrono::NaiveDate;
use dateutil::tz::{self, TzOps};

// Look up an IANA timezone (cached after first call)
let tz = tz::gettz(Some("America/New_York")).unwrap();

let dt = NaiveDate::from_ymd_opt(2026, 7, 15).unwrap()
    .and_hms_opt(12, 0, 0).unwrap();

// UTC offset in seconds (EDT = -4h)
assert_eq!(tz.utcoffset(dt, false), -4 * 3600);

// DST gap/overlap utilities
assert!(tz::datetime_exists(dt, &tz));
assert!(!tz::datetime_ambiguous(dt, &tz));
```

### Easter

```rust
use chrono::NaiveDate;
use dateutil::easter::{easter, EasterMethod};

let date = easter(2026, EasterMethod::Western).unwrap();
assert_eq!(date, NaiveDate::from_ymd_opt(2026, 4, 5).unwrap());
```

### Weekday constants

```rust
use dateutil::common::{MO, TU, FR};

// N-th occurrence (e.g., 2nd Tuesday)
let second_tue = TU.nth(2);
assert_eq!(second_tue.n(), Some(2));
```

## Modules

| Module | Description |
|--------|-------------|
| `dateutil::parser` | Date string parsing (`parse`, `isoparse`, `parse_to_result`) |
| `dateutil::relativedelta` | Relative date arithmetic (`RelativeDelta`, `RelativeDeltaBuilder`) |
| `dateutil::rrule` | RFC 5545 recurrence rules (`RRule`, `RRuleBuilder`, `Recurrence`) |
| `dateutil::rrule::set` | Recurrence rule sets (`RRuleSet`) |
| `dateutil::rrule::parse` | RRULE string parsing (`rrulestr`) |
| `dateutil::tz` | Timezones (`gettz`, `TzOps`, `TimeZone`, `TzFile`, `TzOffset`, `TzUtc`, `TzLocal`) |
| `dateutil::easter` | Easter calculation (`easter`, `EasterMethod`) |
| `dateutil::common` | Weekday constants (`MO`–`SU`, `Weekday`) |
| `dateutil::error` | Error types (`ParseError`, `RRuleError`, `TzError`, etc.) |

## Performance

Benchmarked against python-dateutil (via PyO3 bindings):

| Module | Speedup |
|--------|---------|
| Parser (parse) | 19.5x–36.0x |
| Parser (isoparse) | 13.0x–38.4x |
| RRule | 5.9x–63.7x |
| Timezone | 1.0x–896.7x |
| RelativeDelta | 2.0x–28.1x |
| Easter | 5.0x–7.3x |

Key optimizations: zero-copy tokenizer, PHF compile-time hash tables, bitflag-based filters, `SmallVec` buffer reuse, `FxHashMap` timezone cache, `TzOps` trait for generic zero-clone dispatch.

## License

[MIT](../../LICENSE)
