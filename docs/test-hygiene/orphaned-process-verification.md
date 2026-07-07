# Orphaned Process Verification Guide

## Overview

This guide describes the orphaned process verification system for pdftract tests. Per CLAUDE.md test hygiene rules, **no processes should remain after test runs**. This system provides both automated and manual verification of cleanup.

## Problem Statement

Tests that spawn subprocesses (especially MCP servers, test harness processes) can leave orphaned processes if:

- Tests panic before cleanup runs
- A process doesn't exit when stdin closes
- `wait()` blocks indefinitely on a hung child
- Test timeouts kill the test runner but not the spawned processes

Orphaned processes from previous runs can:
- Block new test runs (port already in use)
- Consume system resources
- Cause flaky test behavior
- Violate test isolation assumptions

## Verification Methods

### 1. Shell Script (Manual/CI)

The `scripts/check-orphaned-processes.sh` script provides shell-level verification:

```bash
# Basic check (exits 0 if clean, 1 if orphans found)
./scripts/check-orphaned-processes.sh

# Verbose output
./scripts/check-orphaned-processes.sh --verbose

# JSON output for parsing
./scripts/check-orphaned-processes.sh --json

# Kill any orphans found
./scripts/check-orphaned-processes.sh --kill

# Custom process pattern
./scripts/check-orphaned-processes.sh --pattern "my-custom-process"
```

#### Exit Codes

- `0` - No orphaned processes found (clean state)
- `1` - Orphaned processes found (and not killed)
- `2` - Error occurred (invalid args, command failed, etc.)

#### JSON Output Format

```json
{
  "status": "clean",  // or "orphaned", "cleaned", "partial_cleanup"
  "orphaned_processes": [
    {"pid": "12345", "command": "pdftract mcp --stdio"},
    {"pid": "12346", "command": "TH-0 test_case"}
  ],
  "count": 2
}
```

### 2. Rust Test Helpers (In-Test)

The `test_helpers::process_guard` module provides programmatic verification:

```rust
use pdftract_core::test_helpers::process_guard::{
    verify_no_orphaned_processes,
    OrphanedProcessGuard,
};

#[test]
fn test_mcp_server_cleanup() {
    // Record initial state, verify cleanup on drop
    let _guard = OrphanedProcessGuard::new();

    let mut server = spawn_mcp_stdio();
    // ... test code ...
    drop(server);

    // Verify no orphans remain
    verify_no_orphaned_processes().unwrap();
}
```

### 3. Post-Test Verification (CI Integration)

Add verification step in test scripts or CI workflows:

```bash
# Run tests
cargo nextest run

# Verify no orphans immediately after
./scripts/check-orphaned-processes.sh --kill
```

## Default Process Patterns

The verification system checks for these process patterns by default:

1. `pdftract mcp` - MCP server subprocess
2. `TH-0` - Test harness process (hyphen variant)
3. `TH_0` - Test harness process (underscore variant)

Custom patterns can be specified for tests that spawn other process types.

## Process Pattern Explanations

### `pdftract mcp` Pattern (MCP Server Subprocess)

**What it is:** The Model Context Protocol (MCP) server mode of pdftract, spawned as a subprocess by tests that verify MCP integration.

**Typical spawn pattern:**
```bash
pdftract mcp --stdio
# or
pdftract mcp --bind 127.0.0.1:8080
```

**Why it orphaned:** MCP servers are long-lived processes designed to handle multiple requests. Tests often:
- Forget to send a shutdown signal
- Drop the stdin/stdout pipes without sending termination
- Panic before calling `.kill()` on the child
- Rely on implicit cleanup when the test exits (unreliable)

**Detection example:**
```bash
$ ./scripts/check-orphaned-processes.sh --pattern "pdftract mcp" --verbose
Checking for processes matching: pdftract mcp
✓ No orphaned pdftract mcp processes found
```

**If orphaned:**
```bash
$ ./scripts/check-orphaned-processes.sh --pattern "pdftract mcp" --verbose
Checking for processes matching: pdftract mcp
⚠ Found 1 orphaned process:
  PID 12345: pdftract mcp --stdio
  Age: 45 seconds
  Parent PPID: 1 (orphaned - parent died)
```

**Manual cleanup:**
```bash
# Find and inspect
pgrep -af "pdftract mcp"
# 12345 pdftract mcp --stdio

# Kill gracefully if possible
kill 12345
# Wait 1 second and force if still running
sleep 1
kill -9 12345 2>/dev/null || true
```

### `TH-0` Pattern (Test Harness Process - Hyphen Variant)

**What it is:** A test harness process with a hyphen in the name. The "TH" prefix indicates "Test Harness", typically spawned by integration tests that need to verify the pdftract binary runs correctly as a subprocess.

**Typical spawn pattern:**
```bash
pdftract extract test.pdf --json -
# or
cargo run --bin pdftract -- extract test.pdf
```

**When it appears:** Tests that use `Command::new()` to spawn pdftract as a subprocess and name the test with a `TH-` prefix pattern, or test harness scripts that use hyphenated naming.

**Why it orphaned:**
- Test assertion fails before cleanup
- Command spawned with Stdio::piped() but pipes never drained
- Parent test killed by timeout but subprocess left running
- `wait()` call blocked indefinitely

**Detection example:**
```bash
$ ./scripts/check-orphaned-processes.sh --pattern "TH-0" --verbose
Checking for processes matching: TH-0
✓ No orphaned TH-0 processes found
```

**If orphaned:**
```bash
$ ./scripts/check-orphaned-processes.sh --pattern "TH-0" --verbose
Checking for processes matching: TH-0
⚠ Found 2 orphaned processes:
  PID 12346: pdftract extract tests/fixtures/vector/test.pdf --json -
  PID 12347: pdftract mcp --stdio --bind 127.0.0.1:0
  Total age: 2 minutes 15 seconds
  Parent PPID: 1 (tests died but children survived)
```

**Manual cleanup:**
```bash
# List all matching
pgrep -af "TH-0"
# 12346 pdftract extract tests/fixtures/vector/test.pdf --json -
# 12347 pdftract mcp --stdio --bind 127.0.0.1:0

# Kill all matching
pkill -f "TH-0"
# Verify
pgrep -af "TH-0" || echo "All cleaned up"
```

### `TH_0` Pattern (Test Harness Process - Underscore Variant)

**What it is:** A test harness process with an underscore in the name. Semantically identical to `TH-0` but uses underscore naming convention, which is common in test harnesses that generate process names dynamically (e.g., `TH_01`, `TH_02`, etc.).

**Typical spawn pattern:**
```bash
# Generated by test harness scripts
TH_0 --test-case test_ipv4_loopback --fixture bomb-10k-2g.pdf
```

**When it appears:** Integration tests that use a separate test harness binary or script, particularly in fuzz testing or property-based testing where the harness is named with underscores for readability.

**Why it orphaned:** Same as `TH-0` - test interruption, hung `wait()`, or panic before cleanup. Additionally common with fuzz harnesses that may be killed by the fuzzer but leave the target process running.

**Detection example (clean):**
```bash
$ ./scripts/check-orphaned-processes.sh --pattern "TH_0" --verbose
Checking for processes matching: TH_0
✓ No orphaned TH_0 processes found
```

**If orphaned (fuzz scenario):**
```bash
$ ./scripts/check-orphaned-processes.sh --pattern "TH_0" --verbose
Checking for processes matching: TH_0
⚠ Found 5 orphaned processes:
  PID 12350: TH_0 --fuzz-target lexer --input crash-12345.bin
  PID 12351: TH_0 --fuzz-target lexer --input crash-12346.bin
  PID 12352: TH_0 --fuzz-target xref --input crash-12347.bin
  PID 12353: TH_0 --fuzz-target lexer --input crash-12348.bin
  PID 12354: TH_0 --fuzz-target streams --input crash-12349.bin
  Total age: 15 minutes (stale from previous fuzz run)
  Parent PPID: 1 (fuzzer processes died, targets survived)
```

**Manual cleanup (bulk):**
```bash
# Kill all TH_0 processes at once
pkill -9 -f "TH_0"
# Verify
./scripts/check-orphaned-processes.sh --pattern "TH_0"
✓ No orphaned TH_0 processes found
```

## Manual Verification Walkthrough

### Scenario 1: After a Test Run

**Step-by-step verification after running tests:**

```bash
# 1. Run your tests
cargo nextest run --test-filter mcp

# 2. Immediately check for orphans
./scripts/check-orphaned-processes.sh

# Expected output (clean):
# ✓ No orphaned processes found

# 3. If orphans exist, see details
./scripts/check-orphaned-processes.sh --verbose

# Example output (orphaned):
# ⚠ Found 2 orphaned processes:
#   PID 12360: pdftract mcp --stdio
#   PID 12361: TH-0 test_ipv4_loopback
# 
# Total: 2 processes

# 4. Kill them and verify
./scripts/check-orphaned-processes.sh --kill
# Expected output:
# Killed 2 orphaned processes

# 5. Verify cleanup
./scripts/check-orphaned-processes.sh
# Expected output:
# ✓ No orphaned processes found
```

### Scenario 2: Before Starting a Test Run

**Pre-flight check to ensure clean state:**

```bash
# 1. Check for stale processes from previous runs
./scripts/check-orphaned-processes.sh --json

# Expected JSON output (clean):
# {
#   "status": "clean",
#   "orphaned_processes": [],
#   "count": 0
# }

# 2. If not clean, kill first
if ! ./scripts/check-orphaned-processes.sh; then
    echo "Cleaning up before test run..."
    ./scripts/check-orphaned-processes.sh --kill
fi

# 3. Now run tests with confidence
cargo nextest run
```

### Scenario 3: Investigating a Leaking Test

**Find which test is leaving orphans:**

```bash
# 1. Start with clean slate
./scripts/check-orphaned-processes.sh --kill
./scripts/check-orphaned-processes.sh
# ✓ No orphaned processes found

# 2. Run tests one-by-one until you find the leak
# Example: running individual integration tests
cargo test --test integration_tests mcp_server_startup
./scripts/check-orphaned-processes.sh --verbose
# ✓ No orphaned processes found

cargo test --test integration_tests mcp_server_timeout
./scripts/check-orphaned-processes.sh --verbose
# ⚠ Found 1 orphaned process:
#   PID 12370: pdftract mcp --stdio
# 
# FOUND IT: mcp_server_timeout test leaves orphans

# 3. Inspect the test's cleanup code
# Look for missing ProcessGuard, bare wait(), or panic before cleanup

# 4. Fix the test and verify
cargo test --test integration_tests mcp_server_timeout
./scripts/check-orphaned-processes.sh
# ✓ No orphaned processes found
```

### Scenario 4: CI Post-Test Verification

**Automated verification in CI:**

```bash
#!/usr/bin/env bash
# .ci/scripts/post-test-check.sh

set -euo pipefail

echo "=== Post-test orphaned process verification ==="

# Run verification
RESULT=$(./scripts/check-orphaned-processes.sh --json 2>&1)
EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
    echo "✓ Clean: No orphaned processes detected"
    echo "$RESULT" | jq .
    exit 0
elif [ $EXIT_CODE -eq 1 ]; then
    echo "✗ FAIL: Orphaned processes found!"
    echo "$RESULT" | jq .
    echo ""
    echo "Details:"
    ./scripts/check-orphaned-processes.sh --verbose
    echo ""
    echo "Attempting cleanup..."
    ./scripts/check-orphaned-processes.sh --kill
    exit 1
else
    echo "✗ ERROR: Verification script failed"
    echo "$RESULT"
    exit 2
fi
```

## Common Orphan Scenarios

### Scenario 1: Test Timeout Leaves Children Alive

**Symptom:** Test suite runs with `cargo nextest run` or `timeout`, test exceeds time limit, test runner killed but spawned processes survive.

**Example:**
```bash
# Test spawns a server
let server = Command::new("pdftract")
    .arg("mcp")
    .arg("--stdio")
    .spawn()?;

# Test takes too long, cargo nextest kills it
# Server process continues running
```

**Verification:**
```bash
$ ./scripts/check-orphaned-processes.sh --pattern "pdftract mcp" --verbose
⚠ Found 1 orphaned process:
  PID 12380: pdftract mcp --stdio
  Age: 5 seconds (recent - likely from timeout)
  Parent PPID: 1 (parent was killed)
```

**Fix:** Use `OrphanedProcessGuard` RAII pattern to ensure cleanup on drop/panic.

### Scenario 2: Panic Before Cleanup

**Symptom:** Test code panics after spawning a process but before cleanup code runs.

**Example:**
```rust
#[test]
fn test_something() {
    let child = Command::new("pdftract")
        .arg("mcp")
        .spawn()
        .unwrap();
    
    // Some test code that might panic
    assert!(some_condition);
    
    // Cleanup never runs if assertion fails
    child.kill().unwrap();
    child.wait().unwrap();
}
```

**Verification:**
```bash
$ ./scripts/check-orphaned-processes.sh --verbose
⚠ Found 1 orphaned process:
  PID 12381: pdftract mcp --stdio
  Age: Variable (depends on when test run)
```

**Fix:** Use RAII guard - cleanup runs even on panic.

### Scenario 3: Undrained Stdio::piped() Blocks wait()

**Symptom:** Long-running server with `Stdio::piped()` fills stdout/stderr buffer, process blocks, `wait()` never returns.

**Example:**
```rust
let child = Command::new("pdftract")
    .arg("mcp")
    .arg("--stdio")
    .stdin(Stdio::piped())   // Server writes to stdout
    .stdout(Stdio::piped())  // but nobody reads it
    .stderr(Stdio::piped())
    .spawn()?;

// This blocks forever if pipe fills
child.wait().unwrap();
```

**Verification:**
```bash
$ ./scripts/check-orphaned-processes.sh --verbose
⚠ Found 1 orphaned process:
  PID 12382: pdftract mcp --stdio
  State: D (disk sleep - waiting for I/O)
  CPU: 0% (blocked on pipe buffer)
```

**Fix:** Use `Stdio::null()` for servers, or drain pipes on a background thread.

### Scenario 4: Port Already in Use from Previous Run

**Symptom:** New test fails with "Address already in use" error, previous test's MCP server still running.

**Example:**
```bash
# First test run
cargo test mcp_server
# Test spawns pdftract mcp --bind 127.0.0.1:8080
# Test panics, server not killed

# Second test run (minutes later)
cargo test mcp_server
# FAIL: AddressAlreadyIn use - port 8080 still bound
```

**Verification:**
```bash
$ ./scripts/check-orphaned-processes.sh --pattern "pdftract mcp" --verbose
⚠ Found 1 orphaned process:
  PID 12383: pdftract mcp --bind 127.0.0.1:8080
  Age: 5 minutes 20 seconds (stale)
  Listening ports: 127.0.0.1:8080
```

**Fix:** Check for orphans at test start, use random ports (`:0`), or enforce cleanup.

### Scenario 5: Fuzz Harness Leaves Target Processes

**Symptom:** Fuzzer crashes or is killed, target pdftract processes continue running in background.

**Example:**
```bash
# Fuzzing runs
cargo fuzz run lexer -- -max_total_time=300
# Fuzzer killed (Ctrl+C or timeout)

# Target processes still running
pgrep -af "pdftract"
# 12390 pdftract /tmp/fuzz-input-12345.bin
# 12391 pdftract /tmp/fuzz-input-12346.bin
# 12392 pdftract /tmp/fuzz-input-12347.bin
```

**Verification:**
```bash
$ ./scripts/check-orphaned-processes.sh --pattern "pdftract" --verbose
⚠ Found 50+ orphaned processes:
  PIDs 12390-12440: pdftract /tmp/fuzz-input-*.bin
  Total age: 30+ minutes (stale fuzz run)
```

**Fix:** Fuzz harness should trap signals and kill children on exit.

## Best Practices

### 1. Use RAII Guards for Process Spawning

Always wrap spawned child processes in RAII guards:

```rust
struct ProcessGuard {
    child: Option<Child>,
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // Graceful shutdown first
            let _ = child.stdin.take();
            
            // Wait with bounded timeout
            let start = Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if start.elapsed() >= Duration::from_millis(200) => {
                        // Force kill if graceful shutdown fails
                        let _ = child.kill();
                        break;
                    }
                    _ => thread::sleep(Duration::from_millis(10)),
                }
            }
        }
    }
}
```

### 2. Verify at Test Boundaries

Check for orphans at these points:

- **After each test** that spawns processes
- **After test suite completion** (in CI)
- **Before starting** new test runs (detect stale orphans)

### 3. Use Timeouts on All Waits

Never use bare `child.wait()` - always use bounded waits:

```rust
// BAD - may block forever
child.wait();

// GOOD - bounded timeout
wait_with_timeout(&mut child, 1000)?;
```

### 4. Give Children Stdio::null() for Long-Running Servers

Servers that live beyond a single request should drain pipes or use null:

```rust
Command::new("pdftract")
    .arg("mcp")
    .arg("--stdio")
    .stdin(Stdio::null())  // Prevents pipe-full blocking
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()?;
```

## CI Integration Example

Add to `.ci/scripts/post-test-check.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Checking for orphaned processes after test run..."

if ./scripts/check-orphaned-processes.sh --json; then
    echo "✓ No orphaned processes found"
    exit 0
else
    echo "⚠ Orphaned processes detected!"
    ./scripts/check-orphaned-processes.sh --kill --verbose
    exit 1
fi
```

## Troubleshooting

### "Orphaned processes found" errors

1. **Identify the processes**:
   ```bash
   ./scripts/check-orphaned-processes.sh --verbose
   ```

2. **Check if they're legitimate** (running from other work):
   ```bash
   ps aux | grep -E "pdftract|TH-0"
   ```

3. **Kill if truly orphaned**:
   ```bash
   ./scripts/check-orphaned-processes.sh --kill
   ```

4. **Find the leaking test**:
   - Run tests individually until one leaves orphans
   - Check the test's ProcessGuard implementation
   - Verify the test doesn't panic before cleanup

### "pgrep command failed" errors

The verification system requires `pgrep` to be installed:

```bash
# On Debian/Ubuntu/NixOS
pgrep --version  # Should show procps or similar

# If missing, install:
# apt install procps  # Debian/Ubuntu
# nix-shell -p procps  # NixOS
```

## References

- CLAUDE.md Test Hygiene Rules
- Post-Test Integration Documentation: `docs/test-hygiene/post-test-orphan-verification-integration.md` (CI workflow integration details)
- Bead bf-5xh7g: Orphaned process verification implementation
- Bead bf-119ys: TH-03 process cleanup with RAII guards
