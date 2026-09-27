# pdftract Diagnostics Contract

**Status:** Decided — canonical contract for machine-readable diagnostics.
**Decided:** 2026-09-26, bead `pdftract-5928d4ce` (child of `pdftract-d79310ab`).
**Inventory update:** 2026-09-26, bead `pdftract-1702243e` (documentation and
inventory only; no emitter or serialization implementation changes).
**Supersedes:** the earlier documentation mismatch between
`docs/integrations/diagnostics-codes.md` (structured objects) and
`docs/errors-array-format.md` (string array) as originally shipped; both
guides now describe the same canonical structured surface plus retained
legacy projection and must continue to agree with it.

This document is the single decision record for what a pdftract diagnostic
*is*. The enforcement artifacts (types, tests, integration guides) are listed
in [What pins this contract](#what-pins-this-contract); if this document and a
pinned test disagree, the test wins and this document must be fixed — if this
document and an *unpinned* doc disagree, this document wins until the doc is
corrected.

## Decision

**The typed diagnostic object is canonical.** `pdftract_core::diagnostics::
Diagnostic` (in-process) and `DiagnosticJson` (serialized) — with `code`,
`message`, `severity`, and optional `page_index`, `location`, `hint` — are the
machine-readable surface. `ExtractionResult.metadata.diagnostics`
(`Vec<String>`) is a **message-only compatibility surface**: it exists so
existing string-array consumers keep working byte-for-byte, and it is not a
place to add information.

Alternatives considered and rejected:

- **Rewrite `diagnostics-codes.md` to document only the string array.**
  Rejected: the string carries no code, severity, page, location, or hint, so
  codes could not be pattern-matched (the plan's Diagnostic Code Catalog
  declares codes part of the public API surface), severities could not drive
  exit-code/quality decisions, and the per-code hint catalog would have
  nowhere to surface. INV-8 (all errors recoverable, surfaced as diagnostics)
  presumes diagnostics are classifiable; a bare string is not.
- **Make the string array canonical and fold context into it**
  (`"CODE: message (byte offset N)"`). Rejected: that is `Diagnostic`'s
  `Display` form, which is lossy and unparseable in general (codes and
  offsets inside human text); every downstream consumer would have to
  substring-guess. This was the historical accident behind the documentation
  mismatch; freezing it would make that ambiguity permanent.

The decision is already implemented and pinned at HEAD; the remaining parent
scope (`pdftract-0f67a34a`, `pdftract-b4b36e15`, `pdftract-b2506ab5`) is
completion/alignment work under this contract, not re-deciding it.

## Canonical typed model (in-process)

Defined in `crates/pdftract-core/src/diagnostics.rs`:

- `DiagCode` — one enum variant per diagnostic code (SCREAMING_SNAKE_CASE
  wire names via `DiagCode::name()`, e.g. `STREAM_DECODE_ERROR`).
  `DiagCode::ALL` iterates the full set; `DiagCode::policy()` returns the
  code's deterministic `DiagnosticPolicy`.
- `Severity` — exactly four variants, serialized lowercase:
  `info` | `warning` | `error` | `fatal`. A code's severity is derived from
  the code (policy), never chosen ad hoc at an emission site.
- `DiagnosticPolicy { severity, message, hint }` — the single source of
  truth for severity, the fallback message, and the actionable hint.
- `DiagnosticContext` — the emission-site builder: start from
  `DiagnosticContext::for_code(code)` (policy-populated), then attach only
  what the site knows: `.with_message(..)`, `.with_byte_offset(_opt)(..)`,
  `.with_object_ref(_parts)(_opt)(..)`, `.with_page_index(_opt)(..)`.
  Location context is attached as structured fields — it is never appended to
  the message, where it would be lossy.
- `Diagnostic` — `code`, `byte_offset: Option<u64>`,
  `object_ref: Option<ObjRef>`, `page_index: Option<u32>` (zero-based),
  `severity` (retained from policy at emission), `message: Cow<'static,str>`,
  `hint: Option<&'static str>` (retained from policy).

`Diagnostic::from_context` asserts the context's severity and hint match the
code's policy — an emission site cannot invent them.

`DIAGNOSTIC_CATALOG` (`&[DiagInfo]`) holds one row per `DiagCode` variant
(including `suggested_action`, the hint source). Variant count and catalog
count must stay equal; `diagnostics_catalog_drift` enforces this plus
doc-table agreement.

## JSON/NDJSON representation

`DiagnosticJson` (`crates/pdftract-core/src/schema/mod.rs`) is the serialized
envelope. Exact field names and order:

```json
{"code":"STREAM_DECODE_ERROR","message":"zlib stream truncated mid-inflation","severity":"warning","page_index":3,"location":{"object_number":12,"generation_number":0},"hint":"Partial output returned for this stream; consider re-saving the PDF through a normalising tool"}
```

| Field | Type | Presence |
|---|---|---|
| `code` | string, `DiagCode::name()` | always |
| `message` | string | always |
| `severity` | `"info"` \| `"warning"` \| `"error"` \| `"fatal"` | always; from the code policy, never a substring guess |
| `page_index` | number, zero-based | only when page-scoped; **omitted** for document-level |
| `location` | `{"object_number": u32, "generation_number": u16}` | only when an indirect object is known; **omitted** otherwise |
| `hint` | string | only when the code's catalog entry defines one; **omitted** otherwise (every current entry defines one, but consumers must tolerate absence) |

Omission rules:

- Optional fields are **omitted, never serialized as `null`** — enforced by
  `#[serde(skip_serializing_if = "Option::is_none")]` on `page_index`,
  `location`, and `hint`. Deserialization uses `#[serde(default)]`-style
  optional handling, so `null` is tolerated on input but never produced on
  output.
- Adding a new *optional* field is backward-compatible; renaming or re-typing
  any existing field, or adding a new severity value, is a breaking change to
  both surfaces (pinned by tests, see below).
- `Diagnostic::byte_offset` is **in-process only** — it is carried on the
  typed model and rendered by `Diagnostic`'s `Display`, but it is NOT part of
  `DiagnosticJson` and does not appear in any machine-readable output. If a
  byte offset ever needs to reach consumers it must be added as a new optional
  JSON field, not folded into `message`.

### Where structured diagnostics appear

| Surface | Field | Empty behavior |
|---|---|---|
| Compact result, legacy | `metadata.diagnostics` (`Vec<String>`, message verbatim) | omitted entirely when empty |
| Compact result, canonical | `metadata.diagnostics_detailed` (`Vec<DiagnosticJson>`) | omitted entirely when empty |
| Full JSON output (`schema_version` 1.0) | top-level `errors` (`Vec<DiagnosticJson>`), populated from `diagnostics_detailed` | **always present**, `[]` when empty |
| NDJSON footer frame | `errors` — a union array (below) | **always present**, `[]` when empty |
| NDJSON page frame | `errors` | present only when that page failed to extract |

NDJSON footer `errors` is a **union**, in this order:

1. One synthetic record per failed page:
   `{"code":"page_extraction_error","severity":"error","message":"<page error text>"}`
   — `page_extraction_error` is a lowercase synthetic wire label; it is
   deliberately **not** a `DiagCode` variant and never appears in
   `metadata.diagnostics_detailed` or the catalog.
2. Then the document's canonical diagnostics (`diagnostics_detailed`) in
   emission order, serialized identically to the full output's `errors`.

Consumers of the footer array must therefore treat `code` case-insensitively
or dispatch on the two known shapes; consumers of every other surface see
only catalog codes.

The mirror invariant: `metadata.diagnostics` and
`metadata.diagnostics_detailed` are one-to-one — same length, same order, and
string element `i` is exactly `diagnostics_detailed[i].message`.

## Legacy string compatibility (`Vec<String>`)

`ExtractionResult.metadata.diagnostics` is produced exclusively by
`pdftract_core::diagnostics_compat::to_legacy_strings`
(`crates/pdftract-core/src/diagnostics_compat.rs`). The compatibility
contract, binding on all future work:

1. **Byte-for-byte stability.** For a given emission sequence, the legacy
   strings must remain identical across releases. Element `i` is the
   diagnostic's `message` verbatim — no `CODE:` prefix, no
   `(byte offset N)` or `[obj G R]` suffix, no trimming, no placeholder for
   empty messages.
2. **Order, length, and duplicates preserved.** The arrays mirror one-to-one
   by construction; nothing is deduplicated or reordered.
3. **No new information in legacy strings.** Code, severity, page, location,
   and hint are only on the structured entry. New diagnostic information is
   added as optional structured fields and must not change legacy bytes.
4. **Severity strings are pinned.** `"info"`, `"warning"`, `"error"`,
   `"fatal"` on the structured surface; a new severity is a breaking change
   to both surfaces.

Because a legacy entry carries no code, consumers that need to *identify* a
diagnostic (rather than display it) must use `diagnostics_detailed` and pair
by index if they also need the legacy string. `Diagnostic`'s `Display`
(`"CODE: message (byte offset N)? [obj G R]?"`) is a human/debug rendering —
it is not the legacy array and must not be used as a serialization format.

## Code registry

- `DiagCode` + `DIAGNOSTIC_CATALOG` in
  `crates/pdftract-core/src/diagnostics.rs` are the **authoritative**
  registry (113 variants / 113 catalog entries at decision time; the drift
  test enforces variant = catalog = doc-row agreement).
- The published catalog table is
  `docs/integrations/diagnostics-codes.md`. `cargo test -p pdftract-core
  --test diagnostics_catalog_drift` (and the same with `--all-features`)
  fails when a documented code's severity disagrees with the emitted policy,
  an emitted code lacks a catalog row, or the doc lists a code nothing emits
  (unless the row is marked `(reserved)` or feature-gated).
- Naming: `CATEGORY_SPECIFIC_ISSUE`, SCREAMING_SNAKE_CASE. Codes are public
  API; renaming requires a Revision History entry and a deprecation window.
- Severities: `info` (no output impact), `warning` (output usable but
  degraded), `error` (this region/page invalid, others OK), `fatal`
  (extraction aborted). Serialized lowercase per the table above.
- `docs/plan/plan.md`'s "Diagnostic Code Catalog" table is a **historical
  planning subset**, not the registry. Its divergences from the registry are
  known and intentional to leave alone until the plan is revised:
  `warn` (vs serialized `warning`), `GLYPH_UNMAPPED` (vs
  `FONT_GLYPH_UNMAPPED`), `BROKENVECTOR_OCR_UNAVAILABLE` (vs
  `OCR_BROKENVECTOR_UNAVAILABLE`). The registry and the integration guide
  win over the plan table.
- Feature-gated codes (e.g. `CJK_*` requires the `cjk` feature) are exempt
  from drift enforcement when compiled out; the doc rows annotate the gate.

## Emission paths downstream work must cover

Mechanism: emission sites build a `DiagnosticContext` (or use the
`Diagnostic::with_*` constructors, which route through the same policy
assertions) and hand it to their layer's diagnostics sink
(`&mut Vec<Diagnostic>` / `take_diagnostics()`); `extract.rs` aggregates all
layers into `metadata.diagnostics_detailed` (as `DiagnosticJson::from`) and
`metadata.diagnostics` (via `to_legacy_strings`), preserving emission order;
`output::json::result_to_output` clones `diagnostics_detailed` into the
top-level `errors`; `output::ndjson::pipeline::footer_errors` builds the
footer union.

Per-layer context threading is pinned by dedicated suites (each named test
asserts the layer's diagnostics retain code/severity/hint policy plus
`page_index`/`object_ref` where the layer knows them):

- Object/lexer/objstm/xref/parser/decoder layers —
  `tests/diagnostics_parser_decoder_context.rs`
- Page parsing, classification, content-stream processing —
  `tests/diagnostics_page_extraction_context.rs`
- Policy/context/serialization round-trip —
  `tests/diagnostics_context_contract.rs`

Known gaps and hazards for the remaining parent scope (beads
`pdftract-0f67a34a` → `pdftract-b4b36e15` → `pdftract-b2506ab5`):

1. **Dead legacy module.** `crates/pdftract-core/src/parser/diagnostic.rs`
   declares its own smaller `DiagCode`/`Severity`/`Diagnostic` (two
   severities, a `phase` field, a default-code footgun in
   `Diagnostic::new`). It has **zero consumers** — `parser/mod.rs`
   re-exports the canonical `crate::diagnostics` types. Downstream work must
   not import from `parser::diagnostic`; it should be deleted or gutted to a
   re-export when touched, so no new site binds to the wrong enum.
2. **Context population is per-site.** The contract requires `page_index`,
   `location`, and hint to be attached **when the site knows them** and
   omitted otherwise — it does not require every emitter to attach a page.
   Threading work (`pdftract-b4b36e15`) is exactly the sweep of emission
   sites that know page/object context but do not yet attach it; sites that
   genuinely cannot know (document-level lexer events) correctly omit.
3. **Reserved codes.** Catalog rows marked `(reserved)` have no emission
   path yet; wiring them (or removing the reservation) is emitter work, not
   contract work. NDJSON page failures use the synthetic
   `page_extraction_error` label (see above), which must stay out of the
   enum.
4. **`error_count` semantics.** `ExtractionMetadata.error_count` counts
   **failed pages** (incremented in `extract.rs` on per-page failure), not
   error/fatal diagnostics. `docs/errors-array-format.md` "Pattern 6"
   records this failed-page behavior; keep that example aligned with the
   implementation if the field changes.
5. **Hint population.** Hints come from the catalog policy
   (`suggested_action`), not from emission sites. Every catalog entry
   currently defines one, so structured diagnostics carry `hint` today, but
   consumers and tests must tolerate its absence per the omission rule.

+## Code inventory and emitter map

The published catalog in
[`docs/integrations/diagnostics-codes.md`](../integrations/diagnostics-codes.md)
is the complete 113-row inventory. The audit manifest below repeats every row
with the machine severity and the committed source modules that emit it. This
makes the documentation decision reviewable without treating a planned code as
implemented.

The context columns use this vocabulary:

- `optional`: the serialized field is available when the emitting layer knows
  the value; the value is zero-based for `page_index`, and
  `location` is an indirect PDF object with `object_number` and
  `generation_number`. It is not a promise that every occurrence has that
  context.
- `—`: no production emitter exists in HEAD, so no occurrence currently
  supplies context. This is not implementation work; the row remains in the
  public catalog for compatibility/planning.
- `catalog`: the hint comes from the code policy's
  `DIAGNOSTIC_CATALOG.suggested_action`, not from an ad hoc emitter string.
  Every current catalog row has a hint; consumers must still tolerate an
  omitted JSON `hint` for forward compatibility.

When an active emitter has no page or object context, it uses the same fallback
as a document-level event: the field is omitted from JSON/NDJSON. A source
path below is an emitter map, not a list of tests or a claim that all
call-sites have been context-threaded.

| Code | Severity | page_index | location | hint | Emitter module(s) in HEAD |
|---|---|---|---|---|---|
| `STRUCT_INVALID_NAME` | warning | optional | optional | catalog | `parser/lexer/mod.rs` |
| `STRUCT_INVALID_HEX` | warning | optional | optional | catalog | `parser/lexer/mod.rs` |
| `STRUCT_INVALID_OCTAL` | warning | optional | optional | catalog | `parser/lexer/mod.rs` |
| `STRUCT_INVALID_STREAM_HEADER` | warning | optional | optional | catalog | `parser/lexer/mod.rs` |
| `STRUCT_UNEXPECTED_BYTE` | warning | optional | optional | catalog | `document.rs`, `conformance.rs`, `parser/lexer/mod.rs`, `parser/object/parser.rs` |
| `STRUCT_UNEXPECTED_EOF` | warning | optional | optional | catalog | `attachment/name_tree.rs`, `attachment/associated_files.rs`, `attachment/filespec.rs`, `threads/mod.rs`, `parser/lexer/mod.rs`, `parser/struct_tree.rs`, `parser/inline_image.rs`, `parser/ocg.rs`, `parser/outline.rs`, `parser/object/parser.rs`, `parser/catalog.rs`, `forms/mod.rs`, `forms/xfa.rs`, `forms/combiner.rs`, `signature/mod.rs` |
| `STRUCT_UNTERMINATED_STRING` | warning | optional | optional | catalog | `parser/lexer/mod.rs` |
| `STRUCT_MISSING_KEY` | warning | optional | optional | catalog | `threads/mod.rs`, `attachment/filespec.rs`, `document.rs`, `render/pdfium_path.rs`, `encryption/mod.rs`, `render/image_compositing.rs`, `encryption/decryptor.rs`, `content_stream.rs`, `parser/struct_tree.rs`, `parser/inline_image.rs`, `parser/catalog.rs`, `parser/ocg.rs`, `parser/object/parser.rs`, `parser/objstm.rs`, `parser/outline.rs`, `parser/pages.rs` |
| `STRUCT_CIRCULAR_REF` | warning | optional | optional | catalog | `parser/struct_tree.rs`, `parser/objstm.rs`, `parser/outline.rs`, `parser/xref.rs`, `parser/object/cache.rs`, `parser/pages.rs` |
| `STRUCT_XOBJECT_CYCLE` | warning | optional | optional | catalog | `content_stream.rs`, `font/type3_rasterizer.rs` |
| `STRUCT_DEPTH_EXCEEDED` | warning | optional | optional | catalog | `content_stream.rs`, `parser/pages.rs`, `parser/xref.rs`, `parser/object/cache.rs`, `parser/outline.rs`, `parser/object/parser.rs`, `parser/objstm.rs` |
| `STRUCT_INVALID_DICT_VALUE` | warning | optional | optional | catalog | `content_stream.rs`, `parser/inline_image.rs`, `parser/object/parser.rs` |
| `STRUCT_INVALID_DICT_KEY` | warning | optional | optional | catalog | `content_stream.rs`, `parser/inline_image.rs`, `parser/object/parser.rs` |
| `STRUCT_INVALID_INDIRECT_HEADER` | warning | optional | optional | catalog | `parser/object/parser.rs` |
| `STRUCT_INTEGER_OVERFLOW` | warning | optional | optional | catalog | `parser/lexer/mod.rs`, `parser/object/parser.rs` |
| `STRUCT_REAL_INVALID` | warning | optional | optional | catalog | `parser/lexer/mod.rs` |
| `STRUCT_INVALID_NUMBER` | warning | optional | optional | catalog | `parser/lexer/mod.rs` |
| `STRUCT_INVALID_ASCII85` | warning | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `STRUCT_INVALID_OBJSTM` | warning | optional | optional | catalog | `parser/objstm.rs` |
| `STRUCT_INVALID_GEOMETRY` | warning | optional | optional | catalog | `fingerprint/canonicalize.rs` |
| `STRUCT_INVALID_TYPE` | warning | optional | optional | catalog | `content_stream.rs`, `attachment/name_tree.rs`, `attachment/associated_files.rs`, `attachment/filespec.rs`, `render/pdfium_path.rs`, `render/image_compositing.rs`, `parser/struct_tree.rs`, `parser/inline_image.rs` |
| `STRUCT_INVALID_UTF16` | warning | optional | optional | catalog | `signature/mod.rs`, `parser/outline.rs` |
| `STRUCT_UNRESOLVED_DESTINATION` | warning | optional | optional | catalog | `parser/outline.rs` |
| `STRUCT_NON_GOTO_OUTLINE` | warning | optional | optional | catalog | `parser/outline.rs` |
| `STRUCT_INVALID_PDFDOC_ENCODING` | warning | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `STRUCT_HYBRID_CONFLICT` | warning | optional | optional | catalog | `parser/xref.rs` |
| `STRUCT_INCOMPLETE_COVERAGE` | info | optional | optional | catalog | `parser/struct_tree.rs` |
| `STRUCT_INVALID_PREV_OFFSET` | warning | optional | optional | catalog | `parser/xref.rs` |
| `STRUCT_INVALID_HINT_STREAM` | warning | optional | optional | catalog | `parser/hint_stream.rs` (`parse_hint_stream`, `parse_hint_stream_from_linearized`) |
| `STRUCT_INVALID_BDC_OPERAND` | info | optional | optional | catalog | `content_stream.rs`, `parser/marked_content_operators.rs` |
| `XREF_INVALID_HEADER` | warning | optional | optional | catalog | `parser/xref.rs` |
| `XREF_INVALID_ENTRY` | warning | optional | optional | catalog | `parser/xref.rs` |
| `XREF_INVALID_SUBSECTION_HEADER` | warning | optional | optional | catalog | `parser/xref.rs` |
| `XREF_OBJECT_ZERO_NOT_FREE` | warning | optional | optional | catalog | `parser/xref.rs` |
| `XREF_TRAILER_NOT_FOUND` | warning | optional | optional | catalog | `parser/xref.rs` |
| `XREF_TRUNCATED` | warning | optional | optional | catalog | `parser/xref.rs` |
| `XREF_REPAIRED` | info | optional | optional | catalog | `parser/xref.rs` |
| `XREF_LINEARIZED_NO_FORWARD_SCAN` | warning | optional | optional | catalog | `parser/xref.rs` |
| `XREF_REMOTE_NO_FORWARD_SCAN` | warning | optional | optional | catalog | `parser/xref.rs` |
| `XREF_INVALID_STREAM_FORMAT` | warning | optional | optional | catalog | `parser/xref.rs` |
| `XREF_INVALID_STREAM_ENTRY` | warning | optional | optional | catalog | `parser/xref.rs` |
| `STREAM_DECODE_ERROR` | warning | optional | optional | catalog | `parser/xref.rs`, `parser/objstm.rs` |
| `STREAM_BOMB` | error | optional | optional | catalog | `parser/stream.rs`, `render/image_compositing.rs` |
| `STREAM_UNKNOWN_FILTER` | warning | optional | optional | catalog | `parser/stream.rs` |
| `STREAM_INVALID_PARAMS` | warning | optional | optional | catalog | `parser/stream.rs` |
| `STREAM_INVALID_JPEG` | warning | optional | optional | catalog | `parser/stream.rs` |
| `STREAM_INVALID_CCITT` | warning | optional | optional | catalog | `parser/stream.rs` |
| `STREAM_TRUNCATED` | warning | optional | optional | catalog | `render/image_compositing.rs` |
| `STREAM_INVALID_JPX` | warning | optional | optional | catalog | `decoder/jpx.rs` |
| `ENCRYPTION_UNSUPPORTED` | fatal | optional | optional | catalog | `encryption/detection.rs` (`detect_encryption`), `encryption/mod.rs`, `encryption/decryptor.rs`, `parser/stream.rs` |
| `ENCRYPTION_WRONG_PASSWORD` | fatal | optional | optional | catalog | `encryption/mod.rs`, `encryption/decryptor.rs`, `parser/stream.rs` |
| `ENCRYPTION_INVALID_DICT` | fatal | optional | optional | catalog | `encryption/detection.rs` (`parse_hash_with_diagnostics`) |
| `PAGE_OUT_OF_RANGE` | error | optional | optional | catalog | `pages.rs` |
| `PAGE_INVALID_COUNT` | warning | optional | optional | catalog | `parser/pages.rs` |
| `PAGE_INVALID_ROTATE` | warning | optional | optional | catalog | `content_stream.rs`, `parser/pages.rs` |
| `FONT_GLYPH_UNMAPPED` | warning | optional | optional | catalog | `font/resolver.rs` |
| `FONT_NOT_FOUND` | warning | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `FONT_INVALID_CMAP` | warning | optional | optional | catalog | `font/cmap.rs`, `font/codespace.rs` |
| `FONT_PARSE_FAILED` | warning | optional | optional | catalog | `font/type3.rs`, `font/type0.rs`, `font/embedded.rs` |
| `FONT_UNSUPPORTED` | warning | optional | optional | catalog | `font/embedded.rs` |
| `FONT_CIDTOGIDMAP_TRUNCATED` | warning | optional | optional | catalog | `font/type0.rs` |
| `ENCODING_DIFFERENCE_OUT_OF_RANGE` | warning | optional | optional | catalog | `font/encoding.rs` |
| `FONT_TYPE3_WIDTHS_LENGTH_MISMATCH` | warning | optional | optional | catalog | `font/type3.rs` |
| `CMAP_INVALID_CODESPACE` | warning | optional | optional | catalog | `cmap/codespace.rs` |
| `CJK_DECODE_MALFORMED` | warning | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `CJK_TOKENIZE_UNKNOWN_BYTE` | warning | optional | — | catalog | `cmap/tokenize.rs` (`tokenize_cjk_bytes`, `cjk` feature) |
| `OCR_JBIG2_UNSUPPORTED` | warning | optional | optional | catalog | `decoder/jbig2.rs`, `parser/stream.rs` |
| `OCR_JPX_UNSUPPORTED` | warning | optional | optional | catalog | `decoder/jpx.rs` |
| `OCR_CCITT_UNSUPPORTED` | warning | optional | optional | catalog | `parser/stream.rs` |
| `OCR_TESSERACT_FAILED` | warning | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `OCR_BROKENVECTOR_UNAVAILABLE` | warning | optional | optional | catalog | `classify.rs` |
| `OCR_LANGUAGE_UNAVAILABLE` | warning | optional | optional | catalog | `ocr.rs` |
| `IMG_SOFTMASK_UNSUPPORTED` | warning | optional | optional | catalog | `render/image_compositing.rs` |
| `IMG_UNSUPPORTED_FORMAT` | warning | optional | optional | catalog | `preprocess.rs`, `render/pdfium_path.rs`, `render/image_compositing.rs` |
| `IMG_DESKEW_OUT_OF_RANGE` | warning | optional | optional | catalog | `preprocess.rs` |
| `IMG_SOURCE_MIXED` | warning | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `REMOTE_FETCH_INTERRUPTED` | error | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `REMOTE_NO_RANGE_SUPPORT` | warning | optional | optional | catalog | `source/mod.rs` |
| `REMOTE_TLS_FAILED` | fatal | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `REMOTE_DNS_FAILED` | fatal | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `REMOTE_URL_PRIVATE_NETWORK` | error | optional | optional | catalog | `url_validation.rs` |
| `REMOTE_INSUFFICIENT_DISK` | error | optional | optional | catalog | `source/http_range.rs` |
| `GSTATE_STACK_OVERFLOW` | warning | optional | optional | catalog | `content_stream.rs`, `render/image_compositing.rs`, `font/type3_rasterizer.rs` |
| `GSTATE_STACK_UNDERFLOW` | warning | optional | optional | catalog | `content_stream.rs`, `font/type3_rasterizer.rs` |
| `GSTATE_BT_ET_MISMATCH` | warning | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `CM_ARG_COUNT` | warning | optional | optional | catalog | `render/image_compositing.rs`, `font/type3_rasterizer.rs` |
| `CM_DEGENERATE` | warning | optional | optional | catalog | `render/image_compositing.rs`, `font/type3_rasterizer.rs` |
| `HORIZ_SCALING_ZERO` | warning | optional | optional | catalog | `content_stream.rs` |
| `TEXT_RENDERING_MODE_CLAMPED` | warning | optional | optional | catalog | `content_stream.rs` |
| `TSTAR_ZERO_LEADING` | warning | optional | optional | catalog | `content_stream.rs` |
| `FONT_RESOURCE_NOT_FOUND` | warning | optional | optional | catalog | `content_stream.rs` |
| `FONT_SIZE_ZERO_OR_NEGATIVE` | warning | optional | optional | catalog | `content_stream.rs` |
| `BT_NESTED` | warning | optional | optional | catalog | `content_stream.rs` |
| `ET_WITHOUT_BT` | warning | optional | optional | catalog | `content_stream.rs` |
| `TEXT_SHOW_OUTSIDE_BT` | warning | optional | optional | catalog | `content_stream.rs` |
| `TAGGED_PDF_STRUCT_TREE_DEFERRED` | info | optional | optional | catalog | `extract.rs` |
| `LAYOUT_READING_ORDER_AMBIGUOUS` | warning | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `LAYOUT_LOW_READABILITY` | warning | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `MCP_TOOL_INVALID_PARAMS` | error | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `MCP_PATH_TRAVERSAL` | error | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `CACHE_ENTRY_CORRUPT` | warning | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `CACHE_WRITE_FAILED` | warning | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `CACHE_INTEGRITY_FAIL` | warning | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `EMC_WITHOUT_BMC` | info | optional | optional | catalog | `parser/marked_content_stack.rs` |
| `MARKED_CONTENT_DEPTH_EXCEEDED` | info | optional | optional | catalog | `parser/marked_content_stack.rs` |
| `UNKNOWN_MARKED_CONTENT_PROPS` | info | optional | optional | catalog | `parser/marked_content_operators.rs` |
| `MCID_REDEFINED` | info | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `INLINE_IMAGE_ID_WHITESPACE_MISSING` | warning | optional | optional | catalog | `parser/inline_image.rs` |
| `INLINE_IMAGE_NO_EI` | warning | optional | optional | catalog | `parser/inline_image.rs` |
| `PROFILE_SECRETS_FORBIDDEN` | error | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `PROFILE_INVALID` | error | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `REPAIR_RESCUED_FROM_BACKWARDS_XREF` | info | — | — | catalog | none in HEAD (reserved/planned or not yet wired) |
| `JAVASCRIPT_PRESENT` | info | optional | optional | catalog | `javascript.rs` |

Rows shown as `none in HEAD` are intentionally not silently assigned a fake
emitter. Some are explicitly marked `(reserved)` in the published catalog;
the remaining gaps are recorded as contract/inventory findings for future
implementation beads, outside this task. The dead
`crates/pdftract-core/src/parser/diagnostic.rs` type is excluded: production
code re-exports the canonical `crate::diagnostics` model and must not bind to
the legacy duplicate.

## Implementation handoff audit (HEAD `3aa3ef15`)

The table above is the complete code-to-module inventory. The following
function-level anchors make the next implementation sweep reproducible:

- Lexer/object syntax: `parser/lexer/mod.rs` (`lex_name`, `lex_hex_string`,
  `lex_literal_string`, `lex_s_keyword`, `lex_next`, `lex_right_angle`,
  `lex_unknown`, `lex_numeric`) and `parser/object/parser.rs`
  (`parse_array`, `parse_dict`, `parse_indirect_object`,
  `parse_integer_or_ref`, `parse_direct_object`, `skip_stream_body`).
- Xref/object graph/page tree: `parser/xref.rs` (`merge_hybrid`,
  `parse_traditional_xref`, `parse_xref_entry`, `parse_trailer_dict`,
  `forward_scan_xref`, `forward_scan_memory`, `parse_xref_stream`,
  `walk_chain`), `parser/objstm.rs` (`load_object_stream_impl`), and
  `parser/pages.rs` (`build_page_dict`, `merge_inherited_attrs`,
  `flatten_page_tree`, `count_pages_walk`, `walk_page_tree`, `next`).
- Structure/navigation: `parser/struct_tree.rs` (`parse`, `parse_struct_tree`,
  `parse_kid_entry`, `resolve`, `process_nums_array`, `walk_number_tree`,
  `check_coverage_for_pages`), `parser/outline.rs`
  (`parse_outline_recursive`, `parse_outlines`, `resolve_destination`,
  `decode_utf16be_bom`), `parser/catalog.rs` (`parse_catalog`), and
  `parser/ocg.rs` (`parse_oc_properties`).
- Content/marked content/inline images: `content_stream.rs` (`can_enter`,
  `process_with_mode_and_diagnostics`, `execute_with_do`,
  `handle_do_operator`, `resolve_xobject_stream`,
  `extract_content_stream_bytes`, `process_tj_array`,
  `parse_inline_dict_from_buffer`, `normalize_glyph_bboxes_by_rotation`),
  `parser/marked_content_stack.rs` (`pop_emc`, `push_bmc`, `push_bdc`),
  `parser/marked_content_operators.rs` (`emit_invalid_bdc_operand`,
  `resolve_property_object`, `emit_unknown_property_name`), and
  `parser/inline_image.rs` (`validate_id_whitespace`,
  `scan_inline_image_data`, `parse_inline_image_header`,
  `parse_color_space_value`, `parse_decode_array`,
  `parse_decode_parms_value`, `parse_filter_value`, `set_header_field`).
- Fonts/CMaps: `font/cmap.rs` (`parse`, `handle_usecmap`, `decode_utf16be`,
  `emit_error`), `font/codespace.rs` (`parse_codespace_block`, `emit_error`),
  `font/encoding.rs` (`parse`), `font/embedded.rs` (`load`),
  `font/type0.rs` (`load`, `load_font_program`, `load_cid_to_gid_map`),
  `font/type3.rs` (`load_char_procs`, `load_font_bbox`, `load_widths`,
  `type3_font_with_cache`), `font/type3_rasterizer.rs` (`op_concat`,
  `op_save`, `op_restore`, `op_do`), `font/resolver.rs`
  (`emit_miss_diagnostic`, `resolve_stream_bytes`, `resolve_type3`,
  `resolve_type3_level4`), `cmap/codespace.rs` (`parse`,
  `parse_codespace_block`, `parse_hex_string`, `emit_error`), and the
  feature-gated `cmap/tokenize.rs` (`tokenize_cjk_bytes`).
- Stream/decoder/image/OCR: `parser/stream.rs` (`decode_stream_impl`,
  `validate_markers`), `decoder/jbig2.rs` (`emit_unsupported_diagnostic`),
  `decoder/jpx.rs` (`emit_unsupported_diagnostic`,
  `emit_invalid_magic_diagnostic`), `render/image_compositing.rs`
  (`collect_image_placements`, `collect_image_xobjects`,
  `parse_inline_image`, `decode_image_xobject`), `render/pdfium_path.rs`
  (`render_page_via_pdfium`), `preprocess.rs` (`deskew`, `grayimage_to_pix`,
  `pix_to_grayimage`), `classify.rs`
  (`apply_broken_vector_escalation_with_diagnostics`), and `ocr.rs`
  (`validate_ocr_languages`).
- Encryption/remote/other document features: `encryption/detection.rs`
  (`detect_encryption`, `parse_hash_with_diagnostics`),
  `encryption/mod.rs` and `encryption/decryptor.rs` (`to_diagnostic`,
  `decrypt_with_password`), `pages.rs` (`parse_pages`), `javascript.rs`
  (`detect_javascript`), `source/mod.rs` (`open_remote`),
  `source/http_range.rs` (`download_to_temp_and_mmap_with_hook`),
  `url_validation.rs` (`validate_url_with_diagnostic`),
  `attachment/associated_files.rs` (`walk_af_array`,
  `extract_af_relationship`), `attachment/filespec.rs` (`extract_one`,
  `extract_filename`, `extract_ef_stream_ref`), `attachment/name_tree.rs`
  (`walk_embedded_files`, `walk_tree_node`, `parse_names_array`),
  `forms/mod.rs` (`walk_acroform_fields`, `walk_field_recursive`),
  `forms/xfa.rs` (`decode_stream_bytes`, `extract_xfa_bytes`,
  `extract_xfa_bytes_from_array`, `parse_xfa_xml`), `forms/combiner.rs`
  (`combine`, `merge_xfa_value_with_acro_type`), `signature/mod.rs`
  (`decode_utf16be_bom`, `walk_acroform_fields`, `walk_field_recursive`),
  `threads/mod.rs` (`discover`, `walk_beads`, `check_and_handle_termination`,
  `get_next_bead_ref`), `conformance.rs` (`detect_conformance_impl`), and
  `fingerprint/canonicalize.rs` (`canonicalize_f64`).

The three rows corrected in this audit are concrete production paths, not
test-only references: hint-stream parsing emits `STRUCT_INVALID_HINT_STREAM`,
encryption detection emits `ENCRYPTION_UNSUPPORTED` and
`ENCRYPTION_INVALID_DICT`, and `tokenize_cjk_bytes` emits
`CJK_TOKENIZE_UNKNOWN_BYTE` when the `cjk` feature is enabled. The last path
passes `offset = cursor as u64`; the other two currently pass no offset,
object, or page context. All three still receive their severity and hint from
the catalog policy (`warning`, `fatal`, and `warning`, respectively).

### Context and fallback matrix

This is the handoff rule for every row and every function above:

| Field | Available at an emitter when | Explicit fallback when absent |
|---|---|---|
| Severity | Always; `DiagCode::severity()` / `DiagnosticPolicy` is authoritative | Never infer or invent one at the call site |
| Byte offset | The parser/decoder has a source cursor or known range; examples include lexer positions, `cmap/codespace.rs` `self.position`, CJK `cursor`, stream marker offsets, and JPX offset `0` | `None` in `Diagnostic`; it is not serialized today and must not be folded into the legacy message |
| Page index | The page extraction/classification/content layer has the zero-based page number; `extract.rs::attach_page_context` fills it only when the diagnostic lacks one | `None`; omit `page_index` in JSON/NDJSON, never use a sentinel |
| Object location | The site has an indirect `ObjRef`; attachment/content/parser/render paths may attach it with `with_object_ref*` | `None`; omit `location` in JSON/NDJSON, never use `0 0` or `null` |
| Hint | Always obtained from `DIAGNOSTIC_CATALOG.suggested_action` for current rows | `None` is the forward-compatible structured fallback; never add a site-specific hint or alter the legacy message |

The source scanner in `tests/diagnostics_catalog_drift.rs` is the coverage
gate for the code-to-catalog/doc-row relation. It scans production source plus
`#[cfg(test)]` text, masks comments/strings, and deliberately excludes only
the canonical registry and the dead `parser/diagnostic.rs`; it does not
recover enclosing function names or distinguish test-only references. The
function index above is therefore the reviewed handoff for call-site names,
while the scanner remains the authoritative “no undocumented code token”
check. A future tooling bead should add function/span and cfg provenance if
machine-generated per-call-site reporting is required.

### Boundaries and unresolved decisions

- The legacy boundary is `diagnostics_compat::to_legacy_string(s)` in
  `crates/pdftract-core/src/diagnostics_compat.rs`: it projects only
  `Diagnostic.message`, in order, retaining duplicates and exact bytes. The
  `Diagnostic` `Display` form is human/debug output and must not become the
  compatibility format.
- The machine-readable boundary is `DiagnosticJson` in
  `schema/mod.rs`, populated into compact metadata by `extract.rs`, copied to
  full JSON `Output.errors` by `output/json.rs::result_to_output`, and merged
  with synthetic lowercase `page_extraction_error` records by
  `output/ndjson/pipeline.rs::footer_errors`. Synthetic page errors are not
  catalog rows and do not enter either metadata array.
- No model or public-output change is part of this handoff. If consumers later
  require byte offsets, the unresolved compatibility decision is whether to
  add a new optional `DiagnosticJson.byte_offset`; until that decision, keep
  offsets in-process only.
- The next child must decide whether to thread missing page/object context at
  individual sites and whether reserved catalog rows should remain reserved;
  absent context must continue to use the omission fallbacks above. It must
  also decide whether to upgrade the scanner's production-vs-test and
  function-span reporting before relying on it as a call-site inventory.

## What pins this contract

| Artifact | Pins |
|---|---|
| `crates/pdftract-core/src/diagnostics.rs` | typed model, `DiagCode`, policy, catalog |
| `crates/pdftract-core/src/diagnostics_compat.rs` | canonical/legacy split, legacy bytes (golden tests in-file) |
| `crates/pdftract-core/src/schema/mod.rs` (`DiagnosticJson`, `ObjectLocationJson`, `Output::errors`) | serialized field names, omission attributes, always-present `errors` |
| `crates/pdftract-core/src/extract.rs` (`ExtractionMetadata`) | mirror pair, empty-omission on both metadata arrays |
| `crates/pdftract-core/src/output/ndjson/pipeline.rs` (`footer_errors`) | footer union order and synthetic page-failure records |
| `crates/pdftract-core/tests/diagnostics_serialization_format.rs` | per-field wire shape |
| `crates/pdftract-core/tests/diagnostics_catalog_drift.rs` | code registry ↔ catalog ↔ doc-table agreement |
| `crates/pdftract-core/tests/diagnostics_context_contract.rs` | deterministic per-code policy, context survival |
| `crates/pdftract-core/tests/diagnostics_output_contract.rs` | compact/full/NDJSON round-trip |
| `crates/pdftract-core/tests/diagnostics_surface_mirror.rs` | string↔structured one-to-one mirror across surfaces |
| `crates/pdftract-core/tests/diagnostics_{page_extraction,parser_decoder}_context.rs` | per-layer context threading |
| `docs/integrations/diagnostics-codes.md`, `docs/errors-array-format.md` | published guides (must agree with this contract) |

The required regression cases are contract tests, not emitter implementation
work: (1) serialize a diagnostic with every field and assert the exact JSON
field names and lowercase severity; (2) serialize a document-level diagnostic
and assert absent page/location/hint fields are omitted rather than `null`;
(3) assert all 113 catalog rows round-trip with the documented severity and
hint; (4) assert the legacy and detailed metadata arrays have equal length,
order, duplicates, and message bytes; (5) assert compact metadata omission,
full-JSON top-level `errors: []`, NDJSON page-error presence, and footer
synthetic-page-error-before-document-diagnostic ordering; (6) assert page and
object context survives the parser, decoder, page, and classification paths;
and (7) run the catalog drift check with default and `cjk` feature sets so
feature-gated rows are handled explicitly.

SDK models follow this envelope (e.g. `pdftract-dotnet`
`Models/Error.cs` — whose `Error` and `ObjectLocation` records mirror
`DiagnosticJson`/`ObjectLocationJson` field-for-field); new SDK surface work
must not invent a third shape.
