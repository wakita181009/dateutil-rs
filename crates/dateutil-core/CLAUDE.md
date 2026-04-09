# dateutil-core Crate Guide

## Rust 1.94 Performance Optimization Notes

Rust 1.94.0 (released March 5, 2026) introduced several features and breaking changes relevant to this crate.

### New APIs for Performance

#### `array_windows` — Bounds-Check-Free Slice Iteration

```rust
// Old: windows() returns dynamic &[T] — runtime bounds checks
for w in data.windows(3) { ... }

// New: array_windows() returns fixed &[T; N] — bounds checks eliminated at compile time
for w in data.array_windows::<3>() { ... }
```

The compiler knows the window size at compile time, so it **eliminates bounds checks entirely**. Apply to token stream scanning in the parser and mask buffer operations in rrule.

#### `Peekable::next_if_map` — Single-Step Conditional Consume + Transform

```rust
// Old: peek + match + next (3 steps)
// New: one method call
let token = iter.next_if_map(|t| match t {
    Token::Number(n) => Some(*n),
    _ => None,
});
```

Useful in the parser for "consume next token if it matches, and transform it" patterns. Reduces branching and improves branch prediction.

#### `<[T]>::element_offset` — Safe Pointer Arithmetic

Returns the offset of an element within a slice without unsafe pointer math. Useful in the zero-copy parser for tracking positions within the original input buffer.

#### `f32/f64::mul_add` Now `const`-Stable

```rust
const RESULT: f64 = f64::mul_add(2.0, 3.0, 1.0); // compile-time FMA
```

Fused multiply-add in const contexts with reduced rounding errors.

#### `LazyCell`/`LazyLock` — `get`/`get_mut`/`force_mut`

```rust
// Check if already initialized without forcing initialization
if let Some(val) = MY_LAZY.get() {
    // fast path — no initialization cost
}
```

Enables fast-path access patterns for lazy caches (e.g., `gettz()` timezone cache).

#### `BinaryHeap` — Relaxed `T: Ord` Bound

Some methods no longer require `T: Ord`, making custom-comparison heaps easier to write.

### Platform-Level Improvements

#### `outline-atomics` Default on AArch64

Atomic operations now auto-detect and use LSE instructions on AArch64 (including Apple Silicon). Caches using `OnceLock<RwLock<HashMap>>` (e.g., `gettz()`) benefit automatically.

#### AVX-512 FP16 / AArch64 NEON FP16 Intrinsics Stabilized

SIMD half-precision float intrinsics are now stable. Not directly relevant to date processing but available for bulk numeric conversions if needed.

### Breaking Changes to Watch

#### Closure Capture Behavior Changed

Closures near `match`/`if let` now capture only the parts they need instead of entire variables. This can:
- Cause **new borrow checker errors** in code that previously compiled
- Change **`Drop` timing** for partially captured values

Run `cargo check` before and after upgrading to catch issues early. The parser and rrule modules are most likely to be affected.

#### Standard Library Macro Import Changed

Stdlib macros (e.g., `matches!`) are now imported via prelude instead of `#[macro_use]` injection. Custom macros with the same name will cause ambiguity errors.

#### `dyn Trait` Lifetime Casting Restricted

Casting between different lifetime bounds on trait objects (e.g., `dyn Foo + 'a` to `dyn Foo + 'b`) is now rejected.

#### `include!()` Shebang Stripping Removed

Files included via `include!()` in expression context no longer have shebang lines stripped. Remove shebangs from any included files.

#### Compiler Output Paths Changed

Workspace paths in diagnostics are now relative instead of absolute. CI/CD scripts that grep for absolute paths will break.

### Recommended Actions for This Crate

| Priority | Action | Target |
|----------|--------|--------|
| High | Run `cargo check` to detect breaking change impact | All modules |
| High | Fix closure capture borrow checker errors | parser, rrule |
| Medium | Replace `windows()` with `array_windows()` where window size is const | parser tokenizer |
| Medium | Adopt `next_if_map` for token consumption patterns | parser |
| Medium | Use `element_offset` to replace unsafe pointer arithmetic | parser (zero-copy) |
| Low | Use `LazyLock::get` for cache fast-path optimization | tz::gettz() |

### References

- [Announcing Rust 1.94.0 | Rust Blog](https://blog.rust-lang.org/2026/03/05/Rust-1.94.0/)
- [1.94.0 | Rust Changelogs](https://releases.rs/docs/1.94.0/)
- [Rust 1.94.0: 10 Breaking Changes - DEV Community](https://dev.to/matheus_releaserun/rust-1940-arraywindows-cargo-config-includes-and-10-breaking-changes-you-should-know-about-5gc7)
- [Draft release notes for 1.94.0 - GitHub](https://github.com/rust-lang/rust/issues/151650)
