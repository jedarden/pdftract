# bf-9d8a5: Update CLAUDE.md - bf close --reason is no longer broken

## Summary

Updated `/home/coding/pdftract/CLAUDE.md` to remove the stale workaround about `bf close --reason` being broken. The `bf close` command now works correctly as of 2026-05-26 verification.

## Changes Made

### 1. CRITICAL: how to close a bead (lines 20-28)
- **Before:** Stated "bf close <id> --reason is BROKEN" and directed users to use `bf batch` instead
- **After:** Documented the standard `bf close <id> --reason "..."` workflow with example usage

### 2. Doing the work step 6 (line 93)
- **Before:** `bf batch --json '[{"op":"close","id":"pdftract-XXX",...}]'`
- **After:** `bf close pdftract-XXX --reason "<cite note + commits + PASS/WARN/FAIL summary>"`

### 3. What NOT to do (anti-loops) section (removed entirely)
- Removed the obsolete section about the `pdftract-1wqec` loop where `bf close --reason` failed repeatedly
- This bug is now fixed and the warning is no longer relevant

## Commit

**Commit:** `docs(bf-9d8a5): update CLAUDE.md - bf close --reason now works`

**Files modified:**
- `CLAUDE.md`: -12 insertions, +3 insertions (net -9 lines)

**Verification:**
- The commit was pushed to forgejo main at `13267a94`
- Both local and remote show the same commit SHA

## Acceptance Criteria

- [x] **PASS**: Removed "bf close --reason is BROKEN" workaround from CLAUDE.md
- [x] **PASS**: Updated step 6 in "Doing the work" to use `bf close` instead of `bf batch`
- [x] **PASS**: Removed obsolete "What NOT to do (anti-loops)" section
- [x] **PASS**: Committed changes with proper Conventional Commits format
- [x] **PASS**: Pushed commit to forgejo main

## Reusable Pattern

When updating CLAUDE.md or similar documentation:
1. Read the file first to understand the exact formatting and whitespace
2. Use targeted edits (`Edit` tool with specific `old_string`) rather than full file rewrites
3. Verify changes by reading the updated file before committing
4. Commit with descriptive message citing the bead ID
5. Push immediately after committing to ensure remote reflects the work
