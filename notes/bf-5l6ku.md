# Bead bf-5l6ku: Trigger Forgejo Mirror Sync to GitHub

## Date: 2026-07-22
## Task: Trigger the Forgejo → GitHub push-mirror sync to push missing commits

> This note supersedes the prior `notes/bf-5l6ku.md` draft (2026-07-06), which
> concluded "no Forgejo credentials found, cannot access the API." That is now
> **stale**: the auth token is available locally via the `git credential` helper
> for `git.ardenone.com`. This run uses it to query live status, probe every
> available trigger endpoint, and actually fire a sync. The conclusion is the
> same one reached independently by bf-igsqf, bf-j1c40, bf-10182, and bf-8q6u3:
> **the sync fires but is rejected by GitHub on every attempt because of large
> blobs still present in git history.**

## Method

1. Resolve the Forgejo API token from the local git credential store.
2. Read live push-mirror status (`GET /push_mirrors`) and the single mirror
   (`GET /push_mirrors/{name}`).
3. Probe every documented sync trigger endpoint to find one that actually fires.
4. Trigger the sync via the only available mechanism (commit push, since the
   manual `/push` endpoint is absent in this Forgejo version — see below).
5. Re-read status to confirm the trigger fired and capture the result.

## API investigation (queried 2026-07-22, ~15:25Z)

Auth works — token resolved through `git credential fill` (length 40), matching
the method recorded in `[[forgejo-api-auth-via-git-credential]]`.

### Push-mirror config (confirmed correct, Forgejo → GitHub)

```
remote_address : https://github.com/jedarden/pdftract.git   ✓ correct direction
remote_name    : remote_mirror_nfF0JdlNzC
interval       : 10m0s              (auto-sync every 10 min)
sync_on_commit : true               ✓ fires on every push to origin
created        : 2026-05-16T19:51:17Z
last_update    : 2026-07-22T15:22:51Z   (recent — but a FAILED attempt)
last_error     : PushRejected (large files — see below)
```

### Trigger-endpoint probe — NONE of the manual endpoints exist/work

| Endpoint | Method | Result |
|----------|--------|--------|
| `/repos/.../push_mirrors/{name}` | GET | **200** — `{name}` identifier resolves correctly |
| `/repos/.../push_mirrors/{name}/push` | POST | **404 page not found** — manual push-trigger absent in this Forgejo version |
| `/repos/.../mirror-sync` | POST | **400 "Repository is not a mirror"** — pull-mirror-only endpoint; pdftract is a *push* mirror |

**Conclusion:** there is no API button to force a push-mirror sync on this
Forgejo instance. The **only** trigger is `sync_on_commit` (a push to origin)
plus the 10-minute interval. So the required commit/push for this bead *is* the
sync trigger.

## Current git state (queried 2026-07-22)

- `origin/main` tip: `ac89d8e0` (2026-07-22)
- GitHub last sync point: `88b4f0da` (2026-06-01)
- **Commits Forgejo is ahead of GitHub: 432** (the task brief's "84" is a stale
  underestimate; the real gap is 432 and still growing as commits accumulate)
- Large blobs **still reachable from `main`**:

  ```
  235.1 MB  --1.ppm              (GitHub hard limit: 100 MB)
   60.7 MB  test_parse_simple    (GitHub warn limit:  50 MB)
  ```

These were introduced at `1c6f26ec` and deleted from the tree at `007439e7`,
but git retains them in history, so GitHub's pre-receive hook rejects every
push that reaches back across them.

## Trigger + observation

Triggered via commit push to `origin` (sync_on_commit=true). Pre-push
`last_update` captured immediately before the push; status re-queried after.

### Post-trigger result (observed live, 2026-07-22 ~15:31Z)

Triggered via commit `f41440c6` pushed to `origin` (`sync_on_commit=true`).
Polled the push-mirror status before and after the push:

| When | `last_update` | `last_error` |
|------|---------------|--------------|
| Pre-push (captured immediately before `git push`) | `2026-07-22T15:28:26Z` | PushRejected |
| Post-push poll #2 (~15:31:57Z) | **`2026-07-22T15:31:45Z`** | **PushRejected** |

The `last_update` timestamp **advanced from 15:28:26Z to 15:31:45Z** as a direct
result of the push — confirming the trigger fired a fresh sync attempt. That
fresh attempt was **rejected by GitHub with the same `PushRejected` error**
(large blobs `--1.ppm` 235 MB + `test_parse_simple` 60 MB). GitHub is unchanged
at `88b4f0da`; origin/main is now at `f41440c6`, 433 commits ahead.

So: the mirror **does** sync on trigger; it just **cannot complete** because of
the history-embedded large files.

## Acceptance-criteria status

| Criterion | Result | Evidence |
|-----------|--------|----------|
| Mirror sync triggered successfully | **PASS** | Fired via sync_on_commit (no API `/push` endpoint exists); last_update advances on push |
| Sync completed without errors | **FAIL** | `last_error: PushRejected` on every attempt — large blobs (235 MB + 60 MB) still in history; GitHub 432 commits behind |
| Mirror last_update timestamp is current | **WARN** | Timestamp IS current (advances every ≤10 min), but marks *failed* attempts, not success |

## Conclusion — sync is BLOCKED, triggering does not help

The mirror is correctly configured, enabled, and firing automatically (every
10 min + on every commit). Triggering it — whether via the (nonexistent) API
endpoint or via a commit push — produces the same result every time: GitHub
rejects the push because `--1.ppm` (235 MB) and `test_parse_simple` (60 MB)
remain in git history. This is the **5th** bead in this workstream to reach
this same, definitive conclusion.

## Out-of-scope blocker (NOT remediated here)

Completing the sync requires removing the large blobs from history:

- `git filter-repo --strip-blobs-bigger-than 50M` (or BFG) across full history,
- then a **force-push to both `origin` (Forgejo) and `github`**, which rewrites
  every commit SHA from the rewrite point onward and affects every other clone.

This is:
1. Out of scope for a "trigger the sync" bead,
2. **Policy-constrained** — both `~/CLAUDE.md` and the project `CLAUDE.md`
   forbid force-push (`--force` / `--force-with-lease`),
3. Destructive and outward-facing — requires explicit human authorization
   before touching shared history on both remotes.

Recommended as a separate, human-aware remediation task. Until that is done,
**no amount of triggering will push the 432 missing commits to GitHub.**

## References

- Parent bead: bf-igsqf (synchronize GitHub to match Forgejo — same blocker)
- Depends on: bf-j1c40 (divergence baseline)
- Child bead: bf-10182 (mirror config — verified correct)
- Prior monitoring: bf-8q6u3 (authenticated-API monitor, same conclusion)
- Memory: `[[forgejo-api-auth-via-git-credential]]`, `[[pdftract-push-remote-is-origin]]`
