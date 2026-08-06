# bf-21hbme: Add error variants for missing char_proc object references

## Summary

Added Type3-specific error handling for missing char_proc object references in Type3 font rasterization.

## Changes Made

### 1. Created Type3Error enum (type3_rasterizer.rs)
Added a new error enum at the module level with three variants:
- `MissingCharProcRef { ref_id: String }` - For when a char_proc reference cannot be found
- `CircularRef { ref_id: String }` - For circular reference detection
- `Io(String)` - For I/O errors during glyph resolution

### 2. Implemented standard traits for Type3Error
- `std::fmt::Display` - Provides user-friendly error messages including the reference ID
- `std::error::Error` - Standard error trait implementation
- `From<ResolveError>` - Automatic conversion from ResolveError to Type3Error

### 3. Updated function signatures
Changed `deref_char_proc_ref` and `extract_content_stream_bytes` to return `Result<T, Type3Error>` instead of `Result<T, ResolveError>`, providing Type3-specific error context.

### 4. Error propagation
Errors propagate correctly through the call stack via the `?` operator, with automatic conversion from ResolveError to Type3Error via the `From` trait implementation.

### 5. Added comprehensive tests
Added 9 new tests covering:
- Type3Error variant creation and Display formatting
- Conversion from ResolveError to Type3Error for all error types
- Error propagation through function calls
- Verification of error messages containing reference IDs

## Acceptance Criteria Status

✅ **PASS**: New error variant in appropriate error enum
- Created `Type3Error::MissingCharProcRef { ref_id: String }`

✅ **PASS**: Function that looks up char_proc_ref returns Result with this error
- `deref_char_proc_ref` now returns `Result<PdfObject, Type3Error>`

✅ **PASS**: Basic error message includes the missing reference ID
- Display implementation formats errors as: "char_proc reference not found: {ref_id}"

✅ **PASS**: Error propagates correctly through call stack
- `From<ResolveError>` trait enables automatic conversion
- `?` operator propagates errors through call stack

## Test Results

All 46 type3_rasterizer tests pass, including 9 new tests for Type3Error:
- `test_type3_error_missing_char_proc_ref` - Verifies error message formatting
- `test_type3_error_circular_ref` - Verifies circular ref error formatting
- `test_type3_error_io` - Verifies IO error formatting
- `test_type3_error_from_resolve_error_not_found` - Verifies NotFound conversion
- `test_type3_error_from_resolve_error_circular_ref` - Verifies CircularRef conversion
- `test_type3_error_from_resolve_error_io` - Verifies Io conversion
- `test_extract_content_stream_bytes_without_resolver_returns_type3_error` - Verifies error propagation
- `test_deref_char_proc_ref_without_context_returns_error` - Updated to use Type3Error
- `test_deref_char_proc_ref_without_resolver_returns_error` - Updated to use Type3Error
- `test_deref_char_proc_ref_without_source_returns_error` - Updated to use Type3Error

Full pdftract-core test suite: PASS (exit code 0)

## Implementation Notes

The implementation follows the pattern used by other font error types in the codebase (Type0Error, CMapError, FontError) with module-specific error enums that capture context-specific information.

The `From<ResolveError>` trait implementation ensures backward compatibility with existing code that uses ResolveError, automatically converting to Type3Error with appropriate context.
