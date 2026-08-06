# bf-39zru7: Add Type3Error::InvalidCharProcType variant

## Summary
Added new error variant to Type3Error enum for invalid char_proc object types.

## Changes Made

### File: crates/pdftract-core/src/font/type3_rasterizer.rs

1. Added `InvalidCharProcType` variant to Type3Error enum (lines 46-52):
   - `got: String` - the actual object type found
   - `expected: String` - description of expected types

2. Updated Display implementation (lines 65-67):
   - Message format: "invalid char_proc type: got {got}, expected {expected}"

3. Added test coverage (lines 1773-1785):
   - `test_type3_error_invalid_char_proc_type` - verifies Display output

## Acceptance Criteria

- ✅ Type3Error enum has InvalidCharProcType variant with got and expected fields
- ✅ Variant is properly integrated into the error type
- ✅ Display impl shows clear message about what was found vs expected
- ✅ All tests compile and pass

## Example Usage

```rust
let error = Type3Error::InvalidCharProcType {
    got: "integer".to_string(),
    expected: "stream or dict".to_string(),
};

// Display output: "invalid char_proc type: got integer, expected stream or dict"
```

## Verification

- Compiled successfully: `cargo check --lib -p pdftract-core`
- Test passes: `test_type3_error_invalid_char_proc_type`
- All Type3Error tests pass: 6 tests in the error test suite

## References

- Plan: lines 3800-3850 (Type3 font error handling)
- ADR-12: Error type design patterns
