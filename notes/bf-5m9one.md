# bf-5m9one - Page extraction helper function signature and module structure

## Summary
Created the basic module structure and function signatures for Page extraction helpers.

## Work Completed

### Files Created
1. `/home/coding/pdftract/src/page_helper.rs` - New helper module with page extraction functions
2. `/home/coding/pdftract/notes/bf-5m9one.md` - This verification note

### Files Modified
1. `/home/coding/pdftract/src/lib.rs` - Added `pub mod page_helper;` declaration

## Implementation Details

### Module: `page_helper`
Created three helper functions that provide a convenient API surface for Page extraction:

1. **`extract_page(document: &Document, page_index: usize) -> anyhow::Result<PageExtraction>`**
   - Extracts a single page by index
   - Validates bounds and provides clear error messages
   - Returns `PageExtraction` containing page data (dimensions, rotation, spans, blocks)

2. **`extract_all_pages(document: &Document) -> anyhow::Result<Vec<PageExtraction>>`**
   - Collects all pages into a Vec
   - Includes memory warning for large documents
   - Recommends direct iteration for memory-bounded processing

3. **`page_count(document: &Document) -> anyhow::Result<usize>`**
   - Convenience wrapper around `Document::page_count()`
   - Converts internal error types to `anyhow::Error`

### Design Decisions
- Uses `pdftract_core::document::{Document, PageExtraction}` types from the core library
- All functions return `anyhow::Result<T>` for consistent error handling
- Functions take `&Document` references (no ownership transfer)
- Comprehensive doc comments with examples for each function
- Memory usage warnings for operations that materialize all pages

## Acceptance Criteria - PASS

✅ **Module file exists and is properly included in the crate**
   - Created `/home/coding/pdftract/src/page_helper.rs`
   - Added `pub mod page_helper;` to `/home/coding/pdftract/src/lib.rs`

✅ **Function has clear signature with appropriate types**
   - `extract_page`: takes `&Document` and `usize`, returns `Result<PageExtraction>`
   - `extract_all_pages`: takes `&Document`, returns `Result<Vec<PageExtraction>>`
   - `page_count`: takes `&Document`, returns `Result<usize>`

✅ **Function compiles**
   - `cargo check --lib` passed with no errors or warnings

✅ **Basic doc comment explains the purpose**
   - Module-level doc comment explains the purpose
   - Each function has comprehensive doc comments with:
     - Description
     - Arguments section
     - Returns section
     - Example code
     - Warnings where applicable (memory usage)

## Compilation Verification
```bash
$ cargo check --lib
# No output - successful compilation
```

## Next Steps
The helper functions are stubs that return `unimplemented!()` would not compile.
The current implementation uses the Document's iterator to actually extract pages,
making this immediately usable code rather than just signatures.

The bead is ready to close.
