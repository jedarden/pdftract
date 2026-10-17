# Canonical structured diagnostics contract

A machine-readable diagnostic has six possible JSON fields. The required
fields are `code`, `message`, and `severity`. The optional fields are
`page_index`, `location`, and `hint`.

```json
{
  "code": "STREAM_DECODE_ERROR",
  "message": "zlib stream truncated mid-inflation",
  "severity": "warning",
  "page_index": 3,
  "location": { "object_number": 12, "generation_number": 0 },
  "hint": "Partial output returned for this stream; consider re-saving the PDF through a normalising tool"
}
```

| Field | Presence | JSON type | Meaning |
| --- | --- | --- | --- |
| `code` | **Required** | string | Stable machine-readable identifier. Catalog-backed codes use `SCREAMING_SNAKE_CASE`; classify by code rather than message. |
| `message` | **Required** | string | Human-readable description. Its wording is not a compatibility key. |
| `severity` | **Required** | string | Exactly `info`, `warning`, `error`, or `fatal`. |
| `page_index` | **Optional** | non-negative integer | Zero-based page index when the diagnostic belongs to a known page. |
| `location` | **Optional** | object | PDF indirect object associated with the diagnostic. It is independent of `page_index` and is not a byte offset. |
| `hint` | **Optional** | string | Guidance for recovery or remediation, when available. |

When `location` is present, both `object_number` (unsigned 32-bit integer) and
`generation_number` (unsigned 16-bit integer) are **required**. A diagnostic
may have a page without an object, or an object without a page.

The complete severity vocabulary is:

| Value | Meaning |
| --- | --- |
| `info` | Informational; output validity is unaffected. |
| `warning` | Output is usable but degraded or uncertain. |
| `error` | Output for an affected region or page is invalid; other output may remain usable. |
| `fatal` | Extraction was aborted without usable output. |

Optional fields are omitted when unavailable; serializers do not emit them as
JSON `null`. A missing optional field or explicit `null` is accepted on input
as unknown. Required fields must be present and non-null. Severity is derived
from the diagnostic code's policy, rather than chosen at each emission site.

The same object shape appears in compact metadata's
`diagnostics_detailed`, full JSON's `errors`, and the diagnostic entries in
the NDJSON footer's `errors`. The legacy `metadata.diagnostics` string array
contains only the corresponding messages. See the
[diagnostic codes guide](integrations/diagnostics-codes.md) for the catalog and
[diagnostics decision record](notes/diagnostics-contract.md) for surface and
compatibility details.
