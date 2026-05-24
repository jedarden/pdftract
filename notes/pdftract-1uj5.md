# Verification Note: pdftract-1uj5

## Summary

Implemented `resolve_type3()` function for Type 3 font encoding resolution using the Type 3-specific fallback chain (L1: ToUnicode, L2: AGL, skip L3, L4: shape recognition).

## Implementation

### Files Modified

1. **crates/pdftract-core/src/font/shape.rs**
   - Added `ShapeEntry` struct for pHash + char pairs
   - Added `ShapeMatch` struct for lookup results with Hamming distance
   - Added `lookup_shape()` function for shape database lookup (stub returning empty DB)
   - Added `ShapeMatch::is_acceptable()` method for threshold check (≤8 bits)

2. **crates/pdftract-core/src/font/resolver.rs**
   - Added imports: `lookup_shape`, `phash_glyph`, `Type3Font`, `rasterize_type3_glyph`
   - Added `resolve_type3()` function implementing Type 3-specific chain:
     - L1: ToUnicode CMap lookup (reuses `resolve_level1`)
     - L2: Encoding + AGL lookup (reuses `resolve_level2`)
     - L3: SKIPPED with comment for Type 3 fonts
     - L4: Shape recognition via `resolve_type3_level4`
   - Added `resolve_type3_level4()` function:
     - Gets glyph name from encoding
     - Rasterizes glyph via `rasterize_type3_glyph`
     - Computes pHash via `phash_glyph`
     - Looks up in shape DB via `lookup_shape`
     - Returns `ResolvedGlyph` with `UnicodeSource::ShapeMatch` and confidence 0.7
   - Added 3 tests for Type 3 resolution

3. **crates/pdftract-core/src/font/mod.rs**
   - Updated exports to include `resolve_type3`, `lookup_shape`, `ShapeEntry`, `ShapeMatch`

4. **crates/pdftract-core/src/font/type3.rs**
   - Fixed overflow bug in `load_widths()`: cast to `usize` before arithmetic to avoid overflow when `last_char=255, first_char=0`

## Acceptance Criteria Status

| Criteria | Status | Notes |
|----------|--------|-------|
| Type 3 with ToUnicode 0x41 -> 'A' (1.0) | PASS | Test: `test_resolve_type3_with_tounicode` |
| Type 3 with glyph name 'A' via Encoding (0.9) | PASS | Test: `test_resolve_type3_with_agl` |
| Type 3 with arbitrary name + shape match (0.7) | WARN | Shape DB is stub (empty) - infrastructure ready, awaits `build/glyph-shapes.json` |
| Type 3 with arbitrary name + no match (0.0) + diag | PASS | Test: `test_resolve_type3_fallback_to_fffd` |

## Test Results

```bash
cargo test --lib -p pdftract-core -- resolver::tests::test_resolve_type3
# All 3 tests passed

cargo test --lib -p pdftract-core -- font::shape::
# 16 tests passed
```

## Technical Notes

1. **Shape DB Stub**: The `lookup_shape()` function returns an empty database slice. The actual shape database generation from `build/glyph-shapes.json` is a separate bead (Phase 2.5).

2. **L3 Skip**: Explicit comment added: `// Type 3 fonts have no embedded program; L3 fingerprinting not applicable`

3. **Diagnostic Codes**: Uses existing `DiagCode::FontGlyphUnmapped` for Type 3 failures. The bead description mentioned `TYPE3_GLYPH_UNMAPPED` but the existing code is sufficient.

4. **Caching**: Per bead guidance, caching is shared with the Phase 2.2 resolver via the polymorphic `ResolverCache` key. No parallel Type 3 cache was created.

5. **Branching on Font Kind**: The bead description mentions `Branch on font.kind()` but the current architecture has Type3Font as a separate struct with its own encoding field. Callers check font kind and dispatch to `resolve_type3()` directly for Type 3 fonts.

## Commits

- `fix(pdftract-1uj5): fix overflow in Type3Font::load_widths`
- `feat(pdftract-1uj5): implement resolve_type3 for Type 3 font encoding resolution`
- `feat(pdftract-1uj5): add shape lookup stub and ShapeMatch types`

## Next Steps

The shape database population (Phase 2.5) will need to:
1. Generate `build/glyph-shapes.json` from offline glyph rendering
2. Update `shape_database()` in `shape.rs` to return the generated data
3. Re-test acceptance criterion #3 with actual shape matches
