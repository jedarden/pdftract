# Git Mirroring Setup and Troubleshooting

**Date:** 2026-08-12 (updated)  
**Purpose:** Comprehensive documentation of git mirroring setup, workspace conventions, and resolution processes for the pdftract repository.

## Current Status (2026-08-13)

✅ **Mirror is operational and in sync**

- **Forgejo main:** `098217975847bcdfe76e1613b6f418075c45633e`
- **GitHub main:** `098217975847bcdfe76e1613b6f418075c45633e`
- **Commits behind:** 0
- **Last sync:** 2026-08-13T09:12:13Z
- **Mirror status:** No errors

The previous 84-160 commit divergence has been fully resolved. The Forgejo push mirror is working correctly with `sync_on_commit: true`.

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

## Manual Sync Triggering

The Forgejo push mirror normally syncs automatically every 10 minutes and on every commit (`sync_on_commit: true`). However, you may need to trigger a manual sync in certain situations:

### When to Manually Sync

- After resolving large file blockers that prevented automatic sync
- After fixing authentication issues
- After recovering from a mirror configuration issue
- When you need to urgently propagate changes to GitHub

### Method 1: Via Forgejo Web UI (Recommended)

1. Navigate to: `https://git.ardenone.com/jedarden/pdftract/settings/mirrors`
2. Find the push mirror entry for GitHub
3. Click the "Sync Now" button next to the mirror
4. Monitor the "Last Sync" timestamp to confirm completion

### Method 2: Via Forgejo API

```bash
# Trigger a manual sync via API push mirror endpoint
FORGEJO_TOKEN="$(git credential fill <<< 'protocol=https
host=git.ardenone.com
' | grep password | cut -d= -f2)"

# Get the mirror ID first
MIRROR_ID="$(curl -s -H "Authorization: token $FORGEJO_TOKEN" \
  https://git.ardenone.com/api/v1/repos/jedarden/pdftract/push_mirrors | \
  python3 -c "import sys, json; print(json.load(sys.stdin)[0]['remote_name'])")"

# Trigger sync (note: API may not have explicit sync endpoint - use Web UI)
echo "Mirror ID: $MIRROR_ID"
echo "Please use Web UI to sync: https://git.ardenone.com/jedarden/pdftract/settings/mirrors"
```

⚠️ **Note:** The Forgejo API may not expose an explicit "sync now" endpoint. The Web UI method is the most reliable way to trigger manual syncs.

### Method 3: Direct Push (Recovery Only)

If the mirror is completely broken and you need to urgently sync to GitHub:

```bash
# Fetch both remotes first
git fetch origin
git fetch github

# Push to GitHub manually (use only for recovery)
git push github origin/main:main --force-with-lease
```

⚠️ **Warning:** Only use this method when the mirror is genuinely broken. Normal operation should rely on the Forgejo push mirror.

### Verifying Manual Sync Success

After triggering a manual sync, verify it succeeded:

```bash
# Wait 10-20 seconds for sync to complete
sleep 20

# Check if GitHub is now in sync
git fetch github
git rev-parse origin/main github/main

# Both SHAs should match
FORGEJO_SHA=$(git rev-parse origin/main)
GITHUB_SHA=$(git rev-parse github/main)

if [ "$FORGEJO_SHA" = "$GITHUB_SHA" ]; then
    echo "✓ Manual sync successful"
else
    echo "✗ Manual sync failed - check mirror logs"
fi
```

---

## API Reference

### Forgejo Push Mirrors API

#### Get All Push Mirrors

```bash
GET /api/v1/repos/{owner}/{repo}/push_mirrors
```

**Example:**
```bash
FORGEJO_TOKEN="$(git credential fill <<< 'protocol=https
host=git.ardenone.com
' | grep password | cut -d= -f2)"

curl -s -X GET "https://git.ardenone.com/api/v1/repos/jedarden/pdftract/push_mirrors" \
  -H "Authorization: token $FORGEJO_TOKEN" \
  -H "Content-Type: application/json"
```

**Response:**
```json
[
  {
    "repo_name": "pdftract",
    "remote_name": "remote_mirror_nfF0JdlNzC",
    "remote_address": "https://github.com/jedarden/pdftract.git",
    "created": "2026-05-16T19:51:17Z",
    "last_update": "2026-08-13T09:12:13Z",
    "last_error": "",
    "interval": "10m0s",
    "sync_on_commit": true
  }
]
```

#### Get Specific Push Mirror

```bash
GET /api/v1/repos/{owner}/{repo}/push_mirrors/{mirror}
```

**Example:**
```bash
curl -s -X GET "https://git.ardenone.com/api/v1/repos/jedarden/pdftract/push_mirrors/remote_mirror_nfF0JdlNzC" \
  -H "Authorization: token $FORGEJO_TOKEN"
```

#### Create Push Mirror

```bash
POST /api/v1/repos/{owner}/{repo}/push_mirrors
```

**Example:**
```bash
curl -s -X POST "https://git.ardenone.com/api/v1/repos/jedarden/pdftract/push_mirrors" \
  -H "Authorization: token $FORGEJO_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "remote_address": "https://github.com/jedarden/pdftract.git",
    "interval": "10m0s",
    "sync_on_commit": true
  }'
```

#### Update Push Mirror

```bash
PATCH /api/v1/repos/{owner}/{repo}/push_mirrors/{mirror}
```

**Example:**
```bash
curl -s -X PATCH "https://git.ardenone.com/api/v1/repos/jedarden/pdftract/push_mirrors/remote_mirror_nfF0JdlNzC" \
  -H "Authorization: token $FORGEJO_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "interval": "5m0s",
    "sync_on_commit": true
  }'
```

#### Delete Push Mirror

```bash
DELETE /api/v1/repos/{owner}/{repo}/push_mirrors/{mirror}
```

**Example:**
```bash
curl -s -X DELETE "https://git.ardenone.com/api/v1/repos/jedarden/pdftract/push_mirrors/remote_mirror_nfF0JdlNzC" \
  -H "Authorization: token $FORGEJO_TOKEN"
```

### GitHub Repository API

#### Get Repository Information

```bash
GET /api/v3/repos/{owner}/{repo}
```

**Example:**
```bash
GH_TOKEN="$(gh auth token)"
curl -s -X GET "https://api.github.com/repos/jedarden/pdftract" \
  -H "Authorization: token $GH_TOKEN" \
  -H "Accept: application/vnd.github.v3+json"
```

### Git Credential Helper

The workspace uses git credential helpers for authentication:

```bash
# Get Forgejo token
git credential fill <<< 'protocol=https
host=git.ardenone.com
' | grep password | cut -d= -f2

# Get GitHub token  
gh auth token
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
| 2026-08-12 | **Divergence resolved** | GitHub synced to Forgejo via push mirror (both at `e0122612`) |

### Resolution (2026-08-12)

The 84-160 commit divergence was automatically resolved by the Forgejo push mirror. Investigation showed:

1. **Push mirror operational**: Mirror `remote_mirror_nfF0JdlNzC` is configured correctly with `sync_on_commit: true`
2. **No current errors**: Mirror `last_error` field is empty
3. **Automatic sync**: After fetching GitHub main, both repositories are at the same commit (`e0122612`)

The large file issue (`--1.ppm`, `test_parse_simple`) that previously blocked sync was resolved in commit `007439e7` ("remove tracked debug/scratch artifacts"), which removed these files from the tree. Once the blocking objects were removed from the recent history, the mirror successfully synced all pending commits.

**Current remote configuration (correct):**
```bash
origin  https://git.ardenone.com/jedarden/pdftract.git (fetch/push)  # Primary
github  https://github.com/jedarden/pdftract.git (fetch/push)         # Mirror
```

**Verification commands:**
```bash
# Check mirror status via API
FORGEJO_TOKEN="$(git credential fill <<< 'protocol=https
host=git.ardenone.com
' | grep password | cut -d= -f2)"
curl -s -X GET "https://git.ardenone.com/api/v1/repos/jedarden/pdftract/push_mirrors" \
  -H "Authorization: token $FORGEJO_TOKEN"

# Verify sync locally
git fetch github && git rev-parse origin/main github/main
```

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

### Final Commit Range (Pre-Resolution)

- **Forgejo/main**: `02bfffef` docs(bf-1o0la): document Forgejo push mirror configuration and status
- **GitHub/main**: `88b4f0da` fix(pdftract-2rc4): fix CI schema gate script and add verification note
- **Commits behind**: 160 (updated from initial 84 estimate during investigation)
- **Merge-base**: `88b4f0da276c7257ade02d3cecfaeb09f7881acc`

### Final Commit Range (Post-Resolution 2026-08-12)

- **Forgejo/main**: `e0122612d0324b870762addea63cfd3482a9baaa` feat(bf-59i1z7): create glyph dict mock with basic properties
- **GitHub/main**: `e0122612d0324b870762addea63cfd3482a9baaa` (same commit)
- **Commits behind**: 0
- **Status**: ✅ Synced

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
**Last Updated:** 2026-08-12 (mirror verified operational)  
**Maintainer:** jedarden  
**Version:** 1.1
