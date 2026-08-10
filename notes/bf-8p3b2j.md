# Bead bf-8p3b2j: Implement single Page extraction from Document

## Status: COMPLETE

## Implementation Summary

The single Page extraction from Document functionality is fully implemented via two complementary APIs:

### 1. `page_helper::extract_page` Function

**Location:** `/home/coding/pdftract/crates/pdftract-core/src/page_helper.rs`

**Implementation:**
- Extracts a single `PageExtraction` from a `Document` by index
- Returns `Result<PageExtraction>` with proper error handling
- Performs validation:
  - Page count validation (checks for empty documents)
  - Bounds checking (page index within document range)
  - Page data validation (positive dimensions, valid rotation)

**Error Handling:**
- `PageError::NoPages` - Document has zero pages
- `PageError::IndexOutOfBounds` - Page index exceeds available pages
- `PageError::InvalidDimensions` - Width or height <= 0
- `PageError::InvalidRotation` - Rotation not in {0, 90, 180, 270}
- `PageError::ExtractionFailed` - Page iteration or data extraction failed

### 2. `Document::extract_page` Method

**Location:** `/home/coding/pdftract/crates/pdftract-core/src/document.rs:1671-1743`

**Implementation:**
- Extracts a single `output::sink::Page` from a `Document` by index
- Returns `DocumentResult<crate::output::sink::Page>` with proper error handling
- Navigates nested Document structure via lazy `pages()` iterator
- Converts `PageExtraction` to `output::sink::Page` for sink compatibility

**Validation:**
- Bounds checking (page index validation)
- Media box validation (coordinates must be finite and well-formed)
- Dimension validation (positive width/height, reasonable max values)
- Rotation validation (must be 0, 90, 180, or 270)

## Verification

### Test Results

**Test File:** `/home/coding/pdftract/crates/pdftract-core/tests/test_page_helper_extract_page.rs`

All 4 tests PASS:
- ✅ `test_extract_single_page_from_document` - Successfully extracts page 0 from valid document
- ✅ `test_extract_page_out_of_bounds` - Correctly returns error for invalid index
- ✅ `test_extract_page_handles_nested_structure` - Properly navigates Document→catalog→pages tree
- ✅ `test_extract_page_from_multi_page_document` - Extracts different pages correctly

### Acceptance Criteria Verification

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Function successfully extracts a Page from valid Document | ✅ PASS | Tests pass, returns PageExtraction with valid fields |
| Returns Err() for Documents missing Page data | ✅ PASS | `PageError::NoPages` for empty documents |
| Handles the nested structure correctly | ✅ PASS | Uses `pages()` iterator to navigate Document→catalog→Pages tree |
| One test demonstrates successful extraction | ✅ PASS | 4 tests demonstrate extraction success |

### Example Usage

```rust
use pdftract_core::{page_helper, Document};

// Open document
let doc = Document::open("document.pdf")?;

// Extract single page by index
let page = page_helper::extract_page(&doc, 0)?;

// Access extracted fields
println!("Page {}: {}x{} points", page.index, page.width, page.height);
```

## Architecture

The extraction follows this path:

```
Document
  ↓
Document::pages() → PageIter (lazy iterator)
  ↓
LazyPageIter (walks page tree depth-first)
  ↓
PageDict (raw page data from parser)
  ↓
PageExtraction (validated page with dimensions/rotation)
  ↓
output::sink::Page (for output sinks)
```

## Dependencies

This implementation depends on:
- `Document::pages()` - Lazy page iterator
- `PageIter` - Iterator over pages
- `PageExtraction` - Intermediate page representation
- `output::sink::Page` - Final page type for sinks
- `DocumentError` / `PageError` - Error types

## Git History

The implementation exists in the current codebase. The functionality was verified to be complete and working via the existing test suite.

## Conclusion

The single Page extraction from Document is **fully implemented and tested**. Both `page_helper::extract_page` and `Document::extract_page` provide working APIs for extracting individual pages with proper error handling and validation.
