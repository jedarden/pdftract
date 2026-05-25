# pdftract-3h9xo: threads JSON output + schema integration

## Bead Description
Phase 7.7.3: Add threads field to ExtractionResult with ThreadJson schema integration.

## Implementation Summary

### 1. Schema (crates/pdftract-core/src/schema/mod.rs)
- Added `ThreadJson` struct with fields: title, author, subject, keywords, beads
- Added `BeadJson` struct with fields: page_index, rect
- Both structs derive Serialize, Deserialize, JsonSchema

### 2. Threads Module (crates/pdftract-core/src/threads/mod.rs)
- Added `thread_to_json()` function to convert ThreadHeader + Bead slice to ThreadJson
- Function properly handles UTF-16 decoded strings from PDF

### 3. Extraction Pipeline (crates/pdftract-core/src/extract.rs)
- Added `threads: Vec<ThreadJson>` field to ExtractionResult
- Implemented Phase 7.7 extraction logic:
  - Build page_ref_to_index map for O(1) page lookups
  - Call discover_threads to find thread headers
  - Call walk_beads for each thread to collect bead chains
  - Convert to ThreadJson via thread_to_json

### 4. Parser Helper (crates/pdftract-core/src/parser/pages.rs)
- Added `build_page_ref_to_index()` helper function
- Creates HashMap<ObjRef, usize> mapping page object refs to indices
- Handles pages tree traversal

### 5. Markdown Sink (crates/pdftract-core/src/markdown.rs)
- Added `threads_to_markdown()` function
- Added `collapse_page_ranges()` helper for compact page display
- Handles duplicate page indices correctly
- Format: "## Article Threads\n\n1. Title (Author) - pages X-Y (N beads)"

### 6. JSON Schema (docs/schema/v1.0/pdftract.schema.json)
- Added ThreadJson definition to $defs
- Added BeadJson definition to $defs
- Integrated threads into extraction result schema

### 7. PyO3 Bindings (crates/pdftract-py/src/lib.rs)
- Added thread_to_py() and bead_to_py() conversion functions
- Integrated threads into extract() function's Python dict output
- Threads returned as list of dicts with title, author, subject, keywords, beads fields
- Beads returned as 2-key dicts (page_index, rect) per spec

### 8. Core Exports (crates/pdftract-core/src/lib.rs)
- Added ThreadJson, BeadJson to pub use schema exports

## Testing Results

### PASS: Threads module tests
- All 32 threads tests pass
- test_bead_new, test_decode_* (string decoding tests)
- test_discover_* (thread discovery tests)
- test_thread_header_* (header parsing tests)
- test_walk_beads_* (bead chain walking tests)

### PASS: Markdown tests
- All 35 markdown span_tests pass
- test_threads_to_markdown_empty
- test_threads_to_markdown_single_thread
- test_threads_to_markdown_multiple_threads
- test_threads_to_markdown_untitled_thread
- test_collapse_page_ranges_single_page
- test_collapse_page_ranges_contiguous
- test_collapse_page_ranges_gaps
- test_collapse_page_ranges_mixed

### PASS: Build verification
- pdftract-core compiles successfully
- pdftract-cli compiles successfully
- pdftract-py compiles successfully

### PASS: Schema generation
- JSON schema updated with ThreadJson and BeadJson definitions
- Proper $ref integration in extraction result

## Acceptance Criteria

1. ✅ ThreadJson struct added with title, author, subject, keywords, beads fields
2. ✅ BeadJson struct added with page_index, rect fields
3. ✅ thread_to_json conversion function implemented
4. ✅ ExtractionResult includes threads field
5. ✅ Phase 7.7 extraction logic implemented in extract.rs
6. ✅ JSON schema updated with ThreadJson and BeadJson definitions
7. ✅ threads_to_markdown function implemented for markdown sink
8. ✅ PyO3 bindings expose threads in extract() output
9. ✅ All threads module tests pass (32/32)
10. ✅ All markdown tests pass (35/35)

## Code Changes Summary
- crates/pdftract-core/src/lib.rs: Added ThreadJson, BeadJson exports
- crates/pdftract-core/src/schema/mod.rs: Added ThreadJson, BeadJson structs
- crates/pdftract-core/src/threads/mod.rs: Added thread_to_json function
- crates/pdftract-core/src/parser/pages.rs: Added build_page_ref_to_index helper
- crates/pdftract-core/src/extract.rs: Added threads field and Phase 7.7 extraction
- crates/pdftract-core/src/markdown.rs: Added threads_to_markdown and collapse_page_ranges
- docs/schema/v1.0/pdftract.schema.json: Added ThreadJson, BeadJson schema definitions
- crates/pdftract-py/src/lib.rs: Added thread_to_py, bead_to_py, integrated into extract()

## Files Modified
1. crates/pdftract-core/src/lib.rs
2. crates/pdftract-core/src/schema/mod.rs
3. crates/pdftract-core/src/threads/mod.rs
4. crates/pdftract-core/src/parser/pages.rs
5. crates/pdftract-core/src/extract.rs
6. crates/pdftract-core/src/markdown.rs
7. docs/schema/v1.0/pdftract.schema.json
8. crates/pdftract-py/src/lib.rs

## Status
COMPLETE - All acceptance criteria met. Threads are now extracted from PDFs and available in JSON output, markdown sink, and Python bindings.
