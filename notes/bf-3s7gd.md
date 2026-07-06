# bf-3s7gd: CMAP skip check for unmapped glyphs

## Status: ALREADY IMPLEMENTED

This bead's work was already completed in a previous iteration. The skip logic was implemented in commit `7660cfc8` (bead `bf-4zgpr`), which refactored an earlier implementation.

## Implementation location

File: `crates/pdftract-core/src/font/encoding.rs`

Lines 224-229:
```rust
// Skip unmapped glyph names (e.g., .notdef) to prevent them from
// appearing in text extraction output. These glyphs have no valid
// Unicode mapping and should emit GLYPH_UNMAPPED diagnostics instead.
if !overlay.is_unmapped_glyph_name(&name) {
    overlay.entries.push((cursor as u8, Arc::clone(name)));
}
```

## Supporting infrastructure

1. **Unmapped glyph names configuration** (`unmapped.rs`):
   - `is_unmapped_glyph_name(name: &str) -> bool` - checks against global UNMAPPED_GLYPH_NAMES set
   - Handles glyph names with or without leading `/`

2. **DifferencesOverlay instance method** (`encoding.rs` lines 274-282):
   ```rust
   fn is_unmapped_glyph_name(&self, name: &str) -> bool {
       let clean_name = if name.starts_with('/') {
           &name[1..]
       } else {
           name
       };
       self.unmapped_glyph_names.contains(clean_name)
   }
   ```

3. **Configuration field** (`encoding.rs` lines 136, 164-168):
   - `unmapped_glyph_names: HashSet<String>` - stores the set of glyph names to skip
   - Defaults to global `UNMAPPED_GLYPH_NAMES` set
   - Can be customized via `with_unmapped_glyph_names()` constructor

## Acceptance criteria verification

- ✅ **Code checks glyph name against unmapped_glyph_names before CMAP entry creation**
  - Lines 227: `if !overlay.is_unmapped_glyph_name(&name)`

- ✅ **Unmapped glyphs are skipped during CMAP generation**
  - Lines 227-229: If glyph is unmapped, `entries.push()` is skipped

- ✅ **Skip happens cleanly (no errors, no panics)**
  - Tests pass: `test_differences_overlay_skips_notdef`, `test_differences_overlay_skips_notdef_with_slash`
  - No diagnostics emitted for skipped glyphs

- ✅ **Code compiles without errors**
  - `cargo test -p pdftract-core --lib differences_overlay_skips_notdef` passes

## Test results

```bash
$ cargo test -p pdftract-core --lib differences_overlay_skips_notdef --no-fail-fast
running 2 tests
test font::encoding::tests::test_differences_overlay_skips_notdef ... ok
test font::encoding::tests::test_differences_overlay_skips_notdef_with_slash ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2895 filtered out
```

## Related beads

- `bf-4zgpr`: Added `unmapped_glyph_names` config field to DifferencesOverlay
- `bf-6cvmz`: Documented unmapped_glyph_names configuration structure
- `bf-o4nrq`: Documented CMAP entry creation point
- `bf-1puzp`: Earlier verification of CMAP skip logic

## CMAP entry creation point

This implementation is at the CMAP entry creation point identified in child beads:
- **Marker comment**: Line 222-223 in `encoding.rs`
- **Context**: Type1 font encoding differences parsing
- **Call site**: `DifferencesOverlay::parse()` method

## Conclusion

All acceptance criteria for this bead were already met by prior work. The implementation correctly skips unmapped glyph names during CMAP generation, preventing them from appearing in text extraction output.
