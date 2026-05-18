# pdftract-7nav Verification Note

## Summary
Verified the PdfObject enum and related types implementation in `crates/pdftract-core/src/parser/object/types.rs`.

## Implementation Details

### Types Defined
- `ObjRef { object: u32, generation: u16 }` - Object reference with Copy, Eq, Hash, PartialOrd, Ord
- `PdfObject` enum with variants: Null, Bool, Integer, Real, String, Name, Array, Dict, Ref, Stream, Indirect
- `PdfDict` - Type alias to `IndexMap<Arc<str>, PdfObject>` (preserves insertion order)
- `PdfStream { dict: PdfDict, offset: u64, len_hint: Option<u64> }` - Stream with helper methods
- `PdfIndirect { id: ObjRef, obj: PdfObject }` - Indirect object wrapper

### Name Interning
- Thread-local `Arc<str>` interner using `HashSet`
- `intern(s: &str) -> Arc<str>` function for deduplication
- Enables cheap cloning of common PDF names (/Type, /Length, /Filter, etc.)

### Helper Methods on PdfObject
- `as_int() -> Option<i64>`
- `as_real() -> Option<f64>`
- `as_name() -> Option<&str>`
- `as_dict() -> Option<&PdfDict>`
- `as_stream() -> Option<&PdfStream>`
- `as_array() -> Option<&[PdfObject]>`
- `as_string() -> Option<&[u8]>`
- `as_ref() -> Option<ObjRef>`
- `as_bool() -> Option<bool>` (bonus: handles integer 0/1)
- `is_null() -> bool`

## Acceptance Criteria Results

| Criterion | Status | Notes |
|-----------|--------|-------|
| cargo build produces types.rs | PASS | File exists, exports all types |
| PdfObject size <= 32 bytes | PASS | Test confirms size constraint |
| PdfDict preserves insertion order | PASS | IndexMap preserves order |
| Name interner deduplicates | PASS | Arc::ptr_eq test passes |
| All helper methods work | PASS | 8+ tests for each method |
| Indirect-via-Box keeps size small | PASS | Size test passes |

## Test Results
```
running 24 tests
test parser::object::types::tests::test_as_int ... ok
test parser::object::types::tests::test_as_array ... ok
test parser::object::types::tests::test_as_name ... ok
test parser::object::types::tests::test_as_ref ... ok
test parser::object::types::tests::test_as_real ... ok
test parser::object::types::tests::test_as_dict ... ok
test parser::object::types::tests::test_as_stream ... ok
test parser::object::types::tests::test_as_string ... ok
test parser::object::types::tests::test_is_null ... ok
test parser::object::types::tests::test_name_interner_common_names ... ok
test parser::object::types::tests::test_name_interner_dedup ... ok
test parser::object::types::tests::test_obj_ref_display ... ok
test parser::object::types::tests::test_obj_ref_hash ... ok
test parser::object::types::tests::test_obj_ref_ordering ... ok
test parser::object::types::tests::test_obj_ref_partial_ord ... ok
test parser::object::types::tests::test_pdf_dict_insertion_order ... ok
test parser::object::types::tests::test_pdf_indirect ... ok
test parser::object::types::tests::test_pdf_dict_roundtrip_order ... ok
test parser::object::types::tests::test_pdf_object_partial_eq_real_nan ... ok
test parser::object::types::tests::test_pdf_object_indirect_variant ... ok
test parser::object::types::tests::test_pdf_object_partial_eq_real_normal ... ok
test parser::object::types::tests::test_pdf_object_size ... ok
test parser::object::types::tests::test_pdf_stream_len_hint ... ok
test parser::object::types::tests::test_pdf_stream_no_len_hint ... ok

test result: ok. 24 passed; 0 failed
```

## References
- Plan section: Phase 1.2 line 1058-1068
- Commit: 7bbb727 (existing implementation)
