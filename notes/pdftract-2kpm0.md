# Verification Note: pdftract-2kpm0

## Summary

Implemented NDJSON frame types with unified `NdjsonFrame` enum using serde internal tagging and `write_frame` helper function.

## Changes Made

### Core Implementation (`crates/pdftract-core/src/output/ndjson/frames.rs`)

- Added `NdjsonFrame` enum with serde internal tagging (`#[serde(tag = "frame", rename_all = "lowercase")]`)
  - `NdjsonFrame::Header(HeaderFrame)`
  - `NdjsonFrame::Page(PageFrame)`
  - `NdjsonFrame::Footer(FooterFrame)`

- Updated frame structs to remove `frame_type` field (now handled by enum tagging):
  - `HeaderFrame`: schema_version, metadata, outline, total_pages
  - `PageFrame`: page_index, page_type, spans, blocks, tables, annotations, errors
  - `FooterFrame`: extraction_quality, errors, threads, attachments, signatures, form_fields, links

- Added `write_frame<W: Write>()` helper function:
  - Serializes frame to JSON
  - Writes trailing newline
  - Flushes writer for immediate delivery to streaming consumers

- Added `#[serde(default)]` to optional fields for proper deserialization:
  - `PageFrame.annotations`, `PageFrame.errors`
  - `FooterFrame.threads`, `FooterFrame.attachments`, `FooterFrame.signatures`, `FooterFrame.form_fields`, `FooterFrame.links`

### Module Exports (`crates/pdftract-core/src/output/ndjson/mod.rs`)

- Updated exports to include `NdjsonFrame` and `write_frame`

### Tests (`crates/pdftract-core/src/output/ndjson/frames.rs`)

- `test_ndjson_frame_header_discriminator`: Verifies "frame":"header" appears first
- `test_ndjson_frame_page_discriminator`: Verifies "frame":"page" appears first
- `test_ndjson_frame_footer_discriminator`: Verifies "frame":"footer" appears first
- `test_write_frame_includes_newline_and_flush`: Verifies write_frame behavior
- `test_roundtrip_header_frame`: Header serialization → deserialization → equality
- `test_roundtrip_page_frame`: Page serialization → deserialization → equality
- `test_roundtrip_footer_frame`: Footer serialization → deserialization → equality
- `test_page_frame_with_empty_collections`: Empty arrays preserved, empty annotations skipped

## Design Decisions

1. **Serde internal tagging**: Used `#[serde(tag = "frame")]` on the enum instead of per-struct fields. This ensures the "frame" key appears first in JSON output and is the standard serde pattern for discriminated unions.

2. **Removed `to_json_line()` methods**: Kept these methods on individual structs for backward compatibility, but the primary API is now `write_frame()` with `NdjsonFrame`.

3. **`#[serde(default)]` on optional fields**: Required for proper roundtrip deserialization since empty collections are skipped during serialization.

## Acceptance Criteria

- [PASS] Roundtrip unit test: write HeaderFrame → parse → equal to original
- [PASS] Frame discriminator order: serialize Page frame → first key is "frame":"page"
- [PASS] Three frames emitted in expected sequence (existing tests verify)
- [PASS] Frame-by-frame writer respects flush after every frame (`write_frame` calls `flush()`)

## Files Modified

- `crates/pdftract-core/src/output/ndjson/frames.rs` - Added NdjsonFrame enum, write_frame helper, updated tests
- `crates/pdftract-core/src/output/ndjson/mod.rs` - Updated exports

## Commit

- `fa57ab3` - feat(pdftract-2kpm0): implement NdjsonFrame enum with internal-tag discriminator and write_frame helper
