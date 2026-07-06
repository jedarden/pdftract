# bf-10vsm: Generator Config Structure Location

## Summary

Found the generator config structure for unmapped glyph names handling.

## Config Structure Location

**Primary config file:** `/home/coding/pdftract/build/unmapped-glyph-names.json`

**Rust config struct:** `crates/pdftract-core/src/font/encoding.rs` - `DifferencesOverlay` struct

## Current Fields and Types

### JSON Config Format (`unmapped-glyph-names.json`)

```json
{
  "unmapped_glyph_names": [
    ".notdef",
    ".null",
    "g000",
    "g001",
    // ... more glyph names
  ],
  "description": "...",
  "version": "1.0"
}
```

**Fields:**
- `unmapped_glyph_names` - Array of strings (glyph names to skip during CMAP entry creation)
- `description` - String (optional documentation)
- `version` - String (format version identifier)

### Rust Struct (`DifferencesOverlay` in `encoding.rs`)

```rust
pub struct DifferencesOverlay {
    entries: Vec<(u8, Arc<str>)>,
    unmapped_glyph_names: std::collections::HashSet<String>,
}
```

**Fields:**
- `entries` - Sparse list of (code, glyph_name) overrides
- `unmapped_glyph_names` - HashSet of glyph names to skip during CMAP entry creation

## Build Process

The config is processed at **build time** by `crates/pdftract-core/build.rs`:

1. Reads `build/unmapped-glyph-names.json`
2. Calls `generate_unmapped_glyph_names()` function (lines 946-1050)
3. Emits `unmapped_glyph_names.rs` in OUT_DIR with a `UNMAPPED_GLYPH_NAMES` static HashSet
4. This is auto-included by `crates/pdftract-core/src/font/unmapped.rs` via `include!()`

## Usage

The config is used in `DifferencesOverlay::parse()` (encoding.rs:190-241) to skip unmapped glyph names when creating CMAP entries. This prevents glyphs like `.notdef` from appearing in text extraction output.

## Files Involved

- Config: `build/unmapped-glyph-names.json`
- Build script: `crates/pdftract-core/build.rs` (lines 946-1050)
- Rust struct: `crates/pdftract-core/src/font/encoding.rs` (lines 128-297)
- Auto-generated: `target/*/build/*/out/unmapped_glyph_names.rs`
- Module: `crates/pdftract-core/src/font/unmapped.rs`

## Verification

- ✅ JSON file exists and is valid
- ✅ Build script reads and processes the file
- ✅ Generated code is included in the font::unmapped module
- ✅ DifferencesOverlay uses the generated UNMAPPED_GLYPH_NAMES set
