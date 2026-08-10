# bf-64vxqk: Remove Unused Imports from parser/stream.rs

## Summary
Removed 5 unused imports from `crates/pdftract-core/src/parser/stream.rs` after verifying each was not used in the codebase.

## Changes Made
1. **Line 3242**: Removed `use secrecy::SecretString;` from `impl<'de> serde::Deserialize<'de> for ExtractionOptions` deserializer function
2. **Line 3996**: Removed `use secrecy::ExposeSecret;` from `integration_tests` test module
3. **Line 5115**: Removed `use secrecy::SecretString;` from `test_extraction_options_deserialize_password` test function
4. **Line 5149**: Removed `use secrecy::SecretString;` from `test_extraction_options_serialize_password_redacted` test function
5. **Line 6139**: Removed `Jbig2GlobalsRef` from jbig2 import in `test_jbig2_extract_globals_ref` test function (changed from `use crate::decoder::jbig2::{Jbig2Decoder, Jbig2GlobalsRef};` to `use crate::decoder::jbig2::Jbig2Decoder;`)

## Verification
- Ran `cargo check --tests -p pdftract-core` - **PASSED** (exit code 0)
- All imports verified as truly unused before removal
- One `ExposeSecret` import was intentionally kept in `predictor_tests` module (line 4647) because it is used at line 5126 in the `test_extraction_options_deserialize_password` test

## Acceptance Criteria Status
- [x] All verified unused imports removed from parser/stream.rs
- [x] `cargo check --tests -p pdftract-core` passes
- [x] No legitimate uses deleted
- [x] Note: Found and removed 5 unused imports (task mentioned 4, but inventory was incomplete)

## References
- Task: bf-64vxqk
- Inventory reference: notes/bf-1v4l0i-unused-imports.txt
- Plan reference: docs/plan/plan.md
