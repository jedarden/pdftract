# pdftract-3c39e12c — auto-split re-issue declined; evidence close at HEAD a0dc9569

**Dispatch:** another auto-split demand for this bead ("failed 5 times in a row", MUST
create 3–5 children), issued 2026-09-08 after `quarantine-until:…T21:52:30Z` expired.
**Action:** declined the split; re-derived all four acceptance criteria at HEAD `a0dc9569`
and closed on that evidence. Decline notes alone have not stopped redispatch (see
`notes/pdftract-1ce1beaf.md`, `notes/pdftract-d5598671.md`); the terminal action recorded
for this treadmill is an evidence close.

## Why the split is declined

1. **Already an umbrella, more than once.** This bead was previously split into
   `c04460c3` / `02de535f` / … (earlier generation, cited in this bead's own prior close
   reason) and again into `b6d69433` (publication), `d5598671` (orphans), `778fc59c`
   (gate status), `f141d045` (consolidated split record, commit `cc83905b`). The demanded
   3–5 new children would duplicate those one-for-one; org bead rules forbid two
   independent "implement X" beads for the same X.
2. **Criteria are met; the failure count is churn.** All four criteria re-verified PASS
   below. `failure-count:5` counts verifier reopens after quarantine expiry, not scope
   failures — this bead was already closed on equivalent evidence by prior workers
   (`glm-coned`; "DECLINED split order (dispatch #6)"; "Declined the split order
   (dispatch 10 of 10 today)" — all visible in `.beads/checkpoint/forensic.jsonl`,
   77 recorded events for this bead).

## Re-derivation at HEAD `a0dc9569` (2026-09-08T23:00Z, read-only git only)

| Criterion | Evidence | Result |
|---|---|---|
| Handoff note exists, quotes verdict + exit code + SHAs | `notes/b716bac5-child1-handoff.md`: "VERDICT: NO-GO", gate exit **101**, certifies `e9036d20`, `336bf8ee`, `10df3676`, `1702ba6b` | PASS |
| Forgejo main contains the verdict-note commit | `git fetch origin && git merge-base --is-ancestor`: `10df3676`, `1702ba6b`, `cc83905b`, `e9036d20`, `336bf8ee` all ancestors of `origin/main` | PASS |
| No chain-spawned orphans | `pgrep -af 'cargo[ ](test|nextest)\|[r]ustc\|pdftract[ ](mcp\|-)'` matched only the check shell itself (self-match artifact); `ps -eo comm=` lists no cargo/rustc/nextest/pdftract process | PASS |
| Commit cites this bead and is pushed | `1702ba6b` = "docs(pdftract-3c39e12c): publish compile-gate NO-GO handoff…", ancestor of `origin/main` | PASS |

No cargo invocation was run and no file under `crates/` was touched, per the bead's scope.

## Gate status re-derived at the current tip (child 778fc59c's scope)

`git log --oneline 0364a3a8..origin/main -- crates/pdftract-core/src/classify.rs
crates/pdftract-core/src/render/scanline.rs` → **empty**: no fixing commit since gate
HEAD `0364a3a8`. **NO-GO stands at `a0dc9569`**, so the handoff statement is unchanged:
the umbrella's children 2–4 may NOT run `cargo test` against this tree until BOTH
`classify.rs` and `render/scanline.rs` clear on `origin/main` — 59 of the 89 gate errors
are committed at the pushed tip and cannot clear via local in-flight edits.

Child `778fc59c` was in_progress under another worker (glm-face) at the time of this
close, so its closure is that worker's step; the re-derivation above is current evidence
it can cite instead of re-running anything.

## Outcome

- `pdftract-3c39e12c` closed on this evidence (this commit + the close reason).
- Split **not** performed — no new children created.
- Downstream `pdftract-fa233a5e` / `pdftract-b716bac5` remain open by design: the verdict
  they consume is NO-GO, and their real blockers are the four broken test modules
  (`classify.rs`, `font/type3_rasterizer.rs`, `render/scanline.rs`, `content_stream.rs`),
  not this handoff, which is published.
