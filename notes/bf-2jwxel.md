# Verification Note: bf-2jwxel

## Issue
PyO3 `extract_markdown()` was returning plain text instead of Markdown.

## Root Cause
The Python binding at `crates/pdftract-py/src/lib.rs:181-187` was incorrectly delegating to `extract_text_fn`, which returns plain text. The working Markdown implementation `pdftract_core::sdk::extract_markdown` was not being used.

## Fix Applied

### 1. Created `/home/coding/pdftract/crates/pdftract-py/src/extract_markdown.rs`
- New module implementing `extract_markdown_fn()` following the same pattern as `extract_text_fn()`
- Calls `pdftract_core::sdk::extract_markdown()` instead of `extract_text()`
- Includes full kwargs parsing with strict validation
- Proper error mapping to Python exception hierarchy
- GIL release during extraction for thread safety

### 2. Updated `crates/pdftract-py/src/lib.rs`
- Added module import: `mod extract_markdown;` and `use extract_markdown::extract_markdown_fn;`
- Replaced stub `extract_markdown()` function with proper wrapper calling `extract_markdown_fn()`
- Removed incorrect TODO comment about implementing Markdown conversion

## Verification

### Build Status
- ✅ Code compiles successfully: `cargo build --package pdftract-py --lib` completes without errors
- ✅ No warnings about unused imports or functions

### Logic Verification
- ✅ `py_extract_markdown()` (lib.rs:193-194) now calls `extract_markdown_fn()` instead of `extract_text_fn()`
- ✅ `extract_markdown_fn()` (extract_markdown.rs:196) calls `pdftract_core::sdk::extract_markdown()` (line 205)
- ✅ SDK function uses `page_to_markdown_with_links()` to generate actual Markdown with:
  - Heading markers (`#`)
  - Link syntax (`[text](url)`)
  - Per-page structure with blocks, spans, and links

### Acceptance Criteria Met
1. ✅ `extract_markdown()` no longer delegates to `extract_text_fn`
2. ✅ Output will differ from `extract_text()` for fixtures containing Markdown constructs
3. ✅ Output matches `sdk::extract_markdown` (by virtue of calling the same function)

## Files Changed
- Created: `crates/pdftract-py/src/extract_markdown.rs` (323 lines)
- Modified: `crates/pdftract-py/src/lib.rs` (module imports and pyfunction signature)

## Testing Note
PyO3 tests fail due to Python symbol linking issues when run without Python interpreter. This is expected behavior for PyO3 code. Runtime testing requires building the Python wheel with maturin, which is not available in this environment. However, the fix is verified by:
1. Compilation success
2. Code review confirming correct function call chain
3. Matching the pattern used by other working Python bindings in the same crate

## Commit
Changes will be committed as part of closing this bead.
