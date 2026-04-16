# python-dateutil-rs

[![PyPI](https://img.shields.io/pypi/v/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![Python](https://img.shields.io/pypi/pyversions/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![License](https://img.shields.io/pypi/l/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![CI](https://github.com/wakita181009/dateutil-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/wakita181009/dateutil-rs/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/wakita181009/dateutil-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/wakita181009/dateutil-rs)

A high-performance, **drop-in replacement** for [python-dateutil](https://github.com/dateutil/dateutil) (v2.9.0), powered by Rust.

> **Drop-in compatible:** Install `python-dateutil-rs` and your existing `from dateutil.parser import parse`, `from dateutil.tz import tzutc`, etc. continue to work — no code changes required, just **2x–897x faster**.

## Features

- **True drop-in replacement** — provides `dateutil` package with the same submodule structure (`dateutil.parser`, `dateutil.tz`, `dateutil.relativedelta`, `dateutil.rrule`, `dateutil.easter`)
- **Zero code changes** — existing imports like `from dateutil.parser import parse` work as-is
- **Rust-accelerated:** all core modules rewritten in Rust via PyO3/maturin
- **Optimized core:** zero-copy parser, PHF lookup tables, bitflag filters, buffer-reusing rrule
- **freezegun compatible** — exposes `dateutil.tz.UTC` constant for seamless time mocking
- **Comprehensive test suite** validated against python-dateutil behavior
- **Python 3.10–3.14** supported on Linux, macOS, and Windows

## Installation

```bash
pip install python-dateutil-rs
```

> **Note:** This package provides the `dateutil` namespace. If you have `python-dateutil` installed, uninstall it first to avoid conflicts: `pip uninstall python-dateutil`.

## Drop-in Replacement

Existing code that uses python-dateutil works without modification:

```python
# These imports work exactly the same as with python-dateutil
from dateutil.parser import parse, isoparse, parserinfo
from dateutil.tz import tzutc, tzoffset, tzlocal, gettz, UTC
from dateutil.relativedelta import relativedelta
from dateutil.rrule import rrule, rruleset, rrulestr, MONTHLY, WEEKLY, MO, FR
from dateutil.easter import easter, EASTER_WESTERN
```

## Usage

```python
from dateutil.parser import parse, isoparse
from dateutil.relativedelta import relativedelta
from dateutil.rrule import rrule, MONTHLY
from dateutil.tz import gettz, tzutc
from dateutil.easter import easter

# Parse date strings (zero-copy tokenizer)
dt = parse("2026-01-15T10:30:00+09:00")

# ISO-8601 strict parsing
dt = isoparse("2026-01-15T10:30:00")

# Relative deltas
next_month = dt + relativedelta(months=+1)

# Recurrence rules (buffer-reusing iterator)
monthly = rrule(MONTHLY, count=5, dtstart=parse("2026-01-01"))
dates = monthly.all()
dates = list(monthly)           # also iterable
first = monthly[0]              # indexing
subset = monthly[1:3]           # slicing
n = monthly.count()             # total occurrences
dt in monthly                   # membership test

# Timezones
tokyo = gettz("Asia/Tokyo")
utc = tzutc()

# Easter
easter_date = easter(2026)
```

### Flat Import Style

All symbols are also re-exported from the top-level `dateutil` package:

```python
from dateutil import parse, relativedelta, rrule, gettz, easter
```

## Development

### Prerequisites

- Python 3.10+
- Rust toolchain
- [uv](https://github.com/astral-sh/uv) (recommended) or pip

### Setup

```bash
git clone https://github.com/wakita181009/dateutil-rs.git
cd dateutil-rs
uv sync --extra dev
```

### Building

```bash
# Build the native extension
maturin develop --release

# Development build (faster compilation)
maturin develop -F python
```

### Running Tests

```bash
# Run the test suite
uv run pytest tests/ -x -q

# Run with coverage
uv run pytest tests/ --cov=dateutil

# Run Rust tests
cargo test -p dateutil-core
cargo test --workspace
```

### Linting

```bash
uv run ruff check tests/ python/
uv run ruff format --check tests/ python/
uv run mypy python/
cargo clippy --workspace
```

### Benchmarks

Performance measured against python-dateutil v2.9.0 (before the drop-in rename). Baseline results are preserved in [benchmarks/BASELINE.md](benchmarks/BASELINE.md).

#### Summary (vs python-dateutil)

| Module | Speedup |
|--------|---------|
| Parser (parse) | **19.5x–36.0x** |
| Parser (isoparse) | **13.0x–38.4x** |
| RRule | **5.9x–63.7x** |
| Timezone | **1.0x–896.7x** |
| RelativeDelta | **2.0x–28.1x** |
| Easter | **5.0x–7.3x** |

> Measured on Apple Silicon (M-series), Python 3.13, release build.

```bash
# Run benchmarks (Rust dateutil only, since the package now occupies the dateutil namespace)
make bench

# Run and save results as JSON
make bench-save
```

> **Note:** Since `python-dateutil-rs` provides the same `dateutil` namespace as `python-dateutil`, both cannot be installed simultaneously. The baseline comparison numbers above were captured before the namespace unification.

## Project Structure

```
dateutil-rs/
├── Cargo.toml                 # Workspace root
├── pyproject.toml             # Python project config (maturin)
├── crates/
│   ├── dateutil-core/         # Pure Rust optimized core (crates.io)
│   │   └── src/
│   │       ├── lib.rs         # Crate root, public API
│   │       ├── common.rs      # Weekday (MO-SU with N-th occurrence)
│   │       ├── easter.rs      # Easter date calculations
│   │       ├── error.rs       # Shared error types
│   │       ├── relativedelta.rs
│   │       ├── parser.rs      # parse() entry point
│   │       ├── parser/        # tokenizer, parserinfo, isoparser
│   │       ├── rrule.rs       # RRule entry point
│   │       ├── rrule/         # iter, parse (rrulestr), set
│   │       └── tz/            # tzutc, tzoffset, tzfile, tzlocal
│   └── dateutil-py/           # PyO3 binding layer → PyPI package
│       └── src/
│           ├── lib.rs         # Module registration
│           ├── py.rs          # Binding root + #[pymodule]
│           └── py/            # Per-module bindings (common, conv, easter, parser, relativedelta, rrule, tz)
├── python/dateutil/        # Python package (drop-in replacement for python-dateutil)
│   ├── __init__.py            # Re-exports from Rust native module
│   ├── _native.pyi            # Type stubs for native module
│   ├── py.typed               # PEP 561 marker
│   ├── parser.py              # dateutil.parser (parse, isoparse, parserinfo)
│   ├── tz.py                  # dateutil.tz (tzutc, tzoffset, gettz, UTC, ...)
│   ├── relativedelta.py       # dateutil.relativedelta
│   ├── rrule.py               # dateutil.rrule (rrule, rruleset, rrulestr, freq constants)
│   └── easter.py              # dateutil.easter (easter, calendar constants)
├── tests/                     # Python test suite
├── benchmarks/                # pytest-benchmark comparisons
├── .github/workflows/         # CI (ci.yml, publish.yml)
├── Makefile
└── LICENSE
```

### Crate Roles

| Crate | Purpose | PyO3 | Publish To |
|-------|---------|------|------------|
| `dateutil-core` | Pure Rust optimized core | No | crates.io |
| `dateutil-py` | PyO3 binding layer | Yes | PyPI (`python-dateutil-rs`) |

## Implementation Status

| Module | Status | Notes |
|--------|:------:|-------|
| common (Weekday) | ✅ | MO-SU constants with N-th occurrence |
| easter | ✅ | 5.0x-7.3x faster, 3 calendar methods |
| relativedelta | ✅ | 2.0x-28.1x faster |
| parser (parse) | ✅ | 19.5x-36.0x faster, zero-copy tokenizer, PHF lookups |
| parser (isoparse) | ✅ | 13.0x-38.4x faster |
| parser (parserinfo) | ✅ | Customizable via Python subclass |
| rrule / rruleset | ✅ | 5.9x-63.7x faster, bitflag filters, buffer reuse |
| rrulestr | ✅ | RFC 5545 string parsing |
| tz (tzutc, tzoffset, tzfile, tzlocal) | ✅ | 1.0x-896.7x faster |
| tz utilities (gettz, datetime_exists, etc.) | ✅ | gettz with caching |

## License

[MIT](LICENSE)
