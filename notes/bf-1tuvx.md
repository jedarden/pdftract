# Verification Note: bf-1tuvx - Document test edge cases and limitations

## Task Completion Date
2026-08-13

## Work Summary

Enhanced the existing documentation file `docs/testing/unmapped-glyphs-tests.md` with comprehensive edge cases and limitations discovered during test runs and code exploration.

## Changes Made

### Existing Documentation (Pre-existing)
The file already contained excellent documentation covering:
- Test timeout and hung test prevention procedures
- Orphaned process management with verification scripts
- 23 configured unmapped glyph names
- Edge cases like leading slash handling, consecutive name assignments
- Platform-specific behaviors (Windows memory guard tests, Unix process management)
- Configuration file handling and checksum verification
- Common failure modes with debugging steps
- Test skip conditions with examples

### New Documentation Added

#### 1. Type 3 Font-Specific Limitations
**Content Stream Resolution:**
- Missing CharProcs → zero-glyph font
- Indirect references not supported
- Direct streams skipped with diagnostic
- Missing glyphs in encoding emit diagnostic

**Widths Array Validation:**
- Length mismatch → truncated/padded with `FontType3WidthsLengthMismatch` diagnostic
- Missing widths → all-zero array
- Indirect widths → defaults to all-zero

**Encoding Limitations:**
- Single-byte only (0-255)
- Multi-byte codes fail at Level 2
- Arbitrary glyph names escalate to Level 4

**Rasterization Limitations:**
- Document context required
- No resolver → cannot fetch glyph content
- Empty content streams → all-white bitmaps
- Default font bbox [0,0,0,0] affects rasterization size

#### 2. Font Resolver Cache Limitations
**Thread-Safety Concurrency:**
- DashMap for thread-safe concurrent access
- Cache key combines font ID (Arc pointer cast) + character code bytes
- Separate DashMap tracks diagnostic emission
- One-time emission per (font, code) pair

**Cache Behavior:**
- Cache hit returns cached result
- Cache miss computes through 4-level fallback chain
- Standard 14 fonts skip Level 3 (no embedded program)
- Level 4 shape recognition only when `shape-db` feature enabled

#### 3. Encoding Resolution Chain Edge Cases
**Level 1: ToUnicode CMap**
- Empty mapping or U+FFFD-only → fall through to Level 2
- Multi-codepoint support (ligature expansion)
- No CMap → immediate fall-through

**Level 2: AGL + Encoding**
- Single-byte only
- Must map code → glyph name → AGL
- Multi-codepoint AGL lookup first, then single-codepoint
- Not in AGL → fall through to Level 3

**Level 3: Font Fingerprint**
- Requires glyph ID (not character code)
- Fonts without embedded programs skip entirely
- Requires pre-populated fingerprint database
- No glyph ID or database → fall through to Level 4

**Level 4: Shape Recognition**
- Feature-gated on `shape-db`
- Must rasterize to 32×32 bitmap
- pHash computation with database lookup
- Hamming distance threshold ≤ 8
- Returns confidence 0.7

**Failure Mode:**
- All levels failed → U+FFFD with confidence 0.0
- `FontGlyphUnmapped` diagnostic emitted once per (font, code)

#### 4. Test Helper Functions and Edge Cases
**Glyph Generation Functions:**
- `make_rect_glyph()`: Tests `re` operator shorthand
- `make_rect_glyph_with_path_commands()`: Tests explicit path commands
- `make_line_glyph()`: Tests stroked paths vs filled
- `make_empty_glyph()`: Tests glyphs with no visible content

**CharProc Mapping Functions:**
- `make_test_char_procs()`: Standard A-Z mapping
- `make_custom_char_procs()`: Custom (name, ObjRef) tuples
- `make_custom_char_procs_from_names()`: Auto-generate sequential IDs

**Resolver Function:**
- `make_test_resolver()`: ASCII-based ID mapping (ID 1→"/A", etc.)
- **Limitation**: ASCII-only assumption
- **Limitation**: Sequential ID assumption
- **Solution**: Custom resolvers for non-standard glyph names

**Test Helper Limitations:**
1. ASCII-only character name assumption
2. Sequential ID assumption
3. No compression support
4. No resource dictionaries
5. No FontMatrix transformations
6. Empty streams vs missing glyphs distinction

## Acceptance Criteria Status

All acceptance criteria met:

- ✅ **Documentation file exists**: `docs/testing/unmapped-glyphs-tests.md` (comprehensive 496-line file)
- ✅ **Edge cases listed with explanations**: Type 3 fonts, resolver cache, encoding chain, test helpers
- ✅ **Skip behavior documented**: Platform-specific, fixture-dependent, performance tests
- ✅ **Examples of common failures provided**: Timeouts, orphaned processes, parse errors
- ✅ **Configuration file references included**: nextest.toml, unmapped-glyph-names.json, CHECKSUMS.sha256

## Verification Steps Completed

1. ✅ Read and analyzed existing documentation (already comprehensive)
2. ✅ Explored actual test code (`type3.rs`, `resolver.rs`, `test_glyph_helper.rs`)
3. ✅ Added Type 3 font-specific edge cases from implementation
4. ✅ Added resolver cache and 4-level encoding resolution details
5. ✅ Added test helper function limitations and usage patterns
6. ✅ Added examples for writing custom glyph tests

## Files Modified

- `docs/testing/unmapped-glyphs-tests.md` - Enhanced with comprehensive edge cases
- `notes/bf-1tuvx.md` - This verification note

## Test Evidence

The documentation is grounded in actual code exploration:

- **Type 3 Implementation**: `crates/pdftract-core/src/font/type3.rs`
- **Resolver Implementation**: `crates/pdftract-core/src/font/resolver.rs`
- **Test Helpers**: `crates/pdftract-core/src/font/test_glyph_helper.rs`
- **Configuration**: `build/unmapped-glyph-names.json`, `.config/nextest.toml`

## Status

**COMPLETE** - Documentation comprehensively covers all test edge cases and limitations for unmapped glyph testing, including Type 3 font specifics, resolver cache behavior, 4-level encoding resolution chain, and test helper function limitations.

## Related Beads

This bead supports:
- `pdftract-qkc77` (Genesis: pdftract Implementation)
- Type 3 font implementation beads
- Encoding resolution chain beads
- Test infrastructure beads
