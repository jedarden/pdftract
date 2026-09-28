# pdftract Diagnostic Codes

This document is the normative diagnostic contract. The canonical
machine-readable surface is the serialized `DiagnosticJson` object (the
in-process equivalent is `Diagnostic`): each catalog-backed entry has a stable
`SCREAMING_SNAKE_CASE` identifier, a human-readable message, a severity level,
and a catalog hint. The same object shape is used unchanged in compact
metadata, full JSON `errors`, and the NDJSON footer `errors` array. The legacy
string array remains available as a compatibility projection; it is not a
competing diagnostic schema.

## Contract decision

pdftract emits structured diagnostics **alongside** the legacy string array.
Structured objects are the surface that new integrations MUST use for machine
classification. Existing integrations MAY continue to consume
`metadata.diagnostics`; that field is retained and its entries are exactly the
structured entries' `message` values in the same order. No code, severity,
page/location context, or hint is encoded into the legacy strings.

## Diagnostic Format

The canonical JSON/NDJSON diagnostic object follows this structure (shown with
every optional field populated):

```json
{
  "code": "STREAM_DECODE_ERROR",
  "message": "zlib stream truncated mid-inflation",
  "severity": "warning",
  "page_index": 3,
  "location": {"object_number": 12, "generation_number": 0},
  "hint": "Partial output returned for this stream; consider re-saving the PDF through a normalising tool"
}
```

The wire `severity` value is one of `info`, `warning`, `error`, or `fatal`.
Recoverable diagnostics are attached to the extraction result. A `fatal`
condition may instead make an extraction API return an error before an
`ExtractionResult` exists; callers must not assume that every fatal diagnostic
is available in one of these arrays.

| Field | JSON type | Serialized presence | Semantics |
|-------|-----------|---------------------|-----------|
| `code` | string | Always | Stable `DiagCode::name()` identifier in `SCREAMING_SNAKE_CASE`; classify on this value. |
| `message` | string | Always | Human-readable display text; it may contain emission-site context and is not a classification key. |
| `severity` | string | Always | Exactly `info`, `warning`, `error`, or `fatal`; derived from the typed code policy. |
| `page_index` | integer | When the owning page is known | Zero-based page index. It is omitted for document- or operation-level diagnostics. |
| `location` | object | When the originating indirect object is known | `{"object_number": u32, "generation_number": u16}`; this identifies a PDF object, not a byte offset. |
| `hint` | string | When the code has a catalog hint | Catalog guidance. Every current catalog row has a non-empty hint string, including rows whose text begins `None —`; consumers must tolerate omission for future codes. |

Optional fields are **omitted, never serialized as `null`**, when unavailable.
For input compatibility, a missing optional field and an explicit JSON `null`
both decode as unknown; re-serializing either form omits the field again. The
required `code`, `message`, and `severity` fields must not be `null` or omitted.
These field names, types, omission rules, and the severity enum are pinned by
`crates/pdftract-core/tests/diagnostics_serialization_format.rs`.

For catalog diagnostics, the structured object is serialized once and copied
through each structured surface: `metadata.diagnostics_detailed`, full JSON
`errors`, and the diagnostic portion of the NDJSON footer `errors` are equal in
field names, values, and order. `byte_offset` is in-process-only and is never
added to any of those objects. The only NDJSON exception is the synthetic
`page_extraction_error` record described below; it represents a failed page and
is not a catalog `DiagCode`.

### In-process byte-offset formatting

An in-process `Diagnostic` may retain a source `byte_offset`, but
`DiagnosticJson` never serializes a `byte_offset` field. Its `Display`
implementation is a separate human-readable rendering:

```text
STREAM_DECODE_ERROR: corrupt flate (byte offset 1234)
STRUCT_INVALID_NAME: bad name (byte offset 99) [7 0 R]
```

The format is `CODE: message`, followed by ` (byte offset N)` when an offset
is known, followed by ` [object generation R]` when an object reference is
known (for example, `[7 0 R]`). This rendering is for logs and debugging; it
is neither the wire object nor the legacy `metadata.diagnostics` value.

### Where structured diagnostics appear

1. **`metadata.diagnostics_detailed`** — the structured array on the extraction
   result's metadata. Mirrors `metadata.diagnostics` one-to-one.
2. **`errors`** — the top-level array of the full JSON output, populated from
   `metadata.diagnostics_detailed`.
3. **NDJSON footer frame `errors`** — a union array containing one synthetic
   page-failure record per failed page, followed by all structured extraction
   diagnostics in metadata emission order. A page-failure record has the shape
   `{"code":"page_extraction_error","severity":"error","message":"...","page_index":N}`;
   `page_index` is the zero-based failed-page index. This lowercase code is a streaming-only label, not a catalog code.

The NDJSON header carries document metadata but no diagnostic array. A
successful page frame omits `errors`; a failed page frame carries only its
synthetic `page_extraction_error` record. The footer is the complete streaming
diagnostic surface: it contains failed-page records first, then the canonical
structured diagnostics.

Empty-array behavior differs by surface: `metadata.diagnostics` and
`metadata.diagnostics_detailed` are omitted entirely when there are no
diagnostics, while the full output's top-level `errors` array and the NDJSON
footer's `errors` array are stable schema fields — always present, `[]` when
nothing was emitted. An NDJSON page frame carries `errors` only when that
page failed. See
[`docs/errors-array-format.md`](../errors-array-format.md#location).

`metadata.error_count` is separate from the diagnostic arrays: it counts
pages whose `PageResult.error` is set, not diagnostics whose severity is
`error` or `fatal`.

The legacy string form is still emitted as `metadata.diagnostics` for
existing callers: one plain string per diagnostic, in emission order, each
**the diagnostic's `message` verbatim** — no code prefix, no byte-offset or
object-location suffix. This is the compatibility surface defined by
`pdftract_core::diagnostics_compat` and pinned byte-for-byte by its golden
test; the structured objects above are the canonical form. Because the
legacy entries carry no code, identify a diagnostic through
`diagnostics_detailed`, which mirrors the string array one-to-one by index
(see [`docs/errors-array-format.md`](../errors-array-format.md)).

The compatibility guarantee is intentionally narrow: legacy entries preserve
message bytes, emission order, length, and duplicates. Code, severity,
page/location context, and hint are available only on the canonical
structured entry; new diagnostic information is added there without changing
the legacy strings.

An NDJSON footer with no failed pages carries the same canonical objects in
its `errors` array:

```ndjson
{"frame":"footer","extraction_quality":{"overall_quality":"medium","ocr_fraction":0.0},"errors":[{"code":"STREAM_DECODE_ERROR","message":"zlib stream truncated mid-inflation","severity":"warning","page_index":3,"location":{"object_number":12,"generation_number":0},"hint":"Partial output returned for this stream; consider re-saving the PDF through a normalising tool"}]}
```

## Migration and compatibility

New integrations should read the structured surface: use
`metadata.diagnostics_detailed` for a compact extraction result, the full
JSON document's top-level `errors`, or the NDJSON footer's `errors`. Match on
`code` and `severity`, and use `page_index`, `location`, and `hint` when they
are present. Do not parse human-readable messages or `Diagnostic`'s debug
`Display` form as a substitute for the structured fields.

Existing integrations may continue reading `metadata.diagnostics`. That field
is retained as a compatibility surface, not removed or changed by this
contract: each entry is still the corresponding structured `message` verbatim,
with the same order, length, and duplicates. During migration, pair the two
metadata arrays by index when an application needs both the legacy message
and its structured code or context. The legacy array does not carry a code,
severity, page, object location, or hint.

This is a compatibility migration, not a legacy-field removal. If a future
release deprecates or removes the string array, it must announce that change
with a release-specific migration path; until then, callers may rely on the
byte-preserving legacy projection while moving new code to the structured
surface.

## Emission and context contract

The following profiles assign the context fields and output destinations that
implementation code must use. `page_index` is always zero-based. `location`
is included whenever the emitter knows the originating indirect PDF object;
otherwise it is omitted. The profile does not permit an emitter to invent a
page or object reference merely to populate a field.

| Profile | `page_index` | `location` | Emission rule and output destinations |
|---------|--------------|------------|---------------------------------------|
| `D-PDF` | Omit; the event is document-scoped | Include when the PDF object is known | Emit a typed `Diagnostic` from document/catalog/xref processing. In an extraction result it flows to both metadata arrays, full-output `errors`, and the NDJSON footer. |
| `D-OP` | Omit | Omit; the event is not caused by a PDF indirect object | Emit a typed operation/source/configuration diagnostic. In an extraction result it follows the same structured destinations; an operation-specific API MAY additionally expose its own error envelope. |
| `P` | REQUIRED; the owning page is known | Include when the PDF object is known | Emit a typed page diagnostic. It flows to both metadata arrays, full-output `errors`, and the NDJSON footer; it is not duplicated into a page frame unless a separate page-failure record is required. |
| `C` | Include when the source stream/object has a known owning page; otherwise omit | Include when the originating indirect object is known | Emit a typed parser/decoder diagnostic and propagate any page/object context available at the forwarding boundary. The same code may therefore be document-level or page-level without changing its code or severity. |

For every emitted catalog code, `code`, `message`, and `severity` are required
and the current catalog hint is populated. The `hint` field remains optional on
the wire so consumers can tolerate a future code without a suggested action.
The only currently defined non-catalog record is the NDJSON-only synthetic
`page_extraction_error`, which has `code`, `message`, `severity`, and the
known `page_index`, but no hint. A catalog row marked `(reserved)` or gated by
a feature is a contract
reservation: it is not required to appear until that implementation/feature is
enabled, but its wire fields and profile are already fixed.

The complete inventory below assigns every code in the catalog exactly one
profile and names its emission owner. The default build has 113 catalog rows;
the two `CJK_*` rows are gated by the `cjk` feature, so an all-features build
has 115 active rows. The severity, phase, description, and exact hint for each
name remain in the catalog tables that follow.

| Profile | Emission owner | Codes |
|---------|----------------|-------|
| `D-PDF` | Document catalog, xref/repair, encryption, and document metadata | `STRUCT_HYBRID_CONFLICT`, `STRUCT_UNRESOLVED_DESTINATION`, `STRUCT_NON_GOTO_OUTLINE`, `STRUCT_INVALID_PREV_OFFSET`, `STRUCT_INVALID_HINT_STREAM`, `XREF_INVALID_HEADER`, `XREF_INVALID_ENTRY`, `XREF_INVALID_SUBSECTION_HEADER`, `XREF_OBJECT_ZERO_NOT_FREE`, `XREF_TRAILER_NOT_FOUND`, `XREF_TRUNCATED`, `XREF_REPAIRED`, `XREF_LINEARIZED_NO_FORWARD_SCAN`, `XREF_REMOTE_NO_FORWARD_SCAN`, `XREF_INVALID_STREAM_FORMAT`, `XREF_INVALID_STREAM_ENTRY`, `ENCRYPTION_UNSUPPORTED`, `ENCRYPTION_WRONG_PASSWORD`, `ENCRYPTION_INVALID_DICT`, `PAGE_INVALID_COUNT`, `REPAIR_RESCUED_FROM_BACKWARDS_XREF`, `JAVASCRIPT_PRESENT` |
| `D-OP` | Page-selection, remote-source, MCP, cache, and profile operations | `REMOTE_FETCH_INTERRUPTED`, `REMOTE_NO_RANGE_SUPPORT`, `REMOTE_TLS_FAILED`, `REMOTE_DNS_FAILED`, `REMOTE_URL_PRIVATE_NETWORK`, `REMOTE_INSUFFICIENT_DISK`, `MCP_TOOL_INVALID_PARAMS`, `MCP_PATH_TRAVERSAL`, `CACHE_ENTRY_CORRUPT`, `CACHE_WRITE_FAILED`, `CACHE_INTEGRITY_FAIL`, `PROFILE_SECRETS_FORBIDDEN`, `PROFILE_INVALID` |
| `P` | Page geometry, page selection, font/CJK, OCR/image, graphics-state, layout, marked-content, and inline-image pipelines | `STRUCT_INVALID_BDC_OPERAND`, `STRUCT_INCOMPLETE_COVERAGE`, `PAGE_OUT_OF_RANGE`, `PAGE_INVALID_ROTATE`, `FONT_GLYPH_UNMAPPED`, `FONT_NOT_FOUND`, `FONT_INVALID_CMAP`, `FONT_PARSE_FAILED`, `FONT_UNSUPPORTED`, `FONT_CIDTOGIDMAP_TRUNCATED`, `ENCODING_DIFFERENCE_OUT_OF_RANGE`, `FONT_TYPE3_WIDTHS_LENGTH_MISMATCH`, `CMAP_INVALID_CODESPACE`, `CJK_DECODE_MALFORMED`, `CJK_TOKENIZE_UNKNOWN_BYTE`, `OCR_JBIG2_UNSUPPORTED`, `OCR_JPX_UNSUPPORTED`, `OCR_CCITT_UNSUPPORTED`, `OCR_TESSERACT_FAILED`, `OCR_BROKENVECTOR_UNAVAILABLE`, `IMG_SOFTMASK_UNSUPPORTED`, `IMG_UNSUPPORTED_FORMAT`, `IMG_DESKEW_OUT_OF_RANGE`, `IMG_SOURCE_MIXED`, `GSTATE_STACK_OVERFLOW`, `GSTATE_STACK_UNDERFLOW`, `GSTATE_BT_ET_MISMATCH`, `CM_ARG_COUNT`, `CM_DEGENERATE`, `HORIZ_SCALING_ZERO`, `TEXT_RENDERING_MODE_CLAMPED`, `TSTAR_ZERO_LEADING`, `FONT_RESOURCE_NOT_FOUND`, `FONT_SIZE_ZERO_OR_NEGATIVE`, `BT_NESTED`, `ET_WITHOUT_BT`, `TEXT_SHOW_OUTSIDE_BT`, `TAGGED_PDF_STRUCT_TREE_DEFERRED`, `LAYOUT_READING_ORDER_AMBIGUOUS`, `LAYOUT_LOW_READABILITY`, `EMC_WITHOUT_BMC`, `MARKED_CONTENT_DEPTH_EXCEEDED`, `UNKNOWN_MARKED_CONTENT_PROPS`, `MCID_REDEFINED`, `INLINE_IMAGE_ID_WHITESPACE_MISSING`, `INLINE_IMAGE_NO_EI` |
| `C` | Lexer, object parser, content-stream parser, stream decoder, and page-optional geometry/OCR setup | `STRUCT_INVALID_NAME`, `STRUCT_INVALID_HEX`, `STRUCT_INVALID_OCTAL`, `STRUCT_INVALID_STREAM_HEADER`, `STRUCT_UNEXPECTED_BYTE`, `STRUCT_UNEXPECTED_EOF`, `STRUCT_UNTERMINATED_STRING`, `STRUCT_MISSING_KEY`, `STRUCT_CIRCULAR_REF`, `STRUCT_XOBJECT_CYCLE`, `STRUCT_DEPTH_EXCEEDED`, `STRUCT_INVALID_DICT_VALUE`, `STRUCT_INVALID_DICT_KEY`, `STRUCT_INVALID_INDIRECT_HEADER`, `STRUCT_INTEGER_OVERFLOW`, `STRUCT_REAL_INVALID`, `STRUCT_INVALID_NUMBER`, `STRUCT_INVALID_ASCII85`, `STRUCT_INVALID_OBJSTM`, `STRUCT_INVALID_GEOMETRY`, `STRUCT_INVALID_TYPE`, `STRUCT_INVALID_UTF16`, `STRUCT_INVALID_PDFDOC_ENCODING`, `STREAM_DECODE_ERROR`, `STREAM_BOMB`, `STREAM_UNKNOWN_FILTER`, `STREAM_INVALID_PARAMS`, `STREAM_INVALID_JPEG`, `STREAM_INVALID_CCITT`, `STREAM_TRUNCATED`, `STREAM_INVALID_JPX`, `OCR_LANGUAGE_UNAVAILABLE` |

Thus a child implementation does not need to infer context from a formatted
message: it selects the code's profile, supplies known page/object context,
uses the catalog severity and hint, and lets the standard serializers produce
the three structured destinations and the legacy message projection. A fatal
operation that returns before an extraction result is created is outside those
result surfaces.

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
| `STRUCT_INVALID_ASCII85` | Warning | Invalid ASCII85 character or malformed stream (reserved) | 1.5 |
| `STRUCT_INVALID_OBJSTM` | Warning | Invalid object stream format | 1.2 |
| `STRUCT_INVALID_GEOMETRY` | Warning | Invalid geometry value (NaN or Inf in MediaBox/CropBox/Rotate) | 1.7 |
| `STRUCT_INVALID_TYPE` | Warning | Invalid object type (expected type not found) | 5.2.1 |
| `STRUCT_INVALID_UTF16` | Warning | Invalid UTF-16BE encoding in string | 1.4 |
| `STRUCT_UNRESOLVED_DESTINATION` | Warning | Unresolved named destination | 1.4 |
| `STRUCT_NON_GOTO_OUTLINE` | Warning | Non-GoTo action in outline | 1.4 |
| `STRUCT_INVALID_PDFDOC_ENCODING` | Warning | Invalid PDFDocEncoding in string (reserved) | 1.4 |
| `STRUCT_HYBRID_CONFLICT` | Warning | Hybrid xref conflict: traditional and stream disagree | 1.3 |
| `STRUCT_INCOMPLETE_COVERAGE` | Info | StructTree coverage below 80% with /Suspects true | 7.1.4 |
| `STRUCT_INVALID_PREV_OFFSET` | Warning | Invalid /Prev offset in xref chain | 1.3 |
| `STRUCT_INVALID_HINT_STREAM` | Warning | Invalid linearized hint stream (/H entry); prefetch disabled, extraction continues | 1.8 |
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
| `STREAM_INVALID_JPX` | Warning | JPXDecode data has invalid JP2 box magic (raw J2K codestream or corruption); data passed through | 1.5 |

### ENCRYPTION_* — Encryption Errors

Errors related to PDF encryption and passwords.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `ENCRYPTION_UNSUPPORTED` | Fatal | Unsupported encryption or no password supplied | 1.4 |
| `ENCRYPTION_WRONG_PASSWORD` | Fatal | Password incorrect | 1.4 |
| `ENCRYPTION_INVALID_DICT` | Fatal | Invalid /Encrypt dictionary (malformed /O or /U, or missing required fields) | 1.4 |

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
| `FONT_NOT_FOUND` | Warning | Font not found or couldn't be parsed (reserved) | 2.1 |
| `FONT_INVALID_CMAP` | Warning | Invalid CMap format | 2.2 |
| `FONT_PARSE_FAILED` | Warning | Font program parsing failed | 2.1 |
| `FONT_UNSUPPORTED` | Warning | Font type not supported for embedded loading | 2.1 |
| `FONT_CIDTOGIDMAP_TRUNCATED` | Warning | CIDToGIDMap stream has odd byte count | 2.1 |
| `ENCODING_DIFFERENCE_OUT_OF_RANGE` | Warning | Character code in /Differences exceeds valid range | 2.2 |
| `FONT_TYPE3_WIDTHS_LENGTH_MISMATCH` | Warning | Type3 font /Widths array length mismatch | 2.4 |
| `CMAP_INVALID_CODESPACE` | Warning | Invalid codespace range in CMap (malformed lo/hi bounds); range skipped | 3 |

### CJK_* — CJK Encoding Errors

Errors related to CJK character encoding.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `CJK_DECODE_MALFORMED` | Warning | Malformed byte sequence in CJK encoding (reserved; requires `cjk` feature) | 2.3 |
| `CJK_TOKENIZE_UNKNOWN_BYTE` | Warning | Byte did not match any codespace range; U+FFFD substituted, once per font and byte value (requires `cjk` feature) | 3 |

### OCR_* — OCR Pipeline Errors

Errors related to OCR processing.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `OCR_JBIG2_UNSUPPORTED` | Warning | JBIG2 decoder not available | 1.5 / 5.2 |
| `OCR_JPX_UNSUPPORTED` | Warning | JPEG2000 (JPX) decoder not available | 1.5 / 5.2 |
| `OCR_CCITT_UNSUPPORTED` | Warning | CCITT fax decoder not available | 1.5 / 5.2 |
| `OCR_TESSERACT_FAILED` | Warning | Tesseract OCR failed (reserved) | 5.4 |
| `OCR_BROKENVECTOR_UNAVAILABLE` | Warning | OCR unavailable on broken-vector page | 4.7 |
| `OCR_LANGUAGE_UNAVAILABLE` | Warning | Requested OCR language pack not available | 5.4 |

### IMG_* — Image Processing Errors

Errors related to image extraction and processing.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `IMG_SOFTMASK_UNSUPPORTED` | Warning | Image soft mask not supported in direct compositing | 5.2.1 |
| `IMG_UNSUPPORTED_FORMAT` | Warning | Image format not supported | 5.2.1 |
| `IMG_DESKEW_OUT_OF_RANGE` | Warning | Deskew angle out of detectable range | 5.3.1 |
| `IMG_SOURCE_MIXED` | Warning | Image sources mixed in unexpected way (reserved) | 5.3.2 |

### REMOTE_* — Remote Source Errors

Errors related to HTTP fetching and remote sources.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `REMOTE_FETCH_INTERRUPTED` | Error | HTTP fetch interrupted or failed (reserved) | 1.8 |
| `REMOTE_NO_RANGE_SUPPORT` | Warning | Server does not support Range requests | 1.8 |
| `REMOTE_TLS_FAILED` | Fatal | TLS handshake failed (reserved) | 1.8 |
| `REMOTE_DNS_FAILED` | Fatal | DNS resolution failed (reserved) | 1.8 |
| `REMOTE_URL_PRIVATE_NETWORK` | Error | URL targets private network (SSRF protection) | 1.8 |
| `REMOTE_INSUFFICIENT_DISK` | Error | Insufficient disk space for fallback full download; extraction aborted | 1.8 |

### GSTATE_* — Graphics State Errors

Errors related to graphics state operators.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `GSTATE_STACK_OVERFLOW` | Warning | Graphics state stack overflow | 3.1 |
| `GSTATE_STACK_UNDERFLOW` | Warning | Graphics state stack underflow | 3.1 |
| `GSTATE_BT_ET_MISMATCH` | Warning | Mismatched BT/ET pair (reserved) | 3.1 |
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
| `LAYOUT_READING_ORDER_AMBIGUOUS` | Warning | Reading order may be incorrect (reserved) | 4.5 |
| `LAYOUT_LOW_READABILITY` | Warning | Low readability score (reserved) | 4.7 |

### MCP_* — MCP Server Errors

Errors related to MCP server operations.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `MCP_TOOL_INVALID_PARAMS` | Error | MCP tool call has invalid parameters (reserved) | 6.7 |
| `MCP_PATH_TRAVERSAL` | Error | MCP path traversal attempt (reserved) | 6.7 |

### CACHE_* — Cache Errors

Errors related to caching operations.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `CACHE_ENTRY_CORRUPT` | Warning | Cache entry is corrupted (reserved) | 6.9 |
| `CACHE_WRITE_FAILED` | Warning | Cache write failed (reserved) | 6.9 |
| `CACHE_INTEGRITY_FAIL` | Warning | Cache entry failed HMAC-SHA-256 integrity check (poisoning or corruption); treated as a miss (reserved) | 6.9 |

### MARKED_CONTENT_* — Marked Content Errors

Errors related to marked content operators.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `EMC_WITHOUT_BMC` | Info | EMC operator without matching BMC/BDC | 3.4 |
| `MARKED_CONTENT_DEPTH_EXCEEDED` | Info | Marked-content stack depth exceeded | 3.4 |
| `UNKNOWN_MARKED_CONTENT_PROPS` | Info | Unknown marked-content property name | 3.4 |
| `MCID_REDEFINED` | Info | MCID redefined in same scope (reserved) | 3.4 |

### INLINE_IMAGE_* — Inline Image Errors

Errors related to inline image scanning in content streams.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `INLINE_IMAGE_ID_WHITESPACE_MISSING` | Warning | Inline image ID keyword not followed by exactly one whitespace byte; raw-bytes scanner started immediately | 3.5 |
| `INLINE_IMAGE_NO_EI` | Warning | Inline image data missing EI terminator; all remaining bytes consumed as image data | 3.5 |

### PROFILE_* — Profile Errors

Errors related to profile configuration.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `PROFILE_SECRETS_FORBIDDEN` | Error | Profile YAML contains forbidden secret keys | 7.10 |
| `PROFILE_INVALID` | Error | Profile YAML is invalid or malformed (reserved) | 5.6.2 |

### REPAIR_* — Repair Recovery

Errors related to document repair operations.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `REPAIR_RESCUED_FROM_BACKWARDS_XREF` | Info | Xref repaired from backwards scan (reserved) | 1.3 |

### SECURITY_* — Security Diagnostics

Security-related diagnostics.

| Code | Severity | Description | Phase |
|------|----------|-------------|-------|
| `JAVASCRIPT_PRESENT` | Info | JavaScript present in PDF (never executed) | 1.2 |

## Catalog Hints

The `hint` field is the exact suggested action serialized for each catalog
code. Keeping these values in a separate two-column table makes the wire
contract reviewable while allowing the catalog-drift test to compare every
emitted code's hint byte-for-byte. A code row and a hint row must be added
together. The lowercase `page_extraction_error` streaming-only label is not
part of this table because it is not a `DiagCode`.

| Code | Hint |
|------|------|
| `STRUCT_INVALID_NAME` | None — the offending name was truncated to 127 bytes per spec |
| `STRUCT_INVALID_HEX` | Inspect the source PDF for malformed hex escapes |
| `STRUCT_INVALID_OCTAL` | Inspect the source PDF for malformed octal escapes |
| `STRUCT_INVALID_STREAM_HEADER` | The stream keyword must be followed by CRLF or LF |
| `STRUCT_UNEXPECTED_BYTE` | Inspect the source PDF for syntax errors |
| `STRUCT_UNEXPECTED_EOF` | The file may be truncated |
| `STRUCT_UNTERMINATED_STRING` | The literal string is missing a closing parenthesis |
| `STRUCT_MISSING_KEY` | Inspect the source PDF; missing keys are typically substituted with safe defaults |
| `STRUCT_CIRCULAR_REF` | None — cycle broken at the second visit; affected object returned as null |
| `STRUCT_XOBJECT_CYCLE` | Investigate the source PDF for a producer bug; cycle is broken at depth 20 |
| `STRUCT_DEPTH_EXCEEDED` | The PDF has excessively nested structures |
| `STRUCT_INVALID_DICT_VALUE` | A dictionary key was not followed by a value |
| `STRUCT_INVALID_DICT_KEY` | A dictionary key is not a name object |
| `STRUCT_INVALID_INDIRECT_HEADER` | The indirect object header (N G obj) is malformed |
| `STRUCT_INTEGER_OVERFLOW` | An integer value exceeded the i64 range and was clamped |
| `STRUCT_REAL_INVALID` | A real number literal could not be parsed as f64; the value was clamped to 0.0 |
| `STRUCT_INVALID_NUMBER` | A numeric literal was malformed (e.g., --5, bare sign, 1.2.3); the value was clamped to 0 |
| `STRUCT_INVALID_ASCII85` | The ASCII85 stream has invalid characters, overflow, or misuse of the 'z' shortcut; the offending byte was skipped |
| `STRUCT_INVALID_OBJSTM` | The object stream has a malformed header or invalid data |
| `STRUCT_INVALID_GEOMETRY` | NaN or Inf in MediaBox/CropBox/Rotate; canonicalized to 0 for fingerprint computation |
| `STRUCT_INVALID_UTF16` | UTF-16BE string has odd length or invalid encoding; the string was replaced with a placeholder |
| `STRUCT_INVALID_PDFDOC_ENCODING` | PDFDocEncoding string could not be decoded to UTF-8; the string was replaced with a placeholder |
| `STRUCT_INVALID_TYPE` | Object is not the expected type; the object was treated as null |
| `STRUCT_INVALID_BDC_OPERAND` | BDC operator's second operand was neither a dictionary nor a name; the MCID was set to None |
| `STRUCT_HYBRID_CONFLICT` | Traditional table entry takes precedence; object marked as Free per traditional table |
| `STRUCT_INCOMPLETE_COVERAGE` | StructTree coverage below 80% with /Suspects true; falling back to XY-cut reading order |
| `STRUCT_UNRESOLVED_DESTINATION` | Named destination resolution is deferred to a future enhancement; the outline destination was recorded as None |
| `STRUCT_NON_GOTO_OUTLINE` | The outline action is not GoTo (e.g., URI); the outline destination was recorded as None |
| `STRUCT_INVALID_HINT_STREAM` | Prefetch optimization was disabled for this document; extraction continues correctly, just slower (without prefetch) |
| `XREF_INVALID_HEADER` | The xref table doesn't start with the xref keyword |
| `XREF_INVALID_ENTRY` | An xref entry doesn't match the 20-byte format |
| `XREF_INVALID_SUBSECTION_HEADER` | An xref subsection header is malformed |
| `XREF_OBJECT_ZERO_NOT_FREE` | Object 0 is not free (violates PDF spec) |
| `XREF_TRAILER_NOT_FOUND` | The trailer dictionary couldn't be located |
| `XREF_TRUNCATED` | The xref table ends unexpectedly |
| `XREF_REPAIRED` | None — the xref was reconstructed via forward scan; output may be incomplete on truncated files |
| `XREF_LINEARIZED_NO_FORWARD_SCAN` | Forward scan is disabled for linearized PDFs |
| `XREF_REMOTE_NO_FORWARD_SCAN` | Forward scan is disabled for HTTP sources (would fetch entire file) |
| `XREF_INVALID_STREAM_FORMAT` | The xref stream has a malformed header or invalid /W array; the stream is skipped |
| `XREF_INVALID_STREAM_ENTRY` | An xref stream entry cannot be parsed due to invalid data |
| `STRUCT_INVALID_PREV_OFFSET` | A trailer's /Prev offset points to invalid data; the xref chain is truncated at this point |
| `STREAM_DECODE_ERROR` | Partial output returned for this stream; consider re-saving the PDF through a normalising tool |
| `STREAM_BOMB` | Increase --max-decompress-gb if the PDF is trusted; otherwise treat as a hostile file |
| `STREAM_UNKNOWN_FILTER` | The filter name is not supported by this version of pdftract |
| `STREAM_INVALID_PARAMS` | The /DecodeParms dictionary is malformed; default parameters are used |
| `STREAM_INVALID_JPEG` | JPEG data is missing SOI/EOI markers; data is passed through anyway |
| `STREAM_INVALID_CCITT` | CCITT data is missing required /Columns parameter; data is passed through anyway |
| `STREAM_INVALID_JPX` | JP2 box magic signature not found; raw J2K codestream (no JP2 wrapper) or corrupted data; data is passed through anyway |
| `ENCRYPTION_UNSUPPORTED` | Supply the correct password via --password, or use an Adobe-side decryption tool first |
| `ENCRYPTION_WRONG_PASSWORD` | The supplied password is incorrect |
| `ENCRYPTION_INVALID_DICT` | The /Encrypt dictionary has invalid or malformed entries; the PDF may be corrupted |
| `PAGE_OUT_OF_RANGE` | Adjust the --pages argument to the actual document page count |
| `PAGE_INVALID_COUNT` | The /Count key in the /Pages tree is invalid |
| `PAGE_INVALID_ROTATE` | The /Rotate value is not a multiple of 90; it was normalized |
| `FONT_GLYPH_UNMAPPED` | The glyph could not be resolved by any of the four levels; output contains U+FFFD |
| `FONT_NOT_FOUND` | A referenced font is missing from the PDF; a fallback font is used |
| `FONT_INVALID_CMAP` | The CMap stream is malformed; it's treated as empty |
| `FONT_PARSE_FAILED` | The embedded font program is corrupt or invalid; the font is treated as having no glyph mappings |
| `FONT_UNSUPPORTED` | A font type was encountered that doesn't support embedded font program loading |
| `FONT_CIDTOGIDMAP_TRUNCATED` | The CIDToGIDMap stream has an odd byte count; the trailing byte was discarded |
| `ENCODING_DIFFERENCE_OUT_OF_RANGE` | A /Differences array contains a character code outside 0-255; the code was clamped |
| `FONT_TYPE3_WIDTHS_LENGTH_MISMATCH` | The /Widths array length did not match LastChar - FirstChar + 1; the array was clamped or padded with zeros |
| `CMAP_INVALID_CODESPACE` | The codespace range had malformed lo/hi bounds; the range was skipped and CMap parsing continued |
| `CJK_DECODE_MALFORMED` | The CJK byte sequence contained malformed bytes, replaced with U+FFFD |
| `CJK_TOKENIZE_UNKNOWN_BYTE` | The byte did not match any codespace range; U+FFFD was emitted for it (once per font and byte value) |
| `OCR_JBIG2_UNSUPPORTED` | Build with --features full-render to enable JBIG2 decoding via PDFium |
| `OCR_JPX_UNSUPPORTED` | Build with --features full-render, or install libopenjp2 system library |
| `OCR_CCITT_UNSUPPORTED` | Install libtiff system library, or build with --features full-render |
| `OCR_TESSERACT_FAILED` | Tesseract crashed or returned an error; the page is treated as vector |
| `OCR_BROKENVECTOR_UNAVAILABLE` | Build with --features ocr to enable OCR recovery on broken-vector pages |
| `OCR_LANGUAGE_UNAVAILABLE` | Requested language pack not installed; extraction proceeded with eng fallback. Run 'pdftract doctor tesseract-langs' to verify installed languages. |
| `IMG_SOFTMASK_UNSUPPORTED` | Soft-masked images not supported in direct compositing; use --features full-render for proper rendering |
| `IMG_UNSUPPORTED_FORMAT` | Image format or bits-per-component not supported; image is skipped |
| `IMG_DESKEW_OUT_OF_RANGE` | Skew angle exceeds detection range (typically +/- 15 deg); image returned unchanged |
| `IMG_SOURCE_MIXED` | Page contains both vector and raster images in an unexpected combination; extraction quality may be degraded |
| `STREAM_TRUNCATED` | Stream has less data than expected; partial data is used |
| `REMOTE_FETCH_INTERRUPTED` | Retry the request; check network connectivity |
| `REMOTE_NO_RANGE_SUPPORT` | None — pdftract falls back to whole-file download; consider hosting on a Range-supporting server |
| `REMOTE_TLS_FAILED` | The TLS handshake failed; check the server's certificate |
| `REMOTE_DNS_FAILED` | The hostname could not be resolved; check the URL |
| `REMOTE_URL_PRIVATE_NETWORK` | URL targets a private network address. Use --allow-private-networks to enable (WARNING: security risk in multi-tenant deployments) |
| `REMOTE_INSUFFICIENT_DISK` | Free disk space on the temp file system (set TMPDIR to a different path if needed), or retry when more space is available |
| `GSTATE_STACK_OVERFLOW` | Investigate the source PDF for a malformed content stream |
| `GSTATE_STACK_UNDERFLOW` | The content stream has more Q operators than q operators |
| `GSTATE_BT_ET_MISMATCH` | The content stream has mismatched BT/ET operators |
| `CM_ARG_COUNT` | The cm operator requires exactly 6 numeric arguments |
| `CM_DEGENERATE` | The cm operator received a degenerate matrix; clamped to identity |
| `HORIZ_SCALING_ZERO` | The Tz operator received 0; clamped to 1.0% to avoid zero-width glyphs |
| `TEXT_RENDERING_MODE_CLAMPED` | The Tr operator received a value outside 0-7; clamped to valid range |
| `TSTAR_ZERO_LEADING` | The T* operator was called with leading == 0; no vertical movement occurred |
| `FONT_RESOURCE_NOT_FOUND` | The Tf operator referenced a font name not found in the resource dictionary; text-show ops will produce no glyphs until a valid font is bound |
| `FONT_SIZE_ZERO_OR_NEGATIVE` | The Tf operator received a font_size <= 0; clamped to 1.0 to avoid zero-height glyphs |
| `BT_NESTED` | BT operator called while already inside a text block; text matrices reset to identity |
| `ET_WITHOUT_BT` | ET operator called without a matching BT; operator ignored |
| `TEXT_SHOW_OUTSIDE_BT` | Text-showing operator (Tj, TJ, ', ") called outside BT/ET block; no glyphs produced |
| `TAGGED_PDF_STRUCT_TREE_DEFERRED` | None — Phase 7.1 will replace this fallback in v1.0.0 |
| `LAYOUT_READING_ORDER_AMBIGUOUS` | The reading order may be incorrect for complex multi-column layouts |
| `LAYOUT_LOW_READABILITY` | The page has low readability; may indicate mojibake or encoding issues |
| `MCP_TOOL_INVALID_PARAMS` | Adjust the tool-call arguments to match the schema in tools/list |
| `MCP_PATH_TRAVERSAL` | The requested path escapes --root; either fix the path or restart the server without --root |
| `CACHE_ENTRY_CORRUPT` | None — the entry was deleted and extraction re-ran |
| `CACHE_INTEGRITY_FAIL` | Cache entry failed HMAC verification; possibly poisoned or corrupted. Entry treated as miss and extraction re-ran. |
| `CACHE_WRITE_FAILED` | Check available disk space; extraction succeeded but the result wasn't cached |
| `EMC_WITHOUT_BMC` | The unmatched EMC operator was ignored; extraction continues |
| `MARKED_CONTENT_DEPTH_EXCEEDED` | BMC/BDC nesting exceeded the maximum depth of 64; the excess frame was discarded and extraction continues |
| `UNKNOWN_MARKED_CONTENT_PROPS` | The BDC property name was not found in the page's /Properties dictionary; the MCID was set to None |
| `MCID_REDEFINED` | Multiple /MCID keys appeared in the same BDC property dict; the last value wins |
| `INLINE_IMAGE_ID_WHITESPACE_MISSING` | The inline image ID keyword was not followed by exactly one whitespace byte; the raw-bytes scanner started immediately and recovery was automatic |
| `INLINE_IMAGE_NO_EI` | The inline image data did not end with the EI keyword; all remaining bytes were consumed as image data |
| `PROFILE_SECRETS_FORBIDDEN` | Remove the forbidden key from the profile YAML. Keys like password, token, secret, api_key are not allowed in profiles checked into source control. |
| `PROFILE_INVALID` | Fix the profile YAML syntax or values. Refer to the profile schema for valid options. |
| `REPAIR_RESCUED_FROM_BACKWARDS_XREF` | None — the xref was reconstructed by scanning backwards from end of file; output may be incomplete on truncated files |
| `JAVASCRIPT_PRESENT` | The PDF contains embedded JavaScript. Review the document metadata.javascript_actions array for details. pdftract never executes embedded JS. |

## Adding New Diagnostic Codes

When adding a new diagnostic code:

1. Choose a category prefix (STRUCT, STREAM, XREF, etc.)
2. Add the variant to the `DiagCode` enum in `crates/pdftract-core/src/diagnostics.rs`
3. Add the name mapping in `DiagCode::name()`
4. Add the category mapping in `DiagCode::category()`
5. Add the severity mapping in `DiagCode::severity()`
6. Add a catalog entry to `DIAGNOSTIC_CATALOG`
7. Add an entry to this document
8. Run `cargo test -p pdftract-core --test diagnostics_catalog_drift` (and the same
   command with `--all-features`) — this integration test reads the production
   emission sites, `DIAGNOSTIC_CATALOG`, and this table. It fails CI if a documented
   code's severity disagrees with the emitted code, if an emitted code lacks a
   catalog row, or if this document lists a code nothing emits (unless the row is
   marked `(reserved)` or annotated with the cargo feature that gates it, e.g.
   ``requires `cjk` feature``).

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
    page = error.get('page_index')          # omitted when document-level

    if code == 'OCR_BROKENVECTOR_UNAVAILABLE':
        # Install Tesseract for OCR recovery
        print(f"Page {page}: Install Tesseract for OCR recovery")

# Payloads shaped as a bare extraction result carry the same objects at
# metadata level, alongside the legacy string array:
detailed = result.get('metadata', {}).get('diagnostics_detailed', [])
strings = result.get('metadata', {}).get('diagnostics', [])
assert len(detailed) == len(strings)        # one-to-one mirror
```
