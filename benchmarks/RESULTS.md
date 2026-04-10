# Benchmark Results

> Last updated: 2026-04-11 | Commit: `4e858f2`

## Environment

- **Python:** 3.13.12
- **Platform:** macOS 26.3.0, Apple Silicon (arm64)
- **Tool:** pytest-benchmark 5.2.3
- **Build:** `maturin develop --release`

## Summary

Benchmarks compare three implementations side-by-side:

| Variant | Description |
|---------|-------------|
| **python-dateutil** | `python-dateutil` from PyPI (v2.9.0.post0) |
| **v0** | Rust port of python-dateutil via PyO3 (`dateutil_rs`) |
| **v1** | Rust-optimized core (`dateutil_rs.v1`) — zero-copy, phf, buffer reuse |

### v0 Speedup vs python-dateutil

| Module | Speedup Range |
|--------|---------------|
| easter | **4.9x – 7.3x** |
| relativedelta | **4.0x – 25.2x** |
| parser (parse) | **1.3x – 3.5x** |
| parser (isoparse) | **5.1x – 23.0x** |
| rrule | **1.7x – 20.6x** |
| tz | **0.4x – 101.2x** ¹ |

### v1 Speedup vs python-dateutil

| Module | Speedup Range |
|--------|---------------|
| easter | **5.0x – 7.3x** |
| relativedelta | **2.0x – 28.1x** ² |
| parser (parse) | **19.5x – 36.0x** |
| parser (isoparse) | **13.0x – 38.4x** |
| rrule | **5.9x – 63.7x** |
| tz | **1.0x – 896.7x** ¹³ |

¹ gettz results are cached in a process-global `RwLock<HashMap>`, matching python-dateutil's `_TzFactory` singleton cache. Single lookups show overhead from lock acquisition; batch lookups benefit massively from avoiding repeated filesystem reads.

² v1 relativedelta creation is slower than v0 due to richer Python-side kwarg handling; arithmetic operations are significantly faster.

³ v1 `tzlocal()` creates a fresh `TzFile` from `/etc/localtime` on every call (no caching), resulting in ~86 µs overhead. Other tz operations show significant gains.

---

## Easter

| Benchmark | python-dateutil | v0 | v1 | v0 × | v1 × |
|-----------|---------------:|---:|---:|-----:|-----:|
| single (Western) | 0.46 µs | 0.06 µs | 0.06 µs | **7.3x** | **7.3x** |
| single (Orthodox) | 0.38 µs | 0.06 µs | 0.06 µs | **6.6x** | **6.8x** |
| single (Julian) | 0.28 µs | 0.06 µs | 0.06 µs | **4.9x** | **5.0x** |
| range 1000 years (Western) | 422.75 µs | 66.50 µs | 65.00 µs | **6.4x** | **6.5x** |
| range 500 years × 3 methods | 551.08 µs | 105.29 µs | 99.96 µs | **5.2x** | **5.5x** |

## RelativeDelta

| Benchmark | python-dateutil | v0 | v1 | v0 × | v1 × |
|-----------|---------------:|---:|---:|-----:|-----:|
| create simple | 0.92 µs | 0.17 µs | 0.19 µs | **5.5x** | **4.9x** |
| create complex | 0.92 µs | 0.23 µs | 0.45 µs | **4.0x** | **2.0x** |
| create absolute | 1.00 µs | 0.18 µs | 0.35 µs | **5.6x** | **2.9x** |
| create weekday | 0.86 µs | 0.11 µs | 0.19 µs | **7.5x** | **4.4x** |
| add months | 1.54 µs | 0.17 µs | 0.10 µs | **9.3x** | **15.4x** |
| add complex | 1.58 µs | 0.11 µs | 0.10 µs | **14.9x** | **16.0x** |
| add weekday | 1.71 µs | 0.13 µs | 0.11 µs | **13.7x** | **15.7x** |
| subtract | 2.88 µs | 0.11 µs | 0.10 µs | **25.2x** | **28.1x** |
| multiply | 1.42 µs | 0.07 µs | 0.07 µs | **20.9x** | **21.5x** |
| diff dates | 2.63 µs | 0.23 µs | 0.17 µs | **11.6x** | **15.5x** |
| diff datetimes | 2.58 µs | 0.22 µs | 0.17 µs | **11.7x** | **15.5x** |
| sequential add (×12) | 17.96 µs | 1.46 µs | 1.33 µs | **12.3x** | **13.5x** |
| month-end overflow | 1.54 µs | 0.11 µs | 0.10 µs | **14.0x** | **15.1x** |

## Parser — parse()

| Benchmark | python-dateutil | v0 | v1 | v0 × | v1 × |
|-----------|---------------:|---:|---:|-----:|-----:|
| simple date | 8.17 µs | 5.38 µs | 0.42 µs | **1.5x** | **19.5x** |
| datetime with time | 13.21 µs | 5.67 µs | 0.51 µs | **2.3x** | **25.9x** |
| datetime with tz | 17.00 µs | 6.33 µs | 0.63 µs | **2.7x** | **27.2x** |
| American format | 13.58 µs | 6.04 µs | 0.51 µs | **2.2x** | **26.5x** |
| European format | 8.38 µs | 5.29 µs | 0.42 µs | **1.6x** | **19.8x** |
| with microseconds | 15.00 µs | 5.71 µs | 0.49 µs | **2.6x** | **30.5x** |
| fuzzy parsing | 28.04 µs | 8.08 µs | — | **3.5x** | — |
| relative with default | 6.21 µs | 4.96 µs | — | **1.3x** | — |
| 10 various formats | 163.63 µs | 60.46 µs | 4.54 µs | **2.7x** | **36.0x** |

> v1 does not support fuzzy mode or parser-level default/relative resolution (excluded by design).

## Parser — isoparse()

| Benchmark | python-dateutil | v0 | v1 | v0 × | v1 × |
|-----------|---------------:|---:|---:|-----:|-----:|
| isoparse date | 0.79 µs | 0.07 µs | 0.06 µs | **10.8x** | **13.0x** |
| isoparse datetime | 1.96 µs | 0.10 µs | 0.08 µs | **20.6x** | **25.4x** |
| isoparse datetime+tz | 2.96 µs | 0.58 µs | 0.08 µs | **5.1x** | **38.4x** |
| isoparse datetime+UTC | 2.21 µs | 0.38 µs | 0.07 µs | **5.9x** | **30.0x** |
| isoparse compact | 1.88 µs | 0.09 µs | 0.08 µs | **19.9x** | **23.4x** |
| isoparse with µs | 2.50 µs | 0.11 µs | 0.09 µs | **23.0x** | **27.5x** |
| isoparse various | 22.38 µs | 2.92 µs | 0.88 µs | **7.7x** | **25.5x** |

## RRule

| Benchmark | python-dateutil | v0 | v1 | v0 × | v1 × |
|-----------|---------------:|---:|---:|-----:|-----:|
| daily 100 | 100.13 µs | 60.50 µs | 4.96 µs | **1.7x** | **20.2x** |
| daily interval=3 | 106.46 µs | 56.04 µs | 5.08 µs | **1.9x** | **21.0x** |
| weekly 52 | 100.79 µs | 31.08 µs | 3.42 µs | **3.2x** | **29.5x** |
| weekly byday interval=2 | 127.88 µs | 44.96 µs | 5.17 µs | **2.8x** | **24.7x** |
| monthly 120 | 427.75 µs | 104.00 µs | 13.00 µs | **4.1x** | **32.9x** |
| monthly byday | 17.17 µs | 10.17 µs | 1.71 µs | **1.7x** | **10.0x** |
| monthly bymonthday | 124.54 µs | 37.00 µs | 4.83 µs | **3.4x** | **25.8x** |
| yearly 100 | 2,062.90 µs | 189.50 µs | 32.40 µs | **10.9x** | **63.7x** |
| yearly bymonth | 657.33 µs | 66.33 µs | 11.13 µs | **9.9x** | **59.1x** |
| hourly 1000 | 1,235.50 µs | 601.00 µs | 45.20 µs | **2.1x** | **27.3x** |
| rruleset union | 125.58 µs | 64.79 µs | 8.04 µs | **1.9x** | **15.6x** |
| rruleset exdate | 133.33 µs | 65.00 µs | 8.71 µs | **2.1x** | **15.3x** |
| rruleset exrule | 598.96 µs | 205.54 µs | 21.13 µs | **2.9x** | **28.4x** |
| rrulestr simple | 2.96 µs | 0.50 µs | 0.50 µs | **5.9x** | **5.9x** |
| rrulestr complex | 6.25 µs | 0.83 µs | 0.92 µs | **7.5x** | **6.8x** |
| rrulestr with dtstart | 14.63 µs | 0.71 µs | 0.58 µs | **20.6x** | **25.1x** |

## Timezone

| Benchmark | python-dateutil | v0 | v1 | v0 × | v1 × |
|-----------|---------------:|---:|---:|-----:|-----:|
| tzutc create | 0.05 µs | 0.06 µs | 0.05 µs | 0.9x | **1.1x** |
| tzoffset create | 0.42 µs | 0.50 µs | 0.09 µs | 0.8x | **4.6x** |
| tzlocal create | 0.46 µs | 0.15 µs | 85.87 µs | **3.1x** | 0.005x ³ |
| gettz UTC | 0.29 µs | 0.42 µs | 0.07 µs | 0.7x | **4.3x** |
| gettz named | 0.29 µs | 0.79 µs | 0.13 µs | 0.4x | **2.3x** |
| gettz offset | 19.13 µs | 5.17 µs | — | **3.7x** | — |
| convert UTC→JST | 1.33 µs | 0.96 µs | 0.24 µs | **1.4x** | **5.7x** |
| convert UTC→Eastern | 1.58 µs | 1.00 µs | 0.26 µs | **1.6x** | **6.1x** |
| convert chain | 6.79 µs | 3.88 µs | 0.83 µs | **1.8x** | **8.2x** |
| localize naive | 0.07 µs | 0.07 µs | 0.07 µs | 1.0x | 1.0x |
| datetime_exists | 3.08 µs | 1.54 µs | 1.38 µs | **2.0x** | **2.2x** |
| datetime_ambiguous | 1.00 µs | 0.58 µs | 1.33 µs | **1.7x** | 0.8x |
| resolve_imaginary | 6.38 µs | 3.00 µs | 1.42 µs | **2.1x** | **4.5x** |
| gettz various (×10) | 708.38 µs | 7.00 µs | 0.79 µs | **101.2x** | **896.7x** |

³ v1 `tzlocal()` reads `/etc/localtime` on every call without caching. v0 and python-dateutil use cached system timezone info.

---

## Key Observations

### v1 Parser — 20x–36x faster
The v1 zero-copy tokenizer and `phf` compile-time hash maps eliminate virtually all allocation overhead. Where v0 achieves 1.5x–3.5x over python-dateutil, v1 reaches **19.5x–36.0x** for `parse()` and **13.0x–38.4x** for `isoparse()`.

### v1 RRule — 6x–64x faster
Buffer reuse and `Range<usize>`-based dayset make v1 rrule dramatically faster. The yearly-100 benchmark shows the largest gain: **63.7x** vs python-dateutil (v0: 10.9x). Even simple patterns like daily-100 see **20.2x** improvement.

### v1 Timezone — mixed results
Cached lookups via `gettz()` are extremely fast (up to **896.7x** for batch lookups). However, `tzlocal()` has a known regression in v1 due to uncached `/etc/localtime` reads. `datetime_ambiguous` is also slightly slower in v1 than python-dateutil.

### v0 RelativeDelta — creation faster than v1
v0 relativedelta creation (0.11–0.23 µs) outperforms v1 (0.19–0.45 µs) due to v1's Python-side `SimpleNamespace` wrapper overhead. For arithmetic operations, v1 is consistently faster.

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
