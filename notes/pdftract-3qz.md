# pdftract-3qz: Phase 2.1 Font Type Detection (coordinator)

## Summary

Coordinator for sub-phase 2.1: Font Type Detection. All 5 child beads completed successfully, delivering a comprehensive font module that can classify, load, and provide metrics for all PDF font types.

## Children Completed

| Bead ID | Title | Commit | Verification Note |
|---------|-------|--------|-------------------|
| pdftract-3uq | Font subtype classifier and BaseFont prefix stripper | 46c515e | notes/pdftract-3uq.md |
| pdftract-juc | Standard 14 font registry with hardcoded metrics | 7429a67 | (included below) |
| pdftract-6ah | Embedded font program loader (ttf-parser/owned_ttf_parser) | ffaaf69 | notes/pdftract-6ah.md |
| pdftract-cv4 | Type 0 composite font + descendant CIDFont loader | 5e2390f | notes/pdftract-cv4.md |
| pdftract-5sh | CIDToGIDMap resolver (Identity and stream forms) | 03aa4da | notes/pdftract-5sh.md |

## Acceptance Criteria Status

| Criterion | Status |
|-----------|--------|
| All children closed | PASS - All 5 child beads closed |
| Classifier returns one of {Type1, Type1Std14, TrueType, Type0, CIDFontType0, CIDFontType2, Type3, OpenTypeCFF} | PASS |
| Subset prefix `ABCDEF+Times-Roman` strips to `Times-Roman` for Std-14 lookup | PASS |
| CIDFontType2 with `/CIDToGIDMap /Identity`: GID == CID | PASS |
| CIDFontType2 with stream CIDToGIDMap: 2-byte big-endian decode verified | PASS |
| Module unit tests in `crates/pdftract-core/src/font/` pass | PASS - 77 tests |
| No unwrap/expect on resource dict access | PASS - uses `.and_then()` and defaults |

## Module Structure

```
crates/pdftract-core/src/font/
├── mod.rs        # FontKind enum, classify_font(), strip_subset_prefix()
├── std14.rs      # Standard 14 font metrics registry (build.rs generated)
├── embedded.rs   # EmbeddedFont, FontMetrics, OpenTypeMetrics, EmptyFontMetrics
└── type0.rs      # Type0Font, DescendantCIDFont, CIDToGIDMap, /W array parsing
```

## Test Results

```
test result: ok. 77 passed; 0 failed; 0 ignored
```

All font module tests pass, covering:
- Font classification (Type1, Type1Std14, TrueType, Type0, CIDFontType0, CIDFontType2, Type3, OpenTypeCFF)
- Subset prefix stripping (valid, invalid, edge cases)
- Standard 14 font detection
- Type0 composite font loading
- CIDToGIDMap resolution (Identity and stream forms)
- /W array parsing (per-CID and range forms)
- Embedded font program loading (TrueType, OpenType CFF)

## Child Bead Summaries

### pdftract-3uq: Font subtype classifier and BaseFont prefix stripper
- Implemented `FontKind` enum with all 8 PDF font types
- `strip_subset_prefix()` - validates exactly 6 ASCII uppercase + `+`
- `classify_font()` - reads `/Subtype`, `/BaseFont`, descendant CIDFont, FontDescriptor
- 21 unit tests covering all branches

### pdftract-juc: Standard 14 font registry with hardcoded metrics
- `build.rs` generates compile-time metrics from AFM-derived JSON
- `Std14Metrics` struct with widths, ascent, descent, italic_angle, font_bbox
- `get_std14_metrics()` lookup by canonical name (post-prefix-strip)
- Symbol/ZapfDingbats use distinct encodings (SymbolEncoding, ZapfDingbatsEncoding)
- Binary footprint: ~20 KB generated source, ~8 KB data (well under 60 KB limit)

### pdftract-6ah: Embedded font program loader
- `EmbeddedFont` wrapping `owned_ttf_parser::OwnedFace`
- `FontMetrics` trait with `glyph_id_for()`, `advance()`, `bbox()`
- `EmptyFontMetrics` fallback for corrupt/missing font programs
- Graceful handling of subset fonts (unmapped chars return None)
- Diagnostic `FONT_PARSE_FAILED` for corrupt programs

### pdftract-cv4: Type 0 composite font + descendant CIDFont loader
- `Type0Font` with descendant `DescendantCIDFont`
- `/DW` default width parsing (default 1000)
- `/W` array parsing (per-CID `[c [w1 w2 ...]]` and range `[cfirst clast w]`)
- Sparse `BTreeMap<u32, u16>` storage for CID widths
- CIDFontType0 (CFF) vs CIDFontType2 (TrueType) detection

### pdftract-5sh: CIDToGIDMap resolver
- `CidToGidMap::{Identity, Array(Box<[u16]>)}` enum
- Identity short-circuit (zero allocation, GID == CID)
- Stream form: 2-byte big-endian u16 array indexed by CID
- Diagnostic `CIDTOGIDMAP_TRUNCATED` for odd-byte-count input
- Out-of-range CID returns GID 0 (notdef glyph)

## Integration Points

This module delivers the `Font` value needed by:
- **Phase 2.2**: Encoding resolution (ToUnicode, differences, AGL fallback)
- **Phase 2.3**: CJK CMap parsing and CID-to-Unicode mapping
- **Phase 2.4**: Type3 font content stream execution
- **Phase 3**: Content stream execution (Tj, TJ, BT/ET operators)

## Files Modified/Created

**Created:**
- `crates/pdftract-core/src/font/mod.rs`
- `crates/pdftract-core/src/font/std14.rs`
- `crates/pdftract-core/src/font/embedded.rs`
- `crates/pdftract-core/src/font/type0.rs`
- `crates/pdftract-core/build.rs`
- `crates/pdftract-core/build/std14-metrics.json`
- `crates/pdftract-core/build/generate_std14_metrics.py`
- `crates/pdftract-core/build/fix_std14_weights.py`

**Modified:**
- `crates/pdftract-core/src/lib.rs` - added `pub mod font;`
- `crates/pdftract-core/src/diagnostics.rs` - added `FONT_PARSE_FAILED`, `CIDTOGIDMAP_TRUNCATED`
- `.gitignore` - added `!/crates/pdftract-core/build/` exceptions

## Commits Referenced

- `46c515e` feat(pdftract-3uq): add font type classifier and subset prefix stripper
- `7429a67` feat(pdftract-juc): implement Standard 14 font metrics registry
- `ffaaf69` feat(pdftract-6ah): implement embedded font program loader
- `5e2390f` feat(pdftract-cv4): Type 0 composite font + descendant CIDFont loader
- `03aa4da` feat(pdftract-5sh): CIDToGIDMap resolver for CIDFontType2
- `075de55` docs(pdftract-cv4): add verification note
- `b7392f1` docs(pdftract-6ah): add verification note

## Notes

- All child beads have verification notes in `notes/` directory
- Type3 font `/CharProcs` execution deferred to Phase 2.4 (as planned)
- OpenType CFF uses same `owned_ttf_parser` entrypoint as TrueType (CFF support via `opentype-layout` feature)
- The classifier handles indirect references gracefully (returns default, does not crash)
- Standard 14 fonts may have embedded font programs; registry serves as fallback

## Ready for Next Phase

Phase 2.1 is complete. The font module is ready for:
- **Phase 2.2**: Encoding resolution (ToUnicode, differences, AGL)
- **Phase 2.3**: CJK CMap parsing
- **Phase 2.4**: Type3 content stream execution
