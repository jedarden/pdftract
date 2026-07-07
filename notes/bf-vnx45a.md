# bf-vnx45a: Manual Verification Examples and Process Pattern Explanations

**Task:** Add manual verification examples and process pattern explanations to orphaned process verification documentation.

**Status:** ✅ COMPLETE

## Summary

Added comprehensive manual verification examples and detailed process pattern explanations to `/home/coding/pdftract/docs/test-hygiene/orphaned-process-verification.md`.

## Changes Made

### 1. Process Pattern Explanations

Added detailed section "Process Pattern Explanations" explaining each default pattern:

- **`pdftract mcp` pattern**: MCP server subprocess
  - What it is: Model Context Protocol server spawned by integration tests
  - Typical spawn patterns (`--stdio`, `--bind`)
  - Why it orphans: Long-lived servers, forgotten shutdown signals, panic before cleanup
  - Detection examples with expected output
  - Manual cleanup commands

- **`TH-0` pattern**: Test harness process (hyphen variant)
  - What it is: Test harness subprocess with hyphenated naming
  - Typical spawn patterns (pdftract CLI subprocess)
  - Why it orphans: Test assertion failure, hung wait(), undrained pipes
  - Detection examples with expected output (including multi-process scenarios)
  - Manual cleanup with `pkill`

- **`TH_0` pattern**: Test harness process (underscore variant)
  - What it is: Test harness with underscore naming (common in fuzz testing)
  - Typical spawn patterns (fuzz harness targets)
  - Why it orphans: Same as TH-0, plus fuzzer-specific scenarios
  - Detection examples with expected output (bulk orphan scenario from fuzz)
  - Manual cleanup (bulk kill)

Each pattern includes:
- Command-line examples with expected clean output
- Examples showing orphaned state with detailed diagnostics
- Manual verification and cleanup commands

### 2. Manual Verification Walkthrough

Added "Manual Verification Walkthrough" section with four detailed scenarios:

**Scenario 1: After a Test Run**
- Step-by-step verification after running `cargo nextest run`
- Shows clean vs orphaned output
- Demonstrates cleanup workflow

**Scenario 2: Before Starting a Test Run**
- Pre-flight check to ensure clean state
- JSON output parsing example
- Conditional cleanup before tests

**Scenario 3: Investigating a Leaking Test**
- Binary search approach to find which test leaves orphans
- Running individual tests and checking after each
- Example of finding `mcp_server_timeout` test as the culprit

**Scenario 4: CI Post-Test Verification**
- Complete bash script for CI integration
- JSON output parsing with jq
- Automated cleanup on failure
- Proper exit codes for CI

### 3. Common Orphan Scenarios

Added "Common Orphan Scenarios" section with five real-world scenarios:

**Scenario 1: Test Timeout Leaves Children Alive**
- Symptom: Test runner killed but children survive
- Example code showing the problem
- Verification output showing parent PPID = 1
- Fix: Use OrphanedProcessGuard RAII pattern

**Scenario 2: Panic Before Cleanup**
- Symptom: Assertion failure prevents cleanup from running
- Example test code that can orphan on panic
- Fix: RAII guard ensures cleanup on panic

**Scenario 3: Undrained Stdio::piped() Blocks wait()**
- Symptom: Process blocked in disk sleep (state D), 0% CPU
- Example code with Stdio::piped() causing pipe buffer block
- Verification showing process state
- Fix: Use Stdio::null() or drain pipes on thread

**Scenario 4: Port Already in Use from Previous Run**
- Symptom: "Address already in use" error
- Timeline showing stale process from 5+ minutes ago
- Fix: Check for orphans at test start, use random ports

**Scenario 5: Fuzz Harness Leaves Target Processes**
- Symptom: 50+ orphaned pdftract processes after fuzz run
- Example from cargo fuzz with Ctrl+C or timeout
- Stale processes from 30+ minutes ago
- Fix: Fuzz harness should trap signals

## Acceptance Criteria Status

- ✅ **Documentation includes manual verification examples**
  - Added 4 detailed walkthrough scenarios covering before/after/investigation/CI
  - Each includes command-line examples with expected output

- ✅ **Explains 'pdftract mcp' pattern (MCP server subprocess)**
  - Dedicated subsection with what/why/detection/cleanup
  - Example outputs for clean and orphaned states
  - Manual verification commands

- ✅ **Explains 'TH-0' pattern (test harness process)**
  - Dedicated subsection with hyphen variant explanation
  - Shows single and multi-process orphan scenarios
  - Manual cleanup with `pkill -f`

- ✅ **Explains 'TH_0' pattern (test process variant)**
  - Dedicated subsection with underscore variant explanation
  - Fuzz-specific context and bulk orphan scenarios
  - Manual cleanup with `pkill -9 -f`

- ✅ **Shows command-line examples with expected output**
  - All three patterns include clean output examples
  - All three patterns include orphaned output examples with diagnostics
  - Walkthrough scenarios show step-by-step command sequences

- ✅ **Lists common scenarios where orphans appear**
  - 5 detailed scenarios with symptoms, examples, verification, and fixes
  - Covers: timeouts, panics, pipe blocking, port conflicts, fuzz harnesses
  - Each includes verification output showing the specific issue

## Verification Commands

Test the documentation:

```bash
# View the updated documentation
cat docs/test-hygiene/orphaned-process-verification.md

# Test the verification script (should work with new patterns)
./scripts/check-orphaned-processes.sh

# Test verbose mode
./scripts/check-orphaned-processes.sh --verbose

# Test JSON output
./scripts/check-orphaned-processes.sh --json | jq .
```

## Files Modified

1. `/home/coding/pdftract/docs/test-hygiene/orphaned-process-verification.md`
   - Added ~270 lines of detailed documentation
   - Three pattern explanation subsections
   - Four walkthrough scenarios
   - Five common orphan scenarios

## Notes

- Documentation follows the existing style and structure
- All examples use realistic commands and outputs based on actual test behavior
- Scenarios cover both interactive manual usage and CI automation
- Each scenario includes "why it happens" explanation to help users understand root causes
- Fix suggestions reference existing patterns (OrphanedProcessGuard, RAII) mentioned elsewhere in the doc
