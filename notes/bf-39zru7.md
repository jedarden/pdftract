# Verification Note: bf-39zru7 - Add Type3Error::InvalidCharProcType variant

## Summary
Added a new error variant `InvalidCharProcType` to the `Type3Error` enum to represent invalid char_proc object types.

## Changes Made

### File: `crates/pdftract-core/src/font/type3_rasterizer.rs`

1. **Added variant to Type3Error enum** (line 47-52):
   ```rust
   InvalidCharProcType {
       got: String,
       expected: String,
   }
   ```

2. **Updated Display implementation** (line 63):
   ```rust
   Type3Error::InvalidCharProcType { got, expected } => {
       write!(f, "invalid char_proc type: got {}, expected {}", got, expected)
   }
   ```

3. **Added test** (line 1760-1770):
   - `test_type3_error_invalid_char_proc_type()` - verifies Display impl shows correct message

## Acceptance Criteria

✅ **PASS**: Type3Error enum has InvalidCharProcType variant with got and expected fields
✅ **PASS**: Variant is properly integrated into the error type
✅ **PASS**: Display impl shows clear message about what was found vs expected

## Test Results

Ran all Type3 error tests:
```bash
$ cargo test --package pdftract-core --lib font::type3_rasterizer::tests::test_type3_error
test result: ok. 7 passed; 0 failed; 0 ignored
```

All 7 Type3 error tests pass, including the new test for InvalidCharProcType.

## Verification

The new variant:
- Is structurally identical to existing error variants in the enum
- Follows the same naming and formatting conventions
- Provides clear, actionable error messages through the Display impl
- Can be used to distinguish type validation errors from other Type3 rasterization errors

## Related Files
- Plan: lines 3800-3850 (Type3 font error handling)
- ADR-12: Error type design patterns

## Git Diff Summary
- Added 1 enum variant (6 lines)
- Updated Display impl (4 lines)
- Added 1 test (11 lines)
