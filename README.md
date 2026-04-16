# python-dateutil-rs

[![PyPI](https://img.shields.io/pypi/v/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![Python](https://img.shields.io/pypi/pyversions/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![License](https://img.shields.io/pypi/l/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![CI](https://github.com/wakita181009/dateutil-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/wakita181009/dateutil-rs/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/wakita181009/dateutil-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/wakita181009/dateutil-rs)

A high-performance Rust-backed port of [python-dateutil](https://github.com/dateutil/dateutil) (v2.9.0).

> **Status:** All core modules (easter, relativedelta, parser, rrule, tz) are rewritten in Rust via PyO3/maturin. The optimized `dateutil-core` + `dateutil-py` architecture delivers **2x-897x** speedups over python-dateutil.

## Features

- **Drop-in replacement** for `python-dateutil` — same API, same behavior
- **Rust-accelerated:** easter, relativedelta, parser (`parse` / `isoparse`), rrule, tz, weekday
- **Optimized core:** zero-copy parser, PHF lookup tables, bitflag filters, buffer-reusing rrule
- **Comprehensive test suite** inherited from the original project
- **Benchmark infrastructure** for side-by-side performance comparison
- **Python 3.10-3.14** supported on Linux and macOS

## Installation

```bash
pip install python-dateutil-rs
```

## Usage

```python
from dateutil import (
    parse, isoparse, relativedelta, rrule, rruleset, rrulestr,
    easter, gettz, tzutc, tzoffset,
    MONTHLY, MO, TU, WE, TH, FR, SA, SU,
)

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

Benchmarks compare the original `python-dateutil` (PyPI) and the Rust extension (`dateutil`) using pytest-benchmark.

#### Summary (vs python-dateutil)

| Module | Speedup |
|--------|---------|
| Parser (parse) | **19.5x-36.0x** |
| Parser (isoparse) | **13.0x-38.4x** |
| RRule | **5.9x-63.7x** |
| Timezone | **1.0x-896.7x** |
| RelativeDelta | **2.0x-28.1x** |
| Easter | **5.0x-7.3x** |

> Measured on Apple Silicon (M-series), Python 3.13, release build. Full results: [benchmarks/RESULTS.md](benchmarks/RESULTS.md)

```bash
# Install the original python-dateutil for comparison
uv pip install python-dateutil

# Run benchmarks
make bench

# Run and save results as JSON
make bench-save
```

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
├── python/dateutil/        # Python package (maturin mixed layout)
│   ├── __init__.py            # Re-exports from Rust native module
│   ├── _native.pyi            # Type stubs for native module
│   ├── py.typed               # PEP 561 marker
│   └── parser.py              # parserinfo (Python subclass support)
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
