# bf-1puzp: CMAP skip logic for unmapped glyphs - VERIFICATION

## Summary

The CMAP skip logic for unmapped glyphs was already implemented in the codebase. This note verifies the implementation meets all acceptance criteria.

## Implementation location

File: `/home/coding/pdftract/crates/pdftract-core/src/font/encoding.rs`
Lines: 196-203
Function: `DifferencesOverlay::parse()`

```rust
// MARKER: CMAP entry creation point - Type1 font encoding differences.
if cursor <= 255 {
    // Skip unmapped glyph names (e.g., .notdef) to prevent them from
    // appearing in text extraction output. These glyphs have no valid
    // Unicode mapping and should emit GLYPH_UNMAPPED diagnostics instead.
    if !crate::font::is_unmapped_glyph_name(&name) {
        overlay.entries.push((cursor as u8, Arc::clone(name)));
    }
}
```

## Supporting infrastructure

File: `/home/coding/pdftract/crates/pdftract-core/src/font/unmapped.rs`
- Defines `UNMAPPED_GLYPH_NAMES: LazyLock<HashSet<&'static str>>` containing ".notdef"
- Provides `is_unmapped_glyph_name(name: &str) -> bool` helper function
- Handles both `/name` and `name` formats (strips leading slash)

## Acceptance criteria verification

✅ **PASS**: Generator checks glyph name against unmapped_glyph_names config before CMAP entry creation
   - Line 200: `if !crate::font::is_unmapped_glyph_name(&name)`

✅ **PASS**: Unmapped glyphs are skipped during CMAP generation
   - Lines 200-203: Unmapped glyphs are not added to `overlay.entries`

✅ **PASS**: Code compiles without errors
   - Verified: `cargo check --lib -p pdftract-core` completed successfully

✅ **PASS**: Simple test demonstrating skip behavior
   - Test: `test_differences_overlay_skips_notdef` (line 676)
   - Test: `test_differences_overlay_skips_notdef_with_slash` (line 696)

## Test results

All 24 encoding tests pass:
```
running 24 tests
test font::encoding::tests::test_differences_overlay_skips_notdef ... ok
test font::encoding::tests::test_differences_overlay_skips_notdef_with_slash ... ok
test result: ok. 24 passed; 0 failed; 0 ignored
```

## Behavior

When a `/Differences` array contains `.notdef` or `/.notdef`:
- The glyph name is checked against `UNMAPPED_GLYPH_NAMES`
- If unmapped, the entry is skipped (not added to the overlay)
- No error is emitted (clean skip)
- Normal glyphs continue to be added

This prevents `.notdef` from appearing in text extraction output while allowing it to remain valid in the PDF font encoding specification.

## Git status

No changes required - implementation was already complete in the codebase.
