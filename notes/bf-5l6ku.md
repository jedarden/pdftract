# Bead bf-5l6ku: Trigger Forgejo Mirror Sync to GitHub

## Current State Assessment

### Commit Divergence
- **Forgejo (origin)**: Latest commit `a5f03923` (2026-07-06 18:46:17 -0400)
- **GitHub**: Latest commit `88b4f0da` (2026-06-01 09:39:29 -0400)
- **Common ancestor**: `88b4f0da` (GitHub's latest commit)
- **Commits behind**: 332 commits from Forgejo → GitHub (not 84 as initially estimated)

### Mirror Configuration Status
- Parent bead `bf-10182` configured the push mirror from Forgejo → GitHub
- Mirror should target: `github.com/jedarden/pdftract`
- Expected configuration: `sync_on_commit=true`

### API Authentication Issue
- Forgejo API endpoints require authentication (token/credentials)
- No Forgejo credentials found in environment or git config
- Cannot access `/api/v1/repos/jedarden/pdftract/push_mirrors` without auth
- Cannot access `/api/v1/repos/jedarden/pdftract/mirror-sync` without auth

## Alternative Approach: Trigger via Push to Origin

Hypothesis: If the mirror is configured with `sync_on_commit=true`, pushing a new commit to Forgejo (origin) should automatically trigger the mirror sync to GitHub.

### Proposed Method
1. Create a trivial commit (or use existing uncommitted changes)
2. Push to Forgejo origin
3. Monitor if mirror sync triggers automatically
4. Verify GitHub receives the commits

## Verification Steps
1. Check mirror status via API (requires auth) or web UI
2. Compare `git log origin/main` vs `git log github/main`
3. Verify commit count matches: `git rev-list --count origin/main..github/main` should be 0
4. Verify tip-to-tip equality: `git diff origin/main github/main` should be empty

## Acceptance Criteria Status
- [ ] Mirror sync triggered successfully
- [ ] Sync completed without errors
- [ ] Mirror last_update timestamp is current

## Notes
- Task requires API or web UI trigger, but neither is accessible in terminal environment
- Alternative: Manual trigger via Forgejo web UI at https://git.ardenone.com/jedarden/pdftract/settings
- Current local working directory has uncommitted changes to:
  - `.needle-predispatch-sha`
  - `crates/pdftract-core/src/extract.rs`
  - `crates/pdftract-core/tests/TH-05-ssrf-block.rs`

## Next Steps
1. Try pushing to origin to trigger automatic mirror sync (if sync_on_commit=true)
2. If that fails, document that manual web UI trigger is required
3. Monitor sync progress and verify GitHub catches up to 332 commits
