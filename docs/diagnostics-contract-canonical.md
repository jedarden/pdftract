# Canonical structured diagnostics contract

This document is the canonical machine-readable diagnostics contract. It resolves any disagreement between `diagnostics-codes.md` and `errors-array-format.md`; when either document differs, this document takes precedence. This is a design decision for the next implementation child and does not, by itself, change emission sites.

## Diagnostic

Every structured diagnostic has this JSON shape:

```json
{
  "code": "text.decode.invalid_utf8",
  "message": "Unable to decode text in the object",
  "severity": "warning",
  "page_index": 0,
  "location": {
    "object_number": 12,
    "generation_number": 0
  },
  "hint": "The extracted text may be incomplete."
}
```

The logical type is:

```text
Diagnostic {
  code: string,
  message: string,
  severity: "info" | "warning" | "error",
  page_index: integer | null,
  location: { object_number: integer, generation_number: integer } | null,
  hint: string | null
}
```

Field contract:

| Field | Type | Contract |
| --- | --- | --- |
| `code` | string | Required, never null. Stable machine-readable identifier; never use the human message as the identifier. |
| `message` | string | Required, never null. Human-readable explanation; wording is not a compatibility key. |
| `severity` | string enum | Required, never null. Exactly `info`, `warning`, or `error`. |
| `page_index` | integer or null | Zero-based page index within the input document. Use `null` for a document-level diagnostic with no affected page. |
| `location` | object or null | PDF source location. Use `null` when no attributable indirect object exists. |
| `location.object_number` | integer | Required when `location` is present; PDF indirect object number. |
| `location.generation_number` | integer | Required when `location` is present; PDF indirect object generation number. |
| `hint` | string or null | Optional remediation or recovery guidance. Serialize `null` when no hint exists; do not omit the field. |

`page_index` identifies the page containing the affected content, not a PDF page label. `location` identifies the PDF object that caused or was being processed when the diagnostic was produced; it is neither a byte offset nor a page number. A diagnostic can have a page without an attributable object and can have an object without a page, so the two nullable fields are independent.

Severity is handling metadata: `info` is informational, `warning` means extraction continued with degraded or uncertain output, and `error` means the affected extraction operation failed or could not produce its promised result. A diagnostic does not by itself require the enclosing request to fail.

## Serialization and surfaces

Use the existing JSON serialization conventions: snake_case names and JSON `null` for nullable values. JSON output exposes diagnostics as an array of Diagnostic objects under the field name already defined by that API surface. NDJSON output uses the same Diagnostic object as the relevant record; it must not encode a diagnostic as a JSON string or invent a second line-oriented grammar. The enclosing extraction result and its surface-specific field name remain unchanged; this document standardizes the element type and semantics.

The typed Rust/API representation must preserve these distinctions: severity is an enum with exactly the three wire values, `page_index` and `location` are optional, and `hint` is optional. Consumers must not rely on JSON object key order.

## Legacy `Vec<String>` compatibility

Existing APIs that expose `Vec<String>` remain supported as deprecated compatibility projections. Each Diagnostic is rendered as:

```text
[<severity>] <code>: <message>[ — <hint>]
```

Include the ` — <hint>` suffix only when `hint` is non-null. Do not put `page_index` or `location` into this string: the legacy type has no structured location contract, and the projection is intentionally lossy and not parseable as a replacement for Diagnostic. New APIs and new emission sites must use structured diagnostics; existing `Vec<String>` fields remain available until a separately approved removal decision.

## Implementation boundary

The next implementation child must update extraction, JSON, NDJSON, and compatibility adapters to produce this one Diagnostic type. It must retain the legacy string projection only at the compatibility boundary and test all severity values, null page/location combinations, null and non-null hints, JSON/NDJSON serialization, and the legacy rendering.
