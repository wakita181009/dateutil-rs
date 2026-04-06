# Benchmark Results

> Last updated: 2026-04-07 | Commit: `83c4711`

## Environment

- **Python:** 3.13.12
- **Platform:** macOS 26.3.1, Apple Silicon (arm64)
- **Tool:** pytest-benchmark 5.2.3
- **Build:** `maturin develop --release`

## Summary

Benchmarks compare three implementations side-by-side:

| Variant | Description |
|---------|-------------|
| **original** | `python-dateutil` from PyPI (v2.9.0.post0) |
| **local** | Pure Python port in `src/dateutil/` |
| **rust** | Rust implementation via PyO3 (`dateutil_rs`) |

### Implementation Status

| Module | Rust Status | Speedup vs original |
|--------|-------------|---------------------|
| easter | Implemented | **4.3x – 6.4x** |
| relativedelta | Implemented | **2.8x – 22.2x** |
| parser (parse) | Implemented | **1.3x – 3.5x** |
| parser (isoparse) | Implemented | **5.1x – 23.6x** |
| rrule | Not yet | — |
| tz | Not yet | — |

---

## Easter

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| single (Western) | 0.49 µs | 0.11 µs | **4.3x** |
| single (Orthodox) | 0.34 µs | 0.06 µs | **5.8x** |
| single (Julian) | 0.29 µs | 0.06 µs | **4.9x** |
| range 1000 years (Western) | 436.74 µs | 68.38 µs | **6.4x** |
| range 500 years × 3 methods | 568.50 µs | 107.30 µs | **5.3x** |

## RelativeDelta

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| create simple | 0.95 µs | 0.18 µs | **5.1x** |
| create complex | 0.96 µs | 0.34 µs | **2.8x** |
| create absolute | 1.10 µs | 0.20 µs | **5.5x** |
| create weekday | 0.90 µs | 0.14 µs | **6.7x** |
| add months | 1.64 µs | 0.18 µs | **9.1x** |
| add complex | 1.67 µs | 0.13 µs | **13.3x** |
| add weekday | 1.82 µs | 0.15 µs | **12.3x** |
| subtract | 2.96 µs | 0.13 µs | **22.2x** |
| multiply | 1.51 µs | 0.08 µs | **18.7x** |
| diff dates | 2.84 µs | 0.32 µs | **9.0x** |
| diff datetimes | 2.72 µs | 0.32 µs | **8.6x** |
| sequential add (×12) | 19.09 µs | 1.74 µs | **11.0x** |
| month-end overflow | 1.62 µs | 0.13 µs | **12.4x** |

## Parser — parse()

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| simple date | 8.47 µs | 5.27 µs | **1.6x** |
| datetime with time | 13.61 µs | 5.68 µs | **2.4x** |
| datetime with tz | 17.77 µs | 6.39 µs | **2.8x** |
| American format | 13.91 µs | 6.12 µs | **2.3x** |
| European format | 8.70 µs | 5.36 µs | **1.6x** |
| with microseconds | 15.51 µs | 5.75 µs | **2.7x** |
| fuzzy parsing | 28.55 µs | 8.27 µs | **3.5x** |
| relative with default | 6.55 µs | 4.94 µs | **1.3x** |
| 10 various formats | 168.25 µs | 61.31 µs | **2.7x** |

## Parser — isoparse()

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| isoparse date | 0.80 µs | 0.08 µs | **10.6x** |
| isoparse datetime | 2.02 µs | 0.14 µs | **14.1x** |
| isoparse datetime+tz | 3.04 µs | 0.59 µs | **5.1x** |
| isoparse datetime+UTC | 2.31 µs | 0.38 µs | **6.1x** |
| isoparse compact | 1.94 µs | 0.14 µs | **13.5x** |
| isoparse with µs | 2.62 µs | 0.11 µs | **23.6x** |
| isoparse various | 23.49 µs | 3.10 µs | **7.6x** |

## RRule

| Benchmark | original | local | Ratio |
|-----------|----------|-------|-------|
| daily 100 | 104.49 µs | 103.33 µs | 1.0x |
| daily interval=3 | 110.09 µs | 108.67 µs | 1.0x |
| weekly 52 | 105.06 µs | 104.60 µs | 1.0x |
| weekly byday interval=2 | 132.07 µs | 131.47 µs | 1.0x |
| monthly 120 | 464.56 µs | 463.70 µs | 1.0x |
| monthly byday | 18.44 µs | 17.79 µs | 1.0x |
| monthly bymonthday | 135.41 µs | 133.44 µs | 1.0x |
| yearly 100 | 2,136.88 µs | 2,135.22 µs | 1.0x |
| yearly bymonth | 672.09 µs | 672.66 µs | 1.0x |
| hourly 1000 | 1,270.19 µs | 1,255.39 µs | 1.0x |
| rruleset union | 131.34 µs | 128.92 µs | 1.0x |
| rruleset exdate | 137.43 µs | 136.18 µs | 1.0x |
| rruleset exrule | 636.90 µs | 609.90 µs | 1.0x |
| rrulestr simple | 2.99 µs | 2.99 µs | 1.0x |
| rrulestr complex | 6.60 µs | 6.38 µs | 1.0x |
| rrulestr with dtstart | 15.21 µs | 15.11 µs | 1.0x |

## Timezone

| Benchmark | original | local | Ratio |
|-----------|----------|-------|-------|
| tzutc create | 0.06 µs | 0.06 µs | 1.0x |
| tzoffset create | 0.42 µs | 0.42 µs | 1.0x |
| tzlocal create | 0.47 µs | 0.47 µs | 1.0x |
| gettz UTC | 0.30 µs | 0.31 µs | 1.0x |
| gettz named | 0.30 µs | 0.31 µs | 1.0x |
| gettz offset | 20.09 µs | 68.25 µs | 0.3x |
| convert UTC→JST | 1.38 µs | 1.38 µs | 1.0x |
| convert UTC→Eastern | 1.65 µs | 1.67 µs | 1.0x |
| convert chain | 6.95 µs | 7.09 µs | 1.0x |
| localize naive | 0.07 µs | 0.07 µs | 1.0x |
| datetime_exists | 3.23 µs | 3.26 µs | 1.0x |
| datetime_ambiguous | 1.03 µs | 1.02 µs | 1.0x |
| resolve_imaginary | 6.58 µs | 6.60 µs | 1.0x |
| gettz various (×10) | 729.69 µs | 731.53 µs | 1.0x |

---

## How to Reproduce

```bash
# Install dependencies
uv pip install python-dateutil

# Build Rust extension (release mode)
maturin develop --release

# Run benchmarks
make bench

# Save results as JSON
make bench-save
```
