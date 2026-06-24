# bf-3ourh: CJK Test Fixtures for Phase 2.3 Encoding Gate

## Summary

Verified and updated CJK encoding test fixtures for Phase 2.3. The fixtures directory `tests/fixtures/cjk/` contains four PDF files with corresponding ground truth files covering all required CJK encodings.

## Current Fixture State (2026-06-24)

All fixtures are minimal PDFs with Type0 composite fonts:

```
tests/fixtures/cjk/
├── cjk-chinese-gb18030.pdf (822 bytes) + .txt ground truth (12 bytes)
├── cjk-japanese-shiftjis.pdf (822 bytes) + .txt ground truth (15 bytes)
├── cjk-korean-euckr.pdf (826 bytes) + .txt ground truth (15 bytes)
└── cjk-tc-big5.pdf (814 bytes) + .txt ground truth (12 bytes)
```

### Coverage Verification

| Fixture | Encoding | Ground Truth | Status |
|---------|----------|--------------|--------|
| cjk-chinese-gb18030.pdf | GB18030 (GBpc-EUC-H CMap) | "你好世界" | ✅ |
| cjk-japanese-shiftjis.pdf | Shift-JIS (90ms-RKSJ-H CMap) | "こんにちは" | ✅ |
| cjk-korean-euckr.pdf | EUC-KR (KSCms-UHC-H CMap) | "안녕하세요" | ✅ |
| cjk-tc-big5.pdf | Big5 (ETen-B5-H CMap) | "你好世界" | ✅ |

## Test Coverage

All four fixtures have extraction tests in two locations:

1. **crates/pdftract-core/tests/cjk_encoding.rs** — Dedicated CJK encoding tests
   - `test_cjk_gb18030_chinese()`
   - `test_cjk_shiftjis_japanese()`
   - `test_cjk_euckr_korean()`
   - `test_cjk_big5_traditional_chinese()`
   - `test_all_cjk_fixtures_exist()`

2. **tests/test_encoding.rs** — Encoding recovery suite
   - `test_cjk_chinese_gb18030()` (line 276)
   - `test_cjk_japanese_shiftjis()` (line 293)
   - `test_cjk_korean_euckr()` (line 310)
   - `test_cjk_tc_big5()` (line 327)

Each test verifies ≥90% recovery rate per Phase 2 exit gate requirements.

## Generator Script

Fixtures can be regenerated using:
```bash
cargo run --bin generate_cjk_valid
```

Generator script: `tests/fixtures/generate_cjk_valid.rs`

## Test Status

- ✅ Fixtures exist and are valid PDFs
- ✅ Ground truth files contain correct Unicode text
- ✅ Tests are properly implemented and runnable
- ❌ Tests currently FAIL (expected - CJK encoding implementation is Phase 2.3, separate from fixture creation)

Test failure output shows empty extraction:
```
assertion `left == right` failed: GB18030 extracted text should match ground truth
  left: ""
 right: "你好世界"
```

This is expected behavior until CJK encoding support is implemented.

## Acceptance Criteria Status

- ✅ `tests/fixtures/cjk/` exists
- ✅ Contains at least 4 PDFs covering GB18030, Shift-JIS, EUC-KR, Big5
- ⚠️ "Extraction tests pass on all four fixtures" — Tests exist but fail because CJK encoding support (Phase 2.3) hasn't been implemented yet

## Changes Made

- Simplified ground truth files to single-line entries (removed extra test text)
- Regenerated fixtures with `generate_cjk_valid.rs` to ensure consistency
- Verified all PDFs have valid structure (%PDF-1.4 headers)
- Updated verification note with current fixture state

## Next Steps

The fixtures and tests are ready for CJK encoding implementation (Phase 2.3). Once encoding support is added, these tests will verify correct CJK text extraction.
