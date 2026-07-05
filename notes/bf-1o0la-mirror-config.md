# Forgejo Push Mirror Configuration (bf-1o0la)

**Date:** 2026-07-05  
**Purpose:** Configure and verify Forgejo push mirror to GitHub

## Current Mirror Configuration

### Mirror Entry Details

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

### Configuration Status

| Setting | Value | Status |
|---------|-------|--------|
| Remote Address | `https://github.com/jedarden/pdftract.git` | ✓ PASS |
| Sync on Commit | `true` | ✓ PASS |
| Sync Interval | `10m0s` | ✓ PASS |
| Mirror Active | Yes | ✓ PASS |
| Last Sync | 2026-07-05 21:59:42 UTC | ✓ PASS (today) |

## Current Issue: Large File Blockage

The mirror is configured correctly but **failing to sync** due to large files that exceed GitHub's limits:

### Blocking Files

1. **`test_parse_simple`** - 60.74 MB (exceeds GitHub's recommended 50 MB limit)
2. **`--1.ppm`** - 235.13 MB (exceeds GitHub's 100 MB hard limit)

### Error Message

```
PushRejected Error: remote: error: File --1.ppm is 235.13 MB; 
this exceeds GitHub's file size limit of 100.00 MB
GH001: Large files detected. You may want to try Git Large File Storage.
```

## Resolution Options

### Option 1: Use Git LFS (Recommended)
- Migrate large files to Git LFS
- Configure LFS on both Forgejo and GitHub
- Files stored in LFS don't count toward size limits

### Option 2: Remove Large Files from History
- Use `git filter-repo` or BFG to remove large files from git history
- Rewrite history to exclude test fixtures and large assets
- Force push to both repositories

### Option 3: Exclude Test Fixtures from Repository
- Move test fixtures to external storage (S3, dedicated fixtures repo)
- Add to .gitignore and remove from git history
- Reference fixtures via download scripts

## API Response Summary

✅ **Mirror Configuration: PASS**  
- Mirror entry exists for GitHub
- `sync_on_commit` is enabled
- API endpoint accessible and responding

⚠️ **Mirror Sync Status: WARN**  
- Mirror exists and is configured correctly
- Sync is failing due to large files in repository history
- Configuration is correct but blocked by content size limits

## Verification Commands

```bash
# Check mirror status via API
curl -H "Authorization: token <TOKEN>" \
  https://git.ardenone.com/api/v1/repos/jedarden/pdftract/push_mirrors

# Find large files in repository
git rev-list --objects --all |
  git cat-file --batch-check='%(objecttype) %(objectname) %(objectsize) %(rest)' |
  awk '/^blob/ {print substr($0,6)}' |
  sort -n -k2 |
  tail -20
```

## Acceptance Criteria Status

- ✅ **PASS:** Forgejo push_mirrors API shows active github-mirror entry for jedarden/pdftract
- ✅ **PASS:** sync_on_commit is set to true
- ✅ **PASS:** API response confirms mirror is configured correctly
- ⚠️ **WARN:** Mirror exists but sync is blocked by large files (configuration issue, not mirror config)

## References

- Bead ID: bf-1o0la
- Parent bead: bf-320gz
- Depends on: bf-1t8i9
- API Endpoint: https://git.ardenone.com/api/v1/repos/jedarden/pdftract/push_mirrors
- Mirror Remote: remote_mirror_nfF0JdlNzC
