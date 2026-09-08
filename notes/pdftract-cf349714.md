# Verification note — pdftract-cf349714 (Add a manifest README to the notes/evidence workspace)

Recorded 2026-09-08 by **pdftract-e3e4847b** (child 2 of 3 of the re-split of umbrella
pdftract-cf349714; grandparent umbrella pdftract-d077ca93). This note consolidates outcomes
that were previously scattered across five beads' close reasons; it records them, it does not
re-derive the checks.

## Purpose

The parent umbrella pdftract-cf349714 owns one deliverable: `notes/evidence/README.md`, the
manifest that makes the evidence workspace self-describing. The deliverable landed and was
verified by a chain of single-purpose beads, but the parent had no closure record in the repo.
This note is that record, so the parent can be closed from the note alone.

## The deliverable

- Introducing commit: **0364a3a8** (`0364a3a89eabf5a02715b7ef7daba764f42d23d7`),
  `docs(pdftract-cf349714): add notes/evidence manifest README`.
- Single-path: touches exactly `notes/evidence/README.md`, status A, +57/−0 (57 insertions).
- Subject follows the `docs(<bead-id>):` convention; body cites grandparent umbrella
  pdftract-d077ca93. Non-merge commit (single parent e9036d20), so the diffstat is complete.
- Content: the four required manifest sections — `## Purpose`, `## Naming convention`
  (`<bead-id>-<kind>.txt`, e.g. `pdftract-d077ca93-baseline.txt`), `## Freshness rule`
  (captures are produced fresh by the run that commits them, never copied forward), and
  `## Evidence beads are measurement-only` (no crates/ changes, no cargo build).
- On origin/main (see pdftract-758dff02 below).

## Outcomes

| Check | Bead | Outcome | Recorded evidence |
|---|---|---|---|
| README content: four required sections | pdftract-0a91c85a | **PASS** | All four sections present and accurate, cited by heading text; HEAD copy byte-identical to 0364a3a8 (no drift); zero tree change (explicitly a valid PASS). |
| Commit is single-path, no crates/ drift | pdftract-ab6a49c2 | **PASS** | Via disposition recorded by pdftract-eaa42093: `git show --name-status` lists exactly one path, A `notes/evidence/README.md`; numstat `57 0` (numeric, not the binary `-` marker); verbatim diffstat in the close reason; crates/ paths = 0; no binary. |
| Commit is on origin/main | pdftract-758dff02 | **PASS** | No-op confirmation (already pushed): `git merge-base --is-ancestor 0364a3a8 origin/main` exit 0 and `git branch -r --contains 0364a3a8` lists `origin/main`; origin/main SHA at check time `576705dd` (`576705ddb9b8fea95367d5dbf21ff9abae14a11a`). |
| Inventory reconciled against manifest | pdftract-974805d9 | **PASS** | Exactly 4 artifacts, all tracked, all tabled, all convention-conforming: `pdftract-ba4e7150-meta.txt`, `pdftract-ba4e7150-raw.txt`, `pdftract-c1f3cf4b-raw.txt`, `pdftract-d077ca93-baseline.txt`; `README.md` tracked as the manifest itself; no untracked files in the dir. Zero tree change. |
| Disposition of the stuck verifier | pdftract-eaa42093 (re-split child 1) | **PASS** | pdftract-ab6a49c2 CLOSED with the verbatim diffstat recorded as its disposition (rev 12, `--if-revision` guarded). Root cause of its 3 prior `verification-failed` closures: workers misread the dirty shared working tree (unrelated in-flight crates/ edits) as crates/ drift; the criterion is scoped to the commit. |
| Consolidated closure record (this document) | pdftract-e3e4847b (re-split child 2) | this note | Deliverable of this bead: the file you are reading, committed single-path. |

## Chain wiring

Original 4-child auto-split of pdftract-cf349714 (each blocked by its predecessor; the parent
closes last):

```
pdftract-0a91c85a (content) -> pdftract-ab6a49c2 (single-path) -> pdftract-758dff02 (ancestry)
                            -> pdftract-974805d9 (inventory) -> parent pdftract-cf349714
```

pdftract-ab6a49c2 failed 3x as `verification-failed` and stuck, so a 3-child re-split owned
what remained:

1. pdftract-eaa42093 — adjudicate the stuck verifier (closed ab6a49c2; PASS).
2. **pdftract-e3e4847b** — record this consolidated note (this document).
3. Child 3 of the re-split — closes parent umbrella pdftract-cf349714 once this note is on
   origin/main.

Grandparent umbrella: pdftract-d077ca93 (prepare the evidence workspace and record the pre-run
baseline for the classify.rs compile check).

## WARN items and caveats

No chain bead recorded a WARN or FAIL in its final outcome; all five closed PASS. Caveats
worth carrying forward:

- **pdftract-ab6a49c2's failure history was a measurement error, not a defect.** The 3 prior
  `verification-failed` closures came from reading the dirty shared working tree as crates/
  drift. Every check in this chain is scoped to commits, never the working tree.
- **Ancestry evidence is time-stamped.** pdftract-758dff02 proved 0364a3a8 an ancestor of
  origin/main while origin/main was at 576705dd. origin/main has since advanced (43e7227d at
  the time of writing); a read-only re-check on 2026-09-08 (`git merge-base --is-ancestor
  0364a3a8 origin/main`, exit 0) confirms the commit is still an ancestor. The commit remains
  the only one ever to touch `notes/evidence/README.md`.
- This note was written from a working tree carrying unrelated in-flight edits elsewhere
  (including under crates/). They belong to other work and are untouched here; the commit for
  this bead adds only this file.
