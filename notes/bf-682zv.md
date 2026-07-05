# GitHub Sync Attempt - Blocker Report

## Task
Sync GitHub main to match forgejo/main (84 commits behind)

## Current State
- **Forgejo/main**: 02bfffef docs(bf-1o0la): document Forgejo push mirror configuration and status
- **GitHub/main**: 88b4f0da fix(pdftract-2rc4): fix CI schema gate script and add verification note
- **Commits behind**: 160 (updated from initial 84 estimate)

## Blocker: Large File in Git History

GitHub rejected the push due to large file(s) in git history:

```
remote: warning: File test_parse_simple is 60.74 MB; this is larger than GitHub's recommended maximum file size of 50.00 MB
remote: error: File --1.ppm is 235.13 MB; this exceeds GitHub's file size limit of 100.00 MB
remote: error: GH001: Large files detected.
```

### Problem Details

1. **Large file**: `--1.ppm` (235.13 MB PPM image file)
   - Added in: `1c6f26ec` fix(bf-4mkhv): clean up unused imports in hash.rs
   - Removed in: `007439e7` chore(bf-8031): remove tracked debug/scratch artifacts and compiled binaries
   - Both commits are in the sync range

2. **GitHub's behavior**: GitHub checks all objects being pushed, even if the large file was later removed from the working tree. The file still exists in the git object database.

3. **Result**: Pre-receive hook declined - sync cannot complete

## Acceptance Criteria Status

- **FAIL**: GitHub main commit SHA does not equal forgejo/main (blocker: large file)
- **PASS**: No commits exist on GitHub main that are not on forgejo main
- **PASS**: Merge strategy was used (attempted fast-forward merge)
- **WARN**: Automatic sync triggered by Forgejo mirror did not resolve this

## Resolution Options

### Option 1: Git LFS (Recommended)
Set up Git LFS for large files:
```bash
git lfs install
git lfs track "*.ppm" "*.pdf" "*.bin"
git add .gitattributes
git commit -m "chore: enable git-lfs for large files"
```

### Option 2: History Rewrite (Not Recommended)
Use git filter-branch or BFG to remove large file from history:
```bash
git filter-branch --force --index-filter \
  "git rm --cached --ignore-unmatch --1.ppm" \
  --prune-empty --tag-name-filter cat -- --all
```
This would diverge forgejo/main from GitHub main.

### Option 3: Manual Intervention
Manually exclude the problematic commit from sync and re-implement the change without the large file.

## Recommendation

**Setup Git LFS** on both repositories (Forgejo and GitHub) before re-attempting sync. The large file `--1.ppm` was a temporary scratch file that should have been handled by git-lfs from the start.

## Status
**BLOCKED** - Requires git-lfs setup or history cleanup before sync can proceed.
