# Platform Smoke Corpus and Goldens

This directory is the repeatable input for the per-release manual platform
smoke test defined in
[docs/operations/manual-platform-smoke.md](../docs/operations/manual-platform-smoke.md)
and filled in per release from
[docs/operations/platform-smoke-checklist-template.md](../docs/operations/platform-smoke-checklist-template.md).

Per release, the executor installs the release archive on each manual-smoke
platform (macOS x86_64, macOS aarch64, Windows x86_64), runs this corpus
through the installed binary, and diffs the output against the committed
goldens. Identical bytes in, identical JSON out — extraction is pure
computation over the PDF bytes and the v1 output schema carries no paths,
timestamps, or platform markers — so any diff is a real finding: a
platform-specific packaging failure, an encoding/locale problem, or an
extraction regression since the last baseline.

## Layout

| Path | Purpose |
|---|---|
| `generate_corpus.py` | Regenerates `corpus/` byte-identically (stdlib only, no wall-clock input) |
| `corpus/*.pdf` | The canonical fixture set — four small, well-formed PDFs |
| `corpus.manifest` | sha256 per fixture, `sha256sum -c` compatible; ties goldens to exact corpus bytes |
| `record_goldens.sh` | Records `goldens/*.json` from a reference binary |
| `diff_goldens.sh` | The comparison step the smoke checklist runs per platform |
| `goldens/` | Recorded reference outputs (see `goldens/README.md`) |

The corpus is deliberately independent of `tests/fixtures/`: that tree churns
with the test suite, and a smoke baseline tied to it would be invalidated by
unrelated test changes. The corpus changes only when someone deliberately
regenerates it, which changes `corpus.manifest` in the same commit and
requires re-recording goldens.

## Golden lifecycle

1. **Record** — from a reference binary that extracts the corpus, run:
   `tests/smoke/record_goldens.sh /path/to/pdftract`
   The normal reference is the Linux release build at the release tag: the
   Linux platform is fully CI-tested, so its output defines what every other
   platform must produce. The script verifies the corpus against
   `corpus.manifest`, writes `goldens/<fixture>.json` plus a
   `goldens/RECORDING.md` provenance note, and fails without recording if any
   extraction fails.
2. **Commit** — goldens, `RECORDING.md`, and (only if the corpus itself
   changed) `corpus.manifest` land in the same commit. Goldens change
   exclusively through this deliberate re-recording; a smoke run never
   rewrites them.
3. **Diff** — every platform smoke run does:
   `tests/smoke/diff_goldens.sh /path/to/installed/pdftract`
   The script re-verifies the corpus manifest, then diffs per fixture. Exit
   0 = pass, 1 = diff or extraction failure, 3 = goldens not yet recorded for
   this corpus revision.

When a release intentionally changes extraction behavior, the release that
carries the change re-records the goldens from its own Linux reference build
and commits them with the release; the following releases diff against the
new baseline. A golden diff is therefore meaningful in both dimensions at
once: cross-platform consistency within a release, and regression-or-intent
across releases.

## Bootstrap status (2026-09-15)

`goldens/` is intentionally empty of `*.json` right now: extraction is broken
at HEAD for every fixture — 0/1379 files under `tests/fixtures/` fail, and a
hand-constructed minimal PDF ("textbook" xref/trailer) also fails — so no
honest golden can be recorded from any current build. This is the pre-existing
parser regression tracked in `pdftract-3628e890` (in progress), with
`pdftract-60c6f90a`, `pdftract-0908d20d`, and `pdftract-d0dbedce` in the same
failure cluster. `diff_goldens.sh` exits 3 ("goldens not recorded") until the
first reference recording exists, so the gate cannot silently pass.

Bootstrap happens at the first milestone release whose build extracts the
corpus: run `record_goldens.sh` against that release's Linux binary, commit
the goldens, and from that release on the corpus+goldens pair is the
cross-release baseline. Until then the manual smoke is gated on exactly the
steps that still work (archive install, `pdftract doctor`) plus the golden
step recorded as BLOCKED with the exit-3 reason in the completed checklist.
