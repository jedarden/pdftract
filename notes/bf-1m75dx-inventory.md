# Import Issues Inventory

**Bead ID:** bf-1m75dx  
**Generated:** 2026-08-09  
**Source:** `cargo check --tests` and `cargo clippy --tests`

## Executive Summary

- **Total Issues:** 99
- **Compilation Errors (Missing Imports):** 1
- **Warnings (Unused Imports):** 98
- **Severity:** 1 ERROR, 98 WARNINGS

### Breakdown by Crate

| Crate | Unused Imports | Compilation Errors |
|-------|----------------|-------------------|
| pdftract-core | 62 | 0 |
| pdftract-cli | 22 | 1 |
| pdftract-py | 14 | 0 |
| **Total** | **98** | **1** |

---

## CRITICAL: Compilation Errors (Must Fix)

### 1. Missing Type: `Path` in `crates/pdftract-cli/src/middleware/audit.rs`

**Location:** Line 190, column 34  
**Error:** `error[E0433]: cannot find type Path in this scope`  
**Context:**
```rust
AuditLogWriter::open(Path::new("/dev/stdout")).unwrap(),
```

**Required Fix:** Add import at top of file:
```rust
use std::path::Path;
```

**Impact:** BLOCKS compilation - MUST fix before any merge

---

## Files with Most Unused Imports (Top 10)

| File | Count | Severity |
|------|-------|----------|
| `crates/pdftract-core/src/font/type3_rasterizer.rs` | 17 | Warning |
| `crates/pdftract-core/src/font/type3_rasterizer_test.rs` | 6 | Warning |
| `crates/pdftract-py/tests/test_search_scaffold.rs` | 5 | Warning |
| `crates/pdftract-core/src/parser/stream.rs` | 4 | Warning |
| `crates/pdftract-py/tests/test_search_integration.rs` | 4 | Warning |
| `crates/pdftract-cli/tests/test_encryption_unsupported.rs` | 3 | Warning |
| `crates/pdftract-cli/tests/conformance.rs` | 3 | Warning |
| `crates/pdftract-core/tests/test_cycle_detection.rs` | 3 | Warning |
| `crates/pdftract-core/tests/xref_integration_test.rs` | 3 | Warning |
| `crates/pdftract-core/tests/object_parser_proptest.rs` | 3 | Warning |

---

## Detailed Inventory by File

### pdftract-core (62 unused imports)

#### `crates/pdftract-core/src/font/type3_rasterizer.rs` (17 issues)
- Line 2214: `crate::parser::object::intern`
- Line 2239: `PdfDict`
- Line 2354: `std::sync::Arc`
- Line 2398: `std::sync::Arc`
- Line 2868: `Mutex`
- Line 3073: `PdfDict`
- Line 3131: `PdfDict`, `PdfStream`
- Line 3132: `crate::parser::xref::XrefResolver`
- Line 3134: `std::sync::Arc`
- Line 3161: `PdfDict`
- Line 3190: `PdfDict`
- Line 3209: `std::sync::Arc`
- Line 3278: `crate::parser::stream::PdfSource`
- Line 3634: `std::sync::Arc`
- Line 3669: `std::sync::Arc`
- Line 3702: `std::sync::Arc`
- Line 3131: Multiple unused in single statement

#### `crates/pdftract-core/src/font/type3_rasterizer_test.rs` (6 issues)
- Line 21: `AtomicBool`, `AtomicU64`
- Line 23: `crate::font::encoding::NamedEncoding`
- Line 24: `DocumentContext`, `StreamResolverFn`
- Line 26: `crate::graphics_state::Matrix3x3`

#### `crates/pdftract-core/src/parser/stream.rs` (4 issues)
- Line 1914: `std::hash::Hasher`
- Line 3983: `secrecy::ExposeSecret`
- Line 5104: `secrecy::SecretString`
- Line 6125: `Jbig2GlobalsRef`

#### Other pdftract-core files (35 issues across 35 files)
- `annotation/json.rs`: `DestArray`
- `cache/key.rs`: `Map`
- `cache/lru.rs`: `entry_path`
- `detection.rs`: `ObjRef`
- `encryption/detection.rs`: `DiagCode`
- `parser/pages.rs`: `intern`
- `parser/resources.rs`: `PdfDict`
- `parser/xref.rs`: ~~`MemorySource`~~ (FALSE POSITIVE - restored, used 12× in test fixtures), `crate::parser::object::intern`
- `table/output.rs`: `TableSpan`, `crate::table::Segment`
- `attachment/filespec.rs`: `PdfDict`
- `attachment/name_tree.rs`: `PdfDict`
- `audit.rs`: `std::io::Cursor`
- `decoder/jbig2.rs`: `indexmap::indexmap`
- `extract.rs`: `crate::diagnostics::DiagCode`
- `forms/mod.rs`: `std::sync::Arc`
- `layout/correction.rs`: `super::*`
- `output/ndjson/frames.rs`: `std::io::Cursor`
- `output/ndjson/pipeline.rs`: `super::*`
- `page_class.rs`: `std::hash::Hasher`
- `parser/ocg.rs`: `std::sync::Arc`
- `source/mmap.rs`: `std::fs`
- `sdk.rs`: `super::*`
- `span/mod.rs`: `crate::font::UnicodeSource`
- Plus 13 additional single-issue files

---

### pdftract-cli (22 unused imports + 1 compilation error)

#### **CRITICAL:** `crates/pdftract-cli/src/middleware/audit.rs`
- **Line 190:** MISSING import `std::path::Path` (COMPILATION ERROR)

#### `crates/pdftract-cli/tests/test_encryption_unsupported.rs` (3 issues)
- Line 10: `pdftract_cli::password`
- Line 12: `DIAGNOSTIC_CATALOG`, `DiagCode`, `DiagInfo`, `Diagnostic`, `DiagnosticsCollector`, `ObjRef`, `Severity`

#### `crates/pdftract-cli/tests/conformance.rs` (3 issues)
- Line 11: `std::collections::HashMap`
- Line 13: `PathBuf`, `Path`

#### `crates/pdftract-cli/src/main.rs` (2 issues)
- Line 4: `std::io::Write`
- Line 1373: `std::io::Write`

#### Other pdftract-cli files (14 issues across 14 files)
- `src/cache_cmd.rs`: `CacheIndex`
- `src/mcp/tools/registry.rs`: `ERROR_NOT_YET_IMPLEMENTED`
- `src/serve.rs`: `HeaderMap`, `HeaderValue`, `Method`, `body::Body`
- `src/mcp/root.rs`: `std::io::Write`
- `tests/test_scientific_paper.rs`: `Path`, `super::*`
- `tests/test_slide_deck.rs`: `Path`, `super::*`
- `tests/test_contract.rs`: `Path`, `super::*`
- `tests/TH-09-inspector-xss.rs`: `Command`
- `tests/test_form.rs`: `Path`
- `tests/root-path-protection.rs`: `PathBuf`
- Plus 5 additional single-issue files

---

### pdftract-py (14 unused imports)

#### `crates/pdftract-py/tests/test_search_scaffold.rs` (5 issues)
- Line 13: `pdftract::PyPdfProcessor`
- Line 17: `CorruptPdfError`, `EncryptionError`, `PdftractError`, `ReceiptVerifyError`, `RemoteFetchInterruptedError`, `SourceUnreachableError`, `TlsError`, `UnsupportedOperationError`
- Line 22: `PyAny`, `PyResult`, `Python`

#### `crates/pdftract-py/tests/test_search_integration.rs` (4 issues)
- Line 10: `AttachmentJson`, `ExtractionOptions`, `PageResult`, `TableJson`
- Line 16: `PyResult`, `Python`, `types::PyDict`

#### Other pdftract-py files (5 issues)
- `src/extract.rs`: `FromPyObject`
- `src/extract_markdown.rs`: `FromPyObject`
- `src/extract_stream.rs`: `FromPyObject`
- `src/lib.rs`: `FromPyObject`
- `src/extract_text.rs`: `FromPyObject`

---

## Categorization by Type

### 1. Standard Library Imports (24)
- `std::path::Path`, `std::path::PathBuf` (6 instances)
- `std::io::Cursor`, `std::io::Write` (5 instances)
- `std::sync::Arc`, `std::sync::Mutex` (8 instances)
- `std::fs`, `std::hash::Hasher`, `std::collections::HashMap` (5 instances)

### 2. Third-Party Imports (12)
- `secrecy::ExposeSecret`, `secrecy::SecretString`
- `indexmap::indexmap`
- `anyhow::Result`
- PyO3 types: `FromPyObject`, `PyResult`, `Python`, `PyAny`, `types::PyDict`
- `proptest::prelude::*`

### 3. Internal pdftract Imports (62)
- Parser objects: `PdfDict`, `PdfObject`, `PdfStream`, `ObjRef`, `intern` (15 instances)
- Diagnostics: `DiagCode`, `Diagnostic`, etc. (8 instances)
- Font-related: `UnicodeSource`, `NamedEncoding` (4 instances)
- Stream/resolver: `PdfSource`, `XrefResolver`, `MemorySource` (5 instances)
- Test utilities: `super::*` (8 instances)
- Other internal types (22 instances)

---

## Recommended Fix Strategy

### Priority 1: Fix Compilation Error (5 minutes)
1. Add `use std::path::Path;` to `crates/pdftract-cli/src/middleware/audit.rs`

### Priority 2: High-Impact Files (30 minutes)
These files have 4+ unused imports and benefit most from cleanup:
1. `font/type3_rasterizer.rs` (17 issues) - Remove unused Arc/PdfDict imports
2. `font/type3_rasterizer_test.rs` (6 issues) - Remove unused test imports
3. `test_search_scaffold.rs` (5 issues) - Remove unused error types
4. `parser/stream.rs` (4 issues) - Remove unused secrecy/jbig2 imports

### Priority 3: Bulk Cleanup (1-2 hours)
Use `cargo fix` for automated fixes:
```bash
cargo fix --lib --tests --allow-dirty
```

This will handle:
- Simple unused imports (single-item removals)
- Basic dead code elimination
- Standard formatting issues

### Priority 4: Manual Review (30 minutes)
Items requiring manual review:
- Ambiguous imports (multiple `Path` types available)
- Complex macro-generated imports
- Conditional compilation imports

---

## Automated Fix Commands

```bash
# Apply automatic fixes for unused imports
cargo fix --lib -p pdftract-core --tests --allow-dirty
cargo fix --lib -p pdftract-cli --tests --allow-dirty
cargo fix --lib -p pdftract-py --tests --allow-dirty

# Run cargo clippy with automatic fixes
cargo clippy --fix --tests --allow-dirty -- -D warnings
```

---

## Verification Checklist

After fixes:
- [ ] `cargo check --tests` passes without errors
- [ ] `cargo clippy --tests` shows 0 unused import warnings
- [ ] All test suites still pass: `cargo test --all-targets`
- [ ] No new compilation errors introduced

---

## Notes

- **No ambiguous imports detected** - all missing imports have clear resolution paths
- **Test files show highest density** of unused imports (expected for test utilities)
- **Type3 rasterizer** is a hot spot due to complex test fixtures with many mock objects
- **PyO3 bindings** have unused imports from template code

**Estimated Total Fix Time:** 2-3 hours (including verification)  
**Risk Level:** Low (mostly warning-level issues, 1 blocking error)