# pdftract-ff5c577d — auto-split dispatch DECLINED (2026-09-08)

## What arrived

The NEEDLE auto-split path dispatched pdftract-ff5c577d as "failed 3 times in a
row, split into 3–5 children, convert to umbrella." This note records why the
split was **not** performed.

## Why the premise is false

`failure-count:3` is reopen churn, not three failed attempts. The forensic
checkpoint (`grep pdftract-ff5c577d .beads/checkpoint/forensic.jsonl`) shows:

| Time (2026-09-08 UTC) | Event | Detail |
|---|---|---|
| 18:40:14 | closed | PASS, full identity block (worker `glm-face` predecessor) |
| 18:42:34 | reopened by `system` | **no reason given** |
| 18:51:31 | closed | PASS, per-line verbatim transcription (worker `glm-tgp`) |
| 18:55:48 | reopened by `system` | **no reason given** |
| 19:07:18 | closed | PASS, full header + SHA provenance (worker `glm-coned`) |
| 19:23:44 | reopened by `system` | **no reason given** |

All three close reasons satisfy the acceptance criteria exactly as written:
full 40-hex SHA, Author, AuthorDate, Commit, CommitDate, and subject lines
quoted verbatim. None of the reopens carries a reason. Same pathology as
`notes/pdftract-87de95ea.md` (commit `d68d9dfe`) and
`notes/pdftract-1f1cf7a5.md` (commit `a5521a88`): evidence-bearing closes
reopened as `verification-failed`, then `failure-count` read as task size.

## Independent re-derivation at decline time (2026-09-08 19:52Z, HEAD = 20fac636)

Read-only, per the bead's hard rules — ran the exact commanded line:

```
git show --stat --format=fuller 0364a3a8
```

The SHA resolves. Verbatim identity block from that output:

```
commit 0364a3a89eabf5a02715b7ef7daba764f42d23d7
Author:     jedarden <github@jedarden.com>
AuthorDate: Tue Sep 8 08:12:53 2026 -0400
Commit:     jedarden <github@jedarden.com>
CommitDate: Tue Sep 8 08:12:53 2026 -0400

    docs(pdftract-cf349714): add notes/evidence manifest README
```

Byte-identical to what all three prior closes transcribed. The stat shows one
file, `notes/evidence/README.md | 57 insertions(+)` — not this child's scope
(child 3 owns the diffstat comparison); quoted for provenance only.

**Verdict: PASS.** The acceptance criteria are met at HEAD right now, and were
met at every one of the three prior closes.

## Why splitting is the wrong action

1. The task is atomic: one read-only git command, six verbatim lines, evidence
   in the close reason. Its only possible "children" are per-line re-reads of
   output the child must produce in one command — each destined for the same
   reopen churn, none independently verifiable without running the parent's
   own command.
2. The bead is **already `split-child` 1 of 4** of the existing split of
   umbrella `pdftract-c0acafa1`. The decomposition the dispatcher is asking
   for already exists one level up; nesting a second split under it deepens
   the treadmill, it does not converge.
3. Sibling `pdftract-7a8da184` (child 2) is blocked directly on this bead,
   which in turn blocks `pdftract-bc29f726` → `pdftract-0cf5f8c4` → the parent
   `pdftract-c0acafa1`. Adding 3–5 intermediate nodes and a
   "parent depends on last child" edge can only postpone the entire chain.
4. The parent itself (`pdftract-c0acafa1`, `failure-count:4`, already
   `umbrella`) is inside the same reopen loop, as are its ancestors
   (`50b9386b` → `ab6a49c2` → `d077ca93`). Splitting has been applied four
   levels deep in this lineage already; no level has ended the loop.

## Actions taken

- No child beads created; no `umbrella` label added; no dependency edges
  added; no SPLIT_COMPLETE emitted (no split was performed).
- Bead left open/unassigned via `bead release` for the operator to decide,
  per the precedent of `pdftract-87de95ea` and `pdftract-1f1cf7a5` — the
  dispatch's "do not close" being the one instruction consistent with the
  evidence.
- Read-only hard rules honored: no tree modifications by this decline beyond
  this note, no cargo build/test, no socket, no spawned server, no history
  rewrite, no blanket staging (single path committed).

## Recommendation

**Close pdftract-ff5c577d** on the strength of the three prior
evidence-bearing closes plus the re-derivation above — the criteria are met
and closing unblocks `pdftract-7a8da184` and the rest of the c0acafa1 chain.
The fix that ends this treadmill is upstream of the workers: the automated
verifier keeps reopening valid closes without a reason, so `failure-count` on
verification-only beads measures the verifier, not the work.
