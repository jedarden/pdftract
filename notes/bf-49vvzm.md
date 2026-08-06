# SDK Type Exports and Structure Exploration

**Bead:** bf-49vvzm
**Date:** 2026-08-06
**Status:** COMPLETE

## Overview

Explored the comprehensive SDK type system across the Rust core implementation and language bindings. This exploration covers the canonical SDK contract defined in Rust (`sdk.rs`) and how types are exported to users through various language SDKs.

## SDK Module Structure

### Canonical SDK Contract (Rust)

The SDK contract is defined in `/home/coding/pdftract/crates/pdftract-core/src/sdk.rs` - this is the authoritative specification that all language SDKs implement.

**9 Core Methods:**
1. `extract()` - Full JSON extraction to structured Document
2. `extract_text()` - Plain text extraction
3. `extract_markdown()` - Markdown format extraction
4. `extract_stream()` - Streaming page-by-page extraction
5. `search()` - Pattern search with Match results
6. `get_metadata()` - PDF metadata retrieval
7. `hash()` - PDF fingerprinting/computing hash
8. `classify()` - Page classification
9. `verify_receipt_from_path()` - Cryptographic receipt verification

### Language Bindings

Multiple language SDKs implement this contract:
- **Python**: `/home/coding/pdftract/crates/pdftract-py/` (PyO3 bindings)
- **JavaScript/TypeScript**: `/home/coding/pdftract/pdftract-node/`
- **Go**: `/home/coding/pdftract/pdftract-go/`
- **Java**: `/home/coding/pdftract/pdftract-java/`
- **C#/.NET**: `/home/coding/pdftract/pdftract-dotnet/`
- **Ruby**: `/home/coding/pdftract/pdftract-ruby/`
- **PHP**: `/home/coding/pdftract/pdftract-php/`
- **Swift**: `/home/coding/pdftract/swift-sdk/`

## Exported Types (Public API)

### SDK-Specific Types (from `sdk.rs`)

**File**: `/home/coding/pdftract/crates/pdftract-core/src/sdk.rs`

| Type | Definition | Purpose |
|------|-----------|---------|
| `SearchMatch` | `pub struct SearchMatch` | Single search match result with page_index, span_index, text, bbox |
| `PdfMetadata` | `pub struct PdfMetadata` | Document metadata (page_count, is_encrypted, is_tagged, has_forms) |

### Extraction Types (from `extract.rs`)

**File**: `/home/coding/pdftract/crates/pdftract-core/src/extract.rs`

| Type | Definition | Purpose |
|------|-----------|---------|
| `ExtractionResult` | `pub struct ExtractionResult` | Complete extraction result with pages, metadata, signatures, form_fields, links, attachments, threads |
| `PageResult` | `pub struct PageResult` | Single page with index, width, height, rotation, spans, blocks |
| `ExtractionMetadata` | `pub struct ExtractionMetadata` | Extraction metadata |

### Schema JSON Types (from `schema/mod.rs`)

**File**: `/home/coding/pdftract/crates/pdftract-core/src/schema/mod.rs`

| Type | Definition | Purpose |
|------|-----------|---------|
| `SpanJson` | `pub struct SpanJson` | Text span with text, bbox, font, size, color, rendering_mode, confidence, confidence_source, lang, flags, receipt, column |
| `BlockJson` | `pub struct BlockJson` | Semantic block with kind, text, bbox, level, reading_order, table_index, spans |
| `CellJson` | `pub struct CellJson` | Table cell with bbox, text, spans, row, col, rowspan, colspan, is_header_row |
| `RowJson` | `pub struct RowJson` | Table row with cells, bbox |
| `TableJson` | `pub struct TableJson` | Table with bbox, columns, rows |
| `AttachmentJson` | `pub struct AttachmentJson` | Embedded file attachment |
| `LinkJson` | `pub struct LinkJson` | Hyperlink annotation |
| `ThreadJson` | `pub struct ThreadJson` | Article thread chain |
| `BeadJson` | `pub struct BeadJson` | Individual bead in thread |
| `FormFieldJson` | `pub struct FormFieldJson` | Form field data |
| `SignatureJson` | `pub struct SignatureJson` | Digital signature |
| `JavascriptActionJson` | `pub struct JavascriptActionJson` | JavaScript action for security review |

### Options Types (from `options.rs`)

**File**: `/home/coding/pdftract/crates/pdftract-core/src/options.rs`

| Type | Definition | Purpose |
|------|-----------|---------|
| `ExtractionOptions` | `pub struct ExtractionOptions` | Extraction configuration (OCR, password, etc.) |
| `OutputOptions` | `pub struct OutputOptions` | Output filtering options (include_headers, include_footers, etc.) |
| `ReceiptsMode` | `pub enum ReceiptsMode` | Receipt generation mode (Off, Lite, SvgClip) |

### Page Classification Types (from `page_class.rs`)

**File**: `/home/coding/pdftract/crates/pdftract-core/src/page_class.rs`

| Type | Definition | Purpose |
|------|-----------|---------|
| `PageClass` | `pub enum PageClass` | Page category (Acoustic, Archaeology, ...) |
| `PageClassification` | `pub struct PageClassification` | Classification result with category, confidence, tags |

### Form Types (from `forms/`)

**File**: `/home/coding/pdftract/crates/pdftract-core/src/forms/`

| Type | Definition | Purpose |
|------|-----------|---------|
| `AcroFieldType` | `pub enum AcroFieldType` | Form field type (Text, Button, Choice, Signature) |
| `AcroFormField` | `pub struct AcroFormField` | Form field definition |
| `ChoiceValue` | `pub struct ChoiceValue` | Choice field value |
| `FormFieldValue` | `pub enum FormFieldValue` | Form field value (Text, Buttons, Choice) |

## Public API Surface (lib.rs Re-exports)

**File**: `/home/coding/pdftract/crates/pdftract-core/src/lib.rs`

The `lib.rs` file re-exports all key types for convenient access:

```rust
// Key extraction types
pub use extract::{
    extract_pdf, extract_pdf_ndjson, extract_pdf_streaming, extract_text,
    ExtractionMetadata, ExtractionResult, PageResult,
};

// Options types
pub use options::{ExtractionOptions, OutputOptions, ReceiptsMode};

// Page classification types
pub use page_class::{page_type_string, PageClass, PageClassification};

// Schema JSON types
pub use schema::{
    AttachmentJson, BeadJson, BlockJson, CellJson, ExtractionQuality,
    RowJson, SpanJson, SpanRef, TableJson, ThreadJson,
};

// Form types
pub use forms::{
    combine, walk_acroform_fields, AcroFieldType, AcroFormField,
    ChoiceValue, FormFieldValue,
};

// Markdown types
pub use markdown::{
    block_to_markdown, form_fields_to_markdown, page_to_markdown,
    page_to_markdown_with_links, parse_anchors, span_to_markdown,
    Anchor, MarkdownOptions,
};

// Other utility types
pub use confidence::{map_confidence_source, ConfidenceSource};
pub use document::{Document, PageExtraction, PageIter, PdfExtractor};
pub use font::std14::{get_std14_metrics, NamedEncoding, Std14Metrics};
pub use parser::pages::{count_pages_tree, LazyPageIter, PageDict, DEFAULT_MEDIABOX};
pub use table::{GridCandidate, PageContext as TablePageContext, TableDetector};
pub use text::{serialize_document_text, serialize_page_text, TextOptions};
pub use word_boundary::{TextState, WordBoundaryDetector, WordBoundaryManager};

// Source types
pub use source::{FileSource, MmapSource};

// Phase 3 Glyph types
pub use glyph::{emit_glyph, new_raw_glyph_list, Glyph};

// Phase 4.1 Span types
pub use span::{merge_glyphs_to_spans, CssHexColor, Span};
```

## User Import Patterns

### Rust Users

```rust
// Pattern 1: Direct SDK module import
use pdftract_core::sdk::{extract, extract_text, extract_markdown, search, get_metadata, hash, classify};

// Pattern 2: Type imports via re-exports
use pdftract_core::{ExtractionResult, PageResult, ExtractionOptions};

// Pattern 3: Module-specific imports
use pdftract_core::schema::{SpanJson, BlockJson};
use pdftract_core::options::{ExtractionOptions, OutputOptions};
use pdftract_core::page_class::{PageClass, PageClassification};
```

### Python Users

```python
# ✅ CORRECT - import from main module
import pdftract
from pdftract import Document, Page, Span

# ❌ INCORRECT - importing from submodules
from pdftract.types import Document  # Works but not recommended
```

## Type Structure Confirmation

✅ **All SDK types are proper Rust structs** - These are NOT type aliases or dictionaries.

The SDK uses:
- `pub struct` for main data types (`ExtractionResult`, `PageResult`, `SpanJson`, etc.)
- `pub enum` for variant types (`PageClass`, `ReceiptsMode`, `ConfidenceSource`, etc.)
- Proper field definitions with public visibility
- Serde serialization support (`#[derive(Serialize, Deserialize)]`)
- JSON Schema generation support (`#[cfg_attr(feature = "schemars"), derive(schemars::JsonSchema)]`)

This means language SDKs can generate proper native classes/structs from these definitions, not just dictionaries.

## File Locations Summary

| Component | Location |
|-----------|----------|
| Main SDK Module | `/home/coding/pdftract/crates/pdftract-core/src/sdk.rs` |
| Public API | `/home/coding/pdftract/crates/pdftract-core/src/lib.rs` |
| Schema Types | `/home/coding/pdftract/crates/pdftract-core/src/schema/mod.rs` |
| Options | `/home/coding/pdftract/crates/pdftract-core/src/options.rs` |
| Extraction Types | `/home/coding/pdftract/crates/pdftract-core/src/extract.rs` |
| Page Classification | `/home/coding/pdftract/crates/pdftract-core/src/page_class.rs` |
| Forms | `/home/coding/pdftract/crates/pdftract-core/src/forms/` |
| Python SDK | `/home/coding/pdftract/crates/pdftract-py/python/pdftract/` |
| Python Types | `/home/coding/pdftract/crates/pdftract-py/python/pdftract/types.py` |
| Python Tests | `/home/coding/pdftract/crates/pdftract-py/tests/test_types.py` |
| SDK Contract Doc | `/home/coding/pdftract/docs/notes/sdk-contract.md` |

## Smoke Test Implications

For SDK smoke testing, we should verify:

1. **Core SDK types can be imported**: `ExtractionResult`, `PageResult`, `SpanJson`, `BlockJson`, etc.
2. **Types have correct field structure**: Verify public fields exist
3. **Types are properly serialized**: Test serde JSON serialization/deserialization
4. **SDK functions return correct types**: Test that `extract()` returns `ExtractionResult`, etc.
5. **Type compatibility**: Ensure types match across different SDK implementations

## Verification: Existing Test Coverage

The Python SDK already has smoke tests in `crates/pdftract-py/tests/test_types.py`:
- `test_extract_returns_typed_document()` - Verifies Document/Page/Span hierarchy
- `test_extract_returns_typed_document_with_valid_minimal()` - Redundant check with different fixture

This confirms the SDK is correctly returning typed objects rather than raw dicts.

## Conclusion

The SDK type system is well-structured with:
- ✅ Clear canonical contract defined in Rust (`sdk.rs`)
- ✅ Comprehensive type exports across multiple modules
- ✅ Proper Rust structs (not dicts) with Serde serialization
- ✅ Re-export pattern for convenient user access
- ✅ Multiple language bindings implementing the same contract
- ✅ Existing smoke test coverage in Python SDK

All acceptance criteria met:
- ✅ List of all exported types documented above
- ✅ File locations for each type identified
- ✅ Import pattern documented for Rust and Python users
- ✅ Types are proper structs/classes (frozen dataclasses in Python), not dicts
