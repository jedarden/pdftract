# Error Handling, Robustness, and Graceful Degradation in PDF Extraction

## Overview

PDF extraction in the real world is not a clean parsing problem. Documents produced by hundreds of different authoring tools over three decades accumulate every imaginable deviation from the ISO 32000 specification: truncated files written by crashed processes, streams whose declared `/Length` disagrees with their actual byte count, circular indirect object references, content streams that open a `BT` block and forget to close it. A production extraction library that fails on any of these inputs is a liability. pdftract is designed on the principle that a document should never be fully unextractable due to a localized structural defect, and that every degraded extraction must come with a precise diagnostic record so callers can make informed decisions about the output they receive.

---

## Error Taxonomy

PDF parsing errors divide into five distinct classes, each demanding a different recovery posture.

**Structural errors** are defects in the file-level framing: a malformed or missing cross-reference table, an absent `%%EOF` marker, or a file that simply ends mid-stream. These affect the parser's ability to locate objects at all. pdftract handles structural errors through a two-pass xref strategy: the primary pass attempts normal xref/trailer parsing; if that fails, a linear scan of the byte stream recovers `obj` markers directly, rebuilding a synthetic object table from first principles. Pages whose object offsets are recovered via linear scan are flagged with `xref_reconstructed` in their per-page diagnostics.

**Object errors** occur when an individual indirect object has malformed syntax — mismatched delimiters, a type tag that contradicts the object's actual content, or a stream object missing its `endstream` keyword. When pdftract encounters an unparseable object, it records the object number and offset, emits an `object_parse_error` diagnostic, and substitutes a null value for that object in the object table. Callers that reference the failed object receive null rather than an exception.

**Stream errors** arise when the bytes of a stream cannot be decoded — an FlateDecode stream whose zlib data is corrupt, a LZWDecode stream with a truncated bitstream, or a declared `/Length` that runs past the actual `endstream` marker. Each stream is decoded independently. A decode failure emits a `stream_decode_error` diagnostic scoped to the specific stream's object number and page, and processing continues with the next stream on the same page. The raw (undecoded) bytes are never surfaced as text content.

**Content stream errors** live one level deeper: the decoded stream bytes parse as valid PDF syntax, but the operators or operands within the content stream are malformed — an unknown operator name, a `Tf` call with a missing operand, or a numeric value where a name is required. These are handled by pdftract's content stream state machine, described in detail below.

**Semantic errors** are structurally valid objects that violate the logical requirements of the PDF spec: a page dictionary lacking a required `/MediaBox`, a font dictionary missing `/Encoding`, a `/Resources` dictionary that references a font name not defined anywhere in the document. These are handled through fallback defaults rather than aborts, also detailed below.

---

## The Permissive Parsing Principle

PDF viewing software has historically tolerated extraordinary levels of spec deviation to avoid frustrating end users with unrenderable documents. The result is that a large fraction of PDFs in the wild contain violations that a strict parser would reject. pdftract adopts a formally permissive posture: parsing never hard-fails on a syntax deviation unless the deviation is so severe that no coherent interpretation of the surrounding bytes exists. Whitespace tolerance, delimiter mismatches, and minor keyword misspellings are handled silently. Every permissive decision is logged internally but only surfaces in the output diagnostics when it crosses a threshold that affects extraction quality. The goal is to extract as much text as the data permits, not to validate the document.

---

## Truncated Files

A truncated PDF — one where the writing process was interrupted before `%%EOF` was emitted, or where the xref section was never completed — is detected during the structural parsing phase. pdftract considers a file truncated when the xref table is absent or incomplete and linear scan recovers fewer object offsets than the trailer's `/Size` field declares.

When truncation is detected, extraction proceeds against the objects that were successfully recovered. Pages whose object graph is fully reconstructable are extracted normally. Pages that reference objects beyond the truncation point are emitted as empty pages with a `page_truncated` diagnostic. The document-level output carries a `truncated: true` flag in its metadata block, and the `extraction_quality` field reflects the proportion of pages that yielded content.

---

## Corrupt Streams

Stream decode errors are isolated at the stream boundary. When FlateDecode, LZWDecode, CCITTFaxDecode, or any other filter raises a decode exception, pdftract catches the exception at the stream decompressor boundary, records a `stream_decode_error` diagnostic with the object number, filter name, and byte offset where decompression failed, and moves on. The page is not aborted. If a page has three content streams and one fails to decode, the text from the other two is still extracted and reported. This per-stream isolation is enforced by the architecture: each stream is decoded in a sandboxed context with no shared mutable state that a failure could corrupt.

---

## Content Stream Error Recovery

The content stream interpreter is implemented as a pushdown state machine. Each token — operand or operator — is consumed one at a time. When the interpreter encounters an unknown operator name, it discards the pending operand stack, emits an `unknown_operator` diagnostic with the operator name and stream position, and resumes consuming tokens. When an operand is malformed (a string that cannot be decoded, a number outside representable range), the bad token is skipped and the state machine continues from the next well-formed token.

BT/ET balance is tracked with a counter. If an `ET` is encountered with no preceding `BT`, or if end-of-stream is reached with an open text block, the text state is reset to a clean initial state and a `bt_et_mismatch` diagnostic is emitted. Text extracted before the mismatch point is preserved; the reset ensures subsequent operators are interpreted against a coherent state rather than stale matrix and font references.

---

## Missing Required Keys and Semantic Fallbacks

When a page dictionary has no `/Contents` key, pdftract emits an empty page with no error — this is a valid degenerate page per the spec, and some documents use empty pages intentionally. When `/Contents` references an object that cannot be resolved, a `missing_contents` diagnostic is emitted and the page is empty.

Font references that appear in a content stream but are absent from `/Resources` enter a glyph fallback pipeline: pdftract attempts to resolve the font by name against a built-in metrics table covering the 14 standard PDF fonts and common Adobe variants. If resolution fails, character codes are passed through as Unicode replacement characters (U+FFFD), and a `font_not_found` diagnostic records the missing font name. The extraction continues; consumers can decide whether unresolved font references are acceptable for their use case.

When `/MediaBox` is absent from a page dictionary — and cannot be inherited from parent nodes in the page tree — pdftract defaults to US Letter dimensions (612×792 points). This default has no effect on text extraction, which is geometry-independent, but it ensures that coordinate normalization for spatial layout features produces coherent results rather than division-by-zero failures.

---

## Circular References

Indirect object references can form cycles: object A resolves to a dictionary containing a reference to object B, which resolves to a dictionary referencing A. Such cycles arise from corrupted or adversarially crafted documents. pdftract tracks the resolution path for each lookup using a thread-local visit set. Before dereferencing any indirect reference, the target object number is checked against the current visit set. If the object number is already present, the lookup returns null and a `circular_reference` warning is logged with the full cycle path. Resolution then unwinds normally. The visit set is cleared between top-level page extractions so that legitimate repeated references (a font shared across many pages) are not falsely flagged.

---

## Stack Overflows in Deeply Nested Structures

Page trees with thousands of intermediate nodes, Form XObjects that nest dozens of levels deep, and arrays or dictionaries containing nested structures that exceed practical recursion depth are handled with explicit stack data structures rather than native call stack recursion. The page tree walker maintains an explicit deque of pending nodes. The content stream interpreter's Form XObject descent uses an explicit execution stack rather than recursive calls. Array and dictionary parsing uses an iterative token accumulator. This design eliminates the class of stack overflow crashes that recursive descent parsers encounter on pathological inputs, and it makes the maximum nesting depth a configurable parameter rather than a property of the platform's call stack size.

---

## Memory Limits for Pathological Inputs

Some malformed PDFs declare enormous object counts, arrays with millions of entries, or xref tables that claim millions of objects. pdftract enforces practical extraction limits: a maximum of 100,000 indirect objects per document, a maximum of 1,000 Form XObject nesting levels per page, and a maximum array or dictionary size of 65,536 entries per object. When a limit is reached, further items are dropped, a `limit_exceeded` diagnostic is emitted with the limit name and the actual count encountered, and processing continues with the truncated structure. These limits are exposed as configurable parameters so callers with specific workload characteristics can tune them without recompiling.

---

## Output Quality Signaling

Every extraction response carries an `extraction_quality` field drawn from a four-value enum:

- **Complete**: all pages extracted without diagnostics affecting content.
- **Partial**: one or more pages are empty due to structural or stream errors, but the majority of the document was extracted successfully. Threshold: fewer than 20% of pages have content-affecting errors.
- **Degraded**: significant extraction failures; more than 20% of pages have content-affecting errors, or the xref required full linear reconstruction.
- **Failed**: no content could be extracted from any page.

The `errors` array in the output contains one entry per diagnostic event. Each entry carries a `code` (e.g., `stream_decode_error`), a human-readable `message`, a `severity` (`warning`, `error`, or `fatal`), and a `location` block with `page`, `object`, and `stream` fields populated where applicable. This structure gives callers a machine-readable audit trail: a document processing pipeline can decide to retry with a different extraction strategy, quarantine the document, or simply log the errors and proceed, based on the specific codes and counts present. The intent is that pdftract never silently degrades — every compromise in extraction quality has a corresponding record in the output.
