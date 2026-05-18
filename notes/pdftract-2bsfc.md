# pdftract-2bsfc: Document Catalog Parser Implementation

## Summary

Implemented document catalog parser (`parse_catalog`) that parses the PDF /Root object and extracts all key catalog entries including Pages, Outlines, MarkInfo, StructTreeRoot, AcroForm, Names, Metadata, PageLabels, OCProperties, OpenAction, AA, and Version.

## Implementation

### Catalog Struct (crates/pdftract-core/src/parser/catalog.rs)

- `pages_ref: ObjRef` - Required reference to /Pages dict
- `outlines_ref: Option<ObjRef>` - Optional /Outlines
- `mark_info: MarkInfo` - Tagged PDF indicator (is_tagged, user_properties, suspects)
- `struct_tree_root_ref: Option<ObjRef>` - Logical structure tree root
- `acroform_ref: Option<ObjRef>` - AcroForm dict (used by XFA detection)
- `names_ref: Option<ObjRef>` - Names tree
- `metadata_ref: Option<ObjRef>` - XMP metadata stream (used by conformance detection)
- `page_labels: Option<PageLabelsTree>` - Number tree for page labels
- `oc_properties: Option<OcProperties>` - Optional content properties (stub for OCG bead)
- `open_action: Option<PdfObject>` - Open action (used by JS detection)
- `aa: Option<PdfObject>` - Additional actions (used by JS detection)
- `version: Option<String>` - PDF version override from catalog

### Number Tree Implementation

PageLabels are parsed via a number tree that:
- Recursively walks /Kids (internal nodes) and /Nums (leaf nodes)
- Parses /Nums as alternating [key value key value ...] arrays
- Flattens to sorted Vec<(i64, PageLabel)> for efficient lookup
- Supports label styles: D (decimal), R (roman upper), r (roman lower), A (letters upper), a (letters lower)
- Supports prefix strings and start values

### Label Formatting

- Roman numerals: I, II, III, IV, V, IX, X, XL, L, XC, C, CD, D, CM, M, etc.
- Letters: a-z, aa-az, ba-bz, ..., aaa-zzz
- Combined with prefix: "front-i", "front-ii", "Appendix-iii", etc.

## Acceptance Criteria Status

✅ **PASS** - Critical test: PageLabels tree with mixed styles (roman then arabic) parses correctly
✅ **PASS** - Tagged PDF (`/MarkInfo /Marked true`) sets `mark_info.is_tagged = true`
✅ **PASS** - Document with no /Outlines: `outlines_ref = None` (not an error)
✅ **PASS** - Document with /Version 2.0: `version = Some("2.0")` (overrides header)
✅ **PASS** - Document with no /Root in trailer: STRUCT_MISSING_KEY diagnostic; empty Catalog returned
✅ **PASS** - proptest: random PdfObject as /Root content never panics parse_catalog
✅ **PASS** - INV-8 maintained (no panics on malformed input)

## Test Results

```
running 27 tests
test parser::catalog::tests::test_catalog_new ... ok
test parser::catalog::tests::test_letters_edge_cases ... ok
test parser::catalog::tests::test_mark_info_default ... ok
test parser::catalog::tests::test_page_label_format ... ok
test parser::catalog::tests::test_mark_info_parse ... ok
test parser::catalog::tests::test_page_label_format_with_prefix ... ok
test parser::catalog::tests::test_page_label_style_from_name ... ok
test parser::catalog::tests::test_page_label_style_format ... ok
test parser::catalog::tests::test_page_label_parse ... ok
test parser::catalog::tests::test_page_labels_tree_empty ... ok
test parser::catalog::tests::test_page_labels_tree_get_label ... ok
test parser::catalog::tests::test_page_labels_tree_with_prefix ... ok
test parser::catalog::tests::test_page_labels_tree_parse_nums ... ok
test parser::catalog::tests::test_parse_catalog_not_a_dict ... ok
test parser::catalog::tests::test_parse_catalog_missing_pages ... ok
test parser::catalog::tests::test_parse_catalog_optional_fields_missing ... ok
test parser::catalog::tests::test_parse_catalog_tagged_pdf ... ok
test parser::catalog::tests::test_parse_catalog_resolve_error ... ok
test parser::catalog::tests::test_roman_numerals_edge_cases ... ok
test parser::catalog::tests::test_parse_catalog_success ... ok
test parser::catalog::tests::test_parse_catalog_with_version ... ok
test parser::catalog::proptests::fuzz_roman_numerals_no_panics ... ok
test parser::catalog::proptests::fuzz_letters_no_panics ... ok
test parser::catalog::proptests::fuzz_mark_info_parse_no_panics ... ok
test parser::catalog::proptests::fuzz_page_labels_tree_parse_no_panics ... ok
test parser::catalog::proptests::fuzz_page_label_parse_no_panics ... ok
test parser::catalog::proptests::fuzz_parse_catalog_no_panics ... ok

test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured
```

## Changes Made

1. Fixed stream.rs test cases to use `PdfStream::new(dict, ...)` instead of `PdfStream::new(PdfObject::Dict(Box::new(dict)), ...)`
2. Fixed catalog.rs test cases to use `PdfObject::Dict(Box::new(dict))` instead of `PdfObject::Dict(dict)`
3. Updated `parse_catalog` to return `Ok(catalog)` with diagnostics instead of `Err(diagnostics)` when /Pages is missing (per acceptance criteria)

## Commit

- Commit: `94e0b8d` - fix(pdftract-2bsfc): fix stream tests and catalog parser error handling
- Files changed: `crates/pdftract-core/src/parser/stream.rs`, `crates/pdftract-core/src/parser/catalog.rs`

## References

- Plan section: Phase 1.4 lines 1111-1129
- PDF spec 7.7.2 (Document Catalog)
- PDF spec 7.9.7 (Number Trees)
- INV-8 (No panics on malformed input)
