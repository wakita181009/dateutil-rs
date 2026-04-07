# Benchmark Results

> Last updated: 2026-04-07 | Commit: `d41836e`

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
| easter | Implemented | **4.0x – 7.2x** |
| relativedelta | Implemented | **3.4x – 18.6x** |
| parser (parse) | Implemented | **1.3x – 3.5x** |
| parser (isoparse) | Implemented | **5.1x – 21.0x** |
| rrule | Not yet | — |
| tz | Implemented | **1.0x – 3.7x** (varies) |

---

## Easter

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| single (Western) | 0.50 µs | 0.13 µs | **4.0x** |
| single (Orthodox) | 0.42 µs | 0.06 µs | **7.2x** |
| single (Julian) | 0.28 µs | 0.06 µs | **4.9x** |
| range 1000 years (Western) | 426.17 µs | 66.04 µs | **6.5x** |
| range 500 years × 3 methods | 557.75 µs | 105.25 µs | **5.3x** |

## RelativeDelta

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| create simple | 0.92 µs | 0.17 µs | **5.5x** |
| create complex | 0.89 µs | 0.26 µs | **3.4x** |
| create absolute | 1.04 µs | 0.19 µs | **5.4x** |
| create weekday | 0.92 µs | 0.13 µs | **7.3x** |
| add months | 1.54 µs | 0.12 µs | **12.6x** |
| add complex | 1.58 µs | 0.12 µs | **13.1x** |
| add weekday | 1.71 µs | 0.14 µs | **12.0x** |
| subtract | 2.83 µs | 0.17 µs | **17.0x** |
| multiply | 1.42 µs | 0.08 µs | **18.6x** |
| diff dates | 2.67 µs | 0.25 µs | **10.7x** |
| diff datetimes | 2.58 µs | 0.29 µs | **8.8x** |
| sequential add (×12) | 18.12 µs | 1.63 µs | **11.2x** |
| month-end overflow | 1.54 µs | 0.17 µs | **9.2x** |

## Parser — parse()

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| simple date | 8.25 µs | 5.04 µs | **1.6x** |
| datetime with time | 13.17 µs | 5.46 µs | **2.4x** |
| datetime with tz | 17.08 µs | 6.12 µs | **2.8x** |
| American format | 13.50 µs | 5.87 µs | **2.3x** |
| European format | 8.42 µs | 5.08 µs | **1.7x** |
| with microseconds | 14.87 µs | 5.50 µs | **2.7x** |
| fuzzy parsing | 28.13 µs | 7.96 µs | **3.5x** |
| relative with default | 6.21 µs | 4.75 µs | **1.3x** |
| 10 various formats | 163.71 µs | 59.00 µs | **2.8x** |

## Parser — isoparse()

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| isoparse date | 0.79 µs | 0.07 µs | **10.8x** |
| isoparse datetime | 1.96 µs | 0.09 µs | **21.0x** |
| isoparse datetime+tz | 2.96 µs | 0.58 µs | **5.1x** |
| isoparse datetime+UTC | 2.21 µs | 0.38 µs | **5.9x** |
| isoparse compact | 1.92 µs | 0.13 µs | **15.3x** |
| isoparse with µs | 2.54 µs | 0.17 µs | **15.3x** |
| isoparse various | 22.37 µs | 2.96 µs | **7.6x** |

## RRule

| Benchmark | original | local | Ratio |
|-----------|----------|-------|-------|
| daily 100 | 96.04 µs | 94.83 µs | 1.0x |
| daily interval=3 | 101.79 µs | 101.25 µs | 1.0x |
| weekly 52 | 95.92 µs | 94.96 µs | 1.0x |
| weekly byday interval=2 | 123.58 µs | 123.29 µs | 1.0x |
| monthly 120 | 408.67 µs | 408.33 µs | 1.0x |
| monthly byday | 16.38 µs | 16.33 µs | 1.0x |
| monthly bymonthday | 119.58 µs | 118.87 µs | 1.0x |
| yearly 100 | 1,965.87 µs | 1,976.29 µs | 1.0x |
| yearly bymonth | 624.38 µs | 622.71 µs | 1.0x |
| hourly 1000 | 1,167.04 µs | 1,163.38 µs | 1.0x |
| rruleset union | 119.54 µs | 118.71 µs | 1.0x |
| rruleset exdate | 125.71 µs | 126.25 µs | 1.0x |
| rruleset exrule | 578.63 µs | 573.29 µs | 1.0x |
| rrulestr simple | 2.71 µs | 2.71 µs | 1.0x |
| rrulestr complex | 5.83 µs | 5.79 µs | 1.0x |
| rrulestr with dtstart | 14.54 µs | 13.92 µs | 1.0x |

## Timezone

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| tzutc create | 0.05 µs | 0.06 µs | 0.9x |
| tzoffset create | 0.42 µs | 0.54 µs | 0.8x |
| tzlocal create | 0.46 µs | 0.21 µs | **2.2x** |
| gettz UTC | 0.29 µs | 20.13 µs | 0.01x (*) |
| gettz named | 0.29 µs | 22.04 µs | 0.01x (*) |
| gettz offset | 18.50 µs | 5.04 µs | **3.7x** |
| convert UTC→JST | 1.33 µs | 1.33 µs | 1.0x |
| convert UTC→Eastern | 1.58 µs | 1.42 µs | **1.1x** |
| convert chain | 6.87 µs | 5.00 µs | **1.4x** |
| localize naive | 0.07 µs | 0.07 µs | 1.0x |
| datetime_exists | 3.12 µs | 1.92 µs | **1.6x** |
| datetime_ambiguous | 1.00 µs | 0.63 µs | **1.6x** |
| resolve_imaginary | 6.46 µs | 2.00 µs | **3.2x** |
| gettz various (×10) | 709.50 µs | 220.12 µs | **3.2x** |

(*) python-dateutil caches `gettz()` results via `_TzFactory`. Single repeated lookups appear faster in the original because the factory returns a cached singleton. The Rust implementation does not cache yet; each call performs a fresh filesystem lookup.

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
