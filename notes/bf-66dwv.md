# bf-66dwv: Implement parsing for unmapped_glyph_names from config files

## Summary

Implemented parsing logic for the `unmapped_glyph_names` field from config files using the `UnmappedGlyphNamesConfig` struct that was added in bf-3vo80.

## Changes Made

### File: `crates/pdftract-core/build.rs`

**Lines 1036-1067**: Updated `generate_unmapped_glyph_names()` function to use proper deserialization

**Before:**
```rust
let data: serde_json::Value = serde_json::from_str(&json_content)
    .unwrap_or_else(|_| panic!("Failed to parse {}", actual_unmapped_path.display()));

// Extract the unmapped_glyph_names array
let names_array = data
    .get("unmapped_glyph_names")
    .and_then(|v| v.as_array())
    .expect("unmapped_glyph_names array missing");
```

**After:**
```rust
// Parse using UnmappedGlyphNamesConfig struct for proper deserialization
let config: UnmappedGlyphNamesConfig = serde_json::from_str(&json_content)
    .unwrap_or_else(|e| panic!("Failed to parse {}: {}", actual_unmapped_path.display(), e));

// Extract the unmapped_glyph_names array from the config
let names_array = &config.unmapped_glyph_names;
```

## Acceptance Criteria Verification

### ✅ PASS: unmapped_glyph_names can be parsed from config files
- The `UnmappedGlyphNamesConfig` struct deserializes the JSON config correctly
- Build script successfully reads `build/unmapped-glyph-names.json` and extracts the `unmapped_glyph_names` array

### ✅ PASS: Parser handles both empty lists and populated lists
- Empty lists work: `Vec<String>` is empty, generates no HashSet insertions
- Populated lists work: Current config has 12 entries, all parsed correctly

### ✅ PASS: Integration with existing deserialization logic works
- No changes required to the rest of the build script
- Generated `unmapped_glyph_names.rs` file output is identical to before (just using proper deserialization now)

### ✅ PASS: Code compiles without errors
- `cargo build -p pdftract-core` succeeds cleanly
- No warnings or errors

## Test Verification

Verified by inspecting the generated file at `target/debug/build/pdftract-core-*/out/unmapped_glyph_names.rs`:

```rust
/// Glyph count: 12
pub static UNMAPPED_GLYPH_NAMES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    set.insert(".notdef");
    set.insert(".null");
    set.insert("g000");
    set.insert("g001");
    set.insert("g002");
    set.insert("g003");
    set.insert("g004");
    set.insert("g005");
    set.insert("g006");
    set.insert("g007");
    set.insert("g008");
    set.insert("g009");
    set
});
```

All 12 glyph names from `build/unmapped-glyph-names.json` are correctly parsed and inserted into the HashSet.

## Config File Format

The current config format (JSON) is:
```json
{
  "unmapped_glyph_names": [".notdef", ".null", "g000", ...],
  "description": "Glyph names that should be skipped...",
  "version": "1.0"
}
```

The `UnmappedGlyphNamesConfig` struct supports:
- **Required field**: `unmapped_glyph_names: Vec<String>`
- **Optional fields**: `description: Option<String>`, `version: Option<String>`

The struct definition uses `#[serde(default)]` for optional fields, making them truly optional during parsing.

## Implementation Notes

1. **Type Safety**: Changed from `serde_json::Value` (dynamic) to `UnmappedGlyphNamesConfig` (static typing), providing better compile-time guarantees and clearer error messages

2. **Better Error Messages**: The new panic message includes the actual deserialization error, making debugging easier if the config file is malformed

3. **No Functional Changes**: The output is identical to the previous implementation; this is purely a refactoring to use the proper deserialization structure
