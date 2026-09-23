# pdftract-66c60ab4 — Expose reusable fixture-discovery helpers

**Dispatch:** claude-code-glm-5.3-glm-pdftract · 2026-09-23 · HEAD before: `4bdc3e9d` · commits: `61b5aa19` (impl), follow-up docs commit (this note).

## What was done

Factored the reusable, fallible fixture-discovery API out of the standalone
`crates/pdftract-cli/tests/fixture_discovery.rs` integration-test binary into the
shared test-support module `crates/pdftract-cli/tests/common/fixture_discovery.rs`
(the scaffold from `7b5da77f` / pdftract-8ec45a90).

Moved (verbatim implementation, now `pub` in the shared module):

- `FixtureInfo` (+ `new` / `from_path` / `Display`, and the `fixture_description`
  category derivation)
- `FixtureDiscoveryError` (+ `Display` / `Error::source` / `From<glob::PatternError>`)
- `discover_all_fixture_infos_result()`
- `discover_fixture_infos_result_in(root)`
- Supporting implementation: `normalize_path`, `fixtures_root`,
  `ancestor_is_symlink` (byte-identical logic; visibility private → `pub`)

Left in the standalone binary (infallible walkdir family, per bead scope):
`discover_all_fixtures`, `discover_fixtures_by_category`, `discover_fixtures_flat`,
`discover_fixtures_in_dir`, `fixture_categories`, `fixture_statistics`,
`FixtureStats`, `discover_all_fixture_infos`, `discover_fixture_infos_by_category`.

Wiring: the standalone binary re-exports the moved items `pub use
common::fixture_discovery::{…}`, preserving (a) its own suite's names via
`use super::*` and (b) the `fixture_discovery::` import paths of the two sibling
binaries that include the file as a module (`forms_integration.rs:12`,
`cli_invocation_fixtures.rs:19`). New consumers import directly via
`mod common; use common::fixture_discovery::…`.

New tests (in the standalone suite, which also runs in the sibling binaries via
module inclusion):

- `test_result_api_importable_through_shared_module` — exercises the
  fully-qualified `common::fixture_discovery::…` route and pins type identity
  between the shared-module items and the re-exports.
- `test_ancestor_is_symlink_flags_dir_descent_not_file_leaf` (`#[cfg(unix)]`) —
  direct guard coverage: symlinked-directory ancestor flagged, symlinked file
  leaf and plain path kept.

## Acceptance criteria

| Criterion | Result | Evidence |
|---|---|---|
| Shared module owns the reusable result API | PASS | all named items + supporting impl defined in `tests/common/fixture_discovery.rs` |
| Callers can import the API through the shared module | PASS | items `pub`; `test_result_api_importable_through_shared_module` passes standalone AND inside `cli_invocation_fixtures` binary (module-included context) |
| Symlink ancestor guard remains intact | PASS | guard logic moved unchanged; direct guard test passes; pre-existing exact-count assertion (`test_discover_all_fixture_infos_result_ok`: glob count == walkdir count + symlinked-pdf count) still passes |
| Existing fixture-discovery tests still target same behavior | PASS | suite 28/28 (26 pre-existing unchanged + 2 new), zero modifications to pre-existing test bodies |

## Verification (clean extraction of committed HEAD `61b5aa19`)

`git archive HEAD` extraction at `/var/tmp/pdftract-66c60ab4-verify` (no `.git`,
so the cargo wrapper runs locally — same path NEEDLE's gate uses):

```
cargo test -p pdftract-cli --test fixture_discovery
  → 28 passed; 0 failed (exit 0)
cargo test -p pdftract-cli --test fixture_discovery ancestor
  → 1 passed (exit 0)
cargo test -p pdftract-cli --test forms_integration --no-run
  → exit 0 (sibling module-inclusion context compiles)
cargo test -p pdftract-cli --test cli_invocation_fixtures --no-run
  → exit 0
cargo test -p pdftract-cli --test cli_invocation_fixtures test_result_api_importable_through_shared_module
  → 1 passed (exit 0)
cargo test -p pdftract-cli --test forms_integration test_ancestor_is_symlink_flags
  → 1 passed (exit 0)
cargo build -p pdftract-cli --all-targets
  → exit 0
```

## Notes / context

- The shared working tree does not compile at dispatch time due to other
  workers' in-flight `pdftract-core/src` edits (pre-existing E0308 set, not
  touched by this bead). Verification was therefore run against a clean
  extraction of the committed HEAD, per checklist.
- `ancestor_is_symlink` provenance note: the original standalone helper lives in
  the repo-root `tests/test_glob_discovery.rs` (registered as a `[[bin]]` of
  pdftract-cli); that binary is outside this bead's scope and was left alone.
- WARN items: none.
