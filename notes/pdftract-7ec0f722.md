# pdftract-7ec0f722 — real-world failure-class fixtures + extraction baseline harness

- Fixture/harness commit: `26eaa805` (attempt 1; never verified — that attempt and
  attempt 2 both died to API errors before any test run).
- This dispatch (attempt 3): verified everything at HEAD, found 2 of the 8 active pins
  asserted **desired** behavior rather than current behavior, re-grounded them, and
  filed the two failure classes the corrected pins expose.
- Verified in a clean `git archive HEAD` extraction at `/var/tmp/pdftract-7ec0f722-LVhl`
  (never the shared tree — the shared working tree does not compile as a union of
  concurrent workers' edits).

## Fixture → class → probed current behavior at HEAD

All pdftract numbers probed 2026-09-22 at HEAD `145c8291` (clean extraction), via both
`cargo test -p pdftract-core --test realworld_extraction_baseline` and a HEAD-built
`pdftract extract --text -` CLI. PyMuPDF column re-validated this dispatch with fitz
1.27.2.2 — attempt 1's recorded numbers reproduce exactly.

| Fixture | Class | PyMuPDF | pdftract at HEAD | Pin | Owner |
|---|---|---|---|---|---|
| `tests/fixtures/realworld/incremental-updates-offer-letter.pdf` | /Prev incremental-update chain (Docusign) | 4 pp / 2,460 ch | extracts ok (4 pp, 2,411 ch via CLI) | regression | — |
| `tests/fixtures/realworld/xref-stream-only-report.pdf` | xref-stream-only (business report) | 6 pp / 1,218 ch | extracts ok | regression | — |
| `tests/fixtures/realworld/startxref-offset-edge.pdf` | startxref misses keyword → "No trailer in xref section" | 2 pp / 2,050 ch (recovers) | fails with exactly that error | FAIL pinned | pdftract-0df07688 |
| `tests/fixtures/realworld/dense-one-page-agreement.pdf` | page-tree control **+ escaped-paren text loss** | 1 pp / 2,050 ch | Ok, 1 page, **0 chars** | FAIL pinned | pdftract-5b4c3d0e (text); pdftract-bcf935ec owns no-pages |
| `crates/.../fixtures/linearized-10.pdf` | multi-section xref | 0 pp / 0 ch | "Failed to resolve /Root: object 3 0 R not found" | FAIL pinned | pdftract-4196ae99 |
| `crates/.../fixtures/multipage-100.pdf` | multi-section xref | 0 pp / 0 ch | "Failed to resolve /Root: object 1 0 R not found" | FAIL pinned | pdftract-4196ae99 |
| `crates/.../fixtures/valid-minimal.pdf` | page tree → "Document contains no pages" | 1 pp / 12 ch | fails with exactly that error | FAIL pinned | pdftract-bcf935ec |
| `tests/fixtures/valid-minimal.pdf` (repo-root control) | xref-offset drift | 1 pp / 5 ch | Ok, 1 page, **0 chars** | FAIL pinned | pdftract-4683109d |

## New findings filed this dispatch (with isolation evidence)

Both classes are **silent empty output** — extraction returns `Ok`, so nothing in the
error surface flags them. Probes used variant PDFs built from the checked-in generator;
all probe files under `/var/tmp/pdftract-7ec0f722-probe/` (transient, re-derivable).

### 1. Escaped parentheses in literals zero the text layer → pdftract-5b4c3d0e

| Variant | text chars |
|---|---|
| `dense-one-page-agreement.pdf` (literals contain `\(startxref offset edge fixture\)`) | 0 |
| identical doc, parens replaced by a dash (only byte difference) | 2,014 |
| same, built 2-page | 2,015 |
| offer-letter page 1 only (no parens, same builders) | 602 |

### 2. Wrong xref entry offset zeroes the text layer → pdftract-4683109d

The repo-root W3C dummy is internally inconsistent: obj 4 actually at byte **290**, its
xref entry says **298**; startxref says **403** but the keyword is at **376**; entries are
**19 bytes** (spec: 20).

| Variant | text chars |
|---|---|
| file as committed | 0 |
| fix ONLY obj4 offset 298→290 (entries still 19-byte, startxref still wrong) | 4 (`Test`) |
| fix ONLY startxref 403→376 | 0 |
| full normalization (20-byte entries + true offsets + true startxref) | 4 |

The shifted strict read truncates the stream mid-literal (`(Test` unclosed) and the page's
spans are silently dropped.

### Disproven hypotheses (do not re-chase)

- Inline font resource dict (`/F1 << /Type /Font ... >>`): extracts 4 chars when built
  with correct geometry — not the trigger.
- `/Length` mismatch (declared 44, actual 36): extracts 4 chars — not the trigger.

## Bead/graph changes this dispatch

- Created `pdftract-5b4c3d0e` (bug, prio 1) and `pdftract-4683109d` (bug, prio 1),
  both labeled `parent-pdftract-257d92c3`; `bead dep add pdftract-257d92c3 <each>`
  wired (parent umbrella now blocked by 5 children).

## Verification record (clean extraction, exit codes)

```
cargo test -p pdftract-core --test realworld_extraction_baseline   # 6 pass / 2 FAIL (aspirational pins, pre-fix)
cargo test -p pdftract-core --test realworld_extraction_baseline   # 8 pass / 0 fail / 6 ignored (post-fix) exit=0
cargo build -p pdftract-cli                                        # exit=0 (HEAD-built probe binary)
bash scripts/check-provenance.sh --all                             # exit=1: 56 pre-existing errors, all on
                                                                   # unrelated fixtures (scanned/receipt,
                                                                   # security/embedded-js, tagged/mc_*); zero
                                                                   # realworld mentions; WARN W3C-Document-License
                                                                   # is pre-existing
```

WARN items: the 56 provenance errors pre-date this bead (the bead commit only added
realworld rows/files — `git diff cf61a299 145c8291 -- tests/fixtures/` shows realworld
additions only). PyMuPDF re-validation was possible this dispatch (fitz installed).

## Harness final state

14 tests: 8 current-behavior pins (all green at HEAD), 6 `#[ignore]`d desired-behavior
pins, each ignore reason naming its owning child (pdftract-0df07688, pdftract-4196ae99 ×2,
pdftract-bcf935ec, pdftract-5b4c3d0e, pdftract-4683109d). No cargo feature gates anywhere.
