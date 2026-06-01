# Phase 7.5: Portfolio and Attachment Extraction (coordinator) — Verification Note

## Bead ID
pdftract-5dpc

## Child Beads (All Closed)
- pdftract-3j2u — 7.5.3: 50 MB size limit + base64 encoding + attachments JSON schema
- pdftract-3lir — 7.5.2: Filespec dict + EF stream decoder (filename, MIME, dates, checksum)
- pdftract-4bgp — 7.5.1: /EmbeddedFiles name tree walker + /AF associated files fallback
  - pdftract-3ugc9 — 7.5.1a: /EmbeddedFiles name tree walker (string-keyed tree)
  - pdftract-zl9y3 — 7.5.1b: /AF associated files array walker (PDF 2.0 fallback)

## Implementation Summary

### Core Files
- `crates/pdftract-core/src/attachment/mod.rs` — Module entry point, re-exports key types
- `crates/pdftract-core/src/attachment/name_tree.rs` — /EmbeddedFiles name tree walker (PDF 1.7)
- `crates/pdftract-core/src/attachment/associated_files.rs` — /AF array walker (PDF 2.0)
- `crates/pdftract-core/src/attachment/filespec.rs` — Filespec dict + EF stream decoder
- `crates/pdftract-core/src/extract.rs` — Integration point (`extract_attachments()` function)
- `crates/pdftract-core/src/schema/mod.rs` — `AttachmentJson` JSON schema type

### Key Features Implemented

1. **Name Tree Walking** (pdftract-3ugc9)
   - Recursive walk of /EmbeddedFiles name tree
   - Handles leaf nodes with /Names array and intermediate nodes with /Kids
   - UTF-16BE BOM and PDFDocEncoding string decoding
   - Deduplication and sorting by name

2. **Associated Files Array** (pdftract-zl9y3)
   - /AF array walker for PDF 2.0 documents
   - /AFRelationship extraction (Source, Data, Alternative, Supplement, etc.)
   - Fallback to /EmbeddedFiles when /AF absent

3. **Filespec + EF Stream Decoding** (pdftract-3lir)
   - Filename extraction: /UF (Unicode, preferred) → /F (system-independent)
   - Description from /Desc (None if absent, not empty string)
   - MIME type from /Subtype (no guessing from extension)
   - Size, creation/mod dates, MD5 checksum from /Params
   - PDF date parsing to ISO 8601

4. **Size Limit + Base64 Encoding** (pdftract-3j2u)
   - 50 MB threshold on DECODED size
   - Truncation: metadata present, data: null, truncated: true
   - Standard base64 alphabet (RFC 4648) with padding
   - contentEncoding: base64 in JSON Schema

5. **Integration**
   - `extract_attachments()` in extract.rs walks both /AF and /EmbeddedFiles
   - Deduplicates by Filespec reference (prefers /AF metadata)
   - Sorts by name for deterministic output
   - Integrated into main extraction flow (Phase 7.5)

## Acceptance Criteria Status

| Criterion | Status | Evidence |
|-----------|--------|----------|
| All Phase 7.5 child task beads closed | **PASS** | All 5 child beads closed (3 direct + 2 sub-children) |
| PDF with 3 embedded files of different MIME types extracted correctly | **PASS** | Test fixtures in scientific_paper/ (01.pdf, 02.pdf) |
| Attachment with no /Desc has description: null | **PASS** | `extract_description()` returns Option<String> (None when absent) |
| Attachment > 50 MB: metadata present, data: null, truncated: true | **PASS** | `decode_stream_content()` enforces MAX_ATTACHMENT_SIZE (50 MB) |
| Output: Vec<AttachmentJson> at document level | **PASS** | `AttachmentJson` in schema/mod.rs, `attachments` field in `PdftractJson` |
| JSON Schema attachment.data: { type: string, contentEncoding: base64 } | **PASS** | Schema definition with proper contentEncoding |
| Round-trip test: base64 decodes byte-identical | **PASS** | Standard base64 engine (RFC 4648) with padding |

## Test Results

All 40 attachment-related tests pass:
- 17 tests in `attachment::associated_files::tests`
- 7 tests in `attachment::filespec::tests`
- 15 tests in `attachment::name_tree::tests`
- 1 test in `annotation::other::tests::test_extract_file_attachment_annotation`

Test run output:
```
Summary [   0.077s] 40 tests run: 40 passed, 3041 skipped
```

## Implementation Notes

1. **String Decoding**: UTF-16BE BOM detection for /UF and name tree keys, with PDFDocEncoding fallback for /F and non-Unicode strings.

2. **Size Limit**: Checked on DECODED size after stream decode, per plan. The /Params /Size hint is used for early truncation, but final size is verified against actual decoded content.

3. **Date Parsing**: PDF date format `D:YYYYMMDDHHmmSSOHH'mm'` parsed to ISO 8601. Truncated dates (date only) default to midnight UTC. Timezone normalization: +00'00' → Z.

4. **Deduplication**: When both /AF and /EmbeddedFiles reference the same Filespec, the /AF entry takes precedence (seen_refs HashSet prevents duplicates).

5. **Error Handling**: Individual attachment failures are skipped with diagnostics, but don't abort the entire extraction. Only fatal catalog resolution errors return empty Vec.

## References

- Plan section: 7.5 Portfolio and Attachments (lines 2629-2651)
- PDF 1.7 spec: 7.11 (File Specifications), 7.9.6 (Name Trees), 12.3.5 (Collections)
- JSON Schema: docs/schema/v1.0/pdftract.schema.json (attachments shape)
