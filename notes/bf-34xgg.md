# Bead bf-34xgg: Default value handling for unmapped_glyph_names

## Implementation Summary

Added `#[serde(default)]` attribute to the `unmapped_glyph_names` field in the `UnmappedGlyphNamesConfig` struct (build.rs). This ensures the field defaults to an empty `Vec<String>` when not specified in config files.

## Changes Made

### File: crates/pdftract-core/build.rs

Modified the `UnmappedGlyphNamesConfig` struct to add the `#[serde(default)]` attribute to `unmapped_glyph_names`:

```rust
#[derive(Debug, serde::Deserialize)]
struct UnmappedGlyphNamesConfig {
    /// List of glyph names to skip during CMAP and ToUnicode entry creation.
    ///
    /// These glyphs have no valid Unicode mapping and should not appear in
    /// text extraction output. Common examples include `.notdef` (the PDF
    /// fallback glyph) and Private Use Area (PUA) glyphs like `g000-g009`.
    ///
    /// Defaults to an empty list if not specified in the config file.
    #[serde(default)]  // <-- ADDED
    unmapped_glyph_names: Vec<String>,

    /// Optional description of the configuration purpose.
    #[serde(default)]
    description: Option<String>,

    /// Configuration format version identifier.
    #[serde(default)]
    version: Option<String>,
}
```

### File: crates/pdftract-core/tests/unmapped_glyph_names_config.rs (NEW)

Added comprehensive integration tests covering:
1. Config without `unmapped_glyph_names` field → defaults to empty list
2. Config with `unmapped_glyph_names` field → parses correctly
3. Config with explicit empty array → works as expected
4. Minimal empty config → all fields default appropriately

## Acceptance Criteria Status

- ✅ **unmapped_glyph_names defaults to empty list when not specified**: The `#[serde(default)]` attribute ensures this behavior.
- ✅ **Default value handling integrates cleanly with existing config logic**: The change is additive and doesn't affect existing functionality.
- ✅ **Code compiles without errors**: Verified with `cargo build -p pdftract-core` and `cargo check -p pdftract-core`.
- ✅ **Both specified and unspecified cases work correctly**: All 4 integration tests pass.

## Test Results

```
running 4 tests
test test_unmapped_glyph_names_defaults_to_empty ... ok
test test_unmapped_glyph_names_empty_array ... ok
test test_unmapped_glyph_names_minimal_config ... ok
test test_unmapped_glyph_names_specified ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Build Verification

- `cargo build -p pdftract-core`: **SUCCESS** (exit code 0)
- Build script compiles and generates code correctly with the default value attribute

## Commits

This implementation is ready to commit as:
- `feat(bf-34xgg): add default value handling for unmapped_glyph_names config field`
