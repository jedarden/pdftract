# pdftract-6696g: Path-traversal protection via --root DIR

## Summary

The `--root` CLI flag for MCP server path-traversal protection was **already implemented** in the codebase. This verification note confirms the implementation meets all acceptance criteria.

## Implementation Location

- **CLI flag**: `crates/pdftract-cli/src/main.rs` lines 112-119
- **Path resolution**: `crates/pdftract-cli/src/mcp/root.rs` lines 39-76 (`resolve_path`)
- **Root validation**: `crates/pdftract-cli/src/mcp/root.rs` lines 78-102 (`canonicalize_root`)
- **Tool integration**: `crates/pdftract-cli/src/mcp/tools/registry.rs` line 203 (`open_pdf` calls `resolve_path`)

## Acceptance Criteria Verification

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Path traversal `../../etc/passwd` → -32602 | ✅ PASS | Test: `test_acceptance_criteria_path_traversal_rejected` |
| Valid path `./subdir/file.pdf` → success | ✅ PASS | Test: `test_acceptance_criteria_valid_path_within_root` |
| Absolute path `/etc/passwd` → -32602 | ✅ PASS | Test: `test_acceptance_criteria_absolute_path_rejected` |
| HTTPS URL bypasses check | ✅ PASS | Test: `test_acceptance_criteria_https_url_bypasses_check` |
| No --root → trust-the-caller mode | ✅ PASS | Test: `test_acceptance_criteria_no_root_trust_the_caller` |
| Symlink escape → -32602 | ✅ PASS | Test: `test_acceptance_criteria_symlink_escape_rejected` |
| Nonexistent --root → startup error | ✅ PASS | Test: `test_acceptance_criteria_nonexistent_root_startup_error` |
| --root is file → startup error | ✅ PASS | Test: `test_acceptance_criteria_file_not_directory_startup_error` |
| Plan critical test | ✅ PASS | Test: `test_plan_critical_test_path_traversal_with_root` |

## Test Results

All 25 tests pass:
- **Unit tests** (root.rs): 12 tests passed
- **Integration tests** (root-path-protection.rs): 13 tests passed

```bash
$ cargo test --test root-path-protection
test result: ok. 13 passed; 0 failed; 0 ignored

$ cargo test --package pdftract-cli --lib mcp::root
test result: ok. 12 passed; 0 failed; 0 ignored
```

## Key Security Features

1. **Canonicalization**: `std::fs::canonicalize` follows symlinks, ensuring the resolved path is checked
2. **Absolute path rejection**: Explicit check for `/` prefix when `--root` is set
3. **HTTPS bypass**: URLs starting with `http://` or `https://` skip path resolution
4. **Structured errors**: All failures return JSON-RPC error code -32602 with descriptive messages

## CLI Help Output

```
--root <DIR>
    Root directory for local filesystem access (enforces path-traversal protection)

    When set, all local-path tool arguments are resolved relative to DIR and any
    path that escapes DIR is rejected with JSON-RPC error code -32602. HTTPS URLs
    are not affected by this flag. Without --root, the server runs in
    trust-the-caller mode (no path-check applied).
```

## No Code Changes Required

The implementation was already complete. This bead was a verification task rather than an implementation task.
