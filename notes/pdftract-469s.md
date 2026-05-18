# pdftract-469s: Implement direct object parser

## Summary

This bead implements the core `ObjectParser::parse_direct_object()` method that handles all PDF direct object variants. The implementation was already present in the codebase; this bead added missing test coverage to ensure correctness.

## Work Done

### 1. Added New Tests

#### Critical Tests from Plan
- **4-level nested dict test** (`test_parse_4_level_nested_dict`): Verifies `<< /A << /B << /C << /D 1 >> >> >> >>` parses correctly with proper nesting
- **Array of mixed types test** (`test_parse_array_5_elements_mixed_types`): Verifies `[1 true (str) /Name null]` produces correct 5-element array
- **Indirect reference test** (`test_parse_indirect_ref`): Already existed, verifies `5 0 R` -> `PdfObject::Ref(ObjRef{5, 0})`

#### Edge Case Tests
- **Depth limit test** (`test_depth_exceeded_at_256`): Verifies that 300-level nested dict triggers `STRUCT_DEPTH_EXCEEDED` at depth 256, returning `PdfNull` at that level
- **Truncated dict test** (`test_truncated_dict_at_eof`): Verifies `<< /Type /Catalog /Pages` (EOF after key) produces partial dict with 2 keys and diagnostic
- **Negative indirect ref test** (`test_negative_indirect_ref`): Verifies invalid negative object numbers are handled

#### Property-Based Tests
- **proptest_random_tokens_no_panic**: Random PDF token sequences never panic (INV-8)
- **proptest_random_bytes_no_panic**: Random byte sequences never panic (INV-8)

### 2. Files Modified

- `crates/pdftract-core/src/parser/object/parser.rs`: Added 5 new tests and 2 proptest tests

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| All direct object variants parse correctly | PASS | Implementation already complete in parser.rs |
| Nested dict 4 levels deep -> correct tree | PASS | test_parse_4_level_nested_dict |
| Array of mixed types -> correct 5 elements | PASS | test_parse_array_5_elements_mixed_types |
| `5 0 R` -> PdfObject::Ref(ObjRef{5, 0}) | PASS | test_parse_indirect_ref (pre-existing) |
| Truncated dict at EOF -> partial dict + diagnostic | PASS | test_truncated_dict_at_eof |
| Depth-300 nested dict -> STRUCT_DEPTH_EXCEEDED | PASS | test_depth_exceeded_at_256 |
| proptest: random tokens never panic | PASS | proptest_random_tokens_no_panic |
| INV-8 maintained | PASS | All error paths use diagnostics, no panics |

## Test Results

```
cargo test --lib -p pdftract-core -- parser::object
test result: ok. 49 passed; 0 failed
```

All tests pass, including:
- 25 parser tests
- 24 type tests
- 2 proptest tests

## Implementation Notes

The core parser implementation was already complete in `parser.rs`:
- `parse_direct_object()` handles all token types
- `parse_integer_or_ref()` implements 3-token lookahead for indirect references
- `parse_array()` handles recursive array parsing with depth limit
- `parse_dict()` handles dictionary parsing with alternating key-value pairs
- Stream detection and body skipping implemented in `parse_dict()`
- Depth limit of 256 enforced via `MAX_DEPTH` constant

## References

- Plan section: Phase 1.2 lines 1057-1068
- INV-8: No panics at public boundaries
- Files modified:
  - crates/pdftract-core/src/parser/object/parser.rs
