# pdftract-7a8da184 — auto-split dispatch DECLINED (2026-09-08)

## What arrived

The NEEDLE auto-split path dispatched pdftract-7a8da184 ("Enumerate the
per-path change types of commit 0364a3a8") as "failed 3 times in a row, split
into 3–5 children, convert to umbrella." The split was **not** performed; the
bead was re-verified at HEAD and closed PASS instead.

## Why the premise is false

`failure-count:3` is reopen churn, not three failed attempts. The forensic
checkpoint (`grep pdftract-7a8da184 .beads/checkpoint/forensic.jsonl`) shows:

| Time (2026-09-08 UTC) | Event | Detail |
|---|---|---|
| 18:44:22 | closed | PASS — verbatim name-status line + both counts (worker glm-seam) |
| 18:46:11 | reopened by `system` | **no reason given** |
| 18:52:45 | closed | PASS — verbatim line, counts, `wc -l` + `cat -A` byte check (worker glm-seam) |
| 18:53:19 | reopened by `system` | **no reason given** |
| 19:11:06 | closed | PASS — verbatim line + counts, SHA provenance (worker glm-cgraph) |
| 19:13:47 | reopened by `system` | **no reason given** |

All three close reasons satisfy the acceptance criteria exactly as written:
every name-status line verbatim, the raw entry count, and the raw path count.
None of the reopens carries a reason. Same pathology as sibling child 1
(`notes/pdftract-ff5c577d.md`, commit `24f65b80`) and the earlier
`pdftract-87de95ea` / `pdftract-1f1cf7a5` declines: evidence-bearing PASS
closes reopened as `verification-failed`, then `failure-count` misread as task
size.

## Independent re-derivation at decline time (2026-09-08, HEAD = 24f65b80)

Read-only, per the bead's hard rules — ran the exact commanded line:

```
git show --name-status --format= 0364a3a8
```

The SHA resolves to `0364a3a89eabf5a02715b7ef7daba764f42d23d7`. Output at byte
level (`cat -A`):

```
A^Inotes/evidence/README.md$
```

Verbatim transcription:

```
A	notes/evidence/README.md
```

- Raw entry count (lines of name-status output): **1**
- Raw path count: **1** (the single `A` entry carries one path; no `R`/rename
  lines are present, so no entry contributes two paths)

Byte-identical to all three prior closes. **Verdict: PASS** — the acceptance
criteria are met at HEAD right now, and were met at every one of the three
prior closes.

## Why splitting is the wrong action

1. This bead is already a split leaf — `split-child` 2 of 4 under umbrella
   `pdftract-c0acafa1`. Splitting a split-child re-applies the operation that
   produced this treadmill; the lineage is already five deep (d077ca93 →
   ab6a49c2 → 50b9386b → c0acafa1 → 7a8da184).
2. The unit of work is one read-only git command returning one line. Its only
   possible "children" are per-line or per-count re-reads of that same command
   — not independently achievable scopes — and each would re-enter the same
   close/reopen churn, multiplying quarantined beads and dispatching more
   workers into the same wall. The bead already carries `over-budget`.
3. The dispatcher's size signal is fabricated from reopen churn: three PASS
   closes are not three failures, and there is nothing here to decompose.

## Action taken

- Split declined: no children created, umbrella conversion not applied.
- Bead closed PASS with the verbatim line plus both counts in the close
  reason (per the bead's hard rule: evidence lives in the close reason, not a
  notes file).
- This note records the decline rationale only.
