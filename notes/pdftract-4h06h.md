# Verification Note: pdftract-4h06h - TH-02 Path Traversal Test

## Bead

**ID:** pdftract-4h06h
**Title:** TH-02 test: MCP path traversal (10 payloads) rejected with PATH_OUTSIDE_ROOT (or no path param accepted)
**Status:** PASS

## Summary

Implemented comprehensive path-traversal security tests at `crates/pdftract-cli/tests/TH-02-path-traversal.rs`. The test suite documents the 10 canonical path-traversal payloads from the threat model (plan line 891) and verifies that the `resolve_path` function properly rejects them when `--root` mode is enabled.

## What Was Done

1. Created `crates/pdftract-cli/tests/TH-02-path-traversal.rs` with 10 test functions
2. Documented all 10 path-traversal payloads from the threat model:
   - Basic traversal (`../../etc/passwd`)
   - Deeper traversal (`../../../etc/passwd`)
   - Very deep traversal (`../../../../etc/passwd`)
   - Absolute paths (`/etc/passwd`)
   - Traversal with valid prefix (`./valid/../../../etc/passwd`)
   - URL-encoded traversal (`valid/..%2F..%2Fetc%2Fpasswd`)
   - Windows separators on Linux (`valid/..\..\..\etc\passwd`)
   - Long traversal with valid prefix (`valid/../../../../etc/passwd`)
   - Special filesystem (`/proc/self/environ`)
   - Windows reserved name (`con`)

3. Test coverage:
   - `test_root_mode_rejects_all_traversal_payloads`: Verifies all 10 payloads are rejected when --root is set
   - `test_root_mode_accepts_valid_paths`: Verifies valid paths within root are accepted
   - `test_without_root_paths_pass_through`: Documents current behavior (paths pass through without --root)
   - `test_https_urls_bypass_root_check`: Verifies HTTPS URLs bypass validation per INV-10
   - `test_symlink_escape_rejected`: Verifies symlinks escaping root are rejected
   - `test_url_encoded_traversal_rejected`: Verifies URL-encoded traversal is caught
   - `test_windows_reserved_name_handling`: Handles Windows reserved names safely
   - `test_special_filesystem_paths_rejected`: Rejects /proc, /dev paths
   - `test_nested_traversal_with_valid_prefix`: Catches traversal after legitimate-looking prefix
   - `test_deep_traversal_rejected`: Verifies various depths of ../ are caught

## Current State Documented

Per the bead description, the test documents the current security posture:
- **Phase 1 (current):** MCP tools accept `path` parameters. Without `--root`, paths pass through as-is (trust-the-caller mode for local stdio). With `--root`, paths are canonicalized and validated.
- **Phase 2 (future):** When `--root DIR` is introduced to `pdftract mcp`, all paths will be validated against the root boundary.

The `resolve_path` function in `crates/pdftract-cli/src/mcp/root.rs` already implements the security boundary (canonicalization + boundary check). The --root mode is not yet wired to the MCP server entry point, which is a known gap documented in the test comments.

## Acceptance Criteria

- ✅ `tests/security/TH-02-path-traversal.rs` exists (created at `crates/pdftract-cli/tests/TH-02-path-traversal.rs`)
- ✅ Phase 1 tests pass: All 10 traversal payloads are rejected when --root is set
- ✅ The 10 traversal payloads are documented in the test file
- ✅ INV-10 cited as the structural mitigation source (referenced in test documentation)

## Test Results

```
cargo nextest run --test TH-02-path-traversal
Summary [   0.009s] 10 tests run: 10 passed, 0 skipped
```

All tests passed:
- test_root_mode_rejects_all_traversal_payloads
- test_root_mode_accepts_valid_paths
- test_without_root_paths_pass_through
- test_https_urls_bypass_root_check
- test_symlink_escape_rejected
- test_url_encoded_traversal_rejected
- test_windows_reserved_name_handling
- test_special_filesystem_paths_rejected
- test_nested_traversal_with_valid_prefix
- test_deep_traversal_rejected

## References

- Plan section: TH-02 entry (line 891)
- INV-10: `pdftract mcp` in HTTP mode MUST NOT accept file-path parameters
- `crates/pdftract-cli/src/mcp/root.rs`: Path resolution and escape checking implementation
