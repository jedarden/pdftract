# Import Error Analysis for pdftract Test Suite

## Task: Identify specific import errors

**Command run:** `cargo check --tests 2>&1`

## Summary

### Import Errors Found: **0**

There are **NO actual import errors** in the test suite. No "cannot find", "unresolved import", or "not found in this scope" errors related to missing types or modules.

### Total Errors Found: 13

All 13 errors are **NOT import errors** - they are type mismatches, wrong argument counts, and trait implementation conflicts.

---

## Detailed Error Catalog

### Error Type E0119: Conflicting trait implementations (2 occurrences)

**Error:** Conflicting implementations of trait `From<PageExtractionError>` for type `anyhow::Error`

**Locations:**
1. `crates/pdftract-core/src/page_extraction_error.rs:267:1`
2. `crates/pdftract-core/src/page_extraction_error.rs:267:1` (duplicate)

**Issue:** Custom `From<PageExtractionError> for anyhow::Error` implementation conflicts with anyhow's blanket implementation `impl<E> From<E> for anyhow::Error where E: StdError + Send + Sync + 'static`

**Fix needed:** Remove the custom `From` implementation; rely on anyhow's blanket implementation.

---

### Error Type E0599: Method not found (2 occurrences)

**Error:** No method named `is_none` found for struct `Arc<ResourceDict>`

**Locations:**
1. `crates/pdftract-core/src/extract.rs:203:23`
2. `crates/pdftract-core/src/extract.rs:203:23` (duplicate)

**Code:**
```rust
if page.resources.is_none() {
```

**Issue:** `page.resources` is of type `Arc<ResourceDict>` (always present), not `Option<Arc<ResourceDict>>`. The `is_none()` method doesn't exist on `Arc`.

**Fix needed:** This is a logic error, not an import error. The code needs to be restructured to handle the fact that `resources` is always present in an `Arc`.

---

### Error Type E0061: Wrong number of arguments (3 occurrences)

**Error:** Function takes 5 arguments but 4 were supplied

**Locations:**
1. `crates/pdftract-core/src/extract.rs:838:35`
2. `crates/pdftract-core/src/extract.rs:1868:35`
3. `crates/pdftract-core/src/extract.rs:2191:35`

**Code example (line 838):**
```rust
let decoded_streams = decode_page_content_streams(
    &page_dict,
    &resolver_arc,
    &source,
    options.max_decompress_bytes,
);
```

**Issue:** Missing 5th argument of type `usize`. The function signature requires 5 arguments but only 4 are provided.

**Fix needed:** Add the missing 5th `usize` argument to all three call sites.

---

### Error Type E0308: Type mismatches (6 occurrences)

**Error:** Type mismatch - expected `&[u8]`, found `&Result<Vec<u8>, PageExtractionError>`

**Locations:**
1. `crates/pdftract-core/src/extract.rs:846:45`
2. `crates/pdftract-core/src/extract.rs:1876:45`
3. `crates/pdftract-core/src/extract.rs:2199:45`
   (plus 3 duplicates)

**Code example (line 846):**
```rust
track_mcids_from_content_stream(&decoded_streams, &mut tracker);
```

**Issue:** `decoded_streams` is of type `Result<Vec<u8>, PageExtractionError>` but `track_mcids_from_content_stream` expects `&[u8]`.

**Expected function signature:**
```rust
pub fn track_mcids_from_content_stream(content_bytes: &[u8], tracker: &mut McidTracker)
```

**Fix needed:** Handle the `Result` properly (e.g., unwrap, propagate, or match) before passing to the function.

---

## Warnings (Not Errors)

### Unused Imports

1. **`crates/pdftract-core/src/audit.rs:183:9`** - `use std::io::Cursor;` (unused)
2. **`crates/pdftract-core/src/table/output.rs:9:5`** - `use anyhow::Result;` (unused)
3. **`crates/pdftract-core/src/table/output.rs:275:9`** - `use crate::table::Segment;` (unused)
4. **`crates/pdftract-core/src/decoder/jbig2.rs:154:9`** - `use indexmap::indexmap;` (unused)
5. **`crates/pdftract-core/src/parser/object.rs`** - `PdfDict` (unused)

---

## Conclusion

**The task was to identify import errors, but the actual findings are:**

1. ✅ **0 import errors** - All types and modules are properly imported
2. ⚠️ **13 other errors** - Type mismatches, wrong argument counts, and trait conflicts
3. ℹ️ **Multiple warnings** - Unused imports that should be cleaned up

**The bead description mentioned "Expected errors: Path type in audit.rs" but no such error was found.** The only reference to audit.rs is an unused import warning at line 183.

---

## Acceptance Criteria Status

- [x] Run `cargo check --tests` and capture all error[E*] output
- [x] Document each error with file path, line number, and missing type
- [x] Verify the total count of errors (13 total, 0 import errors)

**Next steps:** This bead is now about documenting that there are no import errors to fix. The 13 actual errors are a different category of issues (type system errors, not import errors).
