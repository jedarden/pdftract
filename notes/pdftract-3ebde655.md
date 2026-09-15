# pdftract-3ebde655 — Verify reconciled structured diagnostics contract at HEAD

Parent: pdftract-b75df06a (two shipped docs contradicted each other).
Siblings already chose option (a): structured diagnostics shipped alongside
the legacy strings (c034d6ca, 9497b721, 23cf6bb4, 56a4f6c7).

## Verdict

**PASS** on both shipped docs at HEAD — they state the single reconciled
contract with no residual contradiction. **PASS with a fix** on the third-doc
sweep: `docs/user-docs/src/json-schema-reference.md` carried residue and was
reconciled in commit `b9ccef40` (pushed to origin/main).

## Both shipped docs state one contract (option a)

### docs/errors-array-format.md (567 lines)

Documents BOTH parallel forms and their mirror — "The errors array exists in
two parallel shapes, which mirror each other one-to-one (same length, same
order, same diagnostics)":

> - **String form — `ExtractionResult.metadata.diagnostics`**
>   - **Type**: `Vec<String>` … each is the internal `Diagnostic`'s `Display`.
> - **Structured form — `ExtractionResult.metadata.diagnostics_detailed`**
>   - **Type**: `Vec<DiagnosticJson>` … **Prefer this form** for machine consumption.

plus the structured-form field table with the omission rule:

> Fields that do not apply — `page_index`, `location`, and `hint` — are
> omitted, not `null`.

and the Summary:

> - **Structured form**: `result.metadata.diagnostics_detailed` — `Vec<DiagnosticJson>` with `code` / `message` / `severity` / `page_index`? / `location`? / `hint`?; mirrors the string array one-to-one and is the top-level `errors` array of the full JSON output

### docs/integrations/diagnostics-codes.md (349 lines)

Documents the envelope ("All diagnostics follow this structure"), the omission
rule, and the three structured surfaces:

> `code`, `message`, and `severity` are always present. `page_index`,
> `location`, and `hint` are **omitted, never serialized as `null`** … This
> rule (and the severity enum) is pinned by
> `crates/pdftract-core/tests/diagnostics_serialization_format.rs`.

> 1. **`metadata.diagnostics_detailed`** — the structured array on the extraction
>    result's metadata. Mirrors `metadata.diagnostics` one-to-one.
> 2. **`errors`** — the top-level array of the full JSON output, populated from
>    `metadata.diagnostics_detailed`.
> 3. **NDJSON footer frame `errors`** — document-level structured diagnostics …

and the legacy-string bridge:

> The legacy string form documented in `docs/errors-array-format.md`
> (`CODE: message (byte offset N)? [obj G R]?`) is still emitted as
> `metadata.diagnostics` … its `CODE:` prefix always matches the
> `code` of the structured entry at the same index.

Cross-links resolve in both directions: errors-array-format.md →
`integrations/diagnostics-codes.md` (3×) and diagnostics-codes.md →
`../errors-array-format.md#location` (heading `### Location` exists), plus the
internal anchor `#structured-object-form`. The pinning test
`crates/pdftract-core/tests/diagnostics_serialization_format.rs` exists.

## Severity vocabulary matches the code

Docs say `info|warning|error|fatal`. `DiagCode::severity()`
(crates/pdftract-core/src/diagnostics.rs:1360) returns `Severity::{Info, Warning,
Error, Fatal}` and `impl Display for Severity` serializes them lowercase —
exactly the documented vocabulary (unit-pinned at diagnostics.rs:2996-2999).
The generated schema post-processor (xtask/src/bin/gen_schema.rs) emits the
same enum for `DiagnosticJson.severity`.

## Third-doc sweep and residue fixed

Swept `grep -rn diagnostics docs/ README.md`, read every hit describing the
diagnostics format. Residue found in exactly one shipped doc:

**docs/user-docs/src/json-schema-reference.md** (fixed in `b9ccef40`):
- presented the string array as the only form (`diagnostics` row only, no
  `diagnostics_detailed`);
- used a `WARN:`/`ERROR:` severity-prefix vocabulary
  (`"WARN: page 3: low coverage (54%) …"`) that matches neither the code
  (`Diagnostic`'s `Display` is `{CODE}: {message} (byte offset N)? [obj G R]?`,
  no WARN/ERROR prefix) nor the four-level severity enum;
- omitted the top-level `errors` array from the top-level structure.

Fixes: Diagnostics section rewritten to the reconciled contract (typed code,
four severities, both shapes, mirror, omission-not-null, three surfaces,
cross-links to both shipped docs); metadata example/table now show both fields
with a real catalog entry; top-level table gained `errors`; note added
distinguishing the bare extraction-result payload (metadata-level diagnostics,
per result_to_json, extract.rs:1532) from the schema-validated full output
(top-level `errors` only, per result_to_output → schema::Output,
output/json.rs:73). Example hints are quoted verbatim from `DIAGNOSTIC_CATALOG`
(STRUCT_INCOMPLETE_COVERAGE, STREAM_DECODE_ERROR) instead of invented. Also
tightened errors-array-format.md's helper path to
`crates/pdftract-core/tests/test_helpers/diagnostics.rs` and fixed three
pre-existing dangling relative links in the same user doc (plan, research).

Clean (no residue): README.md, docs/user-docs/src/{faq,troubleshooting,cli-reference}.md
(real SCREAMING_SNAKE_CASE codes, no prefix claims),
docs/user-docs/src/troubleshooting/diagnostics.md (explicit Draft placeholder,
empty), docs/testing/unmapped-glyphs-tests.md, docs/operations/prometheus-rules.yaml,
docs/research/* (historical research material), docs/notes/* (historical work
records). plan.md's one `WARN:` hit is the CLI `--FLAG is deprecated` stderr
message, unrelated to diagnostics.

## Observations (not acted on — outside this bead's contract scope)

- `docs/schema/v1.0/pdftract.schema.json` (generated by
  `xtask/src/bin/gen_schema.rs` from `schema_for!(schema::Output)`) declares
  top-level `errors: DiagnosticJson[]` but its `DocumentMetadata` def carries
  no diagnostics fields — correct for the `Output` surface by construction
  (`result_to_output` never puts diagnostics in metadata), so not a
  contradiction; the metadata-level fields live on the extraction-result
  payload the user doc now attributes them to.
- `docs/user-docs/src/json-schema-reference.md`'s metadata table still carries
  Phase-6-era fields (`span_count`, `receipts_mode`, `cache_status`, …) that
  the schema's `DocumentMetadata` does not declare. Pre-existing, unrelated to
  the diagnostics contract; noted for a future user-doc pass.

## Commit

- `b9ccef40` — docs(pdftract-3ebde655): reconcile user-facing schema reference
  with structured diagnostics contract. Pushed to origin (Forgejo), main.
