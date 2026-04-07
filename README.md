# python-dateutil-rs

[![PyPI](https://img.shields.io/pypi/v/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![Python](https://img.shields.io/pypi/pyversions/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![License](https://img.shields.io/pypi/l/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![CI](https://github.com/wakita181009/dateutil-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/wakita181009/dateutil-rs/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/wakita181009/dateutil-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/wakita181009/dateutil-rs)

A high-performance Rust-backed port of [python-dateutil](https://github.com/dateutil/dateutil) (v2.9.0).

> **Status:** All core modules (easter, relativedelta, parser, rrule, tz) are rewritten in Rust via PyO3/maturin, delivering **1.3x–94x** speedups over python-dateutil.

## Features

- **Drop-in replacement** for `python-dateutil` — same API, same behavior
- **Rust-accelerated:** easter, relativedelta, parser (`parse` / `isoparse`), rrule, tz, weekday
- **Full module coverage:** parser, relativedelta, rrule, tz, easter, utils
- **Comprehensive test suite** inherited from the original project
- **Benchmark infrastructure** for side-by-side performance comparison (original vs Rust)
- **Python 3.10–3.14** supported on Linux and macOS

## Installation

```bash
pip install python-dateutil-rs
```

## Usage

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
```

### Linting

```bash
uv run ruff check tests/ python/
uv run ruff format --check tests/ python/
```

### Benchmarks

Benchmarks compare the original `python-dateutil` (PyPI) and the Rust extension (`dateutil_rs`) using pytest-benchmark.

#### Easter

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| single call (Western) | 0.51 µs | 0.13 µs | **3.9x** |
| single call (Orthodox) | 0.35 µs | 0.11 µs | **3.2x** |
| single call (Julian) | 0.29 µs | 0.06 µs | **4.9x** |
| 1000 years (Western) | 437.88 µs | 70.46 µs | **6.2x** |
| 500 years × 3 methods | 567.99 µs | 110.33 µs | **5.2x** |

#### RelativeDelta

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| create simple | 0.94 µs | 0.19 µs | **4.9x** |
| add months to datetime | 1.62 µs | 0.13 µs | **12.7x** |
| subtract from datetime | 3.03 µs | 0.18 µs | **16.5x** |
| multiply by scalar | 1.49 µs | 0.08 µs | **18.7x** |
| diff between datetimes | 2.81 µs | 0.33 µs | **8.6x** |
| sequential add ×12 | 19.41 µs | 1.70 µs | **11.4x** |

#### Parser — parse()

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| simple date | 8.78 µs | 5.96 µs | **1.5x** |
| datetime with tz | 17.99 µs | 6.52 µs | **2.8x** |
| fuzzy parsing | 29.42 µs | 8.45 µs | **3.5x** |
| 10 various formats | 165.30 µs | 63.13 µs | **2.6x** |

#### Parser — isoparse()

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| isoparse date | 0.81 µs | 0.08 µs | **10.7x** |
| isoparse datetime+tz | 3.11 µs | 0.61 µs | **5.1x** |
| isoparse with µs | 2.67 µs | 0.11 µs | **23.5x** |

#### RRule

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| daily 100 | 105.63 µs | 63.11 µs | **1.7x** |
| weekly 52 | 105.33 µs | 34.44 µs | **3.1x** |
| monthly 120 | 448.46 µs | 114.12 µs | **3.9x** |
| yearly 100 | 2,144.58 µs | 235.02 µs | **9.1x** |
| rrulestr complex | 6.60 µs | 0.90 µs | **7.4x** |

#### Timezone

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| gettz various (×10) | 714.93 µs | 7.59 µs | **94.3x** |
| gettz offset | 20.20 µs | 5.26 µs | **3.8x** |
| resolve_imaginary | 6.54 µs | 3.19 µs | **2.0x** |
| datetime_exists | 3.15 µs | 1.62 µs | **1.9x** |
| convert chain | 6.74 µs | 4.08 µs | **1.7x** |

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
├── crates/dateutil-rs/    # Rust implementation (PyO3 extension)
│   └── src/
│       ├── lib.rs         # Crate root + #[pymodule] definition
│       ├── common.rs      # Weekday (MO–SU with N-th occurrence)
│       ├── easter.rs      # Easter date calculations
│       ├── relativedelta.rs # Relative date arithmetic
│       ├── parser/        # Date/time string parsing + ISO-8601
│       ├── rrule/         # Recurrence rules (RFC 5545)
│       ├── tz/            # Timezone support (TZif, POSIX TZ, gettz cache)
│       └── utils.rs       # Utility functions
├── python/dateutil_rs/    # Python package (maturin mixed layout)
│   ├── __init__.py        # Re-exports from Rust native module
│   ├── parser.py          # Rust parse/isoparse + fallback for custom parserinfo
│   ├── relativedelta.py   # Rust RelativeDelta
│   ├── easter.py          # Rust easter
│   ├── rrule.py           # Rust rrule/rruleset/rrulestr
│   ├── tz.py              # Rust timezone classes + gettz (cached)
│   ├── common.py          # Rust weekday constants
│   └── utils.py           # Rust within_delta + python-dateutil fallback
├── tests/                 # Test suite (~13k lines)
├── benchmarks/            # pytest-benchmark comparisons
├── .github/workflows/     # CI (lint + test matrix)
├── pyproject.toml
├── Makefile
└── LICENSE
```

## Implementation Status

| Module | Rust | Notes |
|--------|:----:|-------|
| easter | ✅ | 3.2x–6.2x faster |
| relativedelta | ✅ | 3.5x–18.7x faster |
| parser (parse) | ✅ | 1.3x–3.5x faster; custom parserinfo tables forwarded to Rust |
| parser (isoparse) | ✅ | 5.1x–23.5x faster |
| rrule | ✅ | 1.7x–9.1x faster |
| tz | ✅ | 1.0x–94.3x faster (gettz cached) |
| common (Weekday) | ✅ | |
| utils | ✅ | `today()`, `default_tzinfo()`, `within_delta()` all Rust-native |

## Roadmap

1. **~~Python-only phase~~** — Pure Python port with full test coverage ✅
2. **~~Rust core + PyO3 bindings~~** — easter, relativedelta, parser, weekday, utils ✅
3. **~~Rust rrule~~** — Rewrite recurrence rules in Rust ✅
4. **~~Rust tz~~** — Rewrite timezone support in Rust (with gettz cache) ✅
5. **Release** — Publish to crates.io and PyPI with pre-built wheels (manylinux, macOS, Windows)

## License

[MIT](LICENSE)
