# pdftract Diagnostics Contract

**Status:** Decided — canonical contract for machine-readable diagnostics.
**Decided:** 2026-09-26, bead `pdftract-5928d4ce` (child of `pdftract-d79310ab`).
**Supersedes:** the contradiction between `docs/integrations/diagnostics-codes.md`
(structured objects) and `docs/errors-array-format.md` (string array) as
originally shipped; both guides now describe the contract below and must
continue to agree with it.

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
  substring-guess. This was the historical accident the contradiction came
  from; freezing it would make it permanent.

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
   currently asserts otherwise — the doc alignment bead
   (`pdftract-b2506ab5`) must fix the pattern to match the implementation
   (or the field be renamed/deprecated explicitly); the contract here
   records the implemented behavior as intended.
5. **Hint population.** Hints come from the catalog policy
   (`suggested_action`), not from emission sites. Every catalog entry
   currently defines one, so structured diagnostics carry `hint` today, but
   consumers and tests must tolerate its absence per the omission rule.

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

SDK models follow this envelope (e.g. `pdftract-dotnet`
`Models/Error.cs` — whose `Error` and `ObjectLocation` records mirror
`DiagnosticJson`/`ObjectLocationJson` field-for-field); new SDK surface work
must not invent a third shape.
