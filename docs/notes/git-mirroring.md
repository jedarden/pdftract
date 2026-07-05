# Git Mirroring Setup and Troubleshooting

**Date:** 2026-07-05  
**Purpose:** Comprehensive documentation of git mirroring setup, workspace conventions, and resolution processes for the pdftract repository.

## Table of Contents

- [Workspace Convention](#workspace-convention)
- [Forgejo Push Mirror Configuration](#forgejo-push-mirror-configuration)
- [Verifying Mirror Status](#verifying-mirror-status)
- [Troubleshooting Common Issues](#troubleshooting-common-issues)
- [Incident History: 84-Commit Divergence](#incident-history-84-commit-divergence)
- [References and Related Documentation](#references-and-related-documentation)

---

## Workspace Convention

### Primary vs. Mirror Remotes

The pdftract repository follows a **primary-mirror** git workflow:

| Remote | URL | Role | Usage |
|--------|-----|------|-------|
| `origin` | `https://git.ardenone.com/jedarden/pdftract.git` | **Primary** (Forgejo) | All pushes go here first |
| `github` | `https://github.com/jedarden/pdftract.git` | **Mirror** (GitHub) | Automatic sync from Forgejo |

### Key Principles

1. **Forgejo (`origin`) is the source of truth**
   - All commits must be pushed to Forgejo first
   - CI/CD runs on Forgejo via Argo Workflows
   - GitHub is kept in sync automatically

2. **GitHub is a read-only mirror**
   - Used for GitHub Issues, PRs, and visibility
   - Never push directly to GitHub unless recovering from a sync issue
   - The Forgejo push mirror keeps GitHub up-to-date

3. **Branch tracking must point to `origin`**
   ```bash
   # Correct configuration
   git branch --set-upstream-to=origin/main main
   
   # Verify with:
   git branch -vv
   # Should show: main -> origin/main
   ```

### Verification Commands

```bash
# Check remote configuration
git remote -v

# Check branch tracking
git branch -vv

# Verify remote fetch URLs
git config --get remote.origin.url
git config --get remote.github.url
```

---

## Forgejo Push Mirror Configuration

### Mirror Setup Details

The Forgejo-to-GitHub push mirror is configured in the Forgejo web UI or API:

| Setting | Value | Purpose |
|---------|-------|---------|
| **Remote Address** | `https://github.com/jedarden/pdftract.git` | GitHub mirror target |
| **Sync Interval** | `10m0s` | Sync every 10 minutes |
| **Sync on Commit** | `true` | Sync immediately after each push |
| **Created** | `2026-05-16T19:51:17Z` | Mirror creation timestamp |

### API Response Example

```json
{
  "repo_name": "pdftract",
  "remote_name": "remote_mirror_nfF0JdlNzC",
  "remote_address": "https://github.com/jedarden/pdftract.git",
  "created": "2026-05-16T19:51:17Z",
  "last_update": "2026-07-05T21:59:42Z",
  "interval": "10m0s",
  "sync_on_commit": true,
  "last_error": "PushRejected Error: large files detected"
}
```

### How to Check Mirror Status

#### Via Forgejo Web UI

1. Navigate to: `https://git.ardenone.com/jedarden/pdftract/settings/mirrors`
2. Verify the push mirror to GitHub exists
3. Check the "Last Sync" timestamp
4. Review "Last Error" field for sync failures

#### Via Forgejo API

```bash
# Requires Forgejo API token
curl -H "Authorization: token <FORGEJO_TOKEN>" \
  https://git.ardenone.com/api/v1/repos/jedarden/pdftract/push_mirrors
```

Expected response:
- Mirror entry for GitHub should exist
- `sync_on_commit` should be `true`
- `interval` should be `10m0s`
- Check `last_error` for any sync failures

---

## Verifying Mirror Status

### Quick Health Check

```bash
# 1. Check if Forgejo and GitHub main branches match
FORGEJO_SHA=$(git ls-remote git.ardenone.com:jedarden/pdftract.git refs/heads/main | awk '{print $1}')
GITHUB_SHA=$(git ls-remote github.com:jedarden/pdftract.git refs/heads/main | awk '{print $1}')

if [ "$FORGEJO_SHA" = "$GITHUB_SHA" ]; then
    echo "✓ Mirror is in sync"
else
    echo "✗ Mirror is out of sync"
    echo "Forgejo: $FORGEJO_SHA"
    echo "GitHub: $GITHUB_SHA"
fi
```

### Count Commits Behind

```bash
# Fetch latest from both remotes
git fetch origin
git fetch github

# Count commits behind GitHub
git log --oneline origin/main ^github/main | wc -l
```

### Find Divergence Point

```bash
# Find merge-base (common ancestor)
git merge-base origin/main github/main

# Show when divergence started
git log --oneline --date-order origin/main ^github/main | head -1
```

---

## Troubleshooting Common Issues

### Issue 1: Branch Tracking Wrong Remote

**Symptoms:**
- `git push` goes to GitHub instead of Forgejo
- Commits appear on GitHub but not Forgejo
- Mirror shows old commits

**Diagnosis:**
```bash
git branch -vv
# If main tracks github instead of origin:
# main -> github/main
```

**Fix:**
```bash
git branch --set-upstream-to=origin/main main
git branch -vv  # Verify: main -> origin/main
```

**Prevention:**
- Always use `git push origin main` for explicit pushes
- Never use bare `git push` without verifying remote

### Issue 2: Mirror Sync Failing Due to Large Files

**Symptoms:**
- Forgejo has newer commits than GitHub
- Mirror `last_error` shows "large files detected"
- Push to GitHub rejected with `GH001: Large files detected`

**Diagnosis:**
```bash
# Find large files in history
git rev-list --objects --all |
  git cat-file --batch-check='%(objecttype) %(objectname) %(objectsize) %(rest)' |
  awk '/^blob/ {print substr($0,6)}' |
  sort -n -k2 |
  tail -20
```

**Fix Options:**

#### Option 1: Git LFS (Recommended)
```bash
# Install git-lfs
sudo apt-get install git-lfs  # Debian/Ubuntu
# or
brew install git-lfs  # macOS

# Initialize LFS
git lfs install
git lfs track "*.ppm" "*.pdf" "*.bin" "*.psd"
git add .gitattributes
git commit -m "chore: enable git-lfs for large files"
git push origin main
```

#### Option 2: Remove Large Files from History
```bash
# Use git-filter-repo (preferred over filter-branch)
pip install git-filter-repo
git filter-repo --path --1.ppm --invert-paths
git push origin main --force-with-lease
```

⚠️ **Warning:** History rewrites change commit SHAs and require force-pushes.

### Issue 3: Mirror Disabled or Missing

**Symptoms:**
- GitHub never updates despite Forgejo commits
- No mirror entry in Forgejo settings
- `last_update` timestamp is very old

**Diagnosis:**
```bash
# Check via API
curl -H "Authorization: token <TOKEN>" \
  https://git.ardenone.com/api/v1/repos/jedarden/pdftract/push_mirrors
```

**Fix:**
1. Navigate to Forgejo repository settings
2. Go to "Mirrors" section
3. Add new push mirror: `https://github.com/jedarden/pdftract.git`
4. Enable "Sync on commit"
5. Set interval to `10m`

### Issue 4: Authentication Issues

**Symptoms:**
- Mirror fails with authentication errors
- `last_error` shows "permission denied"

**Fix:**
1. Verify GitHub credentials in Forgejo mirror settings
2. Use GitHub personal access token with `repo` scope
3. Update mirror credentials in Forgejo web UI

---

## Incident History: 84-Commit Divergence

### Timeline

| Date | Event | Details |
|------|-------|---------|
| 2026-06-01 | Divergence started | Commit `1c6f26ec` was first to diverge |
| 2026-07-05 | Issue discovered | GitHub found to be 160 commits behind Forgejo |
| 2026-07-05 | Root cause identified | Local branch tracked `github` instead of `origin` |
| 2026-07-05 | Mirror config verified | Push mirror exists but blocked by large files |
| 2026-07-05 | Remotes fixed | Branch tracking corrected to `origin/main` |
| 2026-07-05 | Documentation created | This comprehensive guide |

### Root Cause Analysis

The divergence had **two compounding issues**:

1. **Local branch tracking misconfiguration**
   - Local `main` branch tracked `github` instead of `origin`
   - Regular `git push` went to GitHub (behind) instead of Forgejo (source of truth)
   - Manual `git push origin main` required for Forgejo updates

2. **Mirror blocked by large files**
   - Forgejo push mirror configured correctly
   - Sync failing due to `--1.ppm` (235.13 MB) and `test_parse_simple` (60.74 MB)
   - GitHub pre-receive hook rejected all sync attempts

### Blocker Details

**Large files in git history:**
- `--1.ppm`: 235.13 MB (exceeds GitHub's 100 MB hard limit)
- `test_parse_simple`: 60.74 MB (exceeds GitHub's 50 MB recommended limit)

Both files were added and removed in the sync range:
- Added in: `1c6f26ec` fix(bf-4mkhv): clean up unused imports in hash.rs
- Removed in: `007439e7` chore(bf-8031): remove tracked debug/scratch artifacts

**GitHub's behavior:** Even though the files were removed from the working tree, they still exist in git's object database. GitHub checks all objects being pushed, including historical ones.

### Resolution Attempt

Attempted to sync GitHub main to Forgejo main:
```bash
git fetch github
git push github main
```

**Result:** Rejected by GitHub pre-receive hook:
```
remote: error: File --1.ppm is 235.13 MB; this exceeds GitHub's file size limit of 100.00 MB
remote: error: GH001: Large files detected.
```

### Recommended Resolution Path

1. **Set up Git LFS** on both repositories
2. **Migrate large files** to LFS tracking
3. **Push to Forgejo** to propagate LFS pointers
4. **Verify mirror sync** succeeds on next push
5. **Confirm GitHub** matches Forgejo

### Final Commit Range

- **Forgejo/main**: `02bfffef` docs(bf-1o0la): document Forgejo push mirror configuration and status
- **GitHub/main**: `88b4f0da` fix(pdftract-2rc4): fix CI schema gate script and add verification note
- **Commits behind**: 160 (updated from initial 84 estimate during investigation)
- **Merge-base**: `88b4f0da276c7257ade02d3cecfaeb09f7881acc`

---

## References and Related Documentation

### Child Bead Documentation

This comprehensive document synthesizes findings from four child beads:

1. **[Diagnosis (bf-1t8i9)](notes/bf-1t8i9-diagnosis.md)**
   - Initial divergence discovery and state assessment
   - Identified local branch tracking misconfiguration
   - Documented 158-commit divergence (updated to 160 during investigation)

2. **[Mirror Configuration (bf-1o0la)](notes/bf-1o0la-mirror-config.md)**
   - Forgejo push mirror API verification
   - Confirmed mirror exists and is configured correctly
   - Identified large file blocker preventing sync

3. **[Sync Verification (bf-682zv)](notes/bf-682zv.md)**
   - Attempted GitHub sync from Forgejo
   - Documented large file rejection by GitHub
   - Recommended Git LFS resolution path

4. **[Remotes Fixed (bf-67zia)](notes/bf-67zia-remotes-fixed.md)**
   - Corrected local branch tracking to `origin/main`
   - Verified remote configuration follows workspace convention
   - Confirmed both remotes are accessible

### Parent Bead

- **[Genesis: Git Mirror Setup and Resolution (bf-320gz)]** - Parent coordination bead for this work

### External References

- **Forgejo API Documentation**: https://forgejo.jedarden.com/api/v1/
- **GitHub Large File Storage**: https://git-lfs.github.com/
- **GitHub File Size Limits**: https://docs.github.com/en/repositories/working-with-files/using-files/large-files

---

## Quick Reference Commands

### Daily Workflow

```bash
# 1. Pull latest from Forgejo
git pull origin main

# 2. Do your work
git commit -m "feat: something"

# 3. Push to Forgejo (source of truth)
git push origin main

# 4. Verify mirror status (optional)
# Check GitHub web UI or use API:
curl -H "Authorization: token <TOKEN>" \
  https://git.ardenone.com/api/v1/repos/jedarden/pdftract/push_mirrors
```

### Recovery Commands

```bash
# If GitHub gets out of sync:
git fetch github
git push github origin/main:main --force-with-lease

# If branch tracking gets messed up:
git branch --set-upstream-to=origin/main main

# If mirror is disabled or missing:
# Check Forgejo web UI: https://git.ardenone.com/jedarden/pdftract/settings/mirrors
```

---

**Document Status:** ✅ Complete  
**Last Updated:** 2026-07-05  
**Maintainer:** jedarden  
**Version:** 1.0
