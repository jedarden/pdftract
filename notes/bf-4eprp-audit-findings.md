# Mirror and Remote Configuration Audit

**Audit Date:** 2026-07-06  
**Task ID:** bf-4eprp  
**Repository:** jedarden/pdftract

## Executive Summary

The audit reveals a **critical configuration issue**: the local git tracking branch is configured to follow `github/main` instead of `origin/main`, violating the workspace convention that `origin` should point to Forgejo (the primary repository). Additionally, the Forgejo-to-GitHub mirror is failing due to large test files exceeding GitHub's size limits.

## Findings

### 1. Forgejo Push Mirror Status

**Mirror Entry:** ✅ EXISTS
- **Remote Address:** `https://github.com/jedarden/pdftract.git`
- **Created:** 2026-05-16T19:51:17Z
- **Last Update Attempt:** 2026-07-06T21:13:08Z
- **Sync on Commit:** true
- **Interval:** 10m0s

**Mirror Status:** ❌ **BLOCKED - Push Rejected**
The mirror is failing consistently with the following error:
```
PushRejected Error: remote: error: File --1.ppm is 235.13 MB; this exceeds GitHub's file size limit of 100.00 MB
remote: warning: File test_parse_simple is 60.74 MB; this is larger than GitHub's recommended maximum file size of 50.00 MB
GH001: Large files detected. You may want to try Git Large File Storage
```

**Root Cause:** Large test fixture files in the repository exceed GitHub's 100MB file size limit.

### 2. Local Git Remote Configuration

**Remote Configuration:**
```
github  -> https://github.com/jedarden/pdftract.git (fetch/push)
origin  -> https://git.ardenone.com/jedarden/pdftract.git (fetch/push)
```

**Branch Tracking Configuration:**
```
branch.main.remote=github    ← ❌ INCORRECT (should be 'origin')
branch.main.merge=refs/heads/main
```

**Issue:** The `main` branch is configured to track `github/main` instead of `origin/main`. This violates the workspace convention that `origin` should point to Forgejo (the canonical source).

### 3. Commit Gap Analysis

**Current State:**
- `origin/main` (Forgejo): beef8453 (current)
- `main` (local): beef8453 (current)
- `github/main` (GitHub): 88b4f0da (308 commits behind)

**Commit Gap:** 308 commits difference between Forgejo and GitHub
- **Forgejo is ahead of GitHub** by 308 commits
- This is the reverse of what was initially stated (84 commits behind)
- The gap has grown since the mirror started failing on large files

**Recent commits that GitHub is missing (showing first 5 of 308):**
1. beef8453 - docs(bf-1j21w): verify assert_stderr_contains method
2. c29fb0cb - docs(bf-224fc): update verification note for forms_integration
3. b91e2b0c - docs(bf-1j21w): document assert_stderr_contents non-existence
4. 951dd56c - docs(bf-snis1): add verification note for forms_integration module
5. 862fe9b3 - feat(bf-4b7pm): implement temporary storage for benchmark metrics

### 4. Large Files Causing Mirror Failure

**Problematic Files:**
- `--1.ppm`: 235.13 MB (exceeds GitHub's 100MB hard limit)
- `test_parse_simple`: 60.74 MB (exceeds GitHub's 50MB recommended limit)

**Location:** These files appear to be test fixtures, likely in `tests/fixtures/` or `tests/fixtures/malformed/` directories.

## Issues Summary

| Issue | Severity | Status |
|-------|----------|--------|
| Branch tracking wrong remote | HIGH | NOT FIXED |
| Mirror blocked by large files | HIGH | BLOCKING SYNC |
| 308 commits not mirrored to GitHub | MEDIUM | BLOCKING |

## Recommendations

### Immediate Actions Required:

1. **Fix Branch Tracking Configuration:**
   ```bash
   git config --local branch.main.remote origin
   git config --local branch.main.merge refs/heads/main
   ```
   This will make `main` track `origin/main` (Forgejo) instead of `github/main`.

2. **Resolve Large File Issue:**
   - Investigate if `--1.ppm` and `test_parse_simple` can be removed or moved to LFS
   - Consider using Git LFS for large test fixtures
   - Alternatively, exclude these files from the repository and generate them programmatically

3. **Verify Mirror Recovery:**
   - After large file issue is resolved, manually trigger a mirror sync
   - Verify that the 308 pending commits successfully push to GitHub

### Follow-up Actions:

1. **Establish File Size Policy:** Create CI checks to prevent files >50MB from being committed
2. **Git LFS Migration:** Consider migrating large test fixtures to Git LFS
3. **Documentation:** Update workspace documentation to specify branch tracking requirements

## Technical Details

### Forgejo Mirror Configuration
- **API Endpoint:** https://git.ardenone.com/api/v1/repos/jedarden/pdftract/push_mirrors
- **Mirror Type:** push (Forgejo → GitHub)
- **Sync Frequency:** Every 10 minutes + on commit
- **Authentication:** Token-based (Forgejo API token)

### Git Configuration Files
- **Local Config:** `/home/coding/pdftract/.git/config`
- **Branch Settings:** `branch.main.remote=github` (needs correction)
- **Remote Settings:** Properly configured (origin=Forgejo, github=GitHub)

### Commit History Analysis
- **Last successful mirror push:** Unknown (mirror has been failing)
- **First problematic commit:** Around the time large files were added
- **Oldest missing commit on GitHub:** 1c6f26ec (fix: clean up unused imports in hash.rs)

## Next Steps for Parent Bead (bf-320gz)

The following child beads should address:
1. Fix branch tracking configuration
2. Resolve large file blocking mirror sync
3. Verify and test mirror recovery
4. Update documentation and CI policies

This audit provides all necessary context for implementing these fixes.
