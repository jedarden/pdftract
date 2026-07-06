# Verification Note: bf-6afe3 - Test demonstrating unmapped glyph skip behavior

## Summary
Added a focused test (`test_unmapped_glyph_skip_behavior`) that verifies unmapped glyphs are correctly skipped during CMAP generation while normal mapped glyphs are preserved.

## Acceptance Criteria

### ✅ PASS: Test exists and passes
- Test added to `crates/pdftract-core/src/font/encoding.rs` (lines 873-902)
- Test passes: `cargo test -p pdftract-core --lib encoding::tests::test_unmapped_glyph_skip_behavior`
- All 27 encoding tests pass

### ✅ PASS: Test demonstrates unmapped glyphs are skipped
- Test verifies that `.notdef` at codes 32 and 67 does NOT appear in the overlay
- Uses `assert_eq!(overlay.get(32), None)` and `assert_eq!(overlay.get(67), None)` with clear error messages

### ✅ PASS: Test demonstrates mapped glyphs still appear
- Test verifies that normal glyphs `A`, `space`, and `B` DO appear in the overlay at codes 65, 66, and 68
- Uses `assert_eq!(overlay.get(65), Some(Arc::from("A")))` and similar assertions

### ✅ PASS: Simple, readable test case
- Test is self-contained with clear documentation comment explaining the behavior
- Uses descriptive variable names and assertion messages
- Covers multiple unmapped glyphs and multiple mapped glyphs in one test

## Implementation Details

The test creates a `/Differences` array with mixed glyphs:
- Code 32: `.notdef` (unmapped, should skip)
- Code 65: `A` (normal, should appear)
- Code 66: `space` (normal, should appear)
- Code 67: `.notdef` (unmapped, should skip)
- Code 68: `B` (normal, should appear)

Then verifies:
1. Unmapped glyphs (`.notdef`) return `None` from `overlay.get()`
2. Mapped glyphs return `Some(name)` from `overlay.get()`
3. Final overlay has exactly 3 entries (not 5)
4. No diagnostics are emitted for normal skip behavior

## Files Modified
- `crates/pdftract-core/src/font/encoding.rs` - Added test (lines 873-902)

## Verification Commands
```bash
# Run the new test
cargo test -p pdftract-core --lib encoding::tests::test_unmapped_glyph_skip_behavior

# Run all encoding tests
cargo test -p pdftract-core --lib encoding::tests
```

## Related Documentation
- The skip logic is in `DifferencesOverlay::parse()` at lines 221-231 in encoding.rs
- The `is_unmapped_glyph_name()` method at lines 274-282 checks against `unmapped_glyph_names` set
- Default unmapped glyphs include `.notdef` and others from `UNMAPPED_GLYPH_NAMES`
