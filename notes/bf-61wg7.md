# bf-61wg7 — Print and analyze extraction result structure

Parent: bf-2goux

## What was done

Added `crates/pdftract-core/examples/dump_extraction_result.rs`, which runs
`extract_pdf()` and prints the full `ExtractionResult` tree in pretty debug
format (`{:#?}`), then explicitly echoes every error/diagnostic location.

Run it with:

```
cargo run --example dump_extraction_result -- <path/to.pdf>
```

Captured debug output (against `tests/fixtures/malformed/truncated-flate.pdf`,
which parses to an empty result) is saved at
`notes/bf-61wg7-debug-output.txt`. The empty case still exercises every
top-level field, so the shape is fully visible.

## Full structure (source: `crates/pdftract-core/src/extract.rs`)

### `ExtractionResult` — top level (extract.rs:237)

| Field | Type | Notes |
|-------|------|-------|
| `fingerprint` | `String` | PDF hash for receipt verification, e.g. `pdftract-v1:ab24…` |
| `pages` | `Vec<PageResult>` | Extracted pages (spans/blocks/tables/errors) |
| `metadata` | `ExtractionMetadata` | Counts, cache status, **diagnostics**, profile info |
| `signatures` | `Vec<SignatureJson>` | Digital signature fields |
| `form_fields` | `Vec<FormFieldJson>` | AcroForm/XFA fields (sorted by name) |
| `links` | `Vec<LinkJson>` | Link annotations (doc-scoped) |
| `attachments` | `Vec<AttachmentJson>` | Embedded files (>50 MB truncated) |
| `threads` | `Vec<ThreadJson>` | Article thread chains |
| `javascript_actions` | `Vec<JavascriptActionJson>` | Detected JS (never executed; TH-04) |

### Where **text / content** is stored

Text lives under `pages`:
`ExtractionResult.pages[i]` (`PageResult`, extract.rs:290) →
- `spans: Vec<SpanJson>` — text fragments with consistent styling (the raw text)
- `blocks: Vec<BlockJson>` — semantic units (paragraphs, headings)
- `tables: Vec<TableJson>` — cell-level table structure
- `annotations: Vec<AnnotationJson>` — non-link annotations

Per-page geometry/labels: `index`, `page_number`, `page_label`, `width`,
`height`, `rotation`, `page_type` (`"text"|"scanned"|"mixed"|"broken_vector"|"blank"|"figure_only"`).

### Where **errors / diagnostics** are stored

There is no single top-level `errors` array. Errors/diagnostics live in three places:

1. **`ExtractionResult.metadata.error_count`** (`usize`) — number of pages that
   failed to extract (`ExtractionMetadata`, extract.rs:396).
2. **`ExtractionResult.metadata.diagnostics`** (`Vec<String>`) — coverage
   warnings and other non-fatal diagnostics emitted during extraction
   (extract.rs:414; serialized only when non-empty).
3. **`ExtractionResult.pages[i].error`** (`Option<String>`) — per-page error
   message when that specific page failed (`PageResult.error`, extract.rs:334;
   serialized only when `Some`).

Fatal errors (unopenable/malformed/encrypted PDF, decompression-bomb limits)
are **not** placed in the struct — `extract_pdf()` returns `Err` instead, so
those never appear in an `ExtractionResult`.

### `ExtractionMetadata` fields (extract.rs:396)

`page_count: usize`, `receipts_mode: ReceiptsMode`, `span_count: usize`,
`block_count: usize`, `cache_status: Option<String>` (`"hit"|"miss"|"skipped"`),
`cache_age_seconds: Option<u64>`, `error_count: usize`,
`reading_order_algorithm: Option<String>` (e.g. `"xy_cut"`),
`diagnostics: Vec<String>`, `profile_name/profile_version: Option<String>`,
`profile_fields: Option<serde_json::Value>`.

## Acceptance criteria — status

- [x] `ExtractionResult` printed with debug format (`{:#?}`)
- [x] Output shows the full structure (see `notes/bf-61wg7-debug-output.txt`)
- [x] errors array location identified (metadata.error_count, metadata.diagnostics, pages[i].error)
- [x] Field names and types documented (tables above)
- [x] Test output can be examined for structure understanding
