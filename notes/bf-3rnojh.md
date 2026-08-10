# bf-3rnojh: Verify workspace compiles cleanly

## Summary
Fixed a compilation error in `crates/pdftract-core/src/document.rs` where a match expression didn't handle the `PdfObject::Indirect(_)` pattern.

## Issue Found
- **Error**: `error[E0004]: non-exhaustive patterns: `&PdfObject::Indirect(_)` not covered`
- **Location**: `crates/pdftract-core/src/document.rs:775`
- **Context**: Match expression checking the `/Type` field in a Pages dictionary

## Fix Applied
Added the missing `PdfObject::Indirect(_)` case to the match arm at line 793. The Indirect case is treated the same as other malformed types (Array, Dict, Stream, etc.) - it returns a `DocumentError::EmptyDocument` error since an indirect reference in the `/Type` field indicates a malformed PDF structure.

## Acceptance Criteria Status
- ✅ `cargo build --workspace` completes successfully with exit code 0
- ✅ No compilation errors in any crate
- ✅ No new compiler warnings introduced (pre-existing warnings remain)
- ✅ All workspace members build successfully

## Verification Commands
```bash
# Build succeeded
cargo build --workspace
# Exit code: 0

# Full workspace verified
cargo build --workspace --all-targets
# Exit code: 0
```

## References
- Parent bead: bf-4izpsx
- Git commit: (to be added after commit)
