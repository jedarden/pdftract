# pdftract-dd40e63a — Audit canonical/compatibility diagnostics contract documentation

Child 4/4 of `pdftract-a126b61f`. Scope: verify the contract recorded in
code-level API documentation matches the behavior established by the
serialization and legacy-compat sibling children; fix drift only.

**Work commit: `016ff6c2`** (this note committed separately so it can cite the sha).

## Contract doc location

`crates/pdftract-core/src/diagnostics_compat.rs:1-79` — module-level docs:
canonical typed surface with the `DiagnosticJson` field table (lines 22-35),
legacy `Vec<String>` message-verbatim surface, pinned severity strings,
additive-optional rule, and a worked example (lines 67-79). The module is
public and the crate is `#![deny(missing_docs)]`.

## Verified claims → code evidence (all PASS)

| Doc claim | Code reality |
|---|---|
| Canonical = typed `Diagnostic` (code, message, optional byte_offset/object_ref/page_index; severity derived) | `diagnostics.rs:2748-2760` (struct fields exactly those), `DiagCode::severity()` + `Severity` Display `info|warning|error|fatal` at `diagnostics.rs:107-116` |
| `code`: string, `DiagCode::name` form, always present | `pub const fn name` (`diagnostics.rs:1238`); test asserts `"XREF_REPAIRED"` |
| `message`, `severity`: always present | `schema/mod.rs:828-836` (plain `String` fields, no skip) |
| `page_index` number (zero-based), only when known | `Option<usize>` + `skip_serializing_if = "Option::is_none"` (`schema/mod.rs:839-840`) |
| `location` = `{object_number, generation_number}`, only when known | `ObjectLocationJson` (`schema/mod.rs:857-863`) + skip on `schema/mod.rs:843-844` |
| `hint`, only when known | `Option<String>` + skip (`schema/mod.rs:847-848`) |
| Optional fields omitted (not `null`) | Pinned by `optional_json_fields_are_omitted_when_none` in `diagnostics_compat.rs` (exact-string assert) and `tests/diagnostics_serialization_format.rs` |
| Legacy = `Vec<String>` of messages verbatim, emission order | `ExtractionMetadata.diagnostics: Vec<String>` (`extract.rs`), produced solely via `to_legacy_strings` at `extract.rs:1090/2017/2334`; byte-equivalence to the historical inline `d.message.as_ref().to_string()` pinned by `golden_legacy_bytes_match_inline_producer_expression` |
| Display-vs-legacy distinction | `Diagnostic: Display` renders `CODE: message (byte offset N) [obj G R]` (`diagnostics.rs:2854-2865`, `ObjRef: Display` at 77-81); legacy carries bare message only — doc's example form matches |
| Severity strings pinned by tests here + catalog drift suite | `all_supported_severities_render_documented_strings` (this module), `tests/diagnostics_serialization_format.rs`, `diagnostics_catalog_drift` suite (referenced from `tests/diagnostics_surface_mirror.rs:22`) |
| Cross-reference: `docs/integrations/diagnostics-codes.md` | Field table and omitted-never-`null` rule agree with the module docs (also `docs/errors-array-format.md` String Format / Structured Object Form). Both files were read as working-tree versions; they carry a concurrent worker's uncommitted edits and were not committed by this dispatch. |

## Drift found and fixed (docs only; no behavior change) — commit `016ff6c2`

1. **`extract.rs:456-461`** (public field doc on the legacy surface itself):
   claimed `metadata.diagnostics` carries the Display form
   ``(`CODE: message (byte offset N)? [object generation R]?`)`` — it is the
   bare message verbatim. Rewrote to the contract wording and linked
   `to_legacy_strings`.
2. **`schema/mod.rs:838`**: `DiagnosticJson.page_index` doc said "or `null`
   for document-level events" — the pinned wire behavior *omits* the field.
   Reworded (also made zero-based explicit, matching the table).
3. **`diagnostics_compat.rs:84-91`** (`to_legacy_string` doc): said the inline
   `d.message.as_ref().to_string()` producers in `extract.rs` "emit today" —
   those sites were replaced by `to_legacy_strings` calls in `19318bf7`
   (legacy-compat sibling). Reworded to the golden-test-pinned equivalence.
4. **`diagnostics.rs` doctests did not compile** (pre-existing at HEAD; file
   was clean before this dispatch, failures at lines 27/2896 unrelated to the
   concurrent one-word comment fix): module-usage example imported
   nonexistent `pdftract_core::diagnostics::emit` (macro is crate-root via
   `#[macro_export]`), both examples used SCREAMING idents where the macro
   requires `DiagCode` variant names (`DiagCode::$code`), and the documented
   `object = 5_0` matches no macro arm — real syntax is `object = (5, 0)`
   (macro arm at `diagnostics.rs:2938`, internal test at 3119). Fixed both
   examples, the `# Parameters` object line, and a `cfg(test)` comment that
   mislabeled the Display form as "legacy string form".

## Compile verification (timeout-wrapped; exit codes from cargo, not the pipe)

```
$ timeout --kill-after=30s 600s cargo test --doc -p pdftract-core diagnostics
EXIT=0   test result: ok. 5 passed; 0 failed; 1 ignored; 0 measured; 326 filtered out
          (incl. diagnostics_compat line 67, diagnostics line 29, diagnostics::emit line 2900)
$ timeout --kill-after=30s 600s cargo test --doc -p pdftract-core diagnostics_compat
EXIT=0   test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 331 filtered out
$ timeout --kill-after=30s 600s cargo test -p pdftract-core --lib diagnostics_compat
EXIT=0   test result: ok. 9 passed; 0 failed; 0 measured; 3615 filtered out
```

Before the fix, the first command reported `2 failed` (the two `diagnostics.rs`
doctests). No run hit the timeout (no exit 124).

## Known nit deliberately left

`pages.rs:337` (inside a `#[cfg(test)]` mod, not API docs) labels a Display
assertion "Legacy string form (docs/errors-array-format.md)". The assertion
itself is correct (it checks `.to_string()`, i.e. Display). Not fixed:
`pages.rs` carries a concurrent worker's ~51-line uncommitted diff, and
committing the file would sweep their in-flight work into this dispatch's
commit.

## Acceptance criteria

- PASS — contract doc present on the public module and matching verified
  behavior; drift fixed and committed (`016ff6c2`; the diff was not empty —
  four drifts, all doc-side).
- PASS — doc examples compile under the timeout-wrapped `--doc` runs; output
  recorded above.
- PASS — this note cites the doc location, compile commands/output, and the
  commit sha.
