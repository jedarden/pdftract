# Verification Note: pdftract-24kut

## Task
6.7.4: Transport mutual exclusion enforcement at CLI parse (ADR-006)

## Summary
Verified that the MCP CLI enforces mutual exclusion between `--stdio` and `--bind` transport modes at parse time via clap's ArgGroup mechanism.

## Implementation Status
The implementation was already present in `crates/pdftract-cli/src/main.rs`:
- Lines 86-98: `Mcp` variant uses `#[group(id = "transport", multiple = false)]` with both `stdio` and `bind` fields having `group = "transport"`
- Lines 186-203: Runtime logic defaults to stdio mode when neither flag is specified
- Help text (lines 82-86) references ADR-006 and explains the mutual exclusion rationale

## Acceptance Criteria Verification

| Criterion | Status | Notes |
|-----------|--------|-------|
| `pdftract mcp` → stdio mode (default) | PASS | `bind.is_none()` triggers stdio default (line 188) |
| `pdftract mcp --stdio` → stdio mode | PASS | Explicit flag sets stdio mode |
| `pdftract mcp --bind 127.0.0.1:8080` → HTTP+SSE mode | PASS | Bind address parsed, HTTP server runs |
| `pdftract mcp --stdio --bind ADDR` → exit 2 | PASS | clap ArgGroup enforces mutual exclusion |
| `pdftract mcp --help` shows mutual exclusivity | PASS | ADR-006 and rationale documented in help |
| Unit test for ArgGroup conflict | PASS | `crates/pdftract-cli/tests/mcp-cli-args.rs` |

## Test Results
```
running 5 tests
test test_stdio_and_bind_mutually_exclusive ... ok
test test_default_to_stdio ... ok
test test_stdio_flag_valid ... ok
test test_bind_flag_valid ... ok
test test_help_mentions_adr_006 ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Manual Verification
```bash
$ ./target/debug/pdftract mcp --stdio --bind 127.0.0.1:8080
error: the argument '--stdio' cannot be used with '--bind <ADDR>'
Usage: pdftract mcp --stdio
Exit code: 2
```

## References
- Plan: Phase 6.7 MCP Server Mode (lines 2286-2349)
- ADR-006: Transport mutual exclusion rationale
- clap derive docs: https://docs.rs/clap/latest/clap/_derive/index.html

## PASS/WARN/FAIL Summary
- PASS: All acceptance criteria met
- WARN: None
- FAIL: None
