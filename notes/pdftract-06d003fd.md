# Baseline fixture selection — pdftract-06d003fd

Recorded by pdftract-0a611b75 (documentation step for umbrella pdftract-06d003fd).
Selection provenance: shortlist by pdftract-025e4bac (closed), selection by
pdftract-dec41118 (closed). pdftract-1524b784 — the bead this step originally
pointed at — was superseded by pdftract-dec41118 (selection-only scope), so the
decision documented here is dec41118's.

## Selected fixture

- **Repo-relative path:** `tests/fixtures/classifier/invoice/01.pdf`
- **Absolute path:** `/home/coding/pdftract/tests/fixtures/classifier/invoice/01.pdf`
- **Size:** 2,143 bytes
- **sha256:** `f4d642e5e31d78486a06067d18b67947f5ffd0d1ea83dcf27902b872e7a7741a`
- **PDF version:** 1.3, single page, ASCII85 + FlateDecode content stream

## Readability confirmation at documentation time (2026-09-08)

```
$ test -r tests/fixtures/classifier/invoice/01.pdf && echo PASS
PASS
$ ls -l tests/fixtures/classifier/invoice/01.pdf
-rw-r--r-- 1 coding users 2143 Jun  6 23:15 tests/fixtures/classifier/invoice/01.pdf
$ sha256sum tests/fixtures/classifier/invoice/01.pdf
f4d642e5e31d78486a06067d18b67947f5ffd0d1ea83dcf27902b872e7a7741a  tests/fixtures/classifier/invoice/01.pdf
```

`test -r`: **PASS**. sha256 is identical to the value recorded at selection time
(pdftract-dec41118), i.e. the file is unchanged since the decision was made.

Byte-validity independently re-derived at documentation time (not trusted from
the decision note): `%PDF-1.3` header; `startxref 1752` lands exactly on
`xref`; xref subsection `0 9`; all 8 in-use entries resolve to `<N> 0 obj` at
their recorded offsets; trailer `/Root` present; `%%EOF` present. Known-good.

## Why this fixture

Smallest of the three byte-valid candidates, satisfying the stated "prefer
smallest representative file" criterion; representative of the invoice document
class the classifier corpus targets, so baseline output is comparable against
`tests/fixtures/classifier/MANIFEST.tsv`; single page with a real
ASCII85+FlateDecode content stream, exercising the actual decode path without
exotic features. Trade-off weighted: minimality + manifest comparability over
broader feature coverage.

## Candidates rejected (sizes re-checked at documentation time)

| Candidate | Size | Reason |
|---|---|---|
| `tests/fixtures/markdown_structure.pdf` | 2,265 B (readable) | Byte-valid but 122 B larger; structure-detection coverage is secondary for a baseline whose job is a known-good minimal input. **Recorded as the first follow-up input** after the baseline run is green. |
| `tests/fixtures/classifier/contract/01.pdf` | 2,337 B (readable) | Largest of the three byte-valid candidates; same generator and corpus manifest as the primary, so no marginal coverage gain. |
| `tests/fixtures/simple-text.pdf` | 587 B (readable) | BROKEN — `startxref` points away from the `xref` keyword; object offsets wrong; lenient rebuild required. Would have won on size alone, but a baseline must be known-good. |
| `tests/fixtures/classify_page_simple.pdf` | 540 B (readable) | BROKEN — same corrupt-generator defect family. |
| `test-minimal.pdf`, `valid-minimal.pdf`, `tagged-suspects-true.pdf`, `encoding/agl-only.pdf` | — | BROKEN — same defect family (per shortlist byte-validation in pdftract-025e4bac). |

## Conflict with pdftract-ea9c483a (resolved: sequenced, not contradictory)

pdftract-ea9c483a (closed, note committed at `ed34eab6`) independently selected
`tests/fixtures/markdown_structure.pdf`, from a root-level-only scan
(`tests/fixtures/FIXTURE_INVENTORY.md`) in which 10 of 11 root-level candidates
have corrupt xrefs — markdown_structure.pdf was the only byte-clean root-level
option under that constraint. Both picks are byte-valid; this is a criterion
difference, not an error. pdftract-dec41118's selection is the baseline input
because it is the bead in the blocked-by chain feeding the run bead
(pdftract-45aa0966); markdown_structure.pdf stands as the recorded first
follow-up input and satisfies ea9c483a's own root-level constraint.

## Carry-forward to the run bead (pdftract-45aa0966)

The shortlist's operational finding still stands: a stale/partially-built
pdftract binary rejects every fixture including this one ("Document contains no
pages"). Rebuild clean and confirm
`pdftract extract tests/fixtures/classifier/invoice/01.pdf` emits JSON before
capturing the baseline, otherwise a parse error gets recorded as the baseline.
`tests/baseline/capture-baseline.sh` is fixture-agnostic (takes the fixture as
arg 1, does its own `-f`/`-r` checks) — the selection imposes no protocol
coupling.
