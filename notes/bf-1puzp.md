# CMAP Skip Logic for Unmapped Glyphs - Verification

**Bead ID**: bf-1puzp
**Date**: 2026-07-06
**Status**: VERIFIED - Already Implemented

## Overview

The CMAP skip logic for unmapped glyphs was already implemented in the codebase. This note verifies the implementation meets all acceptance criteria.

## Implementation Location

**File**: `crates/pdftract-core/src/font/encoding.rs`
**Function**: `DifferencesOverlay::parse()`
**Lines**: 194-203

### Code

```rust
// MARKER: CMAP entry creation point - Type1 font encoding differences.
// See notes/bf-e4uvb-child-1.md for documentation.
if cursor <= 255 {
    // Skip unmapped glyph names (e.g., .notdef) to prevent them from
    // appearing in text extraction output. These glyphs have no valid
    // Unicode mapping and should emit GLYPH_UNMAPPED diagnostics instead.
    if !crate::font::is_unmapped_glyph_name(&name) {
        overlay.entries.push((cursor as u8, Arc::clone(name)));
    }
}
```

## Acceptance Criteria Verification

### ✅ 1. Generator checks glyph name against unmapped_glyph_names config before CMAP entry creation

The code calls `crate::font::is_unmapped_glyph_name(&name)` at line 200, which checks the glyph name against the `UNMAPPED_GLYPH_NAMES` set defined in `crates/pdftract-core/src/font/unmapped.rs`.

### ✅ 2. Unmapped glyphs are skipped during CMAP generation

Glyphs that return `true` from `is_unmapped_glyph_name()` are skipped - the `overlay.entries.push()` call is only executed when the check returns `false` (note the `!` negation).

### ✅ 3. Code compiles without errors

Verified with:
```bash
cargo build -p pdftract-core
```
Compilation successful with no errors.

### ✅ 4. Simple test demonstrating skip behavior

Two existing tests demonstrate the skip behavior:

**Test 1**: `test_differences_overlay_skips_notdef` (encoding.rs:676-693)
```rust
let arr = PdfObject::Array(Box::new(vec![
    PdfObject::Integer(39),
    PdfObject::Name(Arc::from(".notdef")),
    PdfObject::Integer(96),
    PdfObject::Name(Arc::from("grave")),
]));

let overlay = DifferencesOverlay::parse(&arr, &mut diagnostics);

// .notdef should be skipped, only grave should be present
assert_eq!(overlay.get(39), None); // .notdef was skipped
assert_eq!(overlay.get(96), Some(Arc::from("grave")));
assert_eq!(overlay.len(), 1);
```

**Test 2**: `test_differences_overlay_skips_notdef_with_slash` (encoding.rs:696-713)
```rust
let arr = PdfObject::Array(Box::new(vec![
    PdfObject::Integer(10),
    PdfObject::Name(Arc::from("/.notdef")),
    PdfObject::Integer(11),
    PdfObject::Name(Arc::from("A")),
]));

let overlay = DifferencesOverlay::parse(&arr, &mut diagnostics);

// /.notdef should be skipped, only A should be present
assert_eq!(overlay.get(10), None); // /.notdef was skipped
assert_eq!(overlay.get(11), Some(Arc::from("A")));
```

Both tests pass, confirming the skip behavior works correctly.

## Test Execution

```bash
$ cargo test -p pdftract-core --lib -- encoding::tests::test_differences_overlay_skips_notdef
running 2 tests
test font::encoding::tests::test_differences_overlay_skips_notdef ... ok
test font::encoding::tests::test_differences_overlay_skips_notdef_with_slash ... ok

test result: ok. 2 passed; 0 failed; 0 ignored
```

## Implementation Details

### Unmapped Glyph Configuration

**File**: `crates/pdftract-core/src/font/unmapped.rs`

The `UNMAPPED_GLYPH_NAMES` set currently contains:
- `.notdef` - The special fallback glyph defined by PDF/font specifications

Additional unmapped glyph names can be added to this set as needed.

### Check Function

The `is_unmapped_glyph_name()` function:
1. Strips leading `/` if present (handles both `.notdef` and `/.notdef`)
2. Checks if the clean name is in `UNMAPPED_GLYPH_NAMES`
3. Returns `true` if unmapped, `false` otherwise

## Summary

All acceptance criteria are met:
- ✅ Glyph names checked against unmapped_glyph_names config
- ✅ Unmapped glyphs skipped during CMAP generation  
- ✅ Code compiles without errors
- ✅ Tests demonstrate skip behavior

The implementation is complete and working correctly.

## Related Documentation

- `notes/bf-e4uvb-child-1.md` - CMAP and ToUnicode entry creation points documentation
- `crates/pdftract-core/src/font/unmapped.rs` - Unmapped glyph configuration
- `crates/pdftract-core/src/font/encoding.rs` - CMAP entry creation with skip logic
