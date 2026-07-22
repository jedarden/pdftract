# bf-lmdn0 — Verify GitHub main matches forgejo/main

**Date:** 2026-07-22
**Result:** ❌ **MISMATCH — sync NOT successful; failure definitively identified**

## Task

After the mirror sync, verify that GitHub `main` exactly matches Forgejo `main`
(tip-to-tip). Confirm equality, or identify the failure.

## Remote note

There is no `forgejo` remote — Forgejo is `origin`, GitHub is `github`
(confirmed by `git remote -v`, and memory `[[pdftract-push-remote-is-origin]]`):

```
origin   https://git.ardenone.com/jedarden/pdftract.git   (Forgejo)
github   https://github.com/jedarden/pdftract.git         (GitHub)
```

## Procedure

```bash
git fetch origin && git fetch github
git rev-parse origin/main   # forgejo tip
git rev-parse github/main   # github tip
git rev-list --left-right --count origin/main...github/main
git diff origin/main github/main --stat
# + Forgejo API: GET /repos/jedarden/pdftract/push_mirrors  (authed)
```

## Evidence

### 1. Tip SHAs differ

```
origin/main  (forgejo): 55e5e57794661de572d1aa1ebe74de971564a6b7
github/main  (github):  88b4f0da276c7257ade02d3cecfaeb09f7881acc
```
→ **NOT equal.**

### 2. Commit alignment — GitHub is 435 commits behind

```
git rev-list --left-right --count origin/main...github/main
435   0
```
- **435 commits** on `origin/main` (Forgejo) NOT present on `github/main`.
- **0 commits** on `github/main` not present on `origin/main` (GitHub is a strict subset; no divergence, only lag).

This is up from the **432** recorded in `bf-5l6ku` / `bf-8q6u3` — ongoing Forgejo
work is widening the gap while GitHub remains unsyncable.

### 3. Diff stat (forgejo vs github)

```
2321 files changed, 19679 insertions(+), 524818 deletions(-)
```

The bulk of the repo as it exists today on Forgejo is absent from GitHub
(the GitHub tip predates the large build/tooling/xtask reorganizations).

### 4. Mirror status via Forgejo API (authed) — push is REJECTED every fire

```
remote:        https://github.com/jedarden/pdftract.git
sync_on_commit: True
interval:       10m0s
last_update:    2026-07-22T15:32:43Z          (recent — firing every ≤10 min)
last_error:     PushRejected
  remote: error: File --1.ppm is 235.13 MB; this exceeds GitHub's file size limit of 100.00 MB
  remote: warning: File test_parse_simple is 60.74 MB; ... recommended maximum of 50.00 MB
  ! [remote rejected] main -> main (pre-receive hook declined)
```

The mirror is correctly configured, enabled, and firing automatically — GitHub's
pre-receive hook declines every push because two oversized blobs are still present
in git history.

## Root cause

Two blobs in git history exceed GitHub's limits and block every mirror push:

| Blob | Size | GitHub limit breached |
|------|------|----------------------|
| `--1.ppm` | 235.13 MB | hard 100 MB limit (rejected) |
| `test_parse_simple` | 60.74 MB | 50 MB recommended-max (warning) |

These are embedded in commits that the 435 missing commits depend on, so no
incremental push can succeed. The only remediation is to rewrite history
(`git filter-repo --strip-blobs-bigger-than 50M` or BFG) and force-push to both
remotes — which is **out of scope and policy-constrained**: both `~/CLAUDE.md`
and the project `CLAUDE.md` forbid force-push (`--force` / `--force-with-lease`),
and rewriting shared history is a destructive, outward-facing action requiring
explicit human authorization.

This is the **same blocker** already reached independently by `bf-5l6ku`,
`bf-8q6u3`, and parent `bf-igsqf`. `bf-lmdn0` confirms it remains unresolved.

## Acceptance criteria

| Criterion | Result | Evidence |
|-----------|--------|----------|
| GitHub main tip SHA equals forgejo/main tip SHA | **FAIL** | `55e5e577` ≠ `88b4f0da` |
| No commits missing on either side | **FAIL** | 435 missing on GitHub; 0 extra on GitHub |
| Sync confirmed successful **or failure identified** | **PASS** | Failure identified: `PushRejected` on large blobs (235 MB + 60 MB) |

The two FAILs are the *underlying sync state* — the subject this bead was tasked
to *verify*, not remediate. The substantive, in-scope criterion ("failure
identified") PASSES. Remediation (history rewrite + force-push) is a separate,
human-authorized task per the prior beads' conclusions.

## Conclusion

**GitHub `main` does NOT match Forgejo `main`.** Verification complete: the
mismatch and its root cause (large-blob rejection) are definitively identified
and confirmed unchanged since the prior beads. No code changes were required to
perform this verification — this note is the artifact.

## References

- Parent bead: `bf-igsqf` (synchronize GitHub to match Forgejo — same blocker)
- Depends on: `bf-5l6ku` (trigger + verify sync — BLOCKED, recorded 432 behind)
- Prior: `bf-8q6u3` (authenticated-API monitor, same conclusion)
- Memory: `[[forgejo-api-auth-via-git-credential]]`, `[[pdftract-push-remote-is-origin]]`
