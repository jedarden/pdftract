# pdftract Diagnostic Codes

This document catalogs all diagnostic codes emitted by pdftract during PDF extraction. Each diagnostic has a stable SCREAMING_SNAKE_CASE identifier, a severity level, and suggested user action.

## Diagnostic Format

All diagnostics follow this structure:

```json
{
  "code": "DIAGNOSTIC_CODE",
  "message": "Human-readable description",
  "severity": "info|warning|error|fatal",
  "page_index": null | 0-based page number,
  "location": null | {"object_number": N, "generation_number": G},
  "hint": null | "Suggested action"
}
```

## Code Categories

### STRUCT_* — PDF Structure Errors

Errors related to PDF syntax, object parsing, and document structure.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `STRUCT_INVALID_NAME` | Warning | Invalid name character or malformed name object | 1.1 |
| `STRUCT_INVALID_HEX` | Warning | Invalid hex character in hex string or name escape | 1.1 |
| `STRUCT_INVALID_OCTAL` | Warning | Invalid octal escape sequence in literal string | 1.1 |
| `STRUCT_INVALID_STREAM_HEADER` | Warning | Invalid stream header (stream keyword not followed by proper newline) | 1.1 |
| `STRUCT_UNEXPECTED_BYTE` | Warning | Unexpected byte (e.g., stray `>` not part of `>>`) | 1.1 |
| `STRUCT_UNEXPECTED_EOF` | Warning | Unexpected end of file while parsing a token | 1.1 |
| `STRUCT_UNTERMINATED_STRING` | Warning | Unterminated literal string (missing closing paren) | 1.1 |
| `STRUCT_MISSING_KEY` | Warning | Missing required dictionary key | 1.4 |
| `STRUCT_CIRCULAR_REF` | Warning | Circular reference detected (A → B → A) | 1.2 |
| `STRUCT_XOBJECT_CYCLE` | Warning | Form XObject cycle detected | 3.3 |
| `STRUCT_DEPTH_EXCEEDED` | Warning | Dictionary nesting depth exceeds limit | 1.2 |
| `STRUCT_INVALID_DICT_VALUE` | Warning | Invalid dictionary value (missing value after key) | 1.2 |
| `STRUCT_INVALID_DICT_KEY` | Warning | Invalid dictionary key (not a name object) | 1.2 |
| `STRUCT_INVALID_INDIRECT_HEADER` | Warning | Invalid indirect object header (`N G obj`) | 1.2 |
| `STRUCT_INTEGER_OVERFLOW` | Warning | Integer overflow during parsing | 1.2 |
| `STRUCT_REAL_INVALID` | Warning | Invalid real number literal | 1.1 |
| `STRUCT_INVALID_NUMBER` | Warning | Invalid numeric literal | 1.1 |
| `STRUCT_INVALID_ASCII85` | Warning | Invalid ASCII85 character or malformed stream | 1.5 |
| `STRUCT_INVALID_OBJSTM` | Warning | Invalid object stream format | 1.2 |
| `STRUCT_INVALID_GEOMETRY` | Warning | Invalid geometry value (NaN or Inf in MediaBox/CropBox/Rotate) | 1.7 |
| `STRUCT_INVALID_TYPE` | Warning | Invalid object type (expected type not found) | 5.2.1 |
| `STRUCT_INVALID_UTF16` | Warning | Invalid UTF-16BE encoding in string | 1.4 |
| `STRUCT_UNRESOLVED_DESTINATION` | Warning | Unresolved named destination | 1.4 |
| `STRUCT_NON_GOTO_OUTLINE` | Warning | Non-GoTo action in outline | 1.4 |
| `STRUCT_INVALID_PDFDOC_ENCODING` | Warning | Invalid PDFDocEncoding in string | 1.4 |
| `STRUCT_HYBRID_CONFLICT` | Warning | Hybrid xref conflict: traditional and stream disagree | 1.3 |
| `STRUCT_INCOMPLETE_COVERAGE` | Info | StructTree coverage below 80% with /Suspects true | 7.1.4 |
| `STRUCT_INVALID_PREV_OFFSET` | Warning | Invalid /Prev offset in xref chain | 1.3 |
| `STRUCT_INVALID_BDC_OPERAND` | Info | Invalid BDC operand | 3.4 |

### XREF_* — Cross-Reference Table Errors

Errors related to the xref table and trailer.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `XREF_INVALID_HEADER` | Warning | Invalid xref keyword or header | 1.3 |
| `XREF_INVALID_ENTRY` | Warning | Malformed xref entry (not 20 bytes, bad format) | 1.3 |
| `XREF_INVALID_SUBSECTION_HEADER` | Warning | Invalid subsection header (not "start count") | 1.3 |
| `XREF_OBJECT_ZERO_NOT_FREE` | Warning | Object 0 is not free (violates PDF spec) | 1.3 |
| `XREF_TRAILER_NOT_FOUND` | Warning | Trailer dictionary not found or malformed | 1.3 |
| `XREF_TRUNCATED` | Warning | Truncated xref table (unexpected EOF) | 1.3 |
| `XREF_REPAIRED` | Info | Xref was reconstructed via forward scan (EC-07) | 1.3 |
| `XREF_LINEARIZED_NO_FORWARD_SCAN` | Warning | Forward scan disabled for linearized files | 1.3 |
| `XREF_REMOTE_NO_FORWARD_SCAN` | Warning | Forward scan disabled for HTTP sources | 1.3 |
| `XREF_INVALID_STREAM_FORMAT` | Warning | Invalid xref stream format | 1.3 |
| `XREF_INVALID_STREAM_ENTRY` | Warning | Invalid xref stream entry | 1.3 |

### STREAM_* — Stream Decoder Errors

Errors related to stream decompression and filters.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `STREAM_DECODE_ERROR` | Warning | Stream decompression failed (corrupt data) | 1.5 |
| `STREAM_BOMB` | Error | Decompression bomb limit exceeded | 1.5 |
| `STREAM_UNKNOWN_FILTER` | Warning | Unknown filter name | 1.5 |
| `STREAM_INVALID_PARAMS` | Warning | Invalid filter parameters | 1.5 |
| `STREAM_INVALID_JPEG` | Warning | JPEG data has invalid or missing markers | 1.5 |
| `STREAM_INVALID_CCITT` | Warning | CCITT fax data has invalid or missing parameters | 1.5 |
| `STREAM_TRUNCATED` | Warning | Stream data truncated | 1.5 / 5.2.1 |

### ENCRYPTION_* — Encryption Errors

Errors related to PDF encryption and passwords.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `ENCRYPTION_UNSUPPORTED` | Fatal | Unsupported encryption or no password supplied | 1.4 |
| `ENCRYPTION_WRONG_PASSWORD` | Fatal | Password incorrect | 1.4 |

### PAGE_* — Page-Level Errors

Errors related to page structure and properties.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `PAGE_OUT_OF_RANGE` | Error | Page number out of range | 1.8 |
| `PAGE_INVALID_COUNT` | Warning | Invalid /Count in /Pages tree | 1.4 |
| `PAGE_INVALID_ROTATE` | Warning | Invalid /Rotate value (not multiple of 90) | 1.4 |

### FONT_* — Font Pipeline Errors

Errors related to font parsing and glyph mapping.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `FONT_GLYPH_UNMAPPED` | Warning | Glyph could not be mapped to Unicode | 2.2 |
| `FONT_NOT_FOUND` | Warning | Font not found or couldn't be parsed | 2.1 |
| `FONT_INVALID_CMAP` | Warning | Invalid CMap format | 2.2 |
| `FONT_PARSE_FAILED` | Warning | Font program parsing failed | 2.1 |
| `FONT_UNSUPPORTED` | Warning | Font type not supported for embedded loading | 2.1 |
| `FONT_CIDTOGIDMAP_TRUNCATED` | Warning | CIDToGIDMap stream has odd byte count | 2.1 |
| `ENCODING_DIFFERENCE_OUT_OF_RANGE` | Warning | Character code in /Differences exceeds valid range | 2.2 |
| `FONT_TYPE3_WIDTHS_LENGTH_MISMATCH` | Warning | Type3 font /Widths array length mismatch | 2.4 |

### CJK_* — CJK Encoding Errors

Errors related to CJK character encoding.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `CJK_DECODE_MALFORMED` | Warning | Malformed byte sequence in CJK encoding | 2.3 |

### OCR_* — OCR Pipeline Errors

Errors related to OCR processing.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `OCR_JBIG2_UNSUPPORTED` | Warning | JBIG2 decoder not available | 1.5 / 5.2 |
| `OCR_JPX_UNSUPPORTED` | Warning | JPEG2000 (JPX) decoder not available | 1.5 / 5.2 |
| `OCR_CCITT_UNSUPPORTED` | Warning | CCITT fax decoder not available | 1.5 / 5.2 |
| `OCR_TESSERACT_FAILED` | Warning | Tesseract OCR failed | 5.4 |
| `OCR_BROKENVECTOR_UNAVAILABLE` | Warning | OCR unavailable on broken-vector page | 4.7 |
| `OCR_LANGUAGE_UNAVAILABLE` | Warning | Requested OCR language pack not available | 5.4 |

### IMG_* — Image Processing Errors

Errors related to image extraction and processing.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `IMG_SOFTMASK_UNSUPPORTED` | Warning | Image soft mask not supported in direct compositing | 5.2.1 |
| `IMG_UNSUPPORTED_FORMAT` | Warning | Image format not supported | 5.2.1 |
| `IMG_DESKEW_OUT_OF_RANGE` | Warning | Deskew angle out of detectable range | 5.3.1 |
| `IMG_SOURCE_MIXED` | Warning | Image sources mixed in unexpected way | 5.3.2 |

### REMOTE_* — Remote Source Errors

Errors related to HTTP fetching and remote sources.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `REMOTE_FETCH_INTERRUPTED` | Error | HTTP fetch interrupted or failed | 1.8 |
| `REMOTE_NO_RANGE_SUPPORT` | Warning | Server does not support Range requests | 1.8 |
| `REMOTE_TLS_FAILED` | Fatal | TLS handshake failed | 1.8 |
| `REMOTE_DNS_FAILED` | Fatal | DNS resolution failed | 1.8 |
| `REMOTE_URL_PRIVATE_NETWORK` | Error | URL targets private network (SSRF protection) | 1.8 |

### GSTATE_* — Graphics State Errors

Errors related to graphics state operators.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `GSTATE_STACK_OVERFLOW` | Warning | Graphics state stack overflow | 3.1 |
| `GSTATE_STACK_UNDERFLOW` | Warning | Graphics state stack underflow | 3.1 |
| `GSTATE_BT_ET_MISMATCH` | Warning | Mismatched BT/ET pair | 3.1 |
| `CM_ARG_COUNT` | Warning | Invalid argument count for cm operator | 3.1 |
| `CM_DEGENERATE` | Warning | Degenerate matrix (det == 0 or NaN) | 3.1 |
| `HORIZ_SCALING_ZERO` | Warning | Horizontal scaling set to zero (Tz 0) | 3.1 |
| `TEXT_RENDERING_MODE_CLAMPED` | Warning | Text rendering mode clamped to valid range | 3.1 |
| `TSTAR_ZERO_LEADING` | Warning | T* operator when leading == 0 | 3.1 |
| `FONT_RESOURCE_NOT_FOUND` | Warning | Font resource not found | 3.1 |
| `FONT_SIZE_ZERO_OR_NEGATIVE` | Warning | Font size zero or negative | 3.1 |
| `BT_NESTED` | Warning | BT operator nested inside another BT block | 3.1 |
| `ET_WITHOUT_BT` | Warning | ET operator without matching BT | 3.1 |
| `TEXT_SHOW_OUTSIDE_BT` | Warning | Text-show operator outside BT/ET block | 3.1 |

### LAYOUT_* — Layout and Reading Order Errors

Errors related to layout analysis and reading order.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `TAGGED_PDF_STRUCT_TREE_DEFERRED` | Info | Tagged PDF StructTree deferred to Phase 7 | 4.5 |
| `LAYOUT_READING_ORDER_AMBIGUOUS` | Warning | Reading order may be incorrect | 4.5 |
| `LAYOUT_LOW_READABILITY` | Warning | Low readability score | 4.7 |

### MCP_* — MCP Server Errors

Errors related to MCP server operations.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `MCP_TOOL_INVALID_PARAMS` | Error | MCP tool call has invalid parameters | 6.7 |
| `MCP_PATH_TRAVERSAL` | Error | MCP path traversal attempt | 6.7 |

### CACHE_* — Cache Errors

Errors related to caching operations.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `CACHE_ENTRY_CORRUPT` | Warning | Cache entry is corrupted | 6.9 |
| `CACHE_WRITE_FAILED` | Warning | Cache write failed | 6.9 |

### MARKED_CONTENT_* — Marked Content Errors

Errors related to marked content operators.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `EMC_WITHOUT_BMC` | Info | EMC operator without matching BMC/BDC | 3.4 |
| `MARKED_CONTENT_DEPTH_EXCEEDED` | Info | Marked-content stack depth exceeded | 3.4 |
| `UNKNOWN_MARKED_CONTENT_PROPS` | Info | Unknown marked-content property name | 3.4 |
| `MCID_REDEFINED` | Info | MCID redefined in same scope | 3.4 |

### PROFILE_* — Profile Errors

Errors related to profile configuration.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `PROFILE_SECRETS_FORBIDDEN` | Error | Profile YAML contains forbidden secret keys | 7.10 |
| `PROFILE_INVALID` | Error | Profile YAML is invalid or malformed | 5.6.2 |

### REPAIR_* — Repair Recovery

Errors related to document repair operations.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `REPAIR_RESCUED_FROM_BACKWARDS_XREF` | Info | Xref repaired from backwards scan | 1.3 |

### SECURITY_* — Security Diagnostics

Security-related diagnostics.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `JAVASCRIPT_PRESENT` | Info | JavaScript present in PDF (never executed) | 1.2 |

## Adding New Diagnostic Codes

When adding a new diagnostic code:

1. Choose a category prefix (STRUCT, STREAM, XREF, etc.)
2. Add the variant to the `DiagCode` enum in `crates/pdftract-core/src/diagnostics.rs`
3. Add the name mapping in `DiagCode::name()`
4. Add the category mapping in `DiagCode::category()`
5. Add the severity mapping in `DiagCode::severity()`
6. Add a catalog entry to `DIAGNOSTIC_CATALOG`
7. Add an entry to this document

**Code naming convention:** `CATEGORY_SPECIFIC_ISSUE` (SCREAMING_SNAKE_CASE)

**Severity levels:**
- `Info` — does not affect output validity
- `Warning` — output is usable but degraded
- `Error` — output for this region/page is invalid; other regions OK
- `Fatal` — extraction aborted, no usable output

## Programmatic Usage

Diagnostics can be consumed programmatically:

```python
import json

result = json.loads(pdftract_output)
for error in result.get('errors', []):
    code = error['code']
    severity = error['severity']
    page = error.get('page_index')
    
    if code == 'OCR_BROKENVECTOR_UNAVAILABLE':
        # Install Tesseract for OCR recovery
        print(f"Page {page}: Install Tesseract for OCR recovery")
```
