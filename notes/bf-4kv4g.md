# bf-4kv4g: CLI and Build Artifact Disposition

## Summary

Bead bf-4kv4g completed the disposition of tracked modifications in `crates/pdftract-cli/src`, `Cargo.lock`, and `build/agl.json` according to the audit findings in notes/bf-9v6fa-audit.md.

## Work Completed

### 1. CLI Changes Reverted (Pending Bead Assignment)

#### crates/pdftract-cli/src/cli.rs
- **Action**: REVERTED via `git checkout HEAD`
- **Rationale**: Added `#[cfg(feature = "grep")]` re-export for `GrepArgs` with no owning bead
- **Disposition**: Deferred until bead assignment for grep feature work

#### crates/pdftract-cli/tests/test_encryption_errors.rs
- **Action**: REVERTED via `git checkout HEAD`
- **Rationale**: Test infrastructure improvements (error message constants, encryption type constants, documentation) with no owning bead
- **Disposition**: Deferred until bead assignment

### 2. Fuzz Infrastructure Already Committed

#### fuzz/Cargo.lock and fuzz/Cargo.toml
- **Status**: Already committed in prior bead (bf-1bpzw or related fuzz work)
- **Verification**: No tracked changes remain in fuzz/ directory

### 3. Build Artifacts

#### build/agl.json
- **Status**: File does not exist in this repository
- **Disposition**: N/A (not applicable to this codebase)

### 4. Infrastructure Update (2026-07-06)

#### .needle-predispatch-sha
- **Action**: COMMITTED (needle harness bookkeeping)
- **Changes**: Updated predispatch SHA from 125dc668 to e65ff595
- **Rationale**: Needle harness state update, unrelated to feature work
- **Disposition**: Committed with infrastructure rationale

## Acceptance Criteria

### PASS
- ✅ No tracked modifications remain in crates/pdftract-cli/src/
- ✅ No tracked modifications remain in Cargo.lock
- ✅ No tracked modifications remain in fuzz/ directory
- ✅ cargo check passes with no errors
- ✅ Audit document updated with final disposition (notes/bf-9v6fa-audit.md lines 350-393)
- ✅ Working tree is clean for all target directories

### WARN
- None

### FAIL
- None

## References

- Audit document: notes/bf-9v6fa-audit.md
- Plan sections: None directly cited (this was a cleanup/disposition task)
- Related beads: bf-3ewah (core library disposition), bf-15yig (deferred ToUnicode work)

## Verification Commands

```bash
# Verify no tracked changes in target directories
git status crates/pdftract-cli/src/ Cargo.lock build/ fuzz/

# Verify compilation
cargo check

# Verify audit updated
grep -A 30 "bf-4kv4g Final Disposition" notes/bf-9v6fa-audit.md
```

All verification steps passed successfully on 2026-07-06.
