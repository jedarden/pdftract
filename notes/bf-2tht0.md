# bf-2tht0: Implement grep-corpus benchmark command execution

## Summary

Verified that the `execute_grep_command` function in `crates/pdftract-cli/benches/grep_1000.rs` (lines 393-469) satisfies all acceptance criteria for bead bf-2tht0.

## Acceptance Criteria Status

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Function exists that constructs and executes benchmark command | ✅ PASS | `execute_grep_command` at lines 393-469 |
| Command runs with search term 'the' and corpus path argument | ✅ PASS | Command construction at lines 404-409, uses SEARCH_PATTERN "the" |
| Process spawning uses Stdio::piped() for output capture | ✅ PASS | Lines 412-413 configure stdout/stderr pipes |
| Proper error handling for execution failures | ✅ PASS | Error handling at lines 419-420, 441-443, 452-457, 448-449 |
| Command exits without panics or unhandled errors | ✅ PASS | All error paths return Err, no panicking unwraps |

## Implementation Details

### Command Construction
```rust
let mut cmd = Command::new("pdftract");
cmd.arg("grep")
   .arg(pattern)
   .arg(corpus_path)
   .arg("-j")
   .arg(threads.to_string())
   .arg("--progress-json");
```

This produces: `pdftract grep "the" tests/fixtures/grep-corpus/corpus/ -j 4 --progress-json`

### Output Capture
- Stdio::piped() for both stdout and stderr
- Concurrent pipe reading via threads to avoid deadlock
- Match count parsed from progress JSON events in stderr

### Error Handling
- Spawn failure: `"Failed to spawn pdftract grep command: {error}"`
- Wait failure: `"Failed to wait for pdftract grep command: {error}"`
- Non-zero exit: Full stdout/stderr context in error message
- Thread join failures: Graceful error propagation

### Return Value
Returns `Result<(duration_ms, matches_total), String>`:
- `duration_ms`: Wall-clock execution time in milliseconds
- `matches_total`: Total match count parsed from progress events

## Verification Steps Performed

1. ✅ Read `/home/coding/pdftract/crates/pdftract-cli/benches/grep_1000.rs`
2. ✅ Verified function exists at correct location (lines 393-469)
3. ✅ Confirmed command construction uses search term 'the' and corpus path
4. ✅ Verified Stdio::piped() usage for output capture
5. ✅ Reviewed all error handling paths
6. ✅ Confirmed no panicking operations on success paths

## Integration Points

This function is called from:
- `run_benchmark()` (line 548) with pattern="the", threads=4
- Returns metrics used for BenchmarkResult structure
- Metrics validated against CI gates (50 MB/s throughput threshold)

## Status

✅ **COMPLETE** - All acceptance criteria satisfied. The implementation was already present in the codebase and correctly implements the grep-corpus benchmark command execution layer.
