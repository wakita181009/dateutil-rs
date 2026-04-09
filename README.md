# python-dateutil-rs

[![PyPI](https://img.shields.io/pypi/v/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![Python](https://img.shields.io/pypi/pyversions/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![License](https://img.shields.io/pypi/l/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![CI](https://github.com/wakita181009/dateutil-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/wakita181009/dateutil-rs/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/wakita181009/dateutil-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/wakita181009/dateutil-rs)

A high-performance Rust-backed port of [python-dateutil](https://github.com/dateutil/dateutil) (v2.9.0).

> **Status:** All core modules (easter, relativedelta, parser, rrule, tz) are rewritten in Rust via PyO3/maturin, delivering **1.3x-94x** speedups over python-dateutil. A next-gen optimized core (v1) is in active development.

## Features

- **Drop-in replacement** for `python-dateutil` — same API, same behavior
- **Rust-accelerated:** easter, relativedelta, parser (`parse` / `isoparse`), rrule, tz, weekday
- **Full module coverage:** parser, relativedelta, rrule, tz, easter, utils
- **v1 optimized core:** zero-copy parser, PHF lookup tables, bitflag filters, buffer-reusing rrule
- **Comprehensive test suite** inherited from the original project
- **Benchmark infrastructure** for side-by-side performance comparison (original vs Rust)
- **Python 3.10-3.14** supported on Linux and macOS

## Installation

```bash
pip install python-dateutil-rs
```

## Usage

### v0 API (python-dateutil compatible)

```python
from dateutil_rs.parser import parse
from dateutil_rs.relativedelta import relativedelta
from dateutil_rs.rrule import rrule, MONTHLY
from dateutil_rs.tz import gettz, tzutc
from dateutil_rs.easter import easter

# Parse date strings
dt = parse("2024-01-15T10:30:00+09:00")

# Relative deltas
next_month = dt + relativedelta(months=+1)

# Recurrence rules
monthly = rrule(MONTHLY, count=5, dtstart=parse("2024-01-01"))

# Timezones
tokyo = gettz("Asia/Tokyo")
utc = tzutc()

# Easter
easter_date = easter(2024)
```

### v1 API (optimized, streamlined)

```python
from dateutil_rs.v1.parser import parse, isoparse
from dateutil_rs.v1.relativedelta import relativedelta
from dateutil_rs.v1.rrule import rrule, rruleset, MONTHLY
from dateutil_rs.v1.easter import easter
from dateutil_rs.v1.common import MO, TU, WE, TH, FR, SA, SU

# Parse date strings (zero-copy tokenizer)
dt = parse("2024-01-15T10:30:00")

# Recurrence rules (buffer-reusing iterator)
monthly = rrule(MONTHLY, count=5, dtstart=dt)
dates = monthly.all()
```

## Development

### Prerequisites

- Python 3.10+
- [uv](https://github.com/astral-sh/uv) (recommended) or pip

### Setup

```bash
git clone https://github.com/wakita181009/dateutil-rs.git
cd dateutil-rs
uv sync --extra dev
```

### Running Tests

```bash
# Run the test suite
uv run pytest tests/ -x -q

# Run with coverage
uv run pytest tests/ --cov=dateutil

# Run Rust tests
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

Benchmarks compare the original `python-dateutil` (PyPI) and the Rust extension (`dateutil_rs`) using pytest-benchmark.

#### Easter

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| single call (Western) | 0.51 us | 0.13 us | **3.9x** |
| single call (Orthodox) | 0.35 us | 0.11 us | **3.2x** |
| single call (Julian) | 0.29 us | 0.06 us | **4.9x** |
| 1000 years (Western) | 437.88 us | 70.46 us | **6.2x** |
| 500 years x 3 methods | 567.99 us | 110.33 us | **5.2x** |

#### RelativeDelta

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| create simple | 0.94 us | 0.19 us | **4.9x** |
| add months to datetime | 1.62 us | 0.13 us | **12.7x** |
| subtract from datetime | 3.03 us | 0.18 us | **16.5x** |
| multiply by scalar | 1.49 us | 0.08 us | **18.7x** |
| diff between datetimes | 2.81 us | 0.33 us | **8.6x** |
| sequential add x12 | 19.41 us | 1.70 us | **11.4x** |

#### Parser - parse()

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| simple date | 8.78 us | 5.96 us | **1.5x** |
| datetime with tz | 17.99 us | 6.52 us | **2.8x** |
| fuzzy parsing | 29.42 us | 8.45 us | **3.5x** |
| 10 various formats | 165.30 us | 63.13 us | **2.6x** |

#### Parser - isoparse()

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| isoparse date | 0.81 us | 0.08 us | **10.7x** |
| isoparse datetime+tz | 3.11 us | 0.61 us | **5.1x** |
| isoparse with us | 2.67 us | 0.11 us | **23.5x** |

#### RRule

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| daily 100 | 105.63 us | 63.11 us | **1.7x** |
| weekly 52 | 105.33 us | 34.44 us | **3.1x** |
| monthly 120 | 448.46 us | 114.12 us | **3.9x** |
| yearly 100 | 2,144.58 us | 235.02 us | **9.1x** |
| rrulestr complex | 6.60 us | 0.90 us | **7.4x** |

#### Timezone

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| gettz various (x10) | 714.93 us | 7.59 us | **94.3x** |
| gettz offset | 20.20 us | 5.26 us | **3.8x** |
| resolve_imaginary | 6.54 us | 3.19 us | **2.0x** |
| datetime_exists | 3.15 us | 1.62 us | **1.9x** |
| convert chain | 6.74 us | 4.08 us | **1.7x** |

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
├── crates/
│   ├── dateutil-core/     # v1: Pure Rust optimized core
│   │   └── src/
│   │       ├── lib.rs     # Crate root, public API
│   │       ├── common.rs  # Weekday (MO-SU with N-th occurrence)
│   │       ├── easter.rs  # Easter date calculations
│   │       ├── error.rs   # Shared error types
│   │       ├── relativedelta.rs
│   │       ├── parser.rs  # parse() + zero-copy tokenizer
│   │       ├── parser/    # isoparser
│   │       ├── rrule.rs   # RRule + iterator
│   │       └── rrule/     # set, parse (rrulestr), iter
│   ├── dateutil-py/       # PyO3 bindings for v1 core
│   │   └── src/
│   │       ├── lib.rs     # Module registration
│   │       └── py/        # Per-module bindings
│   └── dateutil-rs/       # v0: python-dateutil compat + unified native module
│       └── src/
│           ├── lib.rs     # Crate root + #[pymodule] (v0 + v1)
│           ├── common.rs  # Weekday
│           ├── easter.rs  # Easter
│           ├── relativedelta.rs
│           ├── parser/    # Date/time string parsing + ISO-8601
│           ├── rrule/     # Recurrence rules (RFC 5545)
│           ├── tz/        # Timezone support (TZif, POSIX TZ, gettz cache)
│           └── utils.rs   # Utility functions
├── python/dateutil_rs/    # Python package (maturin mixed layout)
│   ├── __init__.py        # Re-exports from Rust native module
│   ├── _native.pyi        # Type stubs (v0)
│   ├── parser.py          # Rust parse/isoparse + fallback for custom parserinfo
│   ├── relativedelta.py   # Rust RelativeDelta
│   ├── easter.py          # Rust easter
│   ├── rrule.py           # Rust rrule/rruleset/rrulestr
│   ├── tz.py              # Rust timezone classes + gettz (cached)
│   ├── common.py          # Rust weekday constants
│   ├── utils.py           # Rust within_delta + python-dateutil fallback
│   └── v1/                # v1 optimized API
│       ├── _native.pyi    # Type stubs (v1)
│       ├── common.py      # Weekday
│       ├── easter.py      # Easter
│       ├── parser.py      # parse, isoparse
│       ├── relativedelta.py
│       └── rrule.py       # rrule, rruleset, rrulestr
├── tests/                 # Test suite (~13k lines)
├── benchmarks/            # pytest-benchmark comparisons
├── .github/workflows/     # CI (lint + test matrix)
├── pyproject.toml
├── Makefile
└── LICENSE
```

## Implementation Status

### v0 (python-dateutil compat)

| Module | Rust | Notes |
|--------|:----:|-------|
| easter | ✅ | 3.2x-6.2x faster |
| relativedelta | ✅ | 3.5x-18.7x faster |
| parser (parse) | ✅ | 1.3x-3.5x faster; custom parserinfo tables forwarded to Rust |
| parser (isoparse) | ✅ | 5.1x-23.5x faster |
| rrule | ✅ | 1.7x-9.1x faster |
| tz | ✅ | 1.0x-94.3x faster (gettz cached) |
| common (Weekday) | ✅ | |
| utils | ✅ | `today()`, `default_tzinfo()`, `within_delta()` all Rust-native |

### v1 (optimized core)

| Module | Rust Core | PyO3 Bindings | Notes |
|--------|:---------:|:-------------:|-------|
| common (Weekday) | ✅ | ✅ | |
| easter | ✅ | ✅ | |
| relativedelta | ✅ | ✅ | |
| parser | ✅ | ✅ | Zero-copy tokenizer, PHF lookups |
| rrule | ✅ | ✅ | Bitflag filters, buffer reuse |
| tz | ❌ | ❌ | Planned |

## Roadmap

1. **~~Python-only phase~~** — Pure Python port with full test coverage ✅
2. **~~Rust core + PyO3 bindings~~** — easter, relativedelta, parser, weekday, utils ✅
3. **~~Rust rrule~~** — Rewrite recurrence rules in Rust ✅
4. **~~Rust tz~~** — Rewrite timezone support in Rust (with gettz cache) ✅
5. **~~v1 optimized core~~** — common, easter, relativedelta, parser, rrule ✅
6. **v1 timezone** — Rewrite tz module for v1 core
7. **Release** — Publish dateutil-core to crates.io and python-dateutil-rs 1.0 to PyPI

## License

[MIT](LICENSE)
