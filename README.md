# python-dateutil-rs

[![PyPI](https://img.shields.io/pypi/v/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![Python](https://img.shields.io/pypi/pyversions/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![License](https://img.shields.io/pypi/l/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![CI](https://github.com/wakita181009/dateutil-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/wakita181009/dateutil-rs/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/wakita181009/dateutil-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/wakita181009/dateutil-rs)

A high-performance Rust-backed port of [python-dateutil](https://github.com/dateutil/dateutil) (v2.9.0).

> **Status:** Hybrid phase — easter, relativedelta, parser, and isoparser are rewritten in Rust via PyO3/maturin, delivering **1.3x–23.6x** speedups. rrule and tz still delegate to python-dateutil.

## Features

- **Drop-in replacement** for `python-dateutil` — same API, same behavior
- **Rust-accelerated:** easter, relativedelta, parser (`parse` / `isoparse`), weekday
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
| single call (Western) | 0.49 µs | 0.11 µs | **4.3x** |
| single call (Orthodox) | 0.34 µs | 0.06 µs | **5.8x** |
| single call (Julian) | 0.29 µs | 0.06 µs | **4.9x** |
| 1000 years (Western) | 436.74 µs | 68.38 µs | **6.4x** |
| 500 years × 3 methods | 568.50 µs | 107.30 µs | **5.3x** |

#### RelativeDelta

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| create simple | 0.95 µs | 0.18 µs | **5.1x** |
| add months to datetime | 1.64 µs | 0.18 µs | **9.1x** |
| subtract from datetime | 2.96 µs | 0.13 µs | **22.2x** |
| multiply by scalar | 1.51 µs | 0.08 µs | **18.7x** |
| diff between datetimes | 2.84 µs | 0.32 µs | **9.0x** |
| sequential add ×12 | 19.09 µs | 1.74 µs | **11.0x** |

#### Parser — parse()

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| simple date | 8.47 µs | 5.27 µs | **1.6x** |
| datetime with tz | 17.77 µs | 6.39 µs | **2.8x** |
| fuzzy parsing | 28.55 µs | 8.27 µs | **3.5x** |
| 10 various formats | 168.25 µs | 61.31 µs | **2.7x** |

#### Parser — isoparse()

| Benchmark | python-dateutil | dateutil-rs (Rust) | Speedup |
|-----------|----------------:|-------------------:|--------:|
| isoparse date | 0.80 µs | 0.08 µs | **10.6x** |
| isoparse datetime+tz | 3.04 µs | 0.59 µs | **5.1x** |
| isoparse with µs | 2.62 µs | 0.11 µs | **23.6x** |

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
│       └── utils.rs       # Utility functions
├── python/dateutil_rs/    # Python package (maturin mixed layout)
│   ├── __init__.py        # Re-exports from Rust native module
│   ├── parser.py          # Rust parse/isoparse + fallback for custom parserinfo
│   ├── relativedelta.py   # Rust RelativeDelta
│   ├── easter.py          # Rust easter
│   ├── rrule.py           # Delegates to python-dateutil (not yet Rust)
│   ├── tz.py              # Delegates to python-dateutil (not yet Rust)
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
| easter | ✅ | 4.3x–6.4x faster |
| relativedelta | ✅ | 2.8x–22.2x faster |
| parser (parse) | ✅ | 1.3x–3.5x faster; falls back to python-dateutil for custom parserinfo |
| parser (isoparse) | ✅ | 5.1x–23.6x faster |
| common (Weekday) | ✅ | |
| utils (within_delta) | ✅ | `today()` / `default_tzinfo()` still delegate to python-dateutil |
| rrule | ❌ | Delegates to python-dateutil |
| tz | ❌ | Delegates to python-dateutil |

## Roadmap

1. **~~Python-only phase~~** — Pure Python port with full test coverage ✅
2. **~~Rust core + PyO3 bindings~~** — easter, relativedelta, parser, weekday, utils ✅
3. **Rust rrule** — Rewrite recurrence rules in Rust
4. **Rust tz** — Rewrite timezone support in Rust
5. **Release** — Publish to crates.io and PyPI with pre-built wheels (manylinux, macOS, Windows)

## License

[MIT](LICENSE)
