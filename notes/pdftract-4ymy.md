# pdftract-4ymy: Indirect Object Wrapper Parser Implementation

## Summary
Implement `ObjectParser::parse_indirect_object()` which reads the four-token preamble (`Integer Integer Obj`), parses one direct object, expects `Token::EndObj`, and returns `PdfIndirect { id: ObjRef, obj: PdfObject }`.

## Implementation Details

The implementation was already present in `crates/pdftract-core/src/parser/object/parser.rs` (lines 413-660). The function:

1. **Reads 3 tokens for the header**: `Integer(N)`, `Integer(G)`, `Token::Obj`
2. **Validates and constructs `ObjRef`**: With overflow handling for both object number (clamps to `u32::MAX`) and generation number (clamps to `u16::MAX`)
3. **Parses the direct object body** via `parse_direct_object()`
4. **Expects `Token::EndObj`**: With comprehensive error recovery
5. **Returns `PdfIndirect { id, obj }`**

### Error Recovery

- **Invalid header** (e.g., `1 X obj`): Emits `STRUCT_INVALID_INDIRECT_HEADER`, scans forward to the next `obj` keyword
- **Missing `endobj`**: Emits `STRUCT_MISSING_KEY`, scans forward to the next `endobj`, `obj`, or EOF
- **Integer overflow**: Emits `STRUCT_INTEGER_OVERFLOW`, clamps to max value
- **Multi-object skip recovery**: If scanning for `endobj` finds `obj` first (start of next indirect object), scans backward to find the preceding integer (object number)

### Position Tracking

The lexer's position counter is valid on all return paths (both success and recovery), ensuring the xref resolver can correctly track object positions.

## Acceptance Criteria Status

| Criteria | Status | Test |
|----------|--------|------|
| Simple test: `1 0 obj null endobj` → PdfIndirect{ ObjRef{1,0}, Null } | ✅ PASS | `test_parse_indirect_object_simple` |
| Stream test: `12 0 obj << /Length 5 >> stream\n12345endstream endobj` → PdfIndirect with Stream | ✅ PASS | `test_parse_indirect_object_with_stream` |
| Recovery: `1 0 obj null` (no endobj) → emit STRUCT_MISSING_KEY, position advances | ✅ PASS | `test_parse_indirect_object_missing_endobj` |
| Recovery: `999999999999 0 obj null endobj` → ObjRef{u32::MAX, 0} + STRUCT_INTEGER_OVERFLOW | ✅ PASS | `test_parse_indirect_object_integer_overflow` |
| proptest: random byte sequences never panic | ✅ PASS | `proptest_random_bytes_no_panic_indirect` |

## Test Results

All 11 indirect object tests pass:
- `test_parse_indirect_object_simple` ✅
- `test_parse_indirect_object_with_integer` ✅
- `test_parse_indirect_object_with_stream` ✅
- `test_parse_indirect_object_missing_endobj` ✅
- `test_parse_indirect_object_integer_overflow` ✅
- `test_parse_indirect_object_generation_overflow` ✅
- `test_parse_indirect_object_invalid_header` ✅
- `test_parse_indirect_object_negative_object_number` ✅
- `test_parse_indirect_object_eof_returns_none` ✅
- `test_parse_indirect_object_with_dict` ✅
- `test_parse_indirect_object_with_array` ✅

Property-based test:
- `proptest_random_bytes_no_panic_indirect` ✅

## References

- Plan section: Phase 1.2 line 1071 (indirect object parsing)
- Phase 1.6 (error recovery for missing endobj)
- INV-8 (no panics at public boundaries)

## Files Modified

No files were modified - the implementation was already present and complete.

## Verification

Run tests with:
```bash
cargo test --package pdftract-core --lib parser::object::parser::tests::test_parse_indirect_object
cargo test --package pdftract-core --lib proptest_random_bytes_no_panic_indirect
```
