# Troubleshooting Orphaned Processes

This guide provides systematic troubleshooting procedures for investigating, cleaning up, and preventing orphaned processes in pdftract test runs.

## Table of Contents

1. [Quick Diagnosis Flowchart](#quick-diagnosis-flowchart)
2. [Investigation Procedures](#investigation-procedures)
3. [Cleanup Commands and Safety](#cleanup-commands-and-safety)
4. [Force-Kill Warnings](#force-kill-warnings)
5. [Verification Procedures](#verification-procedures)
6. [Escalation Paths](#escalation-paths)
7. [Prevention Strategies](#prevention-strategies)

## Quick Diagnosis Flowchart

```
Are orphaned processes suspected?
│
├─ No → Run verification: ./scripts/check-orphaned-processes.sh
│         └─ Clean? → Done
│         └─ Orphans found → Continue investigation
│
└─ Yes → Start investigation
          │
          ├─ Identify processes (pgrep)
          │   └─ No matches → False alarm, done
          │   └─ Found → Check legitimacy
          │
          ├─ Are they legitimate? (ps aux, age)
          │   ├─ Yes (your work) → Note, no action
          │   └─ No (stale) → Proceed to cleanup
          │
          ├─ Attempt graceful cleanup (kill)
          │   ├─ Success → Verify
          │   └─ Still running → Force cleanup (kill -9)
          │
          └─ Still persistent? → Escalate
```

## Investigation Procedures

### Step 1: Confirm Orphan Existence

**Run the detection script:**

```bash
./scripts/check-orphaned-processes.sh --verbose
```

**Expected outputs:**

- **Clean:** `✓ No orphaned processes found`
- **Orphans:** `⚠ Found N orphaned process(es)` with details

**If script fails:**

```bash
# Manual check using pgrep
pgrep -af "pdftract mcp|TH_0|TH_0"
```

**Interpret results:**

| Output | Meaning | Action |
|--------|---------|--------|
| No output | No matching processes | Done - system clean |
| List with PIDs | Orphaned processes found | Continue to Step 2 |
| "pgrep: command not found" | Missing dependency | Install `procps` package |

### Step 2: Analyze Process Details

**Get full process information:**

```bash
# For each PID found
ps -fp <PID>

# Example output:
# UID   PID  PPID  C STIME TTY      STAT   TIME COMMAND
# 1001 1234     1  0 10:30 pts/0   S+     0:01 pdftract mcp --stdio
```

**Key fields to examine:**

- **PPID (Parent PID):**
  - `1` = Parent died (true orphan)
  - Other value = Parent alive (check if legitimate)

- **STAT (State):**
  - `S` = Sleeping (normal for waiting server)
  - `D` = Uninterruptible sleep (blocked on I/O - may need force kill)
  - `Z` = Zombie (waiting for parent to reap - rare issue)

- **TIME (CPU time):**
  - Low (`0:00-0:05`) = Recent spawn
  - High (`1:00+`) = Long-running (likely stale)

**Check process age:**

```bash
# Get elapsed time
ps -eo pid,etime,cmd | grep -E "pdftract mcp|TH-0|TH_0"

# Example:
# 1234  00:05:12 pdftract mcp --stdio    # 5 minutes old
# 5678  02:30:00 TH_0 test_ipv4        # 2.5 hours old (stale!)
```

### Step 3: Identify Legitimate Processes

**Not all matching processes are orphans. Check if:**

1. **You're actively running tests:**
   ```bash
   # Check if cargo test is running
   pgrep -af cargo test
   ```

2. **You're manually running pdftract:**
   ```bash
   # Check for your user's pdftract processes
   ps -u $USER -o pid,cmd | grep pdftract
   ```

3. **Processes are recent (< 1 minute old):**
   ```bash
   # Filter by age
   ps -eo pid,etime,cmd | awk '$2 ~ /^00:[0-5]/' | grep -E "pdftract|TH-"
   ```

**Decision matrix:**

| Process Age | Parent Alive | Your Test Running? | Verdict |
|-------------|--------------|-------------------|---------|
| < 1 min | Yes | Yes | Legitimate - ignore |
| < 1 min | Yes | No | Suspicious - monitor |
| < 1 min | No (1) | Yes/No | **Orphan - cleanup** |
| > 5 min | Any | Any | **Stale orphan - cleanup** |

### Step 4: Identify the Leaking Test

**If orphans are confirmed, find which test is responsible:**

```bash
# Start from clean slate
./scripts/check-orphaned-processes.sh --kill

# Run tests individually
cargo test --test <test_name> -- --test-threads=1
./scripts/check-orphaned-processes.sh --verbose

# Repeat until you find the leaking test
```

**Common culprits:**

1. **MCP server tests** (`pdftract mcp`):
   - Check tests in `crates/pdftract-core/tests/*.rs` matching `mcp`
   - Look for missing `ProcessGuard` or bare `wait()`

2. **TH-0/TH_0 harness tests**:
   - Check integration tests spawning subprocesses
   - Look for `Command::new("pdftract")` without cleanup

3. **Fuzz tests**:
   - Check if fuzz harness traps signals
   - Verify target processes are reaped on exit

## Cleanup Commands and Safety

### Graceful Shutdown (First Attempt)

**Signal sequence (try in order):**

1. **SIGTERM (signal 15) - Normal shutdown:**
   ```bash
   # Single process
   kill <PID>
   
   # All matching processes
   pkill -TERM -f "pattern"
   ```

2. **SIGINT (signal 2) - Interrupt (like Ctrl+C):**
   ```bash
   # If SIGTERM didn't work
   kill -INT <PID>
   ```

3. **Wait and verify:**
   ```bash
   # Wait up to 2 seconds
   sleep 2
   
   # Check if gone
   ps -p <PID> || echo "Process terminated"
   ```

**Expected behavior:**

- Well-behaved processes (with signal handlers) will:
  1. Catch SIGTERM
  2. Close connections/ports
  3. Flush buffers
  4. Exit cleanly

- Servers with open connections may take 1-2 seconds to drain

### Force Kill (Last Resort)

⚠️ **WARNING:** See [Force-Kill Warnings](#force-kill-warnings) below.

**When to use force kill:**

- Process doesn't respond to SIGTERM after 2-3 seconds
- Process is in uninterruptible sleep (`D` state)
- Process is a zombie (`Z` state)
- Immediate cleanup needed (emergency)

**Commands:**

```bash
# SIGKILL (signal 9) - Immediate termination
kill -9 <PID>

# Or using pkill
pkill -9 -f "pattern"

# With killall (all processes by name)
killall -9 pdftract
```

**What SIGKILL does:**

- Process is terminated immediately by the kernel
- No signal handlers are executed
- No cleanup code runs
- Child processes become orphaned (reparented to init)
- No file buffers are flushed

**Verification after force kill:**

```bash
# Check process is gone
ps -p <PID> && echo "STILL RUNNING" || echo "Terminated"

# Check for orphaned children
ps -o pid,ppid,cmd | awk '$2 == 1 && /pdftract/ {print $0}'
```

## Force-Kill Warnings

### Data Loss Risks

**Force-killing can cause:**

1. **Unflushed buffers:**
   - stdout/stderr buffers lost
   - File writes incomplete
   - Test results not saved

2. **Port leakage:**
   - TCP ports left in TIME_WAIT state
   - May block subsequent tests for 30-60 seconds
   - Check with: `ss -tlnp | grep <PID>`

3. **Resource leakage:**
   - Temporary files not cleaned up
   - Shared memory segments left allocated
   - Semaphores left locked

### When Force-Kill is Necessary

**Acceptable use cases:**

✅ Process is in uninterruptible sleep (`D` state)
✅ Process is a zombie (`Z` state)  
✅ Process is known to be stuck (e.g., infinite loop, deadlock)
✅ Emergency cleanup needed (CI timeout, system overload)
❌ Process is still working (just slow)
❌ Process is legitimately doing work (your manual run)
❌ First attempt at cleanup (try graceful first)

### Mitigating Force-Kill Risks

**Best practices:**

1. **Always try graceful first:**
   ```bash
   # Proper sequence
   kill <PID> && sleep 2 && kill -9 <PID> 2>/dev/null
   ```

2. **Check for child processes first:**
   ```bash
   # Find children before killing parent
   pstree -p <PID>
   
   # Kill children first to prevent re-parenting issues
   pkill -P <PID>
   kill -9 <PID>
   ```

3. **Clean up resources after force kill:**
   ```bash
   # Check for leaked ports
   ss -tlnp | grep :8080 | awk '{print $4}' | xargs -I{} fuser -k {} 2>/dev/null
   
   # Check for temp files (adjust path as needed)
   find /tmp -name "pdftract-*" -user $USER -mtime +0 -delete
   ```

## Verification Procedures

### Immediate Verification

**Right after cleanup:**

```bash
# 1. Run the verification script
./scripts/check-orphaned-processes.sh --verbose

# Expected:
# ✓ No orphaned processes found

# 2. Manual double-check
pgrep -af "pdftract mcp|TH_0|TH_0" || echo "No matching processes"

# 3. Check ports (if servers were running)
ss -tlnp | grep -E "8080|3000" || echo "No leaked ports"
```

### Post-Cleanup Validation

**Ensure system state is clean:**

```bash
# Check for zombie processes
ps aux | grep -c Z

# Should be 0 or very low (1-2). If > 5, investigate:
ps aux | awk '$8 ~ /Z/ {print $2, $11, $12}'

# Check for uninterruptible sleep
ps aux | awk '$8 ~ /D/ {print $2, $11, $12}'

# Should be 0 for pdftract processes
```

### Automated Verification in Tests

**Add to test teardown:**

```rust
#[test]
fn test_with_verification() {
    let _guard = OrphanedProcessGuard::new();
    
    // ... test code ...
    
    // Automatic verification on drop
}
```

### CI Verification

**Already integrated in CI:**

- Runs after every test suite
- See `docs/test-hygiene/post-test-orphan-verification-integration.md`
- Exits with code 1 if orphans detected (causes CI failure)

## Escalation Paths

### Level 1: Standard Cleanup (Automated)

**Trigger:** Verification script finds orphans

**Actions:**
```bash
# Automated cleanup in CI
./scripts/check-orphaned-processes.sh --kill

# Manual cleanup locally
./scripts/check-orphaned-processes.sh --verbose
./scripts/check-orphaned-processes.sh --kill
./scripts/check-orphaned-processes.sh  # Verify
```

**Success criteria:**
- Verification script returns exit code 0
- Manual `pgrep` shows no matching processes

### Level 2: Force Kill (Manual Intervention)

**Trigger:** Standard cleanup fails, processes still running

**Actions:**
```bash
# Identify stuck processes
pgrep -af "pdftract mcp|TH_0|TH_0"

# Check their state
ps -fp <PID>

# If in D or Z state, or simply won't die:
kill -9 <PID>

# Or all matching
pkill -9 -f "pattern"

# Verify
pgrep -af "pattern" || echo "Clean"
```

**Warning:** See [Force-Kill Warnings](#force-kill-warnings)

### Level 3: Stubborn Processes (Advanced)

**Trigger:** Force kill doesn't work, process respawns, or system issues

**Symptoms:**
```bash
# Process comes back after kill -9
kill -9 <PID>
sleep 1
ps -p <PID>  # Still running!

# Multiple instances appearing
pgrep -cf "pattern"  # Count keeps increasing
```

**Potential causes:**
1. **Respawn mechanism** (supervisor, systemd, init script)
2. **Fork bomb** (process spawning new children)
3. **Kernel resource issue** (unreapable zombies)

**Investigation:**
```bash
# Check parent process
ps -fp <PID>  # Look at PPID

# If PPID != 1 and not your shell, investigate parent
ps -fp <PPID>

# Check for respawners
systemctl status <service>  # If systemd-managed
pstree -s <PID>  # Show spawning tree

# Check for fork bombs
pstree -p <PID> | wc -l  # If > 100, possible fork bomb
```

**Resolution options:**

1. **Kill the parent first:**
   ```bash
   # Kill the spawner
   kill -9 <PPID>
   
   # Then kill children
   pkill -9 -f "pattern"
   ```

2. **Stop the service:**
   ```bash
   # If systemd-managed
   sudo systemctl stop <service-name>
   sudo systemctl disable <service-name>
   ```

3. **System-level cleanup (last resort):**
   ```bash
   # ⚠️ DANGER: Kills ALL pdftract processes system-wide
   sudo killall -9 pdftract
   
   # Clean up any user-specific resources
   sudo ipcrm -M <shm-id>  # Remove shared memory if needed
   ```

### Level 4: Kernel/System Issues (Escalate to Admin)

**Trigger:** Processes cannot be killed even by root

**Symptoms:**
```bash
# Even as root:
sudo kill -9 <PID>
# Process still running

# Task state shows uninterruptible sleep
cat /proc/<PID>/status | grep State
# State:	D (disk sleep)

# I/O wait showing process stuck on NFS/dead mount
ps aux | awk '$8 ~ /D/ {print $2, $3, $11}'
```

**Immediate actions:**
```bash
# 1. Don't force reboot yet - try these first:

# Check if stuck on NFS/dead mount
df -h
mount | grep nfs

# If so, unmount the stale filesystem
sudo umount -f /stale/mount/point

# 2. Check for hung block device
lsof /dev/<device>
# If pdftract has it open, close the file descriptor
```

**When to reboot:**
- Multiple processes in `D` state
- System load climbing
- Other services affected
- No non-destructive options left

** escalation path:**
1. Document findings (syslog, dmesg, process state)
2. Contact system administrator
3. May need: system reboot, filesystem check, or hardware diagnostic

## Prevention Strategies

### Code-Level Prevention

**1. Always use RAII guards:**

```rust
// GOOD - RAII ensures cleanup
#[test]
fn test_mcp_cleanup() {
    let _guard = OrphanedProcessGuard::new();
    let server = spawn_mcp();
    // ... test code ...
    // Cleanup happens automatically
}

// BAD - Manual cleanup can be skipped
#[test]
fn test_mcp_cleanup() {
    let server = spawn_mcp();
    // ... test code ...
    server.kill().unwrap();  // Might not run on panic
}
```

**2. Use bounded timeouts:**

```rust
// GOOD - Timeout prevents hangs
wait_with_timeout(&mut child, Duration::from_secs(5))?;

// BAD - May hang forever
child.wait()?;
```

**3. Drain pipes for long-running processes:**

```rust
// GOOD - Prevents pipe block
Command::new("pdftract")
    .arg("mcp")
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()?;

// BAD - May block when pipe fills
Command::new("pdftract")
    .arg("mcp")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;
```

### Test-Level Prevention

**1. Verify at test boundaries:**

```rust
#[test]
fn test_with_verification() {
    let guard = OrphanedProcessGuard::new();
    // ... test code ...
    // Verification happens on drop
}
```

**2. Use random ports:**

```rust
// GOOD - No port conflicts
let listener = TcpListener::bind("127.0.0.1:0")?;
let port = listener.local_addr().port()?;

// BAD - May conflict with previous run
let listener = TcpListener::bind("127.0.0.1:8080")?;
```

**3. Pre-flight cleanup:**

```bash
# In test scripts, clean first
./scripts/check-orphaned-processes.sh --kill
cargo test
```

### CI-Level Prevention

**1. Always run verification after tests:**

```yaml
# In CI workflow
- name: Run tests
  run: cargo nextest run
  
- name: Verify no orphans
  run: ./scripts/check-orphaned-processes.sh --kill
```

**2. Use cgroups for resource limits:**

```yaml
# Limits prevent runaway processes
- name: Run with cgroup
  run: |
    systemd-run --scope -p MemoryMax=4G \
      cargo nextest run
```

**3. Set test timeouts:**

```toml
# In .config/nextest.toml
[profile.ci]
slow-timeout = "60s"
terminate-after = "120s"
```

## Quick Reference Commands

### Detection

```bash
# Run verification script
./scripts/check-orphaned-processes.sh --verbose

# Manual check
pgrep -af "pdftract mcp|TH_0|TH_0"

# With full details
ps -fp $(pgrep -d "," "pdftract mcp|TH_0|TH_0")

# Check ports
ss -tlnp | grep -E "8080|3000"
```

### Cleanup

```bash
# Graceful (try first)
kill <PID>

# All matching
pkill -f "pattern"

# Force kill (if needed)
kill -9 <PID>
pkill -9 -f "pattern"

# With verification
./scripts/check-orphaned-processes.sh --kill
./scripts/check-orphaned-processes.sh  # Verify
```

### Investigation

```bash
# Process tree
pstree -p <PID>

# Open files
lsof -p <PID>

# Network connections
ss -tnp | grep <PID>

# Kernel state
cat /proc/<PID>/status
cat /proc/<PID>/stack  # If stuck
```

## Related Documentation

- **Orphaned Process Verification:** `orphaned-process-verification.md`
- **CI Integration:** `post-test-orphan-verification-integration.md`
- **Test Hygiene Rules:** `CLAUDE.md` section "Test hygiene — never let a hung test stall the loop"

## Implementation Date

2026-07-07

## Implemented By

Bead `bf-md1uka`: Add troubleshooting section and cleanup procedures
