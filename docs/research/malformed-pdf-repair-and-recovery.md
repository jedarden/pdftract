# Malformed PDF Repair and Recovery

**Project:** pdftract — Rust PDF text extraction library  
**Scope:** Graceful handling of corrupt, truncated, and malformed PDF files

---

## 1. Prevalence and Categories of Malformed PDFs

Production PDF extraction cannot assume well-formed input. Malformed PDFs arrive from several distinct failure modes.

**Truncated downloads** are among the most common: a file fetched over HTTP where the connection dropped mid-transfer produces valid PDF prefix bytes followed by an abrupt EOF. The cross-reference table and trailer, which appear at the end of a standard PDF, are typically lost entirely.

**Disk write failures** produce files where the last few kilobytes were never flushed — a power loss or filesystem error after the application finished writing page content but before it wrote the xref. The byte count at `startxref` then points to an offset containing garbage or nothing.

**Buggy authoring tools** contribute a large share of structurally malformed but visually correct PDFs. Microsoft Word's PDF export historically produces incorrect `/Length` entries in stream dictionaries, off by one or two bytes due to CR/LF normalization mismatches. LibreOffice edge cases include object dictionaries with duplicate keys (last-value-wins is the correct resolution per ISO 32000-1 §7.3.7), missing `endobj` tokens on the final object in a file, and xref tables with incorrect byte offsets when the file was written on a platform with different newline conventions.

**Aggressive compression** can produce xref streams (PDF 1.5+) whose compressed payload, when decompressed, is shorter than the dictionary's `/W` field widths imply, causing out-of-bounds reads if the parser trusts the field counts blindly.

**Incremental update corruption** occurs when a PDF viewer appends an update section (new xref + trailer + `%%EOF`) but the process was interrupted. The appended section may be syntactically incomplete, yet the original body of the file remains intact.

**Legacy pre-ISO PDFs** (pre-1.0 through PDF 1.3 from the mid-1990s) use non-standard comment syntax, allow object numbers starting at values other than 1, and sometimes omit the `%%PDF-` header entirely. Some PostScript-derived exporters embed raw PostScript fragments as PDF stream data with no proper dictionary wrapper.

A production extractor must handle all of these rather than surfacing a hard error to the caller. The cost of failure is high: in document processing pipelines, a single corrupt file that panics or returns an opaque error can stall an entire batch.

---

## 2. Cross-Reference Table Recovery

The standard parse path reads `startxref` by scanning backward from `%%EOF`, then seeks to that offset to read the xref section. Recovery proceeds in stages when this fails.

**Stage 1 — Backward scan for `startxref`.** Read the last 1024 bytes of the file. Search backward for the literal token `startxref` followed by a decimal integer on the next line. If the stated offset is within the file bounds and the bytes there begin with `xref` (for traditional xref tables) or match an indirect object header `N G obj` (for xref streams), proceed normally.

**Stage 2 — Full-file object scan.** If stage 1 yields an offset pointing to garbage, scan the entire file byte-by-byte for the pattern `\d+ \d+ obj`. For each match, record `(object_number, generation, byte_offset)`. This reconstructed table is used as a fallback xref. Scanning must handle the case where the bytes `obj` appear inside a stream — use the heuristic that a valid object header is preceded by a newline or is at file start, and that the object and generation numbers are plausible (object number > 0, generation number typically 0 or 1).

**Multiple `%%EOF` markers** appear in linearized PDFs (one near the front for first-page delivery, one at the end) and in every incrementally updated file. The parser must not stop at the first `%%EOF` it encounters when scanning backward — it must collect all `%%EOF` positions and process xref sections anchored to each.

**Object number conflicts** arise when the same object number appears in multiple xref sections. For incremental updates, the correct rule (ISO 32000-1 §7.5.6) is last-definition-wins: the xref section closest to the end of the file takes precedence. During recovery from a full-file object scan, if two `obj` tokens claim the same object number, prefer the one at the higher byte offset, consistent with the incremental update semantics.

---

## 3. Object Stream Recovery

PDF 1.5 introduced xref streams, which replace the plaintext xref table with a compressed binary stream embedded in an indirect object. When this stream is itself corrupt, the parser must fall back to the Stage 2 object scan described above.

Within object streams (`/Type /ObjStm`), multiple objects are packed sequentially. The stream dictionary's `/N` field states the object count and `/First` gives the byte offset of the first object within the decompressed payload. If decompression fails or the `/First` offset exceeds the decompressed length, attempt to extract whatever objects are readable from the start of the decompressed data, stopping at the first parse error rather than discarding all objects in the stream.

---

## 4. Stream Length Repair

The `/Length` entry in a stream dictionary specifies how many bytes to read before `endstream`. This value is wrong frequently enough that every parser needs a repair path.

**Algorithm:**

1. Seek to the start of stream data (the byte immediately after the newline following the `stream` keyword).
2. Read exactly `/Length` bytes.
3. Scan the next 32 bytes for the `endstream` token, allowing for leading whitespace and CR/LF variants.
4. If `endstream` is found within that window, the length was correct. Continue.
5. If not found, the stated length is wrong. Scan forward from the start of stream data for the literal bytes `endstream` preceded by a newline. Use the byte count from stream start to that newline as the actual length. Log a warning with the offset, the stated length, and the actual length.
6. If `/Length` is missing entirely, scan for `endstream` from the start of stream data immediately. A missing `/Length` is a hard spec violation but appears in real files from legacy exporters.
7. The `endobj` token serves as a hard upper boundary: if `endstream` is not found before `endobj`, the stream data is truncated. Extract what is available and mark the stream as partial.

---

## 5. Syntax Error Tolerance

**Missing `endobj`.** If the parser encounters the object header of object N+1 while still parsing object N, treat the boundary as an implicit `endobj`. This covers the common LibreOffice case where the final object in a file has no terminator.

**Unbalanced `q`/`Q` in content streams.** The graphics state stack must not overflow or underflow. Track depth; on underflow (extra `Q`), ignore the operator and log a warning. On EOF with nonzero depth (unclosed `q`), synthesize the missing `Q` operators before returning from stream parsing.

**Invalid object references.** A reference to object 0 is always invalid (object 0 is the head of the free list). A reference to an object number not in the xref is a dangling reference. In both cases, return a null object rather than an error, consistent with how PDF readers handle missing optional entries.

**Non-integer generation numbers.** If the generation field in an object header is non-numeric, treat it as generation 0 and continue.

**Dictionary keys without values.** If a dictionary contains a name token immediately followed by another name token (the first has no value), insert a null value for the key-less entry and continue parsing the dictionary. This prevents the parser from misaligning all subsequent key-value pairs.

---

## 6. Linearization Failures

A linearized PDF places a linearization parameter dictionary as the first object, followed by a first-page xref section. When the linearization dictionary's `/L` (file length) field does not match the actual file size, treat the file as non-linearized and parse from the end using the main xref.

When the hint tables (referenced by `/H` in the linearization dictionary) are corrupt or point past EOF, skip hint table processing entirely. The hint tables are an optimization for byte-range requests; ignoring them does not affect completeness.

False linearization — where the first object claims `/Linearized` but the file structure is actually a standard non-linearized layout — is detected by checking whether the first-page xref section at the declared `/T` offset is present and valid. If not, fall back to end-of-file xref processing unconditionally.

---

## 7. Incremental Update Repair

Each incremental update appends: updated objects, a new xref section, a new trailer dictionary, and `%%EOF`. The trailer's `/Prev` field chains back to the previous xref offset.

When following `/Prev` chains, a corrupt intermediate update presents as an xref section at the chained offset that fails to parse. The repair strategy is to abandon chain-following at that point and instead scan the entire file for all `xref` or xref-stream markers (Stage 2), then sort them by byte offset ascending. Process them in ascending order, applying each xref section's entries to the object table, with later entries overwriting earlier ones. This produces the correct last-definition-wins semantics even when the `/Prev` chain is broken.

A degenerate case is a cyclic `/Prev` chain (offset A's trailer points to B, B's trailer points back to A). Detect cycles by tracking visited offsets in a `HashSet<u64>` and breaking on revisit.

---

## 8. Content Stream Error Recovery

Content stream parsing should be operator-by-operator. On encountering an error, the parser skips to the next operator boundary (next newline or whitespace-separated token that is a known operator or the start of an operand sequence) and resumes.

**Unknown operators** — skip to the next newline and continue. Emit an info-level log entry.

**Unmatched `BT`/`ET`.** A missing `ET` at EOF of the stream: synthesize `ET` before returning, preserving any accumulated text. A spurious `ET` with no preceding `BT`: ignore it.

**Wrong operand count.** If a `Tf` operator receives one operand instead of two, skip the operator. Do not attempt to infer missing operands — the result would be garbage text.

**Corrupt glyph data in `Tj` or `TJ`.** If a string operand contains byte sequences that do not map to any glyph in the current font's encoding, emit a replacement character (U+FFFD) for each unmappable byte and continue. Do not abort the text object.

---

## 9. Partial File Extraction

When a file is truncated mid-stream, extraction proceeds over all pages whose objects are fully recoverable. The extractor tracks the highest page index for which all required content streams and resources were available.

Output metadata includes:

```json
{
  "partial": true,
  "pages_recovered": 14,
  "pages_total_claimed": 20,
  "truncation_offset": 1048576
}
```

`partial: true` signals to callers that the output is incomplete. `pages_recovered` is the count of pages for which text was extracted. `pages_total_claimed` reflects the page count in the document catalog, which may itself be in the corrupt region (in which case it is omitted). `truncation_offset` is the byte offset at which the first unrecoverable structure was encountered.

---

## 10. Error Reporting

Every recovery action is logged as a structured entry alongside the extracted content. The top-level output object contains a `warnings` array:

```json
{
  "warnings": [
    {
      "severity": "warning",
      "offset": 204800,
      "object": 42,
      "error_type": "wrong_stream_length",
      "stated_value": 1024,
      "actual_value": 1031,
      "recovery": "scanned_for_endstream"
    },
    {
      "severity": "error",
      "offset": 819200,
      "object": null,
      "error_type": "xref_corrupt",
      "recovery": "full_file_object_scan"
    }
  ]
}
```

**Severity levels:**

- `info` — a deviation that was resolved without ambiguity (e.g., missing `endobj` at end of file where the next object header was unambiguous).
- `warning` — a deviation that required a heuristic recovery; the extracted content is likely correct but not guaranteed (e.g., wrong `/Length` corrected by `endstream` scan).
- `error` — a structural failure that caused partial loss of content (e.g., an xref section that could not be reconstructed, resulting in unreachable objects).

The `object` field is null when the error occurs in a structural region (xref, trailer) rather than within a specific object. The `recovery` field uses a fixed vocabulary of strategy identifiers so callers can programmatically assess extraction quality without parsing human-readable strings.

Callers should treat any `error`-severity entry as grounds for flagging the output for human review, while `warning`-severity entries indicate likely-correct extractions from imperfect input.
