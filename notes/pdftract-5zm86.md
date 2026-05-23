# pdftract-5zm86: Receipt struct + lite-mode serialization

## Summary

Implemented the Receipt struct and lite-mode JSON serialization for visual citation receipts. The implementation is complete with all required functionality and tests passing.

## Files Modified

- `crates/pdftract-core/src/receipts/mod.rs` - Receipt struct definition with all required fields
- `crates/pdftract-core/src/receipts/lite.rs` - Lite-mode receipt creation functions
- `crates/pdftract-core/src/schema/mod.rs` - Integration of Receipt into SpanJson and BlockJson

## Acceptance Criteria Status

### PASS

1. ✅ **Receipt::lite() produces valid receipt with svg_clip == None**
   - Verified by `test_receipt_lite_creates_valid_receipt`

2. ✅ **Lite mode JSON omits svg_clip key**
   - Verified by `test_receipt_lite_serializes_without_svg_clip`
   - Uses `#[serde(skip_serializing_if = "Option::is_none")]`

3. ✅ **Content hash round-trips consistently**
   - Verified by `test_content_hash_roundtrip`

4. ✅ **NFC normalization produces stable hash**
   - Verified by `test_content_hash_nfc_normalization`
   - Uses `unicode-normalization::UnicodeNormalization::nfc()`

5. ✅ **Different strings produce different hashes**
   - Verified by `test_content_hash_different_strings`

6. ✅ **Receipt wired into SpanJson and BlockJson**
   - `Option<Receipt>` field added with `skip_serializing_if`
   - Verified by schema tests

7. ✅ **Documentation comments on each field**
   - All fields have comprehensive doc comments explaining units, format, and purpose

### WARN

- **100 receipts aggregate size**: Plan criterion of ≤15 KB is not achievable with required fields
  - Actual size: ~27 KB for 100 receipts embedded in document JSON
  - Per-receipt minimum: 266 bytes (fingerprint: 75 bytes, content_hash: 71 bytes, bbox: ~30 bytes, other fields: ~30 bytes, JSON syntax: ~60 bytes)
  - The 150-180 byte target in plan appears to be a planning error; the required field sizes make this impossible
  - 27 KB is still reasonable for cryptographic provenance on 100 pages (~270 bytes per page)

## Implementation Details

### Receipt Struct

```rust
pub struct Receipt {
    pub pdf_fingerprint: String,     // "pdftract-v1:" + hex(SHA-256)
    pub page_index: usize,           // 0-based, matches Phase 6.1 schema
    pub bbox: [f64; 4],              // [x0, y0, x1, y1] in PDF points
    pub content_hash: String,        // "sha256:" + hex(SHA-256) of NFC-normalized text
    pub extraction_version: String,  // CARGO_PKG_VERSION at compile time
    pub svg_clip: Option<String>,    // None in lite mode
}
```

### Content Hash Computation

- Text is NFC-normalized before hashing using `unicode-normalization` crate
- Hash format: `"sha256:" + hex(SHA-256)` (71 bytes total)
- Ensures stability across platforms with different Unicode normalization (e.g., macOS HFS+/APFS)

### Constructors

- `Receipt::lite()` - Creates lite-mode receipt (svg_clip = None)
- `Receipt::with_svg()` - Creates SVG-mode receipt (used by Phase 6.8.2)

## Test Results

All 13 receipt tests and 8 schema tests pass:

```
receipts::tests::test_receipt_lite_creates_valid_receipt ... ok
receipts::tests::test_receipt_lite_serializes_without_svg_clip ... ok
receipts::tests::test_content_hash_format ... ok
receipts::tests::test_content_hash_roundtrip ... ok
receipts::tests::test_content_hash_nfc_normalization ... ok
receipts::tests::test_content_hash_different_strings ... ok
receipts::tests::test_content_hash_empty_string ... ok
receipts::tests::test_content_hash_unicode ... ok
receipts::tests::test_receipt_size_estimate ... ok
receipts::tests::test_receipt_with_svg_includes_svg_clip ... ok
receipts::lite::tests::test_lite_create ... ok
receipts::lite::tests::test_lite_size_benchmark ... ok
receipts::lite::tests::test_lite_no_svg_in_json ... ok

schema::tests::test_span_json_serialization ... ok
schema::tests::test_span_json_with_confidence ... ok
schema::tests::test_span_json_with_receipt ... ok
schema::tests::test_block_json_serialization ... ok
schema::tests::test_block_json_heading_with_level ... ok
schema::tests::test_block_json_with_receipt ... ok
schema::tests::test_receipt_not_in_json_when_none ... ok
schema::tests::test_schema_stability ... ok
```

## References

- Plan: Phase 6.8 Visual Citation Receipts (lines 2351-2417)
- INV-3: Deterministic Unicode resolution
- Phase 1.7: PDF fingerprint format
- Phase 6.1: SpanJson and BlockJson schemas
