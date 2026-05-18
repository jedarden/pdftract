# pdftract-2ka7: Password Ingress Channels

## Summary

Implemented secure password ingress channels for PDF password handling in the CLI.

## Implementation Status

### CLI (pdftract-cli) ✅ PASS

**File:** `crates/pdftract-cli/src/password.rs`

Implemented the complete password resolution logic with priority order:
1. `--password-stdin` flag (reads one line from stdin)
2. `PDFTRACT_PASSWORD` env var
3. `--password VALUE` (only if `PDFTRACT_INSECURE_CLI_PASSWORD=1`)
4. None

**File:** `crates/pdftract-cli/src/main.rs`

Wired the password resolution into the `Extract` command with proper error handling.

### Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| `--password-stdin` flag implemented | ✅ PASS | Reads one newline-terminated line from stdin; empty password = no password |
| `PDFTRACT_PASSWORD` env var | ✅ PASS | Read when present and `--password-stdin` not supplied |
| `--password VALUE` rejected (exit 64) | ✅ PASS | Requires `PDFTRACT_INSECURE_CLI_PASSWORD=1` |
| Stderr warning on opt-in | ✅ PASS | Emits warning when `--password VALUE` accepted |
| Python `password=` kwarg | ⚠️ N/A | `pdftract-py` crate doesn't exist yet |
| MCP password body field | ⚠️ N/A | `pdftract-mcp` crate doesn't exist yet |
| Serve password form field | ⚠️ N/A | `pdftract-serve` crate doesn't exist yet |
| Exit codes match policy | ✅ PASS | Exit code 64 for usage errors |
| TH-07 test passes | ⚠️ WARN | Separate bead (not implemented yet) |

## Test Results

### Unit Tests
```
running 8 tests
test password::tests::test_resolve_password_empty_env_var ... ok
test password::tests::test_resolve_password_opt_in_two_not_accepted ... ok
test password::tests::test_resolve_password_value_rejected_without_opt_in ... ok
test password::tests::test_resolve_password_opt_in_zero_not_accepted ... ok
test password::tests::test_resolve_password_stdin_takes_priority ... ok
test password::tests::test_resolve_password_value_accepted_with_opt_in ... ok
test password::tests::test_resolve_password_env_var ... ok
test password::tests::test_resolve_password_none ... ok

test result: ok. 8 passed; 0 failed
```

### Integration Tests
```bash
# Test --password-stdin
$ echo "testpassword" | ./target/release/pdftract extract --password-stdin -
Password provided via secure channel
# ✅ PASS

# Test PDFTRACT_PASSWORD
$ PDFTRACT_PASSWORD="envpass" ./target/release/pdftract extract -
Password provided via secure channel
# ✅ PASS

# Test --password VALUE rejected (no opt-in)
$ ./target/release/pdftract extract --password secret -
Error: --password VALUE is insecure and rejected by default...
Exit code: 64
# ✅ PASS

# Test --password VALUE with opt-in (warning)
$ PDFTRACT_INSECURE_CLI_PASSWORD=1 ./target/release/pdftract extract --password secret -
WARNING: --password VALUE is insecure (visible via 'ps aux')...
Password provided via secure channel
# ✅ PASS
```

## Implementation Notes

### Stdin Discipline
- When `--password-stdin` is set with `-` (stdin input), the first line is the password and the rest is the PDF bytes
- Newline stripping: trims trailing `\n` or `\r\n` only; preserves leading/trailing whitespace
- Empty password (bare newline) is treated as "no password"

### Future Work
The following will be implemented in future beads when the respective crates are created:
- `pdftract-py/src/lib.rs`: PyO3 `password=` kwarg to `SecretString`
- `pdftract-mcp/src/handlers/extract.rs`: Password parameter in request body
- `pdftract-serve/src/routes/extract.rs`: Multipart form field for password

## References
- Plan: line 878 (TH-07 mitigation), line 902-913 (Secrets Handling), line 949 (NEVER log passwords)
