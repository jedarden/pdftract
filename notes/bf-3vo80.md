# bf-3vo80: Add unmapped_glyph_names Field to Config Struct

## Summary

Successfully added the `UnmappedGlyphNamesConfig` struct to `crates/pdftract-core/build.rs`.

## Implementation

Added a new config struct at the top of `build.rs` (lines 5-41) that represents the JSON configuration structure:

```rust
#[derive(Debug, serde::Deserialize)]
struct UnmappedGlyphNamesConfig {
    /// List of glyph names to skip during CMAP and ToUnicode entry creation.
    unmapped_glyph_names: Vec<String>,

    /// Optional description of the configuration purpose.
    #[serde(default)]
    description: Option<String>,

    /// Configuration format version identifier.
    #[serde(default)]
    version: Option<String>,
}
```

## Files Modified

- `crates/pdftract-core/build.rs` (lines 5-41)

## Acceptance Criteria Status

- ✅ `unmapped_glyph_names` field is added to the config struct (`Vec<String>`)
- ✅ Field has appropriate type (`Vec<String>` for list of glyph names)
- ✅ Field includes comprehensive documentation comment
- ✅ Code compiles without errors (verified with `cargo check --package pdftract-core`)
- ⏸️ Parsing and default value handling not yet implemented (as per acceptance criteria - will be implemented in subsequent beads)

## Verification

```bash
$ cargo check --package pdftract-core
# Compilation successful - no errors
```

## Next Steps

The struct is now ready for parsing implementation in a follow-up bead that will:
1. Update `generate_unmapped_glyph_names()` to use `UnmappedGlyphNamesConfig` instead of generic `serde_json::Value`
2. Implement proper error handling with the typed struct
3. Add default value handling if fields are missing from JSON

## References

- Parent bead: bf-1y3la
- Depends on: bf-10vsm (which identified the config structure location)
- Related config file: `build/unmapped-glyph-names.json`
