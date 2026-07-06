# Bead bf-igsqf: Synchronize GitHub to match Forgejo

## Investigation Summary

### Current State
- **Forgejo main**: `fc88f570` (latest)
- **GitHub main**: `88b4f0da` (309 commits behind)
- GitHub has 0 divergent commits (Forgejo is purely ahead)

### Blocker Discovered: Large Files in Git History

The push to GitHub is **blocked by large files** in commit history:

| File | Size | Status |
|------|------|--------|
| `--1.ppm` | 235.13 MB | Removed in commit 007439e7 but exists in history |
| `test_parse_simple` | 60.74 MB | Removed in commit 007439e7 but exists in history |

GitHub's pre-receive hook rejects any push containing files >100MB in the entire history, even if those files were later deleted.

### Error Output
```
remote: error: File --1.ppm is 235.13 MB; this exceeds GitHub's file size limit of 100.00 MB
remote: error: GH001: Large files detected. You may want to try Git Large File Storage
remote: error: File test_parse_simple is 60.74 MB; larger than recommended 50.00 MB
remote: error: Trace: f338a08cfb9744b571b78169e74d313ba22fced9e24079fc169da8702d9b41b4
To https://github.com/jedarden/pdftract.git
 ! [remote rejected]   main -> main (pre-receive hook declined)
```

### Root Cause
The files `--1.ppm` (235MB) and `test_parse_simple` (60MB) were committed to the repository (likely as test fixtures) and later removed in commit `007439e7 chore(bf-a8031): remove tracked debug/scratch artifacts and compiled binaries`. However, git history retains all objects, so GitHub still sees them when pushing.

### Parent Mirror Configuration (bf-10182)
The parent bead `bf-10182` was about configuring a Forgejo server-side push mirror to GitHub. Without API access, I cannot verify if this mirror is configured and active. If the mirror is properly configured on Forgejo, it may handle large file filtering differently than direct git push.

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| GitHub main equals forgejo/main | **FAIL** | Blocked by large files in git history |
| Any divergence reconciled with merge commit | N/A | No divergent commits (GitHub is purely behind, no fork) |
| No force-push performed | **PASS** | Attempted regular fast-forward push |
| Reconciliation documented | **PASS** | This note |

## Resolution Options

To complete the synchronization, one of these approaches must be taken:

### Option 1: Git History Rewrite (Recommended)
Use `git filter-repo` or BFG Repo-Cleaner to remove large files from git history:

```bash
# Install git-filter-repo
pip install git-filter-repo

# Remove the large files from history
git filter-repo --invert-paths --path --1.ppm --path test_parse_simple

# Force push to GitHub (required after history rewrite)
git push github main --force
```

**Trade-off**: This rewrites commit hashes from the rewrite point onward, which would require re-syncing Forgejo's mirror configuration.

### Option 2: Configure Git LFS Going Forward
Add large files to Git LFS and clean up history:

1. Set up `.gitattributes`:
   ```
   *.ppm filter=lfs diff=lfs merge=lfs -text
   test_parse_simple filter=lfs diff=lfs merge=lfs -text
   ```

2. Migrate existing large files (if any still exist)

3. Rewrite history to move LFS objects

### Option 3: Forgejo Server-Side Mirror with LFS Filtering
If the Forgejo push mirror (from bf-10182) can be configured to handle LFS or filter large files, this would be the cleanest solution. However, without API access, I cannot verify or configure this.

## WARN Items

1. **GitHub remains 309 commits behind Forgejo** due to large file blocker
2. **No force-push was performed** (per workspace convention)
3. **Git history cleanup required** before synchronization can complete
4. **Cannot verify Forgejo push mirror status** without API access token

## Recommendation

Create a follow-up bead to:
1. Rewrite git history using `git filter-repo` to remove large files
2. Update Forgejo push mirror configuration if needed
3. Re-attempt synchronization after cleanup

---

**Verification performed**: 2026-07-06
**Bead ID**: bf-igsqf
**Status**: PARTIAL (blocked by infrastructure constraint - large files in git history)
