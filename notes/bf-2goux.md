# bf-2goux — Run extraction on truncated-flate fixture

Task: extract `tests/fixtures/malformed/truncated-flate.pdf` to examine the
extraction result structure, especially the errors array.

## What was run

```
cargo run --example dump_extraction_result -- tests/fixtures/malformed/truncated-flate.pdf
```

(The `dump_extraction_result` example and the
`test_truncated_flate_extraction_result_structure` test already exist from
sibling beads bf-61wg7 / bf-45n42. This bead exercised them and documented the
result-structure layout.)

## Observed result (this fixture)

```
ExtractionResult {
    fingerprint: "pdftract-v1:ab24a95f...",
    pages: [],                       // <- empty: truncated page not enumerable
    metadata: ExtractionMetadata {
        page_count: 0,
        error_count: 0,              // <- no page-level failure recorded
        diagnostics: [],             // <- empty
        ...
    },
    signatures: [], form_fields: [], links: [], attachments: [],
    threads: [], javascript_actions: [],
}
```

Extraction completes cleanly (no panic, no `Err`). The truncated FlateDecode
stream does **not** surface as an error object — instead the structurally
declared page is not enumerable, so `pages` is empty and `error_count == 0`.

## Where errors / diagnostics live in the result structure

There is no single top-level `errors` array on `ExtractionResult`. Error and
diagnostic information is stored in three places:

1. **`ExtractionResult.metadata.error_count: usize`**
   (`crates/pdftract-core/src/extract.rs:410`) — count of pages that failed to
   extract.
2. **`ExtractionResult.metadata.diagnostics: Vec<String>`**
   (`crates/pdftract-core/src/extract.rs:416`) — coverage warnings / diagnostics
   emitted during extraction. Skipped from JSON when empty.
3. **`PageResult.error: Option<String>`**
   (`crates/pdftract-core/src/extract.rs:336`) — per-page error message, set when
   extraction fails for that specific page. Skipped from JSON when `None`.

Note: the JSON schema layer (`schema/mod.rs:1539`) exposes a document-level
`errors: Vec<DiagnosticJson>`, and lower parser layers (`parser/stream.rs`,
`content_stream.rs`, `font/*`, `parser/catalog.rs`, `parser/xref.rs`, etc.)
each carry their own `diagnostics: Vec<Diagnostic>` collections, but these are
not aggregated onto the core `ExtractionResult` for this code path.

## Acceptance criteria

- [x] Opens `truncated-flate.pdf` (fingerprint resolved).
- [x] Extraction completes without panic (empty pages, no error).
- [x] Result structure understood (see above).
- [x] Error/diagnostic locations identified (three fields, listed above).
