# pdftract-af25f004 — fix compile errors in schema and annotation JSON test modules

Child 1 of 4 of umbrella `pdftract-cf603719` (Catalog::default_off_ocgs three-state
contract tests). Scope: test code only, no production changes.

## Changes

1. `crates/pdftract-core/src/schema/mod.rs` — added `use serde_json::json;` to the
   `#[cfg(test)] mod tests` block (line ~1599). Clears the 3 `error: cannot find macro
   json!` failures at :2172, :2195, :2221 in `test_tables_array_emitted_on_page_output`
   and `test_table_block_emission_shape`. This matches the import style used by sibling
   modules (`extract.rs:62`, `output/ndjson/pipeline.rs:17`, `profiles/apply_profile.rs:14`).
2. `crates/pdftract-core/src/annotation/json.rs` — widened the test-module import to
   `use crate::annotation::links::{DestArray, LinkAnnotation};` (line 193). Clears the
   E0422 `DestArray` not found in `test_link_to_json_explicit_dest` (:249).

## Verification

`timeout --kill-after=30s 600s cargo test -p pdftract-core --lib 2>&1 | grep -E "^error" -A3 | grep -E "schema/mod\.rs|annotation/json\.rs"`
→ no output (grep exit 1). No compile error points at either file.

Error count on the lib test target dropped from 112 (2026-09-07 parent run) to 108.
All remaining errors are in other files, outside this bead's scope:

| file | errors |
|---|---|
| src/classify.rs | 77 |
| src/signature/mod.rs | 24 |
| src/forms/mod.rs | 18 |
| src/parser/xref.rs | 10 |
| src/parser/pages.rs | 9 |
| src/parser/ocg.rs | 9 |
| src/parser/outline.rs | 7 |
| others | 15 scattered |

Compile is bounded by `timeout --kill-after=30s 600s`; exit was normal (no 124/timeout).
The fixes spawn no process and bind no socket.

## Acceptance criteria

- **PASS** — no error locations reported in `schema/mod.rs` or `annotation/json.rs`.
- **PASS** — `git diff` for these two files touches only test-module `use` statements
  (one added line, one widened line). No production code changed.
- **WARN (expected)** — the pdftract-core lib test target as a whole still does not
  compile (108 remaining errors, children 2 and 3 territory), so no test executable ran.
  Out of scope per the bead.

## Hygiene

Staged precise paths only (`git add <two source files> <this note>`); the shared
checkout carries other workers' in-flight edits, untouched.
