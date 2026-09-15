# pdftract-456c2b02 — Reconcile DiagnosticJson field-serialization spec and pin with tests

**Status: CLOSED (delivered in `9497b721`, re-derived and gate-verified at HEAD `a2af776b` on 2026-09-15)**

## The canonical decision

| Question | Canonical rule | Where pinned |
|---|---|---|
| Absent optional fields (`page_index`, `location`, `hint`) | **Omitted, never serialized as `null`** — `#[serde(skip_serializing_if = "Option::is_none")]` | `DiagnosticJson` serde attrs; `optional_fields_are_omitted_not_null` |
| `hint` optionality | Optional like the other two; omitted when the catalog entry carries no suggested action. Every entry in `DIAGNOSTIC_CATALOG` currently carries one, so emitted diagnostics have hints today — consumers must still tolerate absence | `catalog_entries_populate_the_documented_envelope` |
| Always-present fields | `code`, `message`, `severity` | same test |
| Severity enum | Exactly `info \| warning \| error \| fatal` — both docs enumerate all four; emitted set must *equal* the documented set (no undocumented level, no dead documentation) | `emitted_severities_match_documented_enum` |
| Empty arrays | Differ by surface: `metadata.diagnostics` / `metadata.diagnostics_detailed` **omitted entirely** when empty; full-output top-level `errors` and NDJSON footer `errors` are **always present** (`[]` when clean); NDJSON page frame carries `errors` only when the page failed | `compact_json_omits_metadata_diagnostics_when_empty`, `full_output_errors_array_is_always_present`, `footer_frame_errors_are_always_present`, `page_frame_omits_errors_when_page_succeeded` |

Rationale for omit-not-null: `DiagnosticJson`'s optional fields are `Option<T>`
with `skip_serializing_if`, so omission is what the implementation already
does; the docs now say so instead of contradicting themselves (the old
diagnostics-codes.md prose said "omitted (not null)" while its own format
template showed `null | ...`).

## Deliverable — commit `9497b721` (2026-09-15)

- `docs/integrations/diagnostics-codes.md` — format block rewritten: the
  prose rule (omit, never null; code/message/severity always present; hint
  optional with every current catalog entry carrying one) now matches the
  template; a "Where structured diagnostics appear" section documents the
  three surfaces and their empty-array behavior; names the pin test.
- `docs/errors-array-format.md` — Overview now declares all four severities
  including `fatal` (was errors/warnings/info only); the structured-form
  table now shows `page_index`/`location`/`hint` as "no" under
  "Always present?" with the omit-not-null rule stated explicitly; per-surface
  empty-array behavior spelled out; structured-filtering examples use typed
  severity/`page_index`; Summary updated.
- `crates/pdftract-core/tests/diagnostics_serialization_format.rs` — 7 tests,
  ~299 lines, pinning the three rules above. `emitted_severities_match_
  documented_enum` parses the enum out of diagnostics-codes.md itself, so doc
  drift fails CI.

## Verification

### Attempt 1 (initial dispatch) — delivered, 7/7 PASS in working-tree stratum

### Attempt 2 — blocked by a foreign compile error, not by this bead's work

Gate replica at HEAD `49e8723d` failed exit 101: `E0609 no field
diagnostics_detailed on ExtractionMetadata` at
`crates/pdftract-core/src/output/ndjson/pipeline.rs:150`. `23cf6bb4`
(pdftract-8c4bed00) had committed the *consumer* while the producer field
existed only in that worker's uncommitted working-tree stratum. Correctly
not landed from here (the other worker owned the entangled `extract.rs`).

### Attempt 3 (this re-dispatch) — re-derived at HEAD, all green

Resolution of the blocker by the other worker: `56a4f6c7` ("land stranded
diagnostics_detailed producer so HEAD compiles"). Verified:

```
git show HEAD:crates/pdftract-core/src/extract.rs | grep diagnostics_detailed
  → 459: pub diagnostics_detailed: Vec<DiagnosticJson>   # field present at HEAD

# git-archive HEAD extraction (a2af776b) + private hardlink-seeded target:
cargo check --all-targets        → exit 0 (17.27s; warnings only, all pre-existing, unrelated files)
cargo test -p pdftract-core --test diagnostics_serialization_format --no-fail-fast
  → 7 passed; 0 failed (compact_json_omits_metadata_diagnostics_when_empty,
    catalog_entries_populate_the_documented_envelope, emitted_severities_match_documented_enum,
    footer_frame_errors_are_always_present, full_output_errors_array_is_always_present,
    optional_fields_are_omitted_not_null, page_frame_omits_errors_when_page_succeeded)
cargo test -p pdftract-core --test diagnostics_catalog_drift --no-fail-fast
  → 3 passed; 0 failed (doc table vs DiagCode::severity() vs emission sites)
```

Docs/test churn since the deliverable commit: none — `git log 9497b721..HEAD
-- docs/errors-array-format.md docs/integrations/diagnostics-codes.md
crates/pdftract-core/tests/diagnostics_serialization_format.rs` is empty and
`git show 9497b721:docs/errors-array-format.md` is byte-identical to HEAD's.

## Acceptance criteria

- **PASS** — both documents agree on one canonical rule (omit-when-absent for
  `page_index`/`location`/`hint`; `hint` optional; enum `info|warning|error|fatal`
  in both docs).
- **PASS** — serialization tests assert the documented field rule, the
  per-surface empty-array behavior (metadata arrays omitted; top-level/footer
  `errors` always present; page frames omit on success), and that every
  emitted severity is inside — and collectively equal to — the documented enum.
- **PASS** — gate verdict executed at a compiling HEAD (`a2af776b`).
