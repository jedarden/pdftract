# bf-4g6dj — Examine truncated-flate test scaffold and current extraction result

**Type:** explore · **Parent:** [[bf-mzf4i]] · **Children (closed):** [[bf-4cyyf]], [[bf-4fb3b]]
**Re-verified against current source:** 2026-07-22

This bead is the scaffold/extraction-result examination step for the STREAM_DECODE_ERROR
assertion work ([[bf-mzf4i]]). Its children already produced detailed docs
([[bf-4cyyf]] `docs/errors-array-format.md`, [[bf-4fb3b]] `notes/bf-4fb3b.md`); this note
records the *examination itself* with line references re-verified against current source, and
states the single most important structural fact an implementer must not miss. It supersedes an
earlier draft of this note (Jul 6) which mis-located the scaffold as `stream_decoder_fixtures.rs`
and incorrectly claimed the fixture loop "already checks `expected_diags`" — it does not (see §4).

---

## 1. The scaffold: `tests/test_truncated_flate_recovery.rs`

File examined in full (346 lines, 7 tests). Structure:

| Test | What it does | Result surface it touches |
|---|---|---|
| `test_truncated_flate_fixture_exists` | fixture exists + non-empty | filesystem only |
| `test_truncated_flate_parses_as_pdf` | `parse_pdf_file` → non-empty `pages` | `(fingerprint, catalog, pages, resolver)` tuple |
| `test_truncated_flate_emits_diagnostics` | parses; **comments that "Diagnostics are not currently surfaced through parse_pdf_file"** (≈:83–88) | none (scaffold) |
| `test_truncated_flate_partial_content_accessible` | first page mediabox = 4 values | `Page.media_box`, `Page.contents` |
| `test_truncated_flate_extraction_result_structure` | `PdfExtractor::open` → `materialize_pages` → `extract_page(0)`; serializes result to JSON | `PageExtraction` (only called if pages non-empty) |
| `test_truncated_flate_materialize_pages` | `materialize_pages()` returns `Ok`, stable across calls; **notes "the slice is expected to be empty"** (≈:186–187) | `Page` |
| `test_truncated_flate_extract_page_returns_result` | calls `extract_page(0)` **unconditionally** as `Result<PageExtraction>`; **notes it returns `Err` index-out-of-bounds** (≈:301–304) | `PageExtraction` |
| `test_truncated_flate_opens_with_extractor` | `open` + `fingerprint` + `page_count` | handle |

**Key behavior the scaffold encodes:** for `truncated-flate.pdf`, `materialize_pages()` yields
an **empty** page slice, so `extract_page(0)` returns `Err` (index out of bounds). Nothing ever
traverses the truncated FlateDecode stream through the full-extraction path on this fixture.
That is the gap this chain is reconciling.

## 2. The "errors array" — location and format (re-verified)

There is **no single `errors` field on the type the scaffold's tests use**. Error/diagnostic
information is split across two representations and three internal fields. Verified line refs:

### 2a. `PageExtraction` — the type `extract_page()` returns — has NO errors field
`crates/pdftract-core/src/document.rs:626`:
```rust
pub struct PageExtraction {
    pub index: usize, width: f64, height: f64, rotation: i32,
    pub spans: Vec<SpanData>, pub blocks: Vec<BlockData>,
}
```
**This is why a STREAM_DECODE_ERROR assertion cannot be read off the result of the existing
scaffold's `extract_page()` call** — there is nowhere to read it from. This is the single most
load-bearing fact for the implementer; it is what distinguishes this examination from a naïve
"add an assertion that checks `result.errors`."

### 2b. Internal `ExtractionResult` (unit/integration-test surface)
`crates/pdftract-core/src/extract.rs`:
- `ExtractionResult` (`:237`) — `pages: Vec<PageResult>`, `metadata: ExtractionMetadata`.
- `ExtractionMetadata` (`:396`):
  - `error_count: usize` (`:410`) — pages that failed to extract.
  - `diagnostics: Vec<String>` (`:416`) — each `"CODE: message"`; `#[serde(skip_serializing_if = "Vec::is_empty")]`.
- `PageResult.error: Option<String>` (`:336`) — per-page failure; skipped when `None`.

### 2c. JSON `Output.errors` (structured array / JSON consumer surface)
`crates/pdftract-core/src/schema/mod.rs`:
- `Output { ..., pub errors: Vec<DiagnosticJson> }` (`:1539`).
- `DiagnosticJson` (`:813`): `code`, `message`, `severity` (`"info"|"warning"|"error"|"fatal"`),
  `page_index: Option<usize>`, `location: Option<ObjectLocationJson>`, `hint: Option<String>`.
- `ObjectLocationJson` (`:841`): `object_number`, `generation_number`.
- Produced by `result_to_output()` in `output/json.rs`.

### 2d. The code in play
- `DiagCode::StreamDecodeError` → string `"STREAM_DECODE_ERROR"` (`src/diagnostics.rs:465`,
  `:1278`).
- **The bead chain's original title string `STREAM_DECOMPRESS_ERROR` does not exist** in the
  codebase; the correct code is `STREAM_DECODE_ERROR` (settled by [[bf-348zd]]).
- `STREAM_DECODE_ERROR` is severity `warning` (output usable but degraded).

## 3. Why the scaffold's path emits nothing for this fixture (re-verified)

`FlateDecoder::decode_impl` (`src/parser/stream.rs:520–554`) loops reading decoded bytes; on
`UnexpectedEof` (`:542–544`) — a truncated stream — it **`break`s and returns the partial
`output`** (INV-8 soft recovery). Every caller wraps that in `Ok`, so decode of the truncated
fixture returns `Ok(partial)`, never `Err`.

Because `materialize_pages()` yields an empty slice for this fixture (§1), the full extraction
pipeline never traverses the truncated stream, so **no `emit!(… STREAM_DECODE_ERROR …)` fires
on this path**, and `ExtractionMetadata.diagnostics` / `Output.errors` are both empty. Verified
by [[bf-2goux]] running `dump_extraction_result`. An assertion of the form
`output.errors.iter().any(|e| e.code == "STREAM_DECODE_ERROR")` therefore cannot pass for
`truncated-flate.pdf` today — the sibling example in [[bf-4cyyf]] that uses this pattern is
retracted by [[bf-4fb3b]] §0/§2.

## 4. Where to add the assertion (AC-3)

Given §2a + §3, the STREAM_DECODE_ERROR assertion does **not** belong in
`test_truncated_flate_recovery.rs` (the file this bead examines) — that scaffold can only ever
see a `PageExtraction` (no errors field) or an index-out-of-bounds `Err`. The canonical home is
the low-level decoder fixture loop, keyed off the **decode outcome**, not the full-extraction
errors array:

- **File:** `crates/pdftract-core/tests/stream_decoder_fixtures.rs`,
  `test_all_stream_decoder_fixtures()` per-fixture loop.
- **Signal:** decode returns `Ok(partial)` (the `Ok` path), not `Err`.
- **Hard prerequisite:** make `FixtureInfo.expected_diags` **live** (the loop currently does
  *not* read it — it is dead data, `:62`) and set
  `flate_truncated.expected_diags = vec![DiagCode::StreamDecodeError]` (currently `vec![]`).
  The earlier note's claim that the loop "already checks `expected_diags`" is wrong.
- **Avoid:** the false pass from the 0-byte `.expected` file, byte-asserting partial content,
  and `decoded.len()` as an error proxy.

The full ordered implementation guide (steps, ready-to-adapt selector + INV-8 regression guard,
failure-message contract, EC1–EC13 edge-case taxonomy, and the warning against running the
~2 GB `flate_bomb_3gb` fixture) is consolidated in [[bf-4fb3b]] §5. The implementer bead is
`bf-4bx00` (→ [[bf-mzf4i]]).

## 5. Acceptance criteria status

- [x] **Test file examined and structure understood** — §1: all 7 tests mapped, with the empty-
  page-slice / `extract_page`→`Err` behavior the scaffold documents.
- [x] **Errors array location and format documented** — §2: `PageExtraction` (no errors field,
  `document.rs:626`), internal `ExtractionResult.metadata.{error_count,diagnostics}` +
  `PageResult.error` (`extract.rs:237/396/410/416/336`), and JSON
  `Output.errors: Vec<DiagnosticJson>` (`schema/mod.rs:1539/813/841`); code
  `STREAM_DECODE_ERROR` (`diagnostics.rs:1278`), all line-verified.
- [x] **Clear understanding of where to add the assertion** — §3 + §4: not in this scaffold's
  `PageExtraction`/`output.errors` path (vacuous for this fixture); in
  `stream_decoder_fixtures.rs` keyed off the `Ok(partial)` decode outcome; handed off to
  `bf-4bx00` with [[bf-4fb3b]] §5 as the implementation guide.

## 6. References

- Parent: [[bf-mzf4i]] (umbrella — add STREAM_DECODE_ERROR assertion) → genesis `pdftract-qkc77`.
- Children: [[bf-4cyyf]] (`docs/errors-array-format.md`), [[bf-4fb3b]] (`notes/bf-4fb3b.md`,
  the exhaustive consolidation). Implementer: `bf-4bx00`.
