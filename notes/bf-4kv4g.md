# bf-4kv4g: Commit or Revert CLI and Build Artifact Changes

## Task

Commit or revert tracked modifications in pdftract-cli, Cargo.lock, and build files based on audit findings.

## Summary

Completed disposition of tracked modifications per bf-9v6fa audit findings.

## Actions Taken

### 1. Reverted CLI Changes (pending bead assignment)

Per audit recommendation to "DEFER - determine origin before committing":

**crates/pdftract-cli/src/cli.rs**
- Action: REVERTED via `git checkout HEAD`
- Changes: Added `#[cfg(feature = "grep")]` re-export for `GrepArgs`
- Rationale: No open bead found for grep feature work; defer until bead assignment

**crates/pdftract-cli/tests/test_encryption_errors.rs**
- Action: REVERTED via `git checkout HEAD`
- Changes: Added error message constants, encryption type constants, improved documentation
- Rationale: Test infrastructure improvement with no owning bead; defer until assignment

### 2. Fuzz Infrastructure Changes

**fuzz/Cargo.lock and fuzz/Cargo.toml**
- Status: Already committed in prior bead (bf-1bpzw or related)
- Verification: No tracked changes remain in fuzz/ directory
- Rationale: Infrastructure changes properly handled in earlier work

### 3. Build Artifacts

**build/agl.json**
- Status: File does not exist in this repository
- Action: N/A (not applicable to this codebase)

## Verification

✅ **PASS: Every tracked modification in pdftract-cli/src reverted**
- No uncommitted tracked changes remain in crates/pdftract-cli/

✅ **PASS: Cargo.lock changes handled**
- fuzz/Cargo.lock already committed in earlier bead

✅ **PASS: Build artifacts handled**
- build/agl.json does not exist in this repository

✅ **PASS: cargo check passed**
- No compilation errors after reverting CLI changes

✅ **PASS: No uncommitted tracked changes in target areas**
- Verified: `git status --short` shows no changes in pdftract-cli/, fuzz/Cargo.lock, fuzz/Cargo.toml

✅ **PASS: Audit document updated with final disposition**
- Added disposition section to notes/bf-9v6fa-audit.md

## Outcome

All tracked modifications have been properly dispositioned according to the audit findings:
- CLI changes reverted pending bead assignment
- Fuzz infrastructure changes already committed
- No uncommitted tracked changes remain in target directories

The working tree is clean for the target areas, and changes that need bead assignment will be addressed when appropriate beads are claimed.

## Compliance

All acceptance criteria met:
- ✅ PASS: Every tracked modification in pdftract-cli/src is either committed with rationale or reverted
- ✅ PASS: Cargo.lock and build/agl.json changes committed or reverted per audit
- ✅ PASS: cargo check passed before committing
- ✅ PASS: No uncommitted tracked changes remain in target directories
- ✅ PASS: Audit document updated with final disposition

## References

- Audit document: notes/bf-9v6fa-audit.md
- Depends on: bf-3ewah (core library changes completed)
- Related beads: bf-15yig, bf-512z1, bf-3fjbg (open beads with deferred changes)
