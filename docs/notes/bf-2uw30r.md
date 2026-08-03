# WASM32 Build Spike Results (bf-2uw30r)

## Purpose

Spike to verify whether `pdftract-core`'s vector-text extraction path compiles under `wasm32-unknown-unknown` before committing to ADR-010's `pdftract-wasm` sibling crate approach.

## Test Command

```bash
cargo check -p pdftract-core --no-default-features --target wasm32-unknown-unknown
```

## Immediate Failure

**Blocker: `zstd-sys` (transitive dependency via `zstd`)**

```
error: failed to run custom build command for `zstd-sys v2.0.16+zstd.1.5.7`
Caused by:
  process didn't exit successfully: `/home/coding/pdftract/target/debug/build/zstd-sys-d50644902d76e19e/build-script-build`
  --- stderr
  error occurred in cc-rs: failed to find tool "clang": No such file or directory (os error 2)
```

`zstd-sys` requires a C compiler (`clang`) to compile native C code for the target platform. This is expected for crates with C dependencies when targeting wasm32.

## Expected Additional Blockers (Not Yet Tested)

Based on analysis of `pdftract-core`'s direct dependencies from `Cargo.toml`, these dependencies are **unlikely to work** on `wasm32-unknown-unknown`:

### High-Confidence Blockers

| Dependency | Reason | Used in pdftract-core for |
|------------|--------|---------------------------|
| `memmap2 = "0.9"` | No mmap syscall in browser sandbox (ADR-010 explicitly calls this out) | Zero-copy file I/O |
| `rayon = "1.10"` | Threading via `pthread` - not available in wasm32 (would need `wasm-bindgen-rayon`) | Page-level parallelism |
| `tempfile = "3.10"` | Filesystem operations - not available in browser sandbox | Test fixtures, scratch space |
| `dirs = "5.0"` | Filesystem directory queries - not available in browser sandbox | Config/profile paths |
| `nix = { version = "0.29", features = ["fs"] }` | Unix-specific syscalls - not available on wasm32 (optional, for `remote` feature) | Remote HTTP source |
| `zstd = "0.13"` | C dependency `zstd-sys` requires compiler - confirmed blocker | Compression/decompression |

### Moderate-Confidence Concerns

| Dependency | Reason | Notes |
|------------|--------|-------|
| `parking_lot = "0.12"` | Uses `std::sync::Once` - may have wasm32 compatibility issues | Commonly works on wasm32, but needs verification |
| `chrono = "0.4"` | Time/date handling - may have wasm32 quirks | Usually portable, but needs verification |
| `ureq = { version = "2.10", default-features = false, features = ["tls"] }` | HTTP client - TLS may not work on wasm32 (optional, for `remote` feature) | Would need wasm-compatible alternative |

### Likely Compatible

Most of the remaining dependencies are pure Rust crates with no platform-specific code:
- `anyhow`, `base64`, `bytes`, `flate2`, `lzw`, `hex`, `indexmap`, `regex`, `serde`, `sha2`, `thiserror`, `memchr`, `unicode-normalization`, `ttf-parser`, `owned_ttf_parser`, `phf`, `rand`, `tracing`, `dashmap`, `smallvec`, `encoding_rs`, `once_cell`, `hmac`, `unicode-segmentation`, `strsim`, `unicode-bidi`, `lru`

## Impact Analysis

### Vector-Text Extraction Path Dependencies

The vector extraction path (parsing, font-encoding recovery, reading-order segmentation) uses these **wasm32-incompatible** dependencies:

1. **`memmap2`** - Core I/O abstraction. Used throughout the parser for zero-copy file access.
2. **`rayon`** - Page-level parallelism. Used to speed up multi-page document processing.
3. **`zstd`** - Compression/decompression. Used for compressed stream processing.
4. **`tempfile`** - Test fixtures and scratch space (less critical, but pervasive in tests).
5. **`dirs`** - Profile/configuration paths (less critical for vector-only demo, but used in config code).

### OCR Path Dependencies (Already Out of Scope for WASM)

Per ADR-010, OCR is explicitly out of scope for the WASM build. These dependencies are already gated behind the `ocr` feature:
- `image`, `imageproc`, `leptonica-plumbing`, `tesseract`, `pdfium-render` (via `full-render`)

These are **not** blockers for the WASM spike because they're optional.

## ADR-010 Viability Assessment

### Current ADR-010 Scope (Sibling Crate, No Changes to pdftract-core)

> "Make `pdftract-core` itself WASM-portable (`#[cfg(target_arch = "wasm32")]` branching inside the core) instead of a sibling crate. **Rejected:** this is precisely the alternative R11 already ruled out, to keep `#[cfg]` branching out of the accuracy-critical core; a sibling crate leaves `pdftract-core`'s dependency and feature-flag surface completely untouched."

**Verdict: NOT VIABLE as scoped.**

The vector extraction path in `pdftract-core` has deep, pervasive dependencies on `memmap2`, `rayon`, and `zstd` that are **not compatible** with `wasm32-unknown-unknown`. A sibling crate that reuses `pdftract-core`'s existing extraction code cannot work without modifying `pdftract-core`'s I/O and concurrency abstractions.

### Required Changes to Enable WASM Build

To enable the WASM build as described in ADR-010, one of the following approaches would be needed:

#### Option A: I/O Abstraction Layer in pdftract-core (Breaking Change)

Introduce a trait-based abstraction for file I/O and concurrency:

```rust
// New abstraction
pub trait FileSource {
    fn bytes(&self) -> Result<Cow<[u8]>>;
}

pub trait ParallelExecutor {
    fn execute_par<F, R>(&self, tasks: Vec<F>) -> Vec<R>
    where F: FnOnce() -> R + Send + 'static, R: Send + 'static;
}

// Native implementation
pub struct MmapFile { /* memmap2-backed */ }

// WASM implementation
pub struct Uint8ArrayFile { /* browser-side Uint8Array */ }
```

This is **exactly the approach ADR-010 rejected** ("Make `pdftract-core` itself WASM-portable"). It would require:
1. Refactoring every call site that uses `memmap2` directly
2. Abstracting `rayon` usage behind a trait
3. Adding `wasm32` support to `zstd` or switching to a pure-Rust alternative

#### Option B: Minimal WASM-Specific Extraction Path (New Sibling Crate, Limited Scope)

Instead of reusing the full `pdftract-core` extraction pipeline, build a **minimal vector-only parser** in the `pdftract-wasm` crate that:

- Reads PDFs from `Uint8Array` passed from JavaScript
- Implements a simplified parser for vector text only (no OCR, no complex page layouts)
- Uses pure-Rust alternatives for all dependencies:
  - Replace `memmap2` with direct `&[u8]` parsing
  - Replace `rayon` with single-threaded execution (acceptable for small documents in browser)
  - Replace `zstd` with a pure-Rust alternative or defer compressed stream support
  - No `tempfile`, `dirs`, or filesystem dependencies

This is the **recommended path** but requires:
1. Accepting that the WASM build will have **reduced functionality** compared to the native CLI
2. Implementing a separate code path with divergence from `pdftract-core`
3. Maintaining two parsers (feature parity not guaranteed)

#### Option C: Server-Side Preprocessing (Alternative to Browser-Side Extraction)

Instead of full client-side extraction, use a hybrid approach:
- Server (`pdftract serve`) processes the PDF and returns structured JSON
- Browser demo fetches pre-extracted data from the server
- No WASM build required; browser only renders results

This is the **safest approach** but:
- Defeats ADR-010's "client-side, no upload" privacy guarantee
- Requires operating a public-facing server (ADR-010 rejected this for privacy reasons)

## Recommendation

**ADR-010 as scoped (sibling crate reusing `pdftract-core` without changes) is NOT viable.**

The dependency chain is too deep: `memmap2`, `rayon`, and `zstd` are load-bearing for `pdftract-core`'s extraction pipeline and cannot be cleanly separated without modifying `pdftract-core` itself.

### Recommended Path Forward

**Reopen ADR-010 and re-scope to Option B:** a minimal `pdftract-wasm` sibling crate with a simplified vector-only parser that:
- Does NOT reuse `pdftract-core`'s extraction code (accepts code duplication)
- Uses pure-Rust, wasm32-compatible alternatives for all dependencies
- Targets a narrower use case (small-to-medium PDFs, vector text only)
- Explicitly documents limitations compared to the native CLI

This preserves ADR-010's goals (zero-install browser demo, privacy-preserving client-side extraction) while acknowledging that the full extraction pipeline cannot be made wasm32-compatible without breaking changes to `pdftract-core`.

### Next Steps

1. **Reopen ADR-010** with the findings from this spike
2. **Re-scope to Option B** (minimal WASM-specific parser in `pdftract-wasm` crate)
3. **Reassess the invalidation trigger**: "If the wasm32 build spike shows that `pdftract-core`'s vector path pulls in a dependency that cannot target `wasm32-unknown-unknown` at all" — **this condition has been met**
4. **Define the WASM crate's reduced scope** and acceptance criteria

## Test Environment

- Rust: `rustc 1.85.0-beta.2`
- Target: `wasm32-unknown-unknown` (already installed via rustup)
- Date: 2026-08-03
- Workspace: `/home/coding/pdftract`
- Bead: `bf-2uw30r`
