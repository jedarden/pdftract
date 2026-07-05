# bf-10qd4: Mark unpublished install channels as coming-soon

## Summary

Fixed README.md to clearly indicate all unpublished install channels, addressing the issue where install commands pointed to nonexistent artifacts.

## Work Done

### 1. Verified unpublished channels
- **crates.io**: No `pdftract` or `pdftract-core` exists (confirmed via web search)
- **PyPI**: No `pdftract` package exists (confirmed via web search)  
- **Docker Hub**: No `ronaldraygun/pdftract` image exists (confirmed via web search)
- **Homebrew**: No `pdftract` formula exists (confirmed via web search)
- **Git tags**: No version tags exist (confirmed via `git tag`)

### 2. Updated README.md
- **Badges** (lines 3-4): Changed crates.io and PyPI badges from version badges to "coming-soon" badges
- **Installation section** (lines 39-72): 
  - Added prominent warning: "🚧 Public releases coming soon"
  - Moved development installation (build from source) to the top
  - Added "Planned release channels (coming soon)" subsection
  - Marked each install command with status: "*Status: Not yet published to [platform] — tracked in bf-10qd4*"
- **Distribution table** (lines 233-239): Added "**🚧 coming soon**" to all unpublished SDKs

### 3. Commit and push
- Commit: `7a1b0d43` - `docs(bf-10qd4): mark unpublished install channels as coming-soon`
- Pushed to `origin` (git.ardenone.com) successfully

## Acceptance Criteria

**PASS**: All acceptance criteria met:
- ✅ Every unpublished channel is clearly marked as coming-soon
- ✅ No install command points at a nonexistent artifact without clear status
- ✅ README provides alternative (development installation) instructions

**Details:**
- All install commands (cargo, pip, docker, brew) are now marked with "*Status: Not yet published...*" notes
- Users are directed to build from source instead of attempting to install from nonexistent packages
- Badges now show "coming-soon" status instead of non-existent version numbers
- Distribution table clearly indicates which SDKs are not yet published

## Verification

Can verify by:
1. Checking that no install command in README.md points to an artifact without a "coming-soon" or status disclaimer
2. Confirming all badges point to valid URLs (docs.rs badge retained, others redirected to plan)
3. Building from source using the new "Development installation" instructions

## Files Changed
- `README.md`: 40 insertions, 14 deletions

## References
- Bead ID: bf-10qd4
- Commit: 7a1b0d43
- Related: bf-5o8cg (release WorkflowTemplates - still in_progress)
