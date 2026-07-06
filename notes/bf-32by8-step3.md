# bf-32by8-step3: Execute pdftract command with captured output

## State: FAIL

**Date:** 2026-07-06

## Command Executed

```bash
unset RUST_LOG; cargo run -- extract tests/fixtures/encoding/fingerprint-match.pdf --json - 2>&1
```

**Exit Code:** 1 (failure)

## Captured Output Summary

### Build Phase
- **Warnings:** 205 warnings (unused imports, unused variables, dead code)
- **Build Status:** SUCCESS (completed in 43.78s)
- **Binary:** `target/debug/pdftract`

### Execution Result
```
Running `target/debug/pdftract extract tests/fixtures/encoding/fingerprint-match.pdf --json -`
Error: Failed to extract PDF
```

### Detailed Output Structure
1. **Compiler warnings** - 205 warnings about unused code (does not affect functionality)
2. **Build completion** - "Finished `dev` profile" message
3. **Execution start** - "Running" message with full command
4. **Error** - "Failed to extract PDF"
5. **Exit code** - 1

## Analysis

1. **RUST_LOG State**: ✓ Confirmed unset
   - The command explicitly ran with `unset RUST_LOG` before invocation
   - No log output was produced (would have appeared if RUST_LOG was set)

2. **Exit Code**: 1 (failure)
   - Non-zero exit code indicates the command failed
   - The error message "Failed to extract PDF" suggests an extraction issue

3. **Output Streams**:
   - **stdout**: No JSON output (extraction failed before producing output)
   - **stderr**: Error message "Failed to extract PDF" + compiler warnings

## Next Steps

The exit code 1 and error message indicate:
- The command execution was captured correctly
- RUST_LOG was successfully unset (no log output appeared)
- The PDF extraction failed for some reason (to be investigated in subsequent steps)

## Acceptance Criteria Status

- ✅ Run `unset RUST_LOG; cargo run -- -i <fixture> -o - 2>&1` (adapted to correct syntax) - **PASS**
- ✅ Capture both stdout and stderr to a temp file or variable - **PASS**
- ✅ Record the command exit code (Exit code: 1) - **PASS**
- ✅ Save captured output to notes/bf-32by8-step3.md - **PASS** (this file)
