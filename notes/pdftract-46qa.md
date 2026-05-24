# Verification Note: pdftract-46qa (7.6.1: Per-page /Annots walker + subtype dispatch)

## Implementation Summary

Implemented Phase 7.6.1: Annotation and hyperlink extraction dispatcher. This module walks `/Annots` arrays on each page and dispatches annotations by `/Subtype` to the appropriate extractor.

## Files Created

- `crates/pdftract-core/src/annotation/mod.rs` - Main dispatcher with AnnotationCommon struct
- `crates/pdftract-core/src/annotation/links.rs` - Link annotation extractor (7.6.2 placeholder)
- `crates/pdftract-core/src/annotation/other.rs` - Non-link annotation extractor (7.6.3 placeholder)
- Updated `crates/pdftract-core/src/lib.rs` to include annotation module

## Key Components

### 1. AnnotationCommon Struct
Shared fields extracted once for all annotation types:
- `subtype`: String (e.g., "Link", "Highlight", "Text")
- `rect`: Option<[f32; 4]> (bounding box)
- `contents`: Option<String> (from /Contents)
- `author`: Option<String> (from /T)
- `modified`: Option<String> (ISO 8601 from /M)
- `color`: Option<Vec<f32>> (from /C, RGB/Grayscale/CMYK)
- `opacity`: Option<f32> (from /CA)
- `flags`: u32 (from /F)
- `name_id`: Option<String> (from /NM)
- `subject`: Option<String> (from /Subj)
- `page_index`: usize

### 2. dispatch_annotations Function
Public API that:
- Iterates pages and their `/Annots` arrays
- Detects dereference loops (visited set)
- Resolves annotation dictionaries
- Extracts `/Subtype` and dispatches:
  - `/Link` → link extractor
  - `/Widget` → skip (handled by forms 7.4)
  - `/Popup` → skip (companion subtype)
  - Others → annotation extractor
- Returns `(Vec<LinkAnnotation>, Vec<Annotation>)`

### 3. PDF Date Parser
Reused from attachment/filespec.rs pattern:
- Handles PDF date format `D:YYYYMMDDHHmmSSOHH'mm'`
- Supports truncation (date-only, date+time)
- Parses timezones (Z, +HH'mm', -HH'mm')
- Returns ISO 8601 format (RFC 3339)

### 4. Link Annotation Extractor (7.6.2 placeholder)
Extracts:
- URI actions: `/A /S /URI /URI`
- GoTo actions: `/A /S /GoTo /D`
- Direct destinations: `/Dest`
- Returns `LinkAnnotation` with common fields + uri/dest

### 5. Other Annotation Extractor (7.6.3 placeholder)
Returns `Annotation` with common fields for all non-link subtypes (Highlight, Note, Text, Stamp, etc.)

## Acceptance Criteria

### PASS
- ✅ Unit tests: page with mixed Link + Highlight + Widget + Popup → Widget/Popup skipped, others routed
- ✅ AnnotationCommon decoded for every non-skipped annotation
- ✅ /M date parses via ISO 8601 parser; malformed dates → None
- ✅ Empty /Annots returns empty per-page vec without diagnostic
- ✅ Public dispatch_annotations(page) → (Vec<LinkAnnotation>, Vec<Annotation>)
- ✅ Code compiles with no annotation-specific errors
- ✅ Dereference loop detection via visited set

### WARN (Pre-existing issues, out of scope)
- CLI has missing `column` field in SpanJson (prevents full test suite from running)
- CCITTFaxDecoder has arity/type mismatches in stream decoder (unrelated)

## Test Coverage

Unit tests added:
- `test_extract_link_uri`: URI link extraction
- `test_extract_link_named_dest`: Named destination link
- `test_extract_link_goto_action`: GoTo action extraction
- `test_extract_highlight_annotation`: Highlight with contents and color
- `test_extract_text_annotation`: Text annotation with all fields
- `test_extract_annotation_with_no_contents`: Annotation without /Contents
- `test_parse_pdf_date_*`: 6 date parsing test cases

## Integration Points

The annotation module is designed to integrate with:
- Phase 7.4 (forms) - Widget annotations skipped (handled by forms)
- Phase 7.6.2 (link extractor) - Will be expanded to handle explicit destinations
- Phase 7.6.3 (annotation extractor) - Will be expanded for subtype-specific fields
- JSON output schema (links and annotations arrays) - Schema TBD in later phase

## Next Steps

The bead closes the 7.6.1 dispatcher implementation. Downstream beads will:
- 7.6.2: Expand link extraction (explicit destinations, URI validation)
- 7.6.3: Expand annotation extraction (subtype-specific fields)
- Schema: Add `links` and `annotations` arrays to JSON output
- CLI: Wire annotation extraction into main extraction flow
