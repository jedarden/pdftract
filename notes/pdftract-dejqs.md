# pdftract-dejqs: Per-page Resource Dictionary Inheritance

## Summary

Implemented per-page Resource dictionary inheritance as specified in PDF 1.7 Section 7.7.3.3. The implementation was already complete in `resources.rs` and `pages.rs`; this task verified the implementation and added missing test coverage for Arc sharing.

## Changes Made

1. **Added Arc sharing test** (`test_resource_inheritance_page_without_resources`):
   - Modified existing test to verify that when multiple pages have no `/Resources`, they share the same `Arc<ResourceDict>` instance
   - Uses `Arc::ptr_eq` to verify pointer equality (memory efficiency)

2. **Added public API exports** in `parser/mod.rs`:
   - `pub use resources::{ResourceDict, merge_resources, extract_resources};`
   - `pub use pages::{PageDict, flatten_page_tree, DEFAULT_MEDIABOX};`

## Acceptance Criteria Status

| Criterion | Status | Test |
|-----------|--------|------|
| 3-level resource inheritance | ✅ PASS | `test_resource_inheritance_three_level` |
| Per-key override (page's /F1 wins) | ✅ PASS | `test_merge_fonts_last_write_wins` |
| Arc sharing when /Resources missing | ✅ PASS | `test_resource_inheritance_page_without_resources` (new `Arc::ptr_eq` check) |
| ColorSpace inline array preserved | ✅ PASS | `test_merge_colorspace_inline_array` |
| Empty root /Resources propagates | ✅ PASS | `test_resource_inheritance_empty_root` |
| INV-8 maintained (no panics) | ✅ PASS | `proptests::fuzz_*` tests verify no panics on arbitrary input |

## Implementation Details

The `merge_resources` function in `resources.rs` implements per-namespace merging:
- **Font namespace**: IndexMap<Arc<str>, ObjRef> - per-key last-write-wins
- **XObject namespace**: IndexMap<Arc<str>, ObjRef>
- **ExtGState namespace**: IndexMap<Arc<str>, ObjRef>
- **ColorSpace namespace**: IndexMap<Arc<str>, PdfObject> - preserves inline arrays
- **Shading namespace**: IndexMap<Arc<str>, ObjRef>
- **Pattern namespace**: IndexMap<Arc<str>, ObjRef>
- **Properties namespace**: IndexMap<Arc<str>, ObjRef>
- **ProcSet**: Vec<Arc<str>> - deprecated, informational only

The `flatten_page_tree` function in `pages.rs` calls `merge_resources` during traversal:
- Ancestor resources are accumulated in `InheritedAttrs.resources`
- Each leaf page merges its own `/Resources` with inherited resources
- When a page has no `/Resources`, it directly clones the Arc (pointer-sharing, not deep copy)

## Files Modified

- `crates/pdftract-core/src/parser/pages.rs`: Enhanced Arc sharing test
- `crates/pdftract-core/src/parser/mod.rs`: Added public API exports
- `notes/pdftract-dejqs.md`: This verification note

## Test Results

All tests pass:
- 9 tests in `parser::resources::tests`
- 24 tests in `parser::pages::tests`
- 4 proptests for INV-8 compliance (no panics on arbitrary input)

## No Breaking Changes

The implementation was already complete and tested. This task only added:
1. One additional test assertion for Arc sharing
2. Public API exports that were previously internal
