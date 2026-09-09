# pdftract-3c39e12c — third auto-split re-issue declined; evidence close at tip 2644ab93

**Dispatch:** another auto-split demand for this bead ("failed 7 times in a row"; MUST create
3–5 children, chain them, convert this bead to an umbrella, do not close), recorded as an
`updated` event at **2026-09-09T11:43:06Z** (forensic seq 5893) — approximately 26 seconds
after the live `quarantine-until:2026-09-09T11:42:40Z` label expired. Round 3 of the
quarantine/redispatch cycle (`failure-count:7`, `quarantine-round:3`).
**Action:** declined the split; re-derived all four acceptance criteria read-only at the
current tip and closed on that evidence. Same terminal action as decline 1
(`notes/pdftract-3c39e12c-reissue-decline.md`, commit `afe0317a`) and decline 2
(`notes/pdftract-3c39e12c-reissue-decline-2.md`, commit `9b6553a0`).

## Why the split is declined

1. **Already an umbrella — with all children satisfied.** This bead carries the `umbrella`
   label and an existing 4-child decomposition (`b6d69433` publication, `d5598671` orphans,
   `778fc59c` gate status, `f141d045` consolidated split record, commit `cc83905b`; see
   `notes/pdftract-3c39e12c-split-record.md`). Statuses at close time: `b6d69433` **closed**,
   `d5598671` **closed**, `f141d045` **closed**; `778fc59c` closed on its 7th re-affirmation
   at 2026-09-09T07:25Z (commit `d3ba3419`) and was treadmill-reopened afterward — a separate
   dispatch cycle, not missing work. The demanded 3–5 new children would duplicate the
   existing decomposition one-for-one and add a third nested split generation, multiplying
   redispatch surface without producing anything.
2. **`failure-count:7` is reopen churn, not scope failure.** The count advanced
   `5 → 6 → 7` via system label bumps at 2026-09-09T03:42Z (forensic seq 5740–5742) with no
   verifier reason attached to the underlying reopen — same mechanism decline 2 documented at
   seq 5606–5613. The deliverable has passed every re-derivation at every tip since its
   original publication: `afe0317a` (tip `a0dc9569`), `9b6553a0` (tip `c309d1b0`), and now
   this note (tip `2644ab93`).
3. **Precedent.** Twelve-plus committed declines across this chain decline identical
   auto-split orders: `afe0317a` + `9b6553a0` (this bead), `589f8c55` (fa233a5e), `378bd894`
   (bf10a431), `bd4d42e9`/`0e04fc38`/`8ee27f95`/`60d981cf` (778fc59c), `d3ba3419` (778fc59c,
   7th), `c309d1b0` (1f1cf7a5), `b8d3f12b`/`df7b7de2` (d5598671/1ce1beaf). Splits were
   executed where real work remained; none remained here.

## Re-derivation at the current tip (2026-09-09, read-only git only, no cargo)

Local HEAD == `origin/main` == `2644ab93e7da6739b8d5c38260c085c31873905d`
(`git rev-list --count HEAD..origin/main` = 0).

| # | Acceptance criterion | Result |
|---|---|---|
| 1 | `notes/b716bac5-child1-handoff.md` exists, quotes the verdict and exit code, names the certified SHAs | **PASS** — 4,358 bytes, committed `1702ba6b`; quotes `VERDICT: NO-GO`, gate exit code **101**, certifies `e9036d20` / `336bf8ee` / `10df3676` |
| 2 | Forgejo remote's main contains the verdict note's commit | **PASS** — `git merge-base --is-ancestor` exit 0 at tip `2644ab93` for **all four** chain SHAs (`e9036d20`, `336bf8ee`, `10df3676`, `1702ba6b`) |
| 3 | `pgrep -af "cargo test\|pdftract"` clean of chain-spawned processes | **PASS** — only matches were the check shell's own argv and NEEDLE harness `bash` wrappers (classified by comm name); zero `cargo`/`rustc`/`pdftract` binaries; zero kills needed |
| 4 | Commit cites this bead ID; pushed to forgejo main | **PASS** — `1702ba6b`, `afe0317a`, `9b6553a0`, `cc83905b` already on `origin/main`; this note adds one more |

## NO-GO re-affirmation (8th determination)

`git rev-list --count 0364a3a8..origin/main` = **60 commits, every one docs-only — zero
touch any path under `crates/`**. Both tip-broken files are clean in the working tree and
**byte-identical to gate HEAD `0364a3a8`** (`git diff --stat 0364a3a8 -- <file>` empty for
`classify.rs` and `render/scanline.rs`), so their 59 committed `#[cfg(test)]` errors remain
unresolved at the pushed tip. Per the rule fixed by child 3 (`778fc59c`): **NO-GO stands
unless a fixing commit for BOTH `classify.rs` AND `render/scanline.rs` is verifiably on
`origin/main`** — none exists. Consistent with the 7th re-affirmation at tip `b1f2e735`
(commit `d3ba3419`); the umbrella's children 2–4 must still not run `cargo test`.

## Compliance (this re-issue handling)

- Read-only git only (`fetch`, `rev-parse`, `rev-list`, `merge-base --is-ancestor`, `log`,
  `diff --stat`, `status`); no cargo invocation of any kind, per the bead's own prohibition.
- No file under `crates/` touched; the only file written is this note.
- Orphan check re-run at close (criterion 3 above).
