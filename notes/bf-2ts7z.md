# bf-2ts7z: Implement skip logic for unmapped glyphs in CMAP entry creation

## Implementation Status
**COMPLETE** - Skip logic was already implemented in commit `7660cfc8`.

## Verification

### 1. Code Implementation ✓

**Location:** `/home/coding/pdftract/crates/pdftract-core/src/font/encoding.rs`

**Key Components:**

1. **Skip Logic at CMAP Entry Creation Point** (lines 221-230):
```rust
// MARKER: CMAP entry creation point - Type1 font encoding differences.
// See notes/bf-e4uvb-child-1.md for documentation.
if cursor <= 255 {
    // Skip unmapped glyph names (e.g., .notdef) to prevent them from
    // appearing in text extraction output. These glyphs have no valid
    // Unicode mapping and should emit GLYPH_UNMAPPED diagnostics instead.
    if !overlay.is_unmapped_glyph_name(&name) {
        overlay.entries.push((cursor as u8, Arc::clone(name)));
    }
}
cursor = cursor.saturating_add(1);
```

2. **Glyph Name Check Method** (lines 274-282):
```rust
fn is_unmapped_glyph_name(&self, name: &str) -> bool {
    // Strip leading slash if present
    let clean_name = if name.starts_with('/') {
        &name[1..]
    } else {
        name
    };
    self.unmapped_glyph_names.contains(clean_name)
}
```

3. **Struct Integration** (lines 133-168):
- `DifferencesOverlay` struct has `unmapped_glyph_names: std::collections::HashSet<String>` field
- Default constructor uses global `UNMAPPED_GLYPH_NAMES` set
- Custom constructor accepts custom unmapped sets
- Getter/setter methods for configuration

### 2. Acceptance Criteria ✓

- **Generator checks glyph name against unmapped_glyph_names before CMAP entry creation**: ✓
  - Implemented at line 227: `if !overlay.is_unmapped_glyph_name(&name)`

- **Unmapped glyphs are skipped during CMAP generation**: ✓
  - When `is_unmapped_glyph_name()` returns `true`, the entry is NOT added to `overlay.entries`

- **Skip happens cleanly (no errors, no partial entries)**: ✓
  - No error messages are emitted
  - No partial entries are written (skip is atomic)
  - Cursor continues normally with `cursor = cursor.saturating_add(1)`

- **Code compiles without errors**: ✓
  - `cargo check --workspace` succeeds with only unrelated warnings
  - `cargo test --lib -p pdftract-core encoding::tests` - all 26 tests pass

### 3. Test Coverage ✓

All relevant tests pass:

- `test_differences_overlay_skips_notdef`: Verifies `.notdef` is skipped
- `test_differences_overlay_skips_notdef_with_slash`: Verifies `/.notdef` (with slash) is skipped
- `test_differences_overlay_custom_unmapped_glyph_names`: Verifies custom unmapped sets work correctly
- `test_differences_overlay_empty_unmapped_glyph_names`: Verifies empty sets allow all glyphs

### 4. Related Implementation

This is the **child 2** component in the unmapped_glyph_names feature:

- **Child 1** (bf-4zgpr): Added `unmapped_glyph_names` config field to CMAP generator struct
- **Child 2** (bf-2ts7z): **THIS BEAD** - Implements the skip logic at CMAP entry creation
- **Child 3** (bf-2nn5x): Verified wiring of config to CMAP entry creation point

## Implementation Quality

**Excellent** - The implementation is:
- Clean and well-documented
- Handles both `.notdef` and `/.notdef` (with/without leading slash)
- Supports both default and custom unmapped glyph sets
- Has comprehensive test coverage
- No errors or partial entries during skip
- Atomic operation - either entry is added or skipped cleanly

## Git Commits

- `7660cfc8` - feat(bf-4zgpr): add unmapped_glyph_names config field to CMAP generator
- `84043bd9` - docs(bf-3s7gd): document CMAP skip check verification (already implemented)
- `f757d8b6` - docs(bf-2nn5x): verify unmapped_glyph_names config wiring to CMAP entry creation

## Verification Date
2026-07-06

## Conclusion

The skip logic for unmapped glyphs in CMAP entry creation is **fully implemented and tested**. The implementation prevents unmapped glyphs (like `.notdef`) from appearing in the CMAP, making them unmappable from text content and ensuring proper `GLYPH_UNMAPPED` diagnostics instead.
