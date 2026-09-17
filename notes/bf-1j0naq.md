# bf-1j0naq — Provenance pre-commit hook no longer blocks ALL commits

## Problem

`.git-hooks/pre-commit` → `scripts/check-provenance.sh` ran on every commit and, in
the **HEAD** version, scanned the *entire* `tests/fixtures/` tree on every attempt
(`find "$FIXTURES_DIR" ...` + an orphan check against the full tree). Because ~55–99
fixtures already committed at HEAD have no `PROVENANCE.md` entry (e.g.
`scanned/form/form-300dpi.pdf`, `security/embedded-js.pdf`, `profiles/valid/*.yaml`),
**every** commit failed the hook — including doc-only changes that touched no fixture
at all. The only escape was `--no-verify`, which defeats the hook. This made the repo
effectively un-committable and is a likely contributor to the backlog of open beads.

## Fix (Option B — narrow the check to staged files only)

`scripts/check-provenance.sh` was rewritten into two modes:

- **default (`staged`)** — what the pre-commit hook uses. It collects only files staged
  in the current commit via `git diff --cached --name-only --diff-filter=ACMR -- tests/fixtures`
  and validates *those* against `PROVENANCE.md`. A historically-inconsistent tree no
  longer blocks an unrelated commit; the hook now enforces its real invariant —
  *you can never add or modify a fixture without a matching provenance entry* — instead
  of re-auditing the whole tree every time. SHA256 is computed from the staged blob
  (`git cat-file blob :0:<path>`), i.e. exactly what will be committed, not the
  possibly-dirty working copy.
- **`--all`** — full audit of every fixture under `tests/fixtures/`. Intended for manual
  review or a CI job. This still surfaces the historical orphaned fixtures so they are
  not silently forgotten.

`set -e` → `set -u`; error/warning counts now use distinct `^ERROR:`/`^WARN:` line
counts (not raw line counts, which double-counted multi-line messages). Temp files are
cleaned via a `trap … EXIT`.

`.git-hooks/pre-commit` is unchanged (it just runs `bash scripts/check-provenance.sh`,
so it picks up the new staged default automatically). It is installed via the existing
symlink `.git/hooks/pre-commit -> ../../.git-hooks/pre-commit`.

This was **not** fixed with `--no-verify`; the hook's purpose is preserved and narrowed.

## Verification (all run against the working-tree script, no `--no-verify`)

| Test | Command | Result |
|------|---------|--------|
| Unblock — no fixture staged | `bash scripts/check-provenance.sh` (nothing staged under `tests/fixtures/`) | **exit 0** — "No staged fixture files — nothing to validate" |
| Invariant — new fixture w/o provenance | stage `tests/fixtures/_invariant_test.pdf` (no `PROVENANCE.md` row), run default mode | **exit 1** — "ERROR: Fixture file missing from PROVENANCE.md: _invariant_test.pdf" |
| Full audit (CI/manual) | `bash scripts/check-provenance.sh --all` | **exit 1** — surfaces 55 historical orphaned fixtures (no longer blocks commits) |
| Syntax | `bash -n scripts/check-provenance.sh` | OK |

The throwaway invariant-test fixture was created, exercised, then `git rm --cached` +
`rm`'d; working tree is clean afterward.

**Real-commit verification:** this fix + note are committed *without* `--no-verify`.
The commit touches only `scripts/check-provenance.sh` and `notes/bf-1j0naq.md` (no
fixture), so the live pre-commit hook runs the fixed staged-mode script and passes —
demonstrating that a non-fixture change now lands without bypassing the hook.

## Acceptance criteria

- ✅ Option B implemented — hook validates only staged/changed fixtures, not the whole
  tree.
- ✅ Verified with a real `git commit` (not `--no-verify`) that a non-fixture/doc change
  lands.
- ✅ Fix is NOT a `--no-verify` bypass — the invariant (new fixture needs provenance) is
  preserved.

## Re-verification at re-dispatch (2026-09-17, HEAD `86c97d8e`)

The bead was re-dispatched with an auto-split order (failure-count:5). Re-derived the
state at HEAD before splitting: **the fix is already landed and pushed** — `501bde95`
("narrow provenance pre-commit check to staged fixtures only", 2026-07-22) is an
ancestor of `origin/main`, and `git diff HEAD` shows the working tree matches for both
`scripts/check-provenance.sh` and `.git-hooks/pre-commit`. There is nothing left to
decompose into children, so this re-issue closes with fresh evidence instead.

One thing *had* drifted in this checkout: the `.git/hooks/pre-commit` symlink described
above no longer existed (only `pre-commit.sample`), and `core.hooksPath` is unset — so
the hook was not actually running on commits here anymore. Reinstalled it exactly as
the script header documents:
`ln -s ../../.git-hooks/pre-commit .git/hooks/pre-commit`.

Fresh evidence, all run 2026-09-17 at HEAD `86c97d8e`:

| Test | Command | Result |
|------|---------|--------|
| Staged mode, no fixtures staged | `bash scripts/check-provenance.sh` | **exit 0** — "No staged fixture files — nothing to validate" (314 entries loaded) |
| Invariant still enforced | staged `_prov_negtest_bf1j0naq.pdf` (no provenance row) into a **private `GIT_INDEX_FILE` temp index** (shared index untouched: `git diff --cached --name-only -- tests/fixtures` = 0), ran default mode | **exit 1** — "ERROR: Fixture file missing from PROVENANCE.md" |
| Full audit (non-blocking, manual/CI) | `bash scripts/check-provenance.sh --all` | **exit 1** — 63 historical errors (orphaned fixtures + SHA mismatches); by design no longer runs on commit |
| Real commit through the installed hook | `git commit notes/bf-1j0naq.md` (no `--no-verify`) | hook ran the staged-mode check, passed, commit landed; pushed to `origin/main` |

The full-audit error count has drifted 55 → 63 since 2026-07-22 (fixtures keep landing
without provenance rows). That is pre-existing, out of scope for this bead — Option B
was chosen precisely so tree-wide inconsistency cannot block unrelated commits — but it
will keep growing until someone owns a backfill pass (Option A) or a CI `--all` job.

## Note on the historical orphans

Option A (backfill all missing `PROVENANCE.md` entries) was intentionally **not** taken:
those fixtures are pre-existing debt unrelated to the hook's scope bug, and the right
fix is to stop re-auditing them on every commit. They remain visible via
`bash scripts/check-provenance.sh --all` (or a future CI audit step) and can be
backfilled incrementally by whoever owns those fixtures, one staged commit at a time —
each such commit now correctly requires its own provenance row.
