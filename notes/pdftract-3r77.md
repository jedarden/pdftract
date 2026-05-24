# Verification Note: pdftract-3r77

## Bead
7.6.3: Non-link annotation extractor (Highlight/Stamp/FreeText/Note/etc.)

## Summary
Implemented subtype-specific field extraction for non-link annotations.

## Changes Made

### 1. Annotation Struct Enhancement
- Added `AnnotationSpecific` enum to capture subtype-specific fields:
  - `TextMarkup` - for Highlight/Squiggly/StrikeOut/Underline with `/QuadPoints`
  - `Stamp` - for `/Name` icon name
  - `FreeText` - for `/DA` default appearance string
  - `Text` - for sticky notes with `/Open`, `/State`, `/StateModel`
  - `Ink` - for `/InkList` stroke paths
  - `Line` - for `/L` endpoints
  - `Polygon` - for `/Vertices`
  - `FileAttachment` - for `/FS` filespec reference
  - `Other` - for Circle, Square, Caret, Redact, Sound, Movie, Screen, PrinterMark, TrapNet, Watermark, 3D

### 2. Implementation Files
- `crates/pdftract-core/src/annotation/other.rs` - Complete rewrite with subtype-specific extraction
- `crates/pdftract-core/src/annotation/mod.rs` - Updated dispatcher to pass resolver

### 3. Test Coverage
Added comprehensive unit tests for:
- Highlight with QuadPoints
- Stamp with /Name "Approved"
- FreeText with /DA
- Text (sticky note) with /Open, /State, /StateModel
- Ink with multiple strokes
- Line with endpoints
- Polygon/PolyLine with vertices
- FileAttachment with /FS reference
- Circle, Square (Other type)
- Unknown subtypes
- Edge cases (no quads, no name, invalid arrays)

## Acceptance Criteria Status

- [PASS] Critical test: page with Highlight and Note - both extract with correct subtypes
- [PASS] Critical test: annotation with no /Contents -> contents: None
- [PASS] Unit tests: Highlight with QuadPoints
- [PASS] Unit tests: Stamp with /Name "Approved"
- [PASS] Unit tests: FreeText with /DA
- [PASS] Unit tests: Ink with multiple strokes
- [PASS] Public extract_annotation(AnnotationCommon, dict, resolver) -> Annotation
- [PASS] INV: subtype taxonomy stable (all subtypes preserved as-is)

## Compilation Status
- [PASS] cargo check --all-targets
- [PASS] cargo fmt
- [WARN] cargo clippy has pre-existing warnings in other modules (not introduced by this change)

## Notes
- Preserved original /Subtype name casing (do not normalize to lowercase per spec)
- /QuadPoints format is (x1,y1, x2,y2, x3,y3, x4,y4) per quad in reading order
- Color array length varies (1, 3, or 4) and is preserved as-is
- Unknown subtypes emit with AnnotationSpecific::Other (no diagnostic in current implementation)

## Related Files
- crates/pdftract-core/src/annotation/other.rs
- crates/pdftract-core/src/annotation/mod.rs
- crates/pdftract-core/src/content_stream.rs (fixed pre-existing borrow issue)
