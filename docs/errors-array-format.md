# Errors Array Format and Test Integration Guide

This document explains the complete structure of the errors/diagnostics arrays in pdftract extraction results and how to integrate assertions in tests. The serialized `DiagnosticJson` object is canonical: catalog diagnostics have the same object fields, values, and order in compact metadata, full JSON `errors`, and the NDJSON footer `errors` array. The string array is a message-only compatibility surface, not an alternative structured schema. The complete code registry is maintained in [`docs/integrations/diagnostics-codes.md`](integrations/diagnostics-codes.md).

## Overview

The pdftract extraction system uses a unified diagnostic system to report events at four severities — `info`, `warning`, `error`, and `fatal` (the full enum, with the code catalog, lives in [`docs/integrations/diagnostics-codes.md`](integrations/diagnostics-codes.md)). Recoverable parsing and extraction issues are emitted without panicking (per INV-8), allowing recovery to continue. A fatal condition may return an API error before an `ExtractionResult` exists, so it is not guaranteed to appear in an extraction result's arrays.

## Extraction Result Structure

### Main Result Structure

```rust
pub struct ExtractionResult {
    pub fingerprint: String,
    pub pages: Vec<PageResult>,
    pub metadata: ExtractionMetadata,
    pub signatures: Vec<SignatureJson>,
    pub form_fields: Vec<FormFieldJson>,
    pub links: Vec<LinkJson>,
    pub attachments: Vec<AttachmentJson>,
    pub threads: Vec<ThreadJson>,
    pub javascript_actions: Vec<JavascriptActionJson>,
}
```

### Metadata Structure (Contains Errors/Diagnostics)

```rust
pub struct ExtractionMetadata {
    pub page_count: usize,
    pub receipts_mode: ReceiptsMode,
    pub span_count: usize,
    pub block_count: usize,
    pub cache_status: Option<String>,
    pub cache_age_seconds: Option<u64>,
    pub error_count: usize,              // Number of pages that failed to extract
    pub reading_order_algorithm: Option<String>,
    pub diagnostics: Vec<String>,        // message-only compatibility array
    pub diagnostics_detailed: Vec<DiagnosticJson>, // canonical structured array
    pub profile_name: Option<String>,
    pub profile_version: Option<String>,
    pub profile_fields: Option<serde_json::Value>,
}
```

## Errors Array Format

### Location

The compact extraction metadata exposes two parallel arrays, which mirror each
other one-to-one (same length, same order, same diagnostics):

- **String form — `ExtractionResult.metadata.diagnostics`**
  - **Type**: `Vec<String>`
  - **Description**: One plain string per diagnostic, in emission order —
    each entry is the diagnostic's `message` verbatim. This is the
    compatibility surface defined by `pdftract_core::diagnostics_compat`
    (see [String Format](#string-format)).
- **Structured form — `ExtractionResult.metadata.diagnostics_detailed`**
  - **Type**: `Vec<DiagnosticJson>` (see [Structured Object Form](#structured-object-form))
  - **Description**: The same diagnostics as objects with `code`, `message`,
    `severity`, and optional `page_index` / `location` / `hint` fields.
    **Prefer this form** for machine consumption.

The structured metadata array is also projected to the full JSON output's
top-level `errors` array. In NDJSON streaming output, the surfaces are:

| Surface | Diagnostic contents and ordering | Empty behavior |
|---------|---------------------|-----------------|
| Compact JSON `metadata` | `diagnostics_detailed` plus the message-only `diagnostics` compatibility array | Both metadata keys are omitted when empty |
| Full JSON `errors` | The same structured objects as `metadata.diagnostics_detailed` | `errors: []` is always present |
| NDJSON page frame `errors` | One synthetic record only when that page failed | Omitted on successful pages |
| NDJSON footer `errors` | Failed-page records first, in page-frame/page-index order, then the structured metadata diagnostics in emission order | `errors: []` is always present |

The synthetic failed-page record is
`{"code":"page_extraction_error","severity":"error","message":"...","page_index":N}`;
`page_index` is the zero-based failed-page index. This lowercase code is not a
`DiagCode` and never appears in either metadata array.

For every catalog diagnostic, the object in `metadata.diagnostics_detailed` is
the same structured value copied to full JSON `errors` and to the footer after
any synthetic page-failure records. Consumers can therefore use one decoder
for all three catalog-diagnostic surfaces. The metadata arrays and full JSON
`errors` retain diagnostic emission order; only the footer prepends its
synthetic page-failure records. `byte_offset` is retained only on the
in-process `Diagnostic` and is never a serialized field.

The compatibility guarantee for `metadata.diagnostics` is limited to the
message bytes, emission order, length, and duplicates. Code, severity,
page/location context, and hints are intentionally available only in the
structured entry; adding that information must not change legacy strings.

Empty-array behavior differs by surface:

- `metadata.diagnostics` and `metadata.diagnostics_detailed` are **omitted
  entirely** from serialized output when empty.
- The full output's top-level `errors` array and the NDJSON footer's
  `errors` array are stable schema fields: **always present**, `[]` when
  nothing was emitted.
- An NDJSON page frame carries `errors` only when that page failed.

`metadata.error_count` is independent of all diagnostic severities: it is the
number of pages whose `PageResult.error` is set. It is not the number of
structured diagnostics with `severity` equal to `error` or `fatal`.

### String Format

Each entry of `metadata.diagnostics` is exactly the corresponding
diagnostic's `message`:

```
{Human-readable message}
```

There is no `CODE:` prefix and no byte-offset / object-location suffix: the
legacy surface carries the message and nothing else. The internal
`Diagnostic`'s `Display` is a different log/debug rendering:

```text
STREAM_DECODE_ERROR: corrupt flate (byte offset 1234)
STRUCT_INVALID_NAME: bad name (byte offset 99) [7 0 R]
```

Its exact grammar is `CODE: message`, then optional ` (byte offset N)`, then
optional ` [object generation R]`. That rendering is *not* what
`metadata.diagnostics` contains — the conversion is
`pdftract_core::diagnostics_compat::to_legacy_strings`, pinned byte-for-byte
by its golden test.

**Examples** (bare messages, as emitted):
- `zlib stream truncated mid-inflation`
- `Name object exceeds 127-byte limit`
- `Glyph could not be mapped to Unicode`
- `Xref was reconstructed via forward scan`
- `corrupt flate data`

Because the entries carry no code, identify a diagnostic through the
structured array below, which mirrors the string array one-to-one by index.

## Structured Object Form

`metadata.diagnostics_detailed` (and the top-level `errors` array of the full
output) holds one object per diagnostic, as documented in
[`docs/integrations/diagnostics-codes.md`](integrations/diagnostics-codes.md):

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

| Field | JSON type | Serialized presence | Meaning |
|-------|-----------|---------------------|---------|
| `code` | string | always | Stable `DiagCode::name()` identifier; use it for classification. |
| `message` | string | always | Human-readable display text; the legacy string entry at the same index is exactly this text. |
| `severity` | string | always | Exactly `info`, `warning`, `error`, or `fatal`; derived from the typed code policy. |
| `page_index` | integer | when the owning page is known | Zero-based page index; omitted for document- or operation-level diagnostics. |
| `location` | object | when the originating indirect object is known | `{"object_number": u32, "generation_number": u16}`; it is not a byte offset. |
| `hint` | string | when the code has a catalog hint | Catalog guidance; every current catalog row has a non-empty hint string, including `None —` rows, but consumers must tolerate omission. |

Fields that do not apply — `page_index`, `location`, and `hint` — are omitted,
not `null`. A present `location` is an object with the required, non-null
numeric fields `object_number` (`u32`) and `generation_number` (`u16`); it is a
PDF indirect-object reference, not a byte offset. Because `severity`,
`page_index`, and `location` come from the typed diagnostic itself, prefer the
structured form whenever a consumer needs more than a substring search. Byte
offsets remain in-process-only context and are never a `DiagnosticJson` field.

When reading JSON, an absent optional field and an explicit `null` are both
accepted as unknown and decode identically. When writing JSON, both forms are
normalized to omission; `null` is never emitted for these fields. The required
`code`, `message`, and `severity` fields must always be present and non-null.

For example, these two inputs have the same meaning and serialize to the same
minimal object (the optional fields are omitted):

```json
{"code":"XREF_REPAIRED","message":"Xref was reconstructed via forward scan","severity":"info"}
```

```json
{"code":"XREF_REPAIRED","message":"Xref was reconstructed via forward scan","severity":"info","page_index":null,"location":null,"hint":null}
```

In NDJSON streaming output, a footer with no failed pages uses the same
structured object shape:

```ndjson
{"frame":"footer","extraction_quality":{"overall_quality":"medium","ocr_fraction":0.0},"errors":[{"code":"STREAM_DECODE_ERROR","message":"zlib stream truncated mid-inflation","severity":"warning","page_index":3,"location":{"object_number":12,"generation_number":0},"hint":"Partial output returned for this stream; consider re-saving the PDF through a normalising tool"}]}
```

## Migration and compatibility

For new machine-readable integrations, consume
`metadata.diagnostics_detailed` or the structured `errors` array. Match on
`code` and `severity`; use `page_index`, `location`, and `hint` when present,
and tolerate their omission. Messages are for display and diagnostics, not a
stable classification key.

The legacy `metadata.diagnostics` string array remains supported for existing
callers. It is not removed or rewritten as part of this contract. Each entry
is the structured entry's `message` verbatim, preserving bytes, order, length,
and duplicates; it intentionally has no code or context. A caller migrating
incrementally can keep its existing string handling and pair each string with
the structured entry at the same index.

There is no legacy-field removal in this release. Any future deprecation or
removal must be announced with a release-specific migration path; until then,
the string array remains a supported compatibility projection while new code
adopts the structured form.

## Accessing Error Information

### From Extraction Result

```rust
use pdftract_core::extract::extract_pdf;
use std::path::Path;

let result = extract_pdf(Path::new("test.pdf"), &Default::default())?;

// The structured array is canonical for machine-readable checks.
let diagnostics = &result.metadata.diagnostics_detailed;
// The message-only array remains available for legacy callers.
let legacy_messages = &result.metadata.diagnostics;

// Get error count
let error_count = result.metadata.error_count;

// Iterate through all diagnostics
for diagnostic in diagnostics {
    println!("{}: {}", diagnostic.code, diagnostic.message);
}
```

### Checking for Specific Errors

The legacy string entries carry the message only — no code — so key checks
by code on the structured array:

```rust
let detailed = &result.metadata.diagnostics_detailed;

// Check if any stream decode errors occurred
let has_stream_errors = detailed.iter()
    .any(|d| d.code == "STREAM_DECODE_ERROR");

// Check for encryption errors
let has_encryption_errors = detailed.iter()
    .any(|d| d.code.starts_with("ENCRYPTION"));

// Count specific error types
let glyph_unmapped_count = detailed.iter()
    .filter(|d| d.code == "FONT_GLYPH_UNMAPPED")
    .count();
```

### Structured Filtering (Preferred)

The same checks against `metadata.diagnostics_detailed` use the typed
severity instead of substrings:

```rust
// Every warning emitted during extraction
let warnings = result.metadata.diagnostics_detailed.iter()
    .filter(|d| d.severity == "warning");

// Diagnostics for one page, with their catalog hints
let page_problems = result.metadata.diagnostics_detailed.iter()
    .filter(|d| d.page_index == Some(3))
    .map(|d| (d.code.as_str(), d.hint.as_deref()));

// The string entry is exactly the structured entry's message at the same
// index — the two arrays mirror each other one-to-one.
for (s, d) in result.metadata.diagnostics.iter()
    .zip(&result.metadata.diagnostics_detailed)
{
    assert_eq!(s.as_str(), d.message.as_str());
}
```

## Diagnostic Categories and Codes

The complete registry is maintained in
[`docs/integrations/diagnostics-codes.md`](integrations/diagnostics-codes.md):
113 rows in the default build, plus two `CJK_*` rows when the `cjk` feature is
enabled. That registry is the only normative source for code names, severities,
phase origins, hints, reserved/feature-gated status, and the code-to-emission
context profile. This guide intentionally does not repeat a partial list: a
code is documented once, so the two guides cannot drift again.

The registry's `D-PDF`, `D-OP`, `P`, and `C` profiles also specify whether
`page_index` is omitted, required, or conditional and when `location` is
carried. All registered codes use the same structured object fields and output
surfaces described above; only the context profile differs.

Every registry row is covered by the data-driven round-trip contract in
[`crates/pdftract-core/tests/diagnostics_roundtrip_contract.rs`](../crates/pdftract-core/tests/diagnostics_roundtrip_contract.rs),
which checks each code's `code`, `message`, `severity`, catalog `hint`, optional
context, JSON/NDJSON propagation, and message-only legacy projection. The
examples in this guide are representative of that contract rather than
additional code-specific wire formats.

## Assertion Patterns for Tests

### Pattern 1: Check for No Errors (Clean Extraction)

```rust
#[test]
fn test_clean_pdf_extracts_without_errors() {
    let result = extract_pdf(std::path::Path::new("tests/fixtures/auxiliary/plain.pdf"), &Default::default())
        .expect("Extraction should succeed");
    
    // Verify no diagnostics were emitted
    assert!(
        result.metadata.diagnostics.is_empty(),
        "Expected no diagnostics, but got: {:?}",
        result.metadata.diagnostics
    );
    assert!(result.metadata.diagnostics_detailed.is_empty());
    
    // Verify error count is zero
    assert_eq!(result.metadata.error_count, 0);
}
```

### Pattern 2: Check for Specific Error Code

```rust
#[test]
fn test_truncated_stream_emits_decode_error() {
    let result = extract_pdf(std::path::Path::new("tests/fixtures/malformed/truncated-flate.pdf"), &Default::default())
        .expect("Extraction should succeed (with recovery)");
    
    // Check for STREAM_DECODE_ERROR diagnostic (by code, on the
    // structured array)
    let has_decode_error = result.metadata.diagnostics_detailed.iter()
        .any(|d| d.code == "STREAM_DECODE_ERROR");

    assert!(
        has_decode_error,
        "Expected STREAM_DECODE_ERROR diagnostic, got: {:?}",
        result.metadata.diagnostics_detailed
    );
}
```

### Pattern 3: Check Multiple Diagnostic Codes

```rust
fn assert_expected_codes(result: &ExtractionResult) {
    // Check only codes that the fixture's checked-in contract promises; use
    // the structured array rather than searching legacy message text.
    let detailed = &result.metadata.diagnostics_detailed;

    assert!(
        detailed.iter().any(|d| d.code == "STRUCT_INVALID_DICT_VALUE"),
        "Expected STRUCT_INVALID_DICT_VALUE"
    );
}
```

### Pattern 4: Count Specific Errors

```rust
#[test]
fn test_pdf_with_unmapped_glyphs() {
    let result = extract_pdf(std::path::Path::new("tests/fixtures/encoding/unmapped-glyphs.pdf"), &Default::default())
        .expect("Extraction should succeed");
    
    // Count unmapped glyph diagnostics
    let unmapped_count = result.metadata.diagnostics_detailed.iter()
        .filter(|d| d.code == "FONT_GLYPH_UNMAPPED")
        .count();
    
    assert!(
        unmapped_count > 0,
        "Expected at least one FONT_GLYPH_UNMAPPED diagnostic"
    );
    
    println!("Found {} unmapped glyphs", unmapped_count);
}
```

### Pattern 5: Verify a Fatal Diagnostic Envelope

```rust
#[test]
fn test_encryption_diagnostic_envelope() {
    use pdftract_core::diagnostics::{DiagCode, Diagnostic};
    use pdftract_core::schema::DiagnosticJson;

    // Fatal encryption failures may return Err from extract_pdf. When a
    // typed diagnostic is available, its machine-readable envelope is still
    // derived from registry policy, not from a formatted string.
    let diagnostic = Diagnostic::with_static_no_offset(
        DiagCode::EncryptionUnsupported,
        "Unsupported encryption or no password supplied",
    );
    let envelope = DiagnosticJson::from(&diagnostic);

    assert_eq!(envelope.code, "ENCRYPTION_UNSUPPORTED");
    assert_eq!(envelope.severity, "fatal");
    assert_eq!(envelope.message, "Unsupported encryption or no password supplied");
    assert!(envelope.hint.is_some());
}
```

### Pattern 6: Check Error Count Matches

```rust
fn assert_extraction_error_count(result: &ExtractionResult) {
    // Verify error_count matches failed pages, not diagnostic severity.
    let failed_page_count = result.pages.iter()
        .filter(|page| page.error.is_some())
        .count();
    
    assert_eq!(
        result.metadata.error_count,
        failed_page_count,
        "error_count should match the number of pages with an extraction error"
    );
}
```

### Pattern 7: Assert No Fatal Errors

```rust
#[test]
fn test_extraction_has_no_fatal_errors() {
    let result = extract_pdf(std::path::Path::new("tests/fixtures/auxiliary/plain.pdf"), &Default::default())
        .expect("Extraction should succeed");
    
    // Ensure no fatal-level diagnostics were emitted (by typed severity,
    // never by substring)
    let has_fatal = result.metadata.diagnostics_detailed.iter()
        .any(|d| d.severity == "fatal");

    assert!(
        !has_fatal,
        "Extraction should not have fatal errors: {:?}",
        result.metadata.diagnostics_detailed
    );
}
```

### Pattern 8: Verify Expected Number of Diagnostics

```rust
fn assert_three_diagnostics(result: &ExtractionResult) {
    // Apply this assertion only to a fixture whose checked-in contract says
    // it produces exactly three diagnostics; do not infer a count from a
    // message or from a different fixture.
    assert_eq!(
        result.metadata.diagnostics_detailed.len(),
        3,
        "Expected exactly 3 diagnostics, got {}: {:?}",
        result.metadata.diagnostics_detailed.len(),
        result.metadata.diagnostics_detailed
    );
}
```

## Test Integration Examples

### Example 1: Truncated Flate Stream Test

```rust
#[test]
fn test_truncated_flate_stream_recovery() {
    let fixture_path = "tests/fixtures/malformed/truncated-flate.pdf";
    let result = extract_pdf(std::path::Path::new(fixture_path), &Default::default())
        .expect("Extraction should succeed with recovery");
    
    // Should have emitted a stream decode error (by code, on the
    // structured array)
    let has_stream_error = result.metadata.diagnostics_detailed.iter()
        .any(|d| d.code == "STREAM_DECODE_ERROR" || d.code == "STREAM_TRUNCATED");

    assert!(has_stream_error,
        "Expected stream decode error for truncated-flate.pdf");

    // Should still produce some output (recovery)
    assert!(!result.pages.is_empty(),
        "Should produce at least one page despite stream error");

    // Verify the error message mentions truncation
    let error_msg = result.metadata.diagnostics_detailed.iter()
        .find(|d| d.code.starts_with("STREAM"))
        .expect("Should have stream-related diagnostic");

    assert!(error_msg.message.to_lowercase().contains("truncat") ||
            error_msg.message.to_lowercase().contains("incomplete"),
        "Stream error message should mention truncation or incomplete data");
}
```

### Example 2: Encryption Error Test

```rust
#[test]
fn test_encrypted_pdf_without_password() {
    let fixture_path = "tests/fixtures/encrypted/livecycle.pdf";
    let result = extract_pdf(std::path::Path::new(fixture_path), &Default::default());
    
    // Fatal encryption failures are returned as Err by this extraction API;
    // callers should not expect a successful result containing diagnostics.
    assert!(result.is_err(), "Should fail when encrypted PDF lacks password");
}
```

### Example 3: Font Glyph Unmapped Test

```rust
#[test]
fn test_custom_font_with_unmapped_glyphs() {
    let fixture_path = "tests/fixtures/encoding/unmapped-glyphs.pdf";
    let result = extract_pdf(std::path::Path::new(fixture_path), &Default::default())
        .expect("Extraction should succeed");
    
    // Should have FONT_GLYPH_UNMAPPED diagnostics
    let unmapped_diagnostics: Vec<_> = result.metadata.diagnostics_detailed.iter()
        .filter(|d| d.code == "FONT_GLYPH_UNMAPPED")
        .collect();
    
    assert!(!unmapped_diagnostics.is_empty(),
        "Custom font PDF should have unmapped glyph diagnostics");
    
    // Each unmapped glyph produces U+FFFD in output
    let text = result.pages.iter()
        .flat_map(|p| p.blocks.iter().flat_map(|b| b.spans.iter().map(|s| &s.text)))
        .collect::<String>();
    
    let fffd_count = text.matches('�').count();
    assert!(fffd_count > 0,
        "Output should contain replacement character for unmapped glyphs");
}
```

## Best Practices for Test Assertions

1. **Be Specific**: Check for exact diagnostic codes when testing specific error conditions
2. **Allow Recovery**: Most tests should expect extraction to succeed despite errors (per INV-8 recovery principle)
3. **Verify Output**: When errors occur, verify that meaningful output was still produced
4. **Count Errors**: Use counts to verify expected number of diagnostics
5. **Check Messages**: Verify diagnostic messages contain expected information
6. **Test Severity**: Ensure fatal errors only occur for truly unrecoverable conditions — filter `metadata.diagnostics_detailed` by `severity`, never by substring
7. **Use Helper Functions**: Create reusable assertion helpers for common patterns (`crates/pdftract-core/tests/test_helpers/diagnostics.rs` unifies both shapes)

## Helper Functions for Common Assertions

```rust
/// Assert that extraction produces a specific diagnostic code
pub fn assert_has_diagnostic(result: &ExtractionResult, code: &str) {
    assert!(
        result.metadata.diagnostics_detailed.iter().any(|d| d.code == code),
        "Expected diagnostic with code '{}', got: {:?}",
        code,
        result.metadata.diagnostics_detailed
    );
}

/// Assert that extraction has no diagnostics
pub fn assert_no_diagnostics(result: &ExtractionResult) {
    assert!(
        result.metadata.diagnostics.is_empty(),
        "Expected no diagnostics, got: {:?}",
        result.metadata.diagnostics
    );
    assert!(
        result.metadata.diagnostics_detailed.is_empty(),
        "Expected no structured diagnostics, got: {:?}",
        result.metadata.diagnostics_detailed
    );
}

/// Count diagnostics with a specific code
pub fn count_diagnostics(result: &ExtractionResult, code: &str) -> usize {
    result.metadata.diagnostics_detailed.iter()
        .filter(|d| d.code == code)
        .count()
}

/// Get the structured diagnostics with a specific code
pub fn get_diagnostics(result: &ExtractionResult, code: &str) -> Vec<&DiagnosticJson> {
    result.metadata.diagnostics_detailed.iter()
        .filter(|d| d.code == code)
        .collect()
}
```

## Summary

The errors array provides comprehensive visibility into the PDF extraction process:

- **String form**: `result.metadata.diagnostics` — `Vec<String>`, each entry the diagnostic's message verbatim (compatibility surface; see `pdftract_core::diagnostics_compat`)
- **Structured form**: `result.metadata.diagnostics_detailed` — `Vec<DiagnosticJson>` with `code` / `message` / `severity` / `page_index`? / `location`? / `hint`?; mirrors the string array one-to-one and is the top-level `errors` array of the full JSON output
- **Access**: `result.metadata.diagnostics` and `result.metadata.diagnostics_detailed`
- **Failed-page Count**: `result.metadata.error_count` (the number of pages with `PageResult.error`, independent of diagnostic severity)
- **Assertion Patterns**: Check for specific codes, count errors, verify messages, filter by typed severity, ensure recovery
- **Categories and codes**: the complete registry, severities, phases, and catalog hints are in [`docs/integrations/diagnostics-codes.md`](integrations/diagnostics-codes.md)

This unified diagnostic system allows tests to verify that errors are properly detected, reported, and recovered from during PDF extraction.
