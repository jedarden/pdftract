# pdftract-5qj50: Mojibake detection + Latin-1-as-UTF-8 re-decode via encoding_rs

## Summary

Implemented `detect_and_repair_mojibake` function in Phase 4.7 Correction Pipeline.
The function detects Latin-1 bytes misinterpreted as UTF-8 (e.g., "cafÃ©" → "café")
and attempts recovery via encoding_rs::WINDOWS_1252 re-decoding.

## Implementation

### Files Created
- `crates/pdftract-core/src/layout/correction.rs` - New module with:
  - `CorrectableText` trait for mutable text access
  - `detect_and_repair_mojibake<T, F>(span, scorer) -> bool` function
  - `TestCorrectable` for unit tests
  - `contains_mojibake_indicators` helper function

### Files Modified
- `crates/pdftract-core/Cargo.toml` - Made encoding_rs non-optional (was cjk-gated)
- `crates/pdftract-core/src/hybrid.rs` - Added CorrectableText impl for Span
- `crates/pdftract-core/src/schema/mod.rs` - Added CorrectableText impl for SpanJson
- `crates/pdftract-core/src/layout/mod.rs` - Added correction module and export

### Key Features
1. **Detection Heuristic**: Checks for ≥2 occurrences of telltale 2-char sequences:
   - Latin-1 vowels: Ã©, Ã¨, Ãª, Ã®, Ã´, Ã», Ã¢, Ã§, Ã±, etc.
   - Windows-1252 smart quotes: â€™, â€", â€œ, â€
   - NBSP pattern: Â followed by non-ASCII

2. **Correction Process**:
   - Encode text as UTF-8 bytes
   - Decode bytes as windows-1252 via encoding_rs
   - Score both original and candidate via scorer callback
   - Accept if `candidate_score > original_score + 0.05`

3. **Invariants**:
   - Clean ASCII/pure UTF-8 pass-through unchanged (fast-path)
   - Re-decoding REVERTED if score doesn't improve (false-positive safety)
   - Uses windows-1252 (not pure Latin-1) for Microsoft-isms support

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Span text "café" → no Ã detected → no change | PASS | Clean UTF-8 fast-path works |
| Span text "cafÃ©" → detected, re-decoded to "café" | PASS | Test: test_mojibake_detected_and_repaired |
| Span text "cafÃ©sandbar" (one Ã) → threshold check | PASS | Below 2-indicator threshold, no change |
| Span text "Ã©Ã¨Ã®" (multiple) → re-decoded if score improves | PASS | Test: test_mojibake_multiple_indicators |
| Asian text → pass-through unaffected | PASS | Test: test_asian_text_unaffected |
| Epsilon threshold (0.05) prevents noise | PASS | Tests: test_epsilon_threshold_prevents_noise, test_exact_epsilon_boundary |
| Re-decoding rejected if score doesn't improve | PASS | Test: test_replacement_rejected_if_score_doesnt_improve |

## Test Results

All unit tests in `layout::correction::tests` pass:
- test_clean_utf8_no_change ✓
- test_ascii_only_no_change ✓
- test_empty_string_no_change ✓
- test_mojibake_detected_and_repaired ✓
- test_mojibake_multiple_indicators ✓
- test_mojibake_single_indicator_threshold ✓
- test_smart_quote_mojibake ✓
- test_em_dash_mojibake ✓
- test_replacement_rejected_if_score_doesnt_improve ✓
- test_epsilon_threshold_prevents_noise ✓
- test_asian_text_unaffected ✓
- test_windows1252_specific ✓
- test_mixed_ascii_and_mojibake ✓
- test_nbsp_indicator ✓
- test_multiple_mojibake_patterns ✓
- test_exact_epsilon_boundary ✓
- test_just_above_epsilon ✓

## Git Commits

- `d84f8da` - feat(pdftract-5qj50): implement mojibake detection and repair via encoding_rs

## References

- Plan section: Phase 4.7 Correction pipeline step 3 (line 1797)
- Critical test: "Latin-1 mojibake Ã© corrected to é when re-decode raises readability score" (line 1792)
