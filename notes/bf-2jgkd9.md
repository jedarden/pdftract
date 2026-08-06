# Verification: bf-2jgkd9 - Add validation for invalid char_proc object types

## Task Summary
Add validation to check that a char_proc reference points to a valid object structure.

## Implementation Status: COMPLETE ✓

### 1. Error variant (InvalidCharProcType) ✓
**Location**: `crates/pdftract-core/src/font/type3_rasterizer.rs:44-49`

```rust
InvalidCharProcType {
    got: String,
    expected: String,
}
```

### 2. Display implementation ✓
**Location**: `crates/pdftract-core/src/font/type3_rasterizer.rs:63-64`

```rust
Type3Error::InvalidCharProcType { got, expected } => {
    write!(f, "invalid char_proc object type: got '{}', expected '{}'", got, expected)
}
```

### 3. Validation logic ✓
**Location**: `crates/pdftract-core/src/font/type3_rasterizer.rs:944-1017`

The `extract_content_stream_bytes` function validates object types before parsing:

**Valid types**:
- `PdfObject::Stream` → Decodes and returns stream bytes
- `PdfObject::Ref` → Recursively resolves and extracts

**Invalid types** (all return `InvalidCharProcType` error):
- `PdfObject::Null` → got: "Null", expected: "Stream or Ref"
- `PdfObject::Bool(_)` → got: "Boolean", expected: "Stream or Ref"
- `PdfObject::Integer(_)` → got: "Integer", expected: "Stream or Ref"
- `PdfObject::Real(_)` → got: "Real", expected: "Stream or Ref"
- `PdfObject::String(_)` → got: "String", expected: "Stream or Ref"
- `PdfObject::Name(_)` → got: "Name", expected: "Stream or Ref"
- `PdfObject::Array(_)` → got: "Array", expected: "Stream or Ref"
- `PdfObject::Dict(_)` → got: "Dictionary", expected: "Stream or Ref"
- `PdfObject::Indirect(_)` → got: "Indirect", expected: "Stream or Ref"

### 4. Validation timing ✓
Validation runs **before** attempting to parse content stream (first match statement in `extract_content_stream_bytes`).

### 5. Error messages ✓
Error messages clearly indicate:
- What type was found (e.g., "Integer", "Dictionary")
- What was expected (e.g., "Stream or Ref")
- Example: "invalid char_proc object type: got 'Integer', expected 'Stream or Ref'"

## Test Coverage ✓

**Location**: `crates/pdftract-core/src/font/type3_rasterizer.rs:1582-2434`

Comprehensive tests include:
- `test_invalid_char_proc_type_error` (lines 1582-1594)
- `test_extract_content_stream_bytes_validates_object_types` (lines 1597-1640)
- `test_extract_content_stream_returns_invalid_type_error_for_dict` (lines 2322-2341)
- `test_extract_content_stream_returns_invalid_type_error_for_array` (lines 2343-2372)
- `test_extract_content_stream_returns_invalid_type_error_for_integer` (lines 2374-2403)
- `test_extract_content_stream_returns_invalid_type_error_for_null` (lines 2405-2434)

All tests verify:
1. The correct error variant is returned
2. The `got` field matches the actual type
3. The `expected` field is "Stream or Ref"

## Acceptance Criteria

All acceptance criteria met:

1. ✅ **Validation checks object type** - Implemented for 9 invalid types
2. ✅ **Error variant for invalid type** - `InvalidCharProcType { got, expected }`
3. ✅ **Validation runs before parsing** - First match in `extract_content_stream_bytes`
4. ✅ **Clear error messages** - Display impl shows got/expected types

## Conclusion

The task has been fully implemented. The validation system ensures that char_proc references are properly validated before attempting to parse their content streams, providing clear error messages when invalid object types are encountered.
