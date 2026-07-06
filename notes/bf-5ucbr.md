# bf-5ucbr: Capture pdftract debug logs on JavaScript PDF

## Task Summary
Capture pdftract debug logs on JavaScript PDF fixture and preserve the output for verification.

## Execution

### PDF File Used
- File: `tests/fixtures/security/embedded-js.pdf`
- Size: 1.1K
- Contains: JavaScript action with `app.alert("pwn")` in catalog.openaction

### Commands Executed

```bash
# Attempt 1: With debug logging and combined output
RUST_LOG=debug cargo run --bin pdftract -- extract tests/fixtures/security/embedded-js.pdf --json - 2>&1 | tee notes/bf-5ucbr-pdftract-debug.log

# Attempt 2: With trace logging, separated stdout/stderr  
RUST_LOG=trace cargo run --bin pdftract -- extract tests/fixtures/security/embedded-js.pdf --json - > /tmp/js_output.json 2> notes/bf-5ucbr-pdftract-debug-stderr.log
```

### Results

**pdftract Version**: Built from current main branch (dev profile, unoptimized debuginfo)

**Execution Status**: ✅ SUCCESS - pdftract ran successfully on the JavaScript PDF fixture

**JavaScript Detection**: Successfully detected 1 JavaScript action:
```json
{
  "javascript_actions": [
    {
      "code_excerpt": "app.alert(\"pwn\")",
      "location": "catalog.openaction"
    }
  ],
  "metadata": {
    "diagnostics": [
      "Detected 1 JavaScript action(s) in PDF document. JavaScript was NOT executed."
    ]
  }
}
```

**Logging Observation**: ⚠️ RUST_LOG environment variable does not produce debug output. The pdftract CLI (`main.rs`) does not initialize any logging framework (env_logger, tracing, etc.). While `serve.rs` uses tracing for HTTP server logging, the CLI extraction commands don't implement structured logging.

**Output Captured**:
- JSON extraction result: `/tmp/js_output.json`
- Log capture: `notes/bf-5ucbr-pdftract-debug.log` (27 lines - JSON output only)
- Stderr log: `notes/bf-5ucbr-pdftract-debug-stderr.log` (0 lines - no debug output)

### Key Findings

1. **JavaScript Detection Working**: pdftract successfully identifies JavaScript in PDFs and reports code excerpts and locations
2. **No Execution**: JavaScript is detected but NOT executed (security feature confirmed working)
3. **Logging Gap**: The CLI lacks debug logging infrastructure - RUST_LOG has no effect outside of serve mode
4. **Extraction Success**: The PDF extraction pipeline completes successfully on JavaScript-containing files

## Verification

- ✅ pdftract runs successfully on JavaScript PDF fixture
- ✅ Output captured to files (`notes/bf-5ucbr-pdftract-debug.log`, `/tmp/js_output.json`)
- ✅ Debug logging level (RUST_LOG=debug) was attempted (though not implemented in CLI)
- ✅ Log capture preserved for verification

## Recommendations for Future Debug Logging

If debug logging is needed for CLI commands, consider:
1. Add `env_logger::init()` in `main.rs` conditioned on RUST_LOG environment variable
2. Or migrate CLI to use `tracing` framework (already used in `serve.rs`)
3. Add debug log statements at key extraction pipeline points

## Files Generated

- `notes/bf-5ucbr-pdftract-debug.log` - Combined stdout/stderr capture (JSON output)
- `notes/bf-5ucbr-pdftract-debug-stderr.log` - Stderr-only capture (empty - no debug logs)
- `/tmp/js_output.json` - JSON extraction result (temporary file)
