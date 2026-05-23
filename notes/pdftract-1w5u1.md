# pdftract-1w5u1: Output formats verification

## Summary

The three output formats for `pdftract doctor` were already implemented in:
- `crates/pdftract-cli/src/doctor/output/human.rs` - colored table output
- `crates/pdftract-cli/src/doctor/output/json.rs` - JSON output
- `crates/pdftract-cli/src/doctor/output/features.rs` - feature listing

## Acceptance Criteria Verification

### PASS: Default (TTY) colored table output
- Output shows colored table with Check, Status, Detail columns
- ANSI colors: OK=green, WARN=yellow, FAIL=red
- Summary line at bottom: "N OK, M WARN, K FAIL"
- Verified: `./target/release/pdftract doctor` produces properly formatted table

### PASS: Non-TTY plain text output
- Piped output (via `| cat`) strips ANSI escape codes
- Same content structure as TTY but without colors
- Summary line included
- Verified: `./target/release/pdftract doctor | cat` produces plain text

### PASS: --json output
- Single JSON object with `{summary: {ok, warn, fail, not_applicable}, checks: [...]}`
- JSON parses correctly with `jq`
- Status values are "OK", "WARN", "FAIL", "N/A"
- Verified: `./target/release/pdftract doctor --json | jq '.summary.fail'` returns integer

### PASS: --json jq filtering
- `./target/release/pdftract doctor --json | jq '.summary'` shows correct counts
- `./target/release/pdftract doctor --json | jq '.checks[] | select(.status == "WARN")'` filters correctly

### PASS: --features output
- Lists compiled features, one per line
- Exit code 0
- No diagnostic checks run
- Verified: `./target/release/pdftract doctor --features` returns exit code 0
- Note: Default build has no features enabled ("default" feature set), so output is empty

### PASS: --no-color output
- Plain text output even in TTY
- No ANSI escape codes
- Same structure as default
- Verified: `./target/release/pdftract doctor --no-color` produces plain text table

### PASS: 80-column terminal width
- Output fits within 80 columns without wrapping
- Column widths: name=30, status=6, detail=flex
- Separator line is exactly 80 characters

### PASS: N/A row handling
- N/A checks excluded from human output (verified in code: line 41-43 of human.rs)
- N/A checks included in JSON with status="N/A" (verified in code: line 28-31 of json.rs)

## Dependencies

- `atty` crate for TTY detection (already in dependencies)
- `terminal_size` crate for terminal width detection (already in dependencies)
- `termcolor` crate for ANSI color handling (already in dependencies)

## Implementation Notes

The output modules were already implemented with:
1. **TTY detection** via `std::io::IsTerminal` trait (nightly feature stabilized)
2. **Color control** via `termcolor` crate with `ColorChoice` enum
3. **Terminal width detection** via `terminal_size` crate
4. **Detail truncation** for long strings in TTY mode
5. **Summary line** at bottom with bold formatting
6. **Stderr output** for failures (in addition to stdout summary)

All acceptance criteria are met. No changes were required to the codebase.
