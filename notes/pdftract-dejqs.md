# pdftract-dejqs: Per-page Resource Dictionary Inheritance

## Summary

Verified that the per-page Resource dictionary inheritance implementation is complete and correct. The implementation was already present in `crates/pdftract-core/src/parser/resources.rs` and integrated into the page tree flattening in `crates/pdftract-core/src/parser/pages.rs`.

## Implementation Details

### ResourceDict Structure (`crates/pdftract-core/src/parser/resources.rs`)

The `ResourceDict` struct contains all resource namespaces:
- `fonts: IndexMap<Arc<str>, ObjRef>` — /Font namespace
- `xobjects: IndexMap<Arc<str>, ObjRef>` — /XObject namespace
- `ext_gstates: IndexMap<Arc<str>, ObjRef>` — /ExtGState namespace
- `color_spaces: IndexMap<Arc<str>, PdfObject>` — /ColorSpace namespace (supports inline arrays)
- `shadings: IndexMap<Arc<str>, ObjRef>` — /Shading namespace
- `patterns: IndexMap<Arc<str>, ObjRef>` — /Pattern namespace
- `properties: IndexMap<Arc<str>, ObjRef>` — /Properties namespace
- `proc_set: Vec<Arc<str>>` — /ProcSet (deprecated, informational only)

### merge_resources Function

The `merge_resources(ancestor: &ResourceDict, child: &PdfObject) -> ResourceDict` function implements per-namespace merging with per-key last-write-wins semantics:

1. Starts with a clone of the ancestor's ResourceDict
2. For each namespace in the child's /Resources:
   - Merges the child's entries into the ancestor's entries
   - Per-key last-write-wins: if child has the same key as ancestor, child's value wins
   - Different keys are accumulated (not replaced)
3. Returns the merged ResourceDict

### Page Tree Integration (`crates/pdftract-core/src/parser/pages.rs`)

The `InheritedAttrs` struct tracks the accumulated ResourceDict during page tree traversal:
- `merge_inherited_attrs()`: Merges /Resources from /Pages nodes into the accumulator
- `build_page_dict()`: Merges /Resources from leaf /Page nodes and stores the result in `PageDict.resources: Arc<ResourceDict>`
- When a page has no /Resources, it inherits the parent's Arc (memory efficiency via Arc::ptr_eq)

## Acceptance Criteria Verification

### ✅ 1. Critical test: 3-level resource inheritance

Tests: `test_resource_inheritance_three_level` (pages.rs), `test_three_level_inheritance` (resources.rs)

The 3-level inheritance test creates:
- Grandparent /Pages with /F1 and /Im1
- Parent /Pages adds /F2
- Page 1 adds /F3 and overrides /F1
- Page 2 has no /Resources (inherits all)

Result: Page 1 has F1 (overridden), F2 (inherited), F3 (new), Im1 (inherited). Page 2 has F1, F2, Im1 (all inherited).

### ✅ 2. Per-key override test

Test: `test_merge_fonts_last_write_wins` (resources.rs)

Verifies that when a page declares `/Font << /F1 >>`, the F1 on the page overrides F1 on the ancestor (last-write-wins per-key).

### ✅ 3. /Resources missing on page: inherits parent's

Tests: `test_resource_inheritance_page_without_resources` (pages.rs), `test_merge_null_child_returns_ancestor` (resources.rs)

When a page has no /Resources, it inherits the parent's ResourceDict. The test verifies that the inherited resources are present and accessible.

### ✅ 3b. Arc<ResourceDict> is the SAME instance (Arc::ptr_eq)

Test: `test_resource_inheritance_arc_sharing` (pages.rs)

When multiple pages have no /Resources, they share the same Arc<ResourceDict> instance for memory efficiency. The test uses `Arc::ptr_eq()` to verify this.

### ✅ 4. ColorSpace inline-array test

Test: `test_merge_colorspace_inline_array` (resources.rs)

Verifies that ColorSpace values can be inline arrays (not just refs). The test creates an inline CalRGB color space array and verifies it's preserved in the merged dict.

### ✅ 5. Empty root /Resources: empty ResourceDict propagates

Test: `test_resource_inheritance_empty_root` (pages.rs)

When the root /Pages has an empty /Resources dict, the empty ResourceDict propagates to all leaf pages. The test verifies that the page's resources are empty.

### ✅ 6. INV-8 maintained: no panics on arbitrary input

Tests: All fuzz tests in `proptests` modules (pages.rs, resources.rs, catalog.rs, outline.rs, ocg.rs)

The property tests verify that:
- `fuzz_parse_rect_no_panics`: parse_rect never panics on arbitrary arrays
- `fuzz_build_page_dict_no_panics`: build_page_dict never panics on arbitrary input
- `fuzz_flatten_page_tree_no_panics`: flatten_page_tree handles arbitrary /Pages structures
- `fuzz_rotate_clamping_no_panics`: arbitrary rotate values are handled without panicking

## Test Results

All 18 resource-related tests pass:
- `test_empty_resource_dict`
- `test_resource_dict_not_empty`
- `test_merge_fonts_last_write_wins`
- `test_merge_xobjects`
- `test_merge_colorspace_inline_array`
- `test_merge_procset_dedup`
- `test_merge_null_child_returns_ancestor`
- `test_three_level_inheritance`
- `test_merge_all_namespaces`

All 26 page tree tests pass:
- `test_resource_inheritance_three_level`
- `test_resource_inheritance_page_without_resources`
- `test_resource_inheritance_arc_sharing`
- `test_resource_inheritance_empty_root`
- ... and 22 other page tree tests

All 16 fuzz tests pass:
- `fuzz_parse_rect_no_panics`
- `fuzz_build_page_dict_no_panics`
- `fuzz_flatten_page_tree_no_panics`
- `fuzz_rotate_clamping_no_panics`
- ... and 12 other fuzz tests

## Conclusion

The per-page Resource dictionary inheritance implementation is complete and correct. All acceptance criteria are met, and the tests cover the critical cases including 3-level inheritance, per-key override, Arc sharing, ColorSpace inline arrays, empty root /Resources, and INV-8 (no panics on arbitrary input).
