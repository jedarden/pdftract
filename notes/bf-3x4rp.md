# Verification Note: bf-3x4rp - Standard Library Imports for Encryption Tests

## Summary
Added missing `std::process::Stdio` import to encryption test files and updated code to use the imported type instead of fully qualified paths.

## Changes Made

### 1. `crates/pdftract-cli/tests/test_encryption_errors.rs`
**Added:**
- `std::process::Stdio` to the import statement (changed from `use std::process::Command;` to `use std::process::{Command, Stdio};`)

**Updated:**
- Line 126: Changed `std::process::Stdio::piped()` to `Stdio::piped()` (3 occurrences)

### 2. `crates/pdftract-cli/tests/test_encryption_unsupported.rs`
**Added:**
- `std::process::Stdio` to the import statement (changed from `use std::process::Command;` to `use std::process::{Command, Stdio};`)

**Updated:**
- Line 73: Changed `std::process::Stdio::piped()` to `Stdio::piped()` (3 occurrences)

## Import Coverage (per bead spec)
- ✅ `std::error` - Not needed for encryption tests
- ✅ `std::fs` - Present in `test_encryption_errors.rs` (used in `fs::read()`)
- ✅ `std::io` - Not needed (Stdio is in std::process)
- ✅ `std::path` - Present in `test_encryption_errors.rs` (used in helper functions)
- ✅ `std::process` - Present in both files (Command + Stdio)
- ✅ `std::time` - Not needed for encryption tests

## Acceptance Criteria Status
✅ All necessary std imports present and organized
✅ No unused imports remain
✅ Files compile successfully after imports are finalized

## Compilation Check
```bash
cargo check --package pdftract-cli --tests
# SUCCESS: No errors or warnings
```

## References
- Parent bead: bf-2nl4x
- Plan line 258, Failure Mode Taxonomy: ENCRYPTION_UNSUPPORTED
