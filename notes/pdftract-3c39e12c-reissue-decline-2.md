# pdftract-3c39e12c — second auto-split re-issue declined; evidence close at tip c309d1b0

**Dispatch:** another auto-split demand for this bead ("failed 6 times in a row"; MUST create
3–5 children, chain them, convert this bead to an umbrella, do not close), issued
2026-09-09T03:31Z, immediately after `quarantine-until:2026-09-09T03:29:40Z` expired.
**Action:** declined the split; re-derived all four acceptance criteria read-only at the
current tip and closed on that evidence. Same terminal action as the first decline
(`notes/pdftract-3c39e12c-reissue-decline.md`, commit `afe0317a`).

## Why the split is declined

1. **Already an umbrella — twice.** This bead carries the `umbrella` label and an existing
   4-child decomposition (`b6d69433` publication, `d5598671` orphans, `778fc59c` gate status,
   `f141d045` consolidated split record, commit `cc83905b`), with an earlier generation
   (`c04460c3`/`02de535f`) before that. All children are closed. The demanded 3–5 new
   children would duplicate them one-for-one; org bead rules forbid two independent
   "implement X" beads for the same X, and a third nested split generation multiplies
   redispatch surface without addressing the reopen mechanism at all.
2. **`failure-count:6` is reopen churn, not scope failure.** The forensic log
   (`.beads/checkpoint/forensic.jsonl`, event seq 5606) shows the reopen as
   `kind: "reopened"`, `actor: "system"`, `prior_base_status: "closed"`, at
   2026-09-08T23:29:39Z — no verifier reason attached — mechanically followed by
   `failure-count:6` + `quarantine-round:2` labels (seq 5607–5613). This bead's deliverable
   was already closed on equivalent evidence at `a0dc9569` (`afe0317a`).
3. **Precedent.** At least nine committed declines across this chain decline identical
   auto-split orders: `afe0317a` (this bead), `589f8c55` (fa233a5e), `378bd894`
   (bf10a431), `abde046f`/`0e04fc38`/`bd4d42e9` (778fc59c, 4th–6th), `c309d1b0`
   (1f1cf7a5), `b8d3f12b`/`df7b7de2` (d5598671/1ce1beaf).

## Re-derivation at the current tip (2026-09-09T03:37Z, read-only git only, no cargo)

Local HEAD `c309d1b0`; `origin/main` `bd4d42e9` (HEAD is 1 commit ahead — the sibling
worker's `docs(pdftract-1f1cf7a5)` decline commit, pushed together with this note per the
shared-checkout convention).

| Criterion | Evidence at this tip | Result |
|---|---|---|
| Handoff note exists, quotes verdict + exit code + SHAs | `notes/b716bac5-child1-handoff.md` (tracked, worktree-clean): "VERDICT: NO-GO", gate exit **101**, certifies `e9036d20`, `336bf8ee`, `10df3676`, published as `1702ba6b` | PASS |
| Forgejo main contains the verdict-note commit | `git merge-base --is-ancestor` vs `origin/main`: `10df3676`, `1702ba6b`, `e9036d20`, `336bf8ee`, `cc83905b`, `afe0317a` all ancestors | PASS |
| No chain-spawned orphans | `pgrep -af 'cargo[ ](test\|nextest)\|[r]ustc\|pdftract[ ](mcp\|-)'` matched only its own check shell (self-match artifact, PID 829126 = the sweep's bash); `ps -eo comm=` lists **zero** cargo/rustc/pdftract/nextest processes | PASS |
| Commit cites this bead and is pushed | `1702ba6b` = "docs(pdftract-3c39e12c): publish compile-gate NO-GO handoff…", ancestor of `origin/main` | PASS |

No cargo invocation was run and no file under `crates/` was touched, per the bead's scope.

## Gate status re-derived for currency (child 778fc59c's scope, cited not re-run)

`git rev-list --count 0364a3a8..origin/main` = **55**; the range test over all four
inventory files (`classify.rs`, `render/scanline.rs`, `font/type3_rasterizer.rs`,
`content_stream.rs`) is **empty** — no commit touches any of them; the fixing-commit test
(`0364a3a8..origin/main -- classify.rs render/scanline.rs`) is empty, so GO's condition (a
fixing commit on `origin/main` for BOTH tip-broken files) is unmet. **NO-GO stands**, and
the published handoff statement — the umbrella's children 2–4 may not run `cargo test`
against this tree until both files clear on `origin/main` — is current as written.

## Outcome

Evidence close as the terminal action. If this close is reopened again without a verifier
reason, re-derive at HEAD and re-close; the load-bearing facts (deliverable published at
`1702ba6b`; no fixing commit for either tip-broken file) do not change by re-derivation.
