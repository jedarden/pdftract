# pdftract-62d653ec — verbatim transcription: the recorded non-owner reason for pdftract-adda71a4

**Bead:** pdftract-62d653ec (auto-split child 1 of 4 of umbrella pdftract-1f1cf7a5;
read-only verification chain). **Guard honored:** read-only bead commands plus this
note only — no mmap-suite run, no cargo invocation of any kind, no production code
change, no other bead's status touched.

**Subject:** the already-recorded one-line non-owner reason for **pdftract-adda71a4**,
as recorded by coordination bead pdftract-cd0c29e6 ("Check the mmap-re-run sibling
beads for duplicate ownership"). Both evidence sources were read directly; each quote
below is a byte-for-byte transcription.

## Source (a) — Notes field of coordination bead pdftract-cd0c29e6

**Citation:** live bead store `.beads/` (prefix `pdftract`), bead `pdftract-cd0c29e6`,
**Notes field**, revision 14 (Updated 2026-09-08T13:29:50Z), check stamped
"Coordination check 2026-09-08T12:18Z". Retrieved read-only via
`bead show pdftract-cd0c29e6` and cross-checked byte-for-byte against
`bead show pdftract-cd0c29e6 --json` (raw field, no display formatting).

**Verbatim** (the sentence sits in the "NON-OWNER REASONS (one line each):" list; the
semicolon after "re-run)" separates it from the following `bf-5o22rf` entry):

> pdftract-adda71a4 = umbrella NOTE CONSOLIDATION only (writes notes/bf-5o22rf.md from children's evidence; its final suite check is a confirmation step, not the owned re-run)

## Source (b) — notes/pdftract-cd0c29e6.md at commit 3dcfb6be

**Citation:** `notes/pdftract-cd0c29e6.md` at commit **3dcfb6be**
("docs(pdftract-cd0c29e6): mmap re-run duplicate-ownership check — single owner
b716bac5 confirmed, sweep none found"), section "## Non-owner reasons (one line each)",
table row for pdftract-adda71a4. The file is unchanged from 3dcfb6be through HEAD
(`git diff 3dcfb6be HEAD -- notes/pdftract-cd0c29e6.md` is empty), so this is also the
current checked-in text. That file records its own check as performed 2026-09-08T11:41Z.

**Verbatim** (reason column of the table row; bead and status columns omitted):

> Terminal umbrella consolidation (child 5 of 5) — writes notes/bf-5o22rf.md from other children's evidence; consumes the re-run verdict, executes nothing itself.

## Mismatch between the two sources — EXPLICIT

**The two sources do NOT carry the same sentence. Both are genuine records of the same
conclusion, but the wording differs, and neither contains the other's text.**

- Same in both: pdftract-adda71a4 is an umbrella whose role is **note consolidation** —
  it assembles `notes/bf-5o22rf.md` from *other* beads' evidence, and it does not itself
  perform the owned mmap-suite runtime re-run. The non-owner conclusion is identical.
- Different: source (a) (bead Notes, 12:18Z pass) justifies it as "its final suite check
  is a confirmation step, not the owned re-run"; source (b) (note file, 11:41Z pass)
  justifies it as "consumes the re-run verdict, executes nothing itself". Source (b)
  additionally carries the positional detail "(child 5 of 5)", which source (a) omits;
  source (a) uses the capitalization "NOTE CONSOLIDATION", which source (b) does not.

**Secondary discrepancies between the two records** (context for the timestamp in this
bead's title, not part of the adda71a4 reason): the bead Notes field stamps the check
2026-09-08T12:18Z and reports a sweep of 2,705 beads with 24 created after the 05:15 UTC
cutoff, while the note file records the check at 2026-09-08T11:41Z and a sweep of 2,697
beads with 16 created after cutoff. The Notes field (revision 14, updated 13:29:50Z)
therefore reflects a later re-verification pass than the version committed in the note
file at 3dcfb6be; the note file was never updated to the 12:18Z numbers.

**Neither discrepancy affects the conclusion being transcribed:** both records
consistently rule pdftract-adda71a4 out as the owner of the bf-5o22rf criterion-(a)
mmap-suite runtime re-run, which remains owned by pdftract-b716bac5's chain.
