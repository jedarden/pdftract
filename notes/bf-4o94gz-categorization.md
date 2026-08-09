# Compiler Warnings Categorization

Generated from `cargo check --all-targets` and `cargo test --all-targets` runs on 2026-08-09.

## Summary

- **Total warnings detected**: 436 unique warnings
- **Categories identified**: 7 distinct warning types
- **Build status**: Compilation fails due to 8 compilation errors (E0119, E0599, E0308, E0061)

## Warning Categories

### 1. Dead Code Warnings (unused fields/structs)
**Count**: 1

**Files affected**:
- `crates/pdftract-core/build.rs:37-43` - `UnmappedGlyphNamesConfig` struct fields `description` and `version`

**Description**: Struct fields that are defined but never read.

---

### 2. Unused Imports Warnings
**Count**: 98+

**Most affected files**:
- `crates/pdftract-core/src/extract.rs` - 15+ unused imports
- `crates/pdftract-core/src/font/type3_rasterizer.rs` - 12+ unused imports
- `crates/pdftract-core/src/forms/mod.rs` - 8+ unused imports
- `crates/pdftract-core/src/signature/mod.rs` - 8+ unused imports
- `crates/pdftract-core/src/parser/xref.rs` - 5+ unused imports
- `crates/pdftract-core/src/parser/ocg.rs` - 4+ unused imports
- `crates/pdftract-core/src/parser/outline.rs` - 5+ unused imports
- `crates/pdftract-core/src/parser/pages.rs` - 3+ unused imports
- `crates/pdftract-core/src/layout/correction.rs` - 3+ unused imports
- `crates/pdftract-core/src/sdk.rs` - 8+ unused imports
- `crates/pdftract-core/src/cache/key.rs` - 1 unused import (`Map`)
- `crates/pdftract-core/src/cache/lru.rs` - 1 unused import (`entry_path`)
- `crates/pdftract-core/src/annotation/json.rs` - 1 unused import (`DestArray`)
- `crates/pdftract-core/src/detection.rs` - 1 unused import (`ObjRef`)
- `crates/pdftract-core/src/document.rs` - 3 unused imports
- `crates/pdftract-core/src/encryption/detection.rs` - 1 unused import (`DiagCode`)
- `crates/pdftract-core/src/encryption/decryptor.rs` - 2 unused imports
- `crates/pdftract-core/src/conformance.rs` - 2 unused imports
- `crates/pdftract-core/src/content_stream.rs` - 2 unused imports
- `crates/pdftract-core/src/javascript.rs` - 1 unused import (`ObjRef`)
- `crates/pdftract-core/src/layout/figure.rs` - 1 unused import (`Arc`)
- `crates/pdftract-core/src/layout/list.rs` - 1 unused import (`OnceLock`)
- `crates/pdftract-core/src/layout/reading_order.rs` - 1 unused import (`HashSet`)
- `crates/pdftract-core/src/log_policy.rs` - 1 unused import (`anyhow::Result`)
- `crates/pdftract-core/src/output/markdown/links.rs` - 1 unused import (`FitType`)
- `crates/pdftract-core/src/output/ndjson/pipeline.rs` - 1 unused import (`PageClass`)
- `crates/pdftract-core/src/output/sink.rs` - 4 unused imports
- `crates/pdftract-core/src/page_helper.rs` - 1 unused import (`anyhow`)
- `crates/pdftract-core/src/parser/catalog.rs` - 1 unused import (`intern`)
- `crates/pdftract-core/src/parser/hint_stream.rs` - 1 unused import (`FlateDecoder`)
- `crates/pdftract-core/src/parser/marked_content.rs` - 2 unused imports
- `crates/pdftract-core/src/parser/object/cache.rs` - 1 unused import (`RESOLVING`)
- `crates/pdftract-core/src/parser/object/cycle.rs` - 1 unused import
- `crates/pdftract-core/src/parser/resources.rs` - 1 unused import (`PdfDict`)
- `crates/pdftract-core/src/parser/stream.rs` - 4+ unused imports
- `crates/pdftract-core/src/parser/struct_tree.rs` - 1 unused import (`MarkInfo`)
- `crates/pdftract-core/src/parser/lexer/mod.rs` - 1 unused import
- `crates/pdftract-core/src/receipts/ocr_fallback.rs` - 1 unused import (`base64::prelude::*`)
- `crates/pdftract-core/src/source/mod.rs` - 3 unused imports
- `crates/pdftract-core/src/schema/mod.rs` - 1 unused import (`serde_json::json`)
- `crates/pdftract-core/src/table/output.rs` - 2 unused imports
- `crates/pdftract-core/src/atomic_file_writer.rs` - 1 unused import (`Write`)
- `crates/pdftract-core/src/attachment/filespec.rs` - 1 unused import
- `crates/pdftract-core/src/attachment/name_tree.rs` - 1 unused import
- `crates/pdftract-core/src/audit.rs` - 1 unused import (`Cursor`)
- `crates/pdftract-core/src/decoder/jbig2.rs` - 1 unused import (`indexmap::indexmap`)
- `crates/pdftract-core/src/font/agl.rs` - 1 unused import (`DiagCode`)
- `crates/pdftract-core/src/font/fingerprint.rs` - 1 unused import (`Arc`)
- `crates/pdftract-core/src/font/resolver.rs` - 6+ unused imports
- `crates/pdftract-core/src/font/type0.rs` - 1 unused import (`OpenTypeMetrics`)
- `crates/pdftract-core/src/font/type3_rasterizer.rs` - 12+ unused imports
- `crates/pdftract-core/src/font/type3_test_fixtures.rs` - 1 unused import (`Ordering`)
- `crates/pdftract-core/src/glyph/mod.rs` - 4 unused imports
- `crates/pdftract-core/src/semaphore.rs` - 1 unused import
- `crates/pdftract-core/src/table/cell.rs` - 1 unused import
- `crates/pdftract-core/src/table/grid.rs` - 1 unused import
- `crates/pdftract-core/src/threads/mod.rs` - 2 unused imports
- `crates/pdftract-core/src/parser/inline_image.rs` - 1 unused import
- `crates/pdftract-core/src/parser/objstm.rs` - 2 unused imports
- `crates/pdftract-core/src/page_class.rs` - 1 unused import
- `crates/pdftract-core/src/span/mod.rs` - 1 unused import

**Common unused imports**:
- `std::sync::Arc` (appears in multiple files)
- `intern` (parser object helper)
- `PdfDict` (PDF dictionary type)
- `PdfObject` (PDF object type)
- `ObjRef` (object reference type)
- `anyhow::Result` (error type)
- Various serde and std library imports

---

### 3. Unused Variable Warnings
**Count**: 70+

**Most affected files**:
- `crates/pdftract-core/src/layout/reading_order.rs` - 6 unused variables
- `crates/pdftract-core/src/font/type3_rasterizer_test.rs` - 8 unused variables
- `crates/pdftract-core/src/parser/pages.rs` - 7 unused variables (assignments)
- `crates/pdftract-core/src/parser/outline.rs` - 6 unused variables
- `crates/pdftract-core/src/classify.rs` - 4 unused variables
- `crates/pdftract-core/src/extract.rs` - 10+ unused variables
- `crates/pdftract-core/src/font/type3_rasterizer.rs` - 8 unused variables
- `crates/pdftract-core/src/font/resolver.rs` - 4 unused variables
- `crates/pdftract-core/src/forms/mod.rs` - 2 unused variables
- `crates/pdftract-core/src/parser/xref.rs` - 3 unused variables
- `crates/pdftract-core/src/signature/mod.rs` - 5 unused variables
- `crates/pdftract-core/src/layout/header_footer.rs` - 4 unused variables
- `crates/pdftract-core/src/parser/ocg.rs` - 2 unused variables
- `crates/pdftract-core/src/encryption/rc4.rs` - 2 unused variables
- `crates/pdftract-core/src/glyph/mod.rs` - 4 unused variables
- `crates/pdftract-core/src/parser/lexer/mod.rs` - 2 unused variables
- `crates/pdftract-core/src/parser/objstm.rs` - 2 unused variables
- `crates/pdftract-core/src/layout/correction.rs` - 2 unused variables
- `crates/pdftract-core/src/table/output.rs` - 2 unused variables
- `crates/pdftract-core/src/parser/struct_tree.rs` - 1 unused variable
- `crates/pdftract-core/src/cmap/codespace.rs` - 2 unused variables
- `crates/pdftract-core/src/document.rs` - 1 unused variable
- `crates/pdftract-core/src/parser/stream.rs` - 3 unused variables
- `crates/pdftract-core/src/semaphore.rs` - 1 unused variable
- `crates/pdftract-core/src/table/cell.rs` - 1 unused variable
- `crates/pdftract-core/src/threads/mod.rs` - 2 unused variables
- `crates/pdftract-core/src/atomic_file_writer.rs` - 1 unused variable
- `crates/pdftract-core/src/attachment/filespec.rs` - 1 unused variable
- `crates/pdftract-core/src/attachment/name_tree.rs` - 1 unused variable
- `crates/pdftract-core/src/audit.rs` - 1 unused variable
- `crates/pdftract-core/src/decoder/jbig2.rs` - 1 unused variable
- `crates/pdftract-core/src/graphics_state.rs` - 1 unused variable
- `crates/pdftract-core/src/layout/columns.rs` - 2 unused variables
- `crates/pdftract-core/src/parser/inline_image.rs` - 1 unused variable
- `crates/pdftract-core/src/parser/resources.rs` - 1 unused variable
- `crates/pdftract-core/src/render/scanline.rs` - 1 unused variable
- `crates/pdftract-core/src/word_boundary.rs` - 1 unused variable
- `crates/pdftract-core/src/parser/ocg.rs` - 2 unused variables
- `crates/pdftract-core/src/font/type3_test_fixtures.rs` - 1 unused variable
- `crates/pdftract-core/src/layout/line.rs` - 1 unused variable
- `crates/pdftract-core/src/parser/hint_stream.rs` - 1 unused import
- `crates/pdftract-core/src/output/ndjson/frames.rs` - 1 unused import
- `crates/pdftract-core/src/output/ndjson/pipeline.rs` - 1 unused import
- `crates/pdftract-core/src/page_class.rs` - 1 unused import
- `crates/pdftract-core/src/source/mmap.rs` - 1 unused import
- `crates/pdftract-core/src/table/output.rs` - 1 unused import

**Common patterns**:
- Function parameters that are not used (e.g., `resolver`, `catalog`, `diagnostics`)
- Test variables that are created but not read
- Loop counters that are ignored
- Pattern match variables that aren't used

---

### 4. Unused `mut` Warnings
**Count**: 85+

**Most affected files**:
- `crates/pdftract-core/src/forms/mod.rs` - 20+ unused `mut` declarations
- `crates/pdftract-core/src/signature/mod.rs` - 20+ unused `mut` declarations
- `crates/pdftract-core/src/parser/xref.rs` - 10+ unused `mut` declarations
- `crates/pdftract-core/src/parser/ocg.rs` - 8+ unused `mut` declarations
- `crates/pdftract-core/src/font/type3_rasterizer.rs` - 5+ unused `mut` declarations
- `crates/pdftract-core/src/parser/pages.rs` - 3+ unused `mut` declarations
- `crates/pdftract-core/src/parser/outline.rs` - 2+ unused `mut` declarations
- `crates/pdftract-core/src/cache/compression.rs` - 2 unused `mut` declarations
- `crates/pdftract-core/src/attachment/filespec.rs` - 2 unused `mut` declarations
- `crates/pdftract-core/src/attachment/name_tree.rs` - 1 unused `mut`
- `crates/pdftract-core/src/parser/lexer/mod.rs` - 1 unused `mut`
- `crates/pdftract-core/src/parser/inline_image.rs` - 1 unused `mut`
- `crates/pdftract-core/src/parser/objstm.rs` - 2 unused `mut` declarations
- `crates/pdftract-core/src/encryption/aes_128.rs` - 2 unused `mut` declarations
- `crates/pdftract-core/src/render/scanline.rs` - 1 unused `mut`
- `crates/pdftract-core/src/word_boundary.rs` - 1 unused `mut`
- `crates/pdftract-core/src/table/grid.rs` - 1 unused `mut`
- `crates/pdftract-core/src/font/type3_rasterizer_test.rs` - 1 unused `mut`
- `crates/pdftract-core/src/atomic_file_writer.rs` - 2 unused `mut` declarations

**Common patterns**:
- Test helper functions creating resolvers/catalogs that are never mutated
- Variables that are initialized with `mut` but only read from
- Loop variables that don't need mutability

---

### 5. Unused Assignment Warnings
**Count**: 15+

**Most affected files**:
- `crates/pdftract-core/src/parser/pages.rs` - 7 unused assignments (inherited variable)
- `crates/pdftract-core/src/layout/reading_order.rs` - 3 unused assignments (region_count, small_region_count)
- `crates/pdftract-core/src/parser/xref.rs` - 2 unused assignments (depth variable)
- `crates/pdftract-core/src/parser/objstm.rs` - 1 unused assignment (offset variable)
- `crates/pdftract-core/src/font/type3_rasterizer.rs` - 1 unused assignment (dummy_array)
- `crates/pdftract-core/src/parser/lexer/mod.rs` - 1 unused assignment (sign_count variable)

**Description**: Values are assigned to variables but immediately overwritten before being read.

---

### 6. Unreachable Pattern Warnings
**Count**: 1

**Files affected**:
- `crates/pdftract-core/src/layout/correction.rs:376` - Duplicate match arm for `0x0178` (Latin letter Y with diaeresis)

**Description**: Pattern match arms that can never be reached because previous patterns cover all possible values.

---

### 7. Unused Doc Comment Warnings
**Count**: 2

**Files affected**:
- `crates/pdftract-core/src/parser/object/cache.rs:50-53` - Doc comment on macro invocation
- `crates/pdftract-core/src/parser/object/cycle.rs:33-37` - Doc comment on macro invocation

**Description**: Documentation comments placed on macro invocations, where rustdoc doesn't generate documentation.

---

## Bundle Size Warnings

**Count**: 3 (informational)

**Files affected**:
- `crates/pdftract-inspector-ui` - Frontend bundle size warnings (Raw: 1.95 KB, Gzipped: 0.87 KB / 80 KB limit)

**Description**: Informational warnings about frontend bundle sizes - these are within acceptable limits.

---

## Compilation Errors (blocking compilation)

**Count**: 8 errors

**Error types**:
1. **E0119** (conflicting trait implementations) - 2 occurrences
   - `PageExtractionError` → `anyhow::Error` implementation conflict
2. **E0599** (method not found) - 2 occurrences
   - `is_none()` called on `Arc<ResourceDict>` which doesn't have this method
3. **E0308** (type mismatch) - 4 occurrences
   - Expected `&[u8]`, found `&Result<Vec<u8>, PageExtractionError>`
4. **E0061** (incorrect function arguments) - 3 occurrences
   - `decode_page_content_streams` called with 4 arguments but requires 5

**Files affected**:
- `crates/pdftract-core/src/page_extraction_error.rs:267` - Conflicting From implementation
- `crates/pdftract-core/src/extract.rs:203, 838, 846, 1868, 1876, 2191, 2199` - Method/type errors

---

## Files with Highest Warning Counts

1. **crates/pdftract-core/src/extract.rs** - 30+ warnings
2. **crates/pdftract-core/src/font/type3_rasterizer.rs** - 20+ warnings
3. **crates/pdftract-core/src/forms/mod.rs** - 30+ warnings
4. **crates/pdftract-core/src/signature/mod.rs** - 30+ warnings
5. **crates/pdftract-core/src/parser/xref.rs** - 20+ warnings
6. **crates/pdftract-core/src/parser/ocg.rs** - 15+ warnings
7. **crates/pdftract-core/src/parser/pages.rs** - 15+ warnings
8. **crates/pdftract-core/src/layout/reading_order.rs** - 10+ warnings
9. **crates/pdftract-core/src/parser/outline.rs** - 10+ warnings
10. **crates/pdftract-core/src/font/type3_rasterizer_test.rs** - 10+ warnings

---

## Recommended Fix Priority

### High Priority (blocking compilation)
1. Fix compilation errors first (8 errors in `extract.rs` and `page_extraction_error.rs`)

### Medium Priority (cleanest impact)
1. Remove unused imports (98+ warnings, can be auto-fixed with `cargo fix`)
2. Remove unused `mut` declarations (85+ warnings, easy cleanup)

### Low Priority (requires careful review)
1. Fix unused variables (70+ warnings, some may indicate incomplete code)
2. Fix unused assignments (15+ warnings, indicates code flow issues)
3. Address dead code (1 warning, may be intentional for future use)
4. Fix unreachable patterns (1 warning, code cleanup needed)
5. Fix doc comments (2 warnings, documentation issue)

---

## Raw Data Files

- `warnings-check.txt` - Output from `cargo check --all-targets`
- `warnings-test.txt` - Output from `cargo test --all-targets`

Both files contain identical warning sets with test-specific additions.
