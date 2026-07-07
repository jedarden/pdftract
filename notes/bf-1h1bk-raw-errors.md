# Raw Error Catalog - Fuzz Build Logs

**Bead ID:** bf-1h1bk
**Date:** 2026-07-06
**Task:** Extract and catalog raw error messages from fuzz build logs

## Log Files Examined

All fuzz build logs were **empty (0 bytes)**:

- `fuzz-build-lexer.log` - 0 bytes
- `fuzz-build-verbose.log` - 0 bytes
- `fuzz/fuzz-build-capture.log` - 0 bytes
- `fuzz/fuzz-build-full.log` - 0 bytes
- `fuzz/fuzz-build-with-errors.log` - 0 bytes
- `fuzz/fuzz-clean-build.log` - 0 bytes

## Finding

**No compilation errors were found in the fuzz build logs.** All log files were empty, indicating either:
1. The fuzz build process succeeded without errors
2. The build output was not captured/written to the log files
3. The logs were cleared after successful builds

## Related Compiler Warnings Found

While not from the fuzz build logs, compiler warnings were found in `notes/bf-60vlq-test-run.log`:

### Dead Code Warnings

**File:** `crates/pdftract-core/build.rs`
**Lines:** 37, 43
**Type:** `dead_code`
**Severity:** warning
**Message:** Fields `description` and `version` are never read in struct `UnmappedGlyphNamesConfig`

```rust
struct UnmappedGlyphNamesConfig {
    description: Option<String>,  // Line 37 - never read
    version: Option<String>,      // Line 43 - never read
}
```

### Unused Import Warnings

1. **File:** `crates/pdftract-schema-migrate/src/bin/migrate-schema.rs`
   **Line:** 9
   **Type:** `unused_imports`
   **Message:** Imports `read_json` and `write_json` are unused

2. **File:** `crates/pdftract-core/src/annotation/json.rs`
   **Line:** 6
   **Type:** `unused_imports`
   **Message:** Import `DestArray` is unused

3. **File:** `crates/pdftract-core/src/cache/key.rs`
   **Line:** 10
   **Type:** `unused_imports`
   **Message:** Import `Map` is unused

4. **File:** `crates/pdftract-core/src/cache/lru.rs`
   **Line:** 8
   **Type:** `unused_imports`
   **Message:** Import `entry_path` is unused

### Bundle Size Warning

**Warning:** `pdftract-inspector-ui@0.1.0`
- Raw: 1.95 KB
- Gzipped: 0.87 KB / 80 KB limit
- Status: Well under limit (passes)

## Conclusion

The fuzz build logs contain no error messages. All examined logs were empty files. The fuzz build process either completed successfully without errors, or the output was not captured in the log files.

**Recommendation:** If fuzz build errors need to be captured in the future, ensure the build process redirects both stdout and stderr to the log files using a command like:
```bash
cargo fuzz build [...] > fuzz-build.log 2>&1
```
