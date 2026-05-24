# pdftract-5rmc: encoding_rs adapter for CJK encodings

## Summary

Implemented a thin wrapper around `encoding_rs` for decoding the four major CJK byte encodings used in legacy PDFs: Shift-JIS, GB18030, Big5, and EUC-KR.

## Implementation

### Files Modified
- `crates/pdftract-core/src/font/cjk_encoding.rs` (NEW) - CJK encoding adapter

### API
```rust
pub enum CjkEncoding {
    ShiftJis,  // Japanese (JIS X 0208)
    Gb18030,   // Chinese (PRC standard)
    Big5,      // Traditional Chinese (with Big5-HKSCS extension)
    EucKr,     // Korean (KS X 1001 + Unified Hangul)
}

pub fn decode_cjk_bytes(enc: CjkEncoding, bytes: &[u8]) -> (String, bool)
// Returns (decoded_text, had_malformed_bytes)
```

### Design Decisions
1. **Uses `decode_without_bom_handling`**: PDF byte streams never have a BOM
2. **Returns malformed indicator**: Caller decides whether to emit `CJK_DECODE_MALFORMED` diagnostic
3. **Feature-gated**: `#[cfg(feature = "cjk")]` alongside predefined CMap registry
4. **encoding_rs singletons**: Uses `SHIFT_JIS`, `GB18030`, `BIG5`, `EUC_KR` directly

### Supporting Changes (Already in Place)
- `encoding_rs` dependency added to `Cargo.toml` (optional, enabled by `cjk` feature)
- `CjkDecodeMalformed` diagnostic code added to `diagnostics.rs`
- Module exports added to `font/mod.rs`

## Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| All 4 encodings decode known sample byte sequences | PASS | Tests use encoding_rs to verify correct encoding |
| Malformed input produces U+FFFD chars | PASS | encoding_rs replaces malformed sequences automatically |
| Diagnostic emission capability | PASS | `bool` return value indicates malformed; caller emits diagnostic |
| Empty input returns empty string | PASS | Explicit check at start of `decode_cjk_bytes` |
| No panic on any input | PASS | `test_malformed_no_panic` verifies various malformed inputs |
| API is `cfg(feature = "cjk")`-gated | PASS | Module and exports gated behind `cjk` feature |
| Round-trip tests (encode → decode → verify) | PASS | All 4 encodings round-trip correctly |

## Test Results
```
running 15 tests
test font::cjk_encoding::tests::test_decode_ascii_passthrough ... ok
test font::cjk_encoding::tests::test_big5_hkscs_extension ... ok
test font::cjk_encoding::tests::test_decode_empty_input ... ok
test font::cjk_encoding::tests::test_decode_big5_valid ... ok
test font::cjk_encoding::tests::test_decode_euc_kr_valid ... ok
test font::cjk_encoding::tests::test_decode_malformed_gb18030 ... ok
test font::cjk_encoding::tests::test_decode_gb18030_valid ... ok
test font::cjk_encoding::tests::test_decode_malformed_shift_jis ... ok
test font::cjk_encoding::tests::test_decode_shift_jis_valid ... ok
test font::cjk_encoding::tests::test_encoding_names ... ok
test font::cjk_encoding::tests::test_round_trip_big5 ... ok
test font::cjk_encoding::tests::test_malformed_no_panic ... ok
test font::cjk_encoding::tests::test_round_trip_euc_kr ... ok
test font::cjk_encoding::tests::test_round_trip_shift_jis ... ok
test font::cjk_encoding::tests::test_round_trip_gb18030 ... ok

test result: ok. 15 passed; 0 failed; 0 ignored
```

## Notes
- encoding_rs is the gold-standard Rust implementation (powers Firefox)
- Big5 implementation includes Big5-HKSCS extension for Hong Kong-specific characters
- GB18030 is 1-2-4 byte variable-width; encoding_rs handles this correctly
- EUC-KR covers KS X 1001 + Unified Hangul
- Fallback path only fires when: raw encoding name OR unrecognized CMap + CJK lead byte range

## References
- Plan section: Phase 2.3 (lines 1382-1386)
- encoding_rs crate: https://docs.rs/encoding_rs/
