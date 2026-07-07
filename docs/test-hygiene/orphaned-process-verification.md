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
- Bead bf-5xh7g: Orphaned process verification implementation
- Bead bf-119ys: TH-03 process cleanup with RAII guards
