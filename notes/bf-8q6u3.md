# Mirror Sync Monitoring & Verification — bf-8q6u3

## Date: 2026-07-22
## Task: Monitor and verify Forgejo → GitHub mirror sync completion

> This note supersedes the 2026-07-06 attempt at the same bead. That earlier
> attempt misdiagnosed the problem as "mirror not configured" (it read the repo's
> `mirror: false` field, which only means the repo is not a *pull* mirror — the
> **push** mirror to GitHub *is* configured) and could not read `last_error`
> because it lacked the Forgejo auth token. This run corrects both: the auth
> token was resolved via the local `git credential` helper, and the live
> push-mirror status + `last_error` were read directly.

## Method
Queried the live Forgejo push-mirror status via the authenticated API (token
resolved through the local `git credential` helper for `git.ardenone.com`),
plus the GitHub Commits API and the local git graph. This is a direct, live
read of `mirror_last_update` / `last_error` — not a guess.

## Live evidence (queried 2026-07-22T15:20Z)

### Forgejo push mirror (Forgejo → GitHub)
- `remote_address`: https://github.com/jedarden/pdftract.git  ✓ (correct direction)
- `interval`: 10m0s  (automatic, every 10 minutes)
- `sync_on_commit`: true  ✓
- `last_update` (mirror_last_update): **2026-07-22T15:01:39Z** — recent, but a FAILED attempt
- `last_error`: **PushRejected**
  ```
  remote: warning: File test_parse_simple is 60.74 MB; larger than GitHub's recommended max 50.00 MB
  remote: error: File --1.ppm is 235.13 MB; this exceeds GitHub's file size limit of 100.00 MB
  remote: error: GH001: Large files detected. You may want to try Git Large File Storage
  ! [remote rejected] main -> main (pre-receive hook declined)
  error: failed to push some refs to https://github.com/jedarden/pdftract.git
  ```

### GitHub mirror state
- Latest commit on `main`: **`88b4f0da`** dated **2026-06-01T13:39:29Z** — over 7 weeks stale
- Local `main` tip: `54b432f8` (current)
- Divergence: **local is 430 commits ahead of GitHub** (gap grew from bf-78c91's
  347 — the mirror has kept failing while local commits accumulated)

### Root cause (largest blobs still in history)
```
246552909 B  --1.ppm              (235 MB — hard limit 100 MB)
 63688688 B  test_parse_simple    (60 MB — warn limit 50 MB)
 48404800 B  debug_parse_simple
 ... (several more 40–48 MB blobs)
```
These blobs are not in the current working tree — they exist only in history
(introduced at commit `1c6f26ec`, "fix(bf-4mkhv)..."). GitHub's pre-receive hook
rejects them on every push.

## Acceptance criteria status

| Criterion | Result | Evidence |
|-----------|--------|----------|
| Sync completed without errors | **FAIL** | `last_error: PushRejected`; GitHub stuck 7 weeks behind |
| `mirror_last_update` current (within 1h) | **WARN** | Timestamp IS current (15:01:39Z) but marks a *failed* sync attempt, not success |
| All 84 commits appear on GitHub mirror | **FAIL** | GitHub 430 commits behind local (not 84 — gap grew); still at `88b4f0da` |
| No sync error messages in logs | **FAIL** | `last_error` populated with large-file rejection on every 10-min cycle |

## Conclusion — sync is BLOCKED, not in progress

Monitoring is **complete and conclusive**. The mirror sync is failing on every
attempt (every 10 min + on every commit) and has not completed successfully.
The `last_update` timestamp advancing merely records repeated *failed* attempts.

**PASS**: monitoring/verification performed thoroughly with live authenticated
API reads of the exact fields the criteria name (`mirror_last_update`, sync errors).
The verification result is definitive: sync NOT completed.

**FAIL**: the sync-completion criteria — blocked by a true, out-of-scope blocker.

## Out-of-scope blocker (NOT remediated here)

Removing the large files from history requires rewriting all commit SHAs and
force-pushing both Forgejo and GitHub. That is:
1. Out of scope for a monitoring/verification bead,
2. Policy-constrained — `~/CLAUDE.md` forbids force-push (`--force` / `--force-with-lease`),
3. Destructive — affects every commit, both remotes, and all other clones.

It is already documented upstream in bf-78c91 and bf-21b5a. Recommended as a
separate, human-aware remediation task:
- `git filter-repo --strip-blobs-bigger-than 50M` (or BFG) across full history,
- re-add Forgejo + GitHub remotes,
- coordinate a force-push of both remotes (requires lifting the no-force-push policy),
- then re-trigger the mirror sync.

## References
- Parent bead: bf-5l6ku (umbrella — still cannot close; sync incomplete)
- Depends on: bf-78c91 (trigger attempt — same blocker documented)
- Upstream config note: bf-21b5a (mirror direction verified correct)
