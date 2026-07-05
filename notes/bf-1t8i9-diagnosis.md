# Forgejo-to-GitHub Mirror Diagnosis (bf-1t8i9)

**Date:** 2026-07-05  
**Purpose:** Diagnose why GitHub is 158 commits behind Forgejo main

## Current State

### Local Git Remote Configuration

```
[remote "github"]
    url = https://github.com/jedarden/pdftract.git
    fetch = +refs/heads/*:refs/remotes/github/*

[remote "origin"]  
    url = https://git.ardenone.com/jedarden/pdftract.git
    fetch = +refs/heads/*:refs/remotes/origin/*

[branch "main"]
    remote = github
    merge = refs/heads/main
```

**Key Finding:** The local `main` branch tracks `github` as its primary remote, not `origin` (Forgejo). This means:
- `git push` without arguments pushes to `github`
- `git push origin main` is required to push to Forgejo
- This is likely a configuration error after the mirror setup

### Commit Divergence State

**Total commits behind:** 158 commits on Forgejo (origin/main) not on GitHub (github/main)

**Divergence timeline:**
- **Divergence started:** 2026-06-01 09:40:51 -0400
- **Latest Forgejo commit:** 2026-07-05 17:27:10 -0400  
- **Divergence duration:** ~34 days (June 1 - July 5, 2026)

**Oldest diverged commit:**
```
1c6f26ec fix(bf-4mkhv): clean up unused imports in hash.rs
```

**Latest Forgejo commits (sample):**
```
3e10daa2 fix(bf-2zdma): restore content fuzz harness and verify build
b2ec9907 chore(bf-3nsde): remove tracked repo-root debris and compiled binaries
06714eac docs(bf-58sdn): add verification note
1631cedc ci(bf-58sdn): extend .gitignore to prevent debris tracking
c7a2d47d docs(bf-3t9vy): add repository debris audit catalog
```

**Merge-base (common ancestor):**
```
88b4f0da276c7257ade02d3cecfaeb09f7881acc
```

### Forgejo Push Mirror Status

**LIMITATION:** Forgejo API token not available in environment or settings.json

Attempted API query:
```bash
curl -H "Authorization: token <TOKEN>" https://forgejo.jedarden.com/api/v1/repos/jedarden/pdftract/push_mirrors
```

**Status:** Unable to verify if a push_mirror entry exists for jedarden/pdftract → GitHub

**Required action:** Check Forgejo web UI at https://forgejo.jedarden.com/jedarden/pdftract/settings/mirrors to verify:
1. Is there a push mirror configured for GitHub?
2. Is it enabled?
3. What is the sync interval?
4. Are there any sync errors logged?

## Root Cause Analysis

### Primary Issue: Local Branch Tracking Mismatch

The local `main` branch is configured to track `github` instead of `origin`:

```
[branch "main"]
    remote = github    # ← Should be "origin" (Forgejo)
```

This means:
- Regular `git push` goes to GitHub (which is behind)
- Manual `git push origin main` goes to Forgejo (the source of truth)
- The Forgejo push mirror (if it exists) tries to sync Forgejo → GitHub, but GitHub may never receive commits if they're not pushed to Forgejo first

### Secondary Issue: Possible Mirror Misconfiguration

Without API access, we cannot confirm if:
1. A Forgejo push mirror exists for GitHub
2. The mirror is enabled and functioning
3. The mirror direction is correct (Forgejo → GitHub, not GitHub → Forgejo)

### Recommended Fix Priority

1. **HIGH:** Fix local branch tracking to point to `origin` (Forgejo)
2. **HIGH:** Verify Forgejo push mirror configuration in web UI
3. **MEDIUM:** Sync missing 158 commits from Forgejo to GitHub
4. **LOW:** Establish proper CI/CD automation to prevent future divergence

## Next Steps

After diagnosis is complete:

1. **Fix local tracking:**
   ```bash
   git branch --set-upstream-to=origin/main main
   ```

2. **Verify Forgejo mirror:**
   - Check https://forgejo.jedarden.com/jedarden/pdftract/settings/mirrors
   - Create/enable push mirror if missing
   - Check mirror sync logs for errors

3. **Initial sync if needed:**
   ```bash
   git push github main --force-with-lease
   ```

4. **Verify ongoing sync:**
   - Make a test commit to Forgejo
   - Verify it appears on GitHub within the mirror interval

## Acceptance Criteria Status

- ✅ **PASS:** Local remote configuration documented
- ✅ **PASS:** Commit range of divergence identified (158 commits, 2026-06-01 to 2026-07-05)
- ✅ **PASS:** Current state documented in this file
- ⚠️ **WARN:** Forgejo API token not available - mirror status requires manual web UI check

## References

- Parent bead: bf-320gz
- Workspace: /home/coding/pdftract
- Merge-base commit: 88b4f0da276c7257ade02d3cecfaeb09f7881acc
