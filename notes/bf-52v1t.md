# Research Findings: cargo nextest Timing Output Flags

**Bead:** bf-52v1t  
**Task:** Research cargo nextest CLI reference and documentation to identify all flags related to per-test timing data output.  
**Date:** 2026-07-07

## Summary

Research completed. Identified all timing-related flags in cargo nextest CLI and documentation. Findings are ready for local testing.

## Identified Timing-Related Flags

### 1. Build Timing Output

**Flag:** `--timings[=<FMTS>]`  
**Status:** Unstable  
**Purpose:** Generate build timing reports (NOT per-test timing)  
**Syntax:**
```bash
cargo nextest run --timings=html
cargo nextest run --timings=json
cargo nextest run --timings=html,json
```

**Formats:** `html`, `json` (comma-separated)  
**Scope:** This generates build-time timing reports, not per-test execution timing. It's about compilation duration, not test runtime.

---

### 2. Test Status Visibility Flags

These flags control which test results are displayed, including timing information for slow tests.

#### 2.1 `--status-level <LEVEL>`

**Purpose:** Control which test statuses are output during the run  
**Env var:** `NEXTEST_STATUS_LEVEL`  
**Values:**
- `none` - No status output during run
- `fail` - Only failed tests
- `retry` - Retry attempts
- `slow` - **Slow tests (timing-related)**
- `leak` - Memory leaks
- `pass` - Passed tests
- `all` - All statuses

**Syntax:**
```bash
cargo nextest run --status-level=slow
cargo nextest run --status-level=all
```

**Note:** The `slow` value specifically controls display of tests that exceed the configured `slow-timeout` threshold.

#### 2.2 `--final-status-level <LEVEL>`

**Purpose:** Control which test statuses are output at the END of the run  
**Env var:** `NEXTEST_FINAL_STATUS_LEVEL`  
**Values:**
- `none` - No final status output
- `fail` - Only failed tests
- `flaky` - Flaky tests
- `slow` - **Slow tests (timing-related)**
- `skip` - Skipped tests
- `pass` - Passed tests
- `all` - All statuses

**Syntax:**
```bash
cargo nextest run --final-status-level=slow
cargo nextest run --final-status-level=all
```

---

### 3. Machine-Readable Output with Timing

#### 3.1 `--message-format <FORMAT>`

**Purpose:** Output test results in structured format (includes timing data)  
**Status:** Experimental  
**Env var:** `NEXTEST_MESSAGE_FORMAT`  
**Values:**
- `human` - Default human-readable format
- `libtest-json` - **Same format as libtest (includes timing)**
- `libtest-json-plus` - **libtest format + nextest metadata (includes detailed timing)**

**Syntax:**
```bash
cargo nextest run --message-format=libtest-json
cargo nextest run --message-format=libtest-json-plus
```

**Timing data included:** Both JSON formats include execution time per test.

#### 3.2 `--message-format-version <VERSION>`

**Purpose:** Pin structured output format version for stability  
**Status:** Experimental  
**Env var:** `NEXTEST_MESSAGE_FORMAT_VERSION`  
**Syntax:**
```bash
cargo nextest run --message-format=libtest-json --message-format-version=1
```

**Use case:** Ensures consistent parsing across nextest versions when consuming JSON output programmatically.

---

### 4. Slow Test Threshold (Config File)

**Not a CLI flag, but critical for timing behavior.**

**Config file:** `.config/nextest.toml` (profile section)  
**Setting:** `slow-timeout`  
**Purpose:** Defines what constitutes a "slow" test  

**Syntax:**
```toml
[profile.ci]
slow-timeout = { period = "60s", terminate-after = 3 }
```

**Components:**
- `period` - Threshold time before a test is marked "slow"
- `terminate-after` - Multiplier for hard timeout (kills test after period × N)

**Default behavior:** Tests exceeding `period` are tagged as "slow" and displayed when `--status-level=slow` or `--final-status-level=slow` is set.

---

### 5. Additional Reporter Options

#### 5.1 `--show-progress <SHOW_PROGRESS>`

**Purpose:** Control progress display (affects timing visibility)  
**Env var:** `NEXTEST_SHOW_PROGRESS`  
**Values:**
- `auto` - Auto-detect based on TTY
- `none` - No progress display
- `bar` - Progress bar with running tests
- `counter` - Counter per completed test
- `only` - **Progress bar only, hides successful output (shows slow tests)**

**Syntax:**
```bash
cargo nextest run --show-progress=only
```

**Note:** The `only` value is equivalent to `--show-progress=bar --status-level=slow --final-status-level=none`, which surfaces slow timing information.

#### 5.2 `--max-progress-running <N>`

**Purpose:** Limit how many running tests display in progress bar  
**Syntax:**
```bash
cargo nextest run --max-progress-running=10
```

---

## Complete Flag Reference Table

| Flag | Purpose | Status | Timing Scope | Example |
|------|---------|--------|---------------|---------|
| `--timings[=<FMTS>]` | Build timing reports | Unstable | **Build time**, not test time | `--timings=json` |
| `--status-level` | Status visibility during run | Stable | **Marks slow tests** | `--status-level=slow` |
| `--final-status-level` | Status visibility at end | Stable | **Shows slow test summary** | `--final-status-level=slow` |
| `--message-format` | Structured output | Experimental | **Includes execution time** | `--message-format=libtest-json-plus` |
| `--message-format-version` | Pin JSON schema version | Experimental | **Stable parsing** | `--message-format-version=1` |
| `--show-progress` | Progress display mode | Stable | **Surface slow tests** | `--show-progress=only` |
| `slow-timeout` (config) | Define slow threshold | Stable | **Slow test definition** | `slow-timeout = { period = "60s" }` |

---

## Key Findings for Testing Strategy

1. **For per-test execution timing:** Use `--message-format=libtest-json-plus` to get structured JSON with timing data for each test.

2. **For slow test detection:** Configure `slow-timeout` in profile, then use `--status-level=slow` or `--final-status-level=slow` to surface tests exceeding the threshold.

3. **For stable programmatic consumption:** Combine `--message-format=libtest-json-plus` with `--message-format-version=N` to ensure schema stability across runs.

4. **The `--timings` flag is misleading:** Despite its name, `--timings` generates build compilation timing reports, NOT per-test execution timing. This is a common point of confusion.

## Next Steps

These findings are ready for local testing. Recommended test sequence:

1. Test `--message-format=libtest-json-plus` output structure
2. Verify `slow-timeout` threshold with `--status-level=slow`
3. Confirm `--message-format-version` produces stable schema across runs
4. Validate that `--timings` produces build timing (not test timing)

---

**Acceptance Criteria Status:**

- ✅ **List of candidate timing-related flags compiled** (7 total)
- ✅ **Flag purpose and syntax documented** (comprehensive table above)
- ✅ **Findings ready for local testing** (next steps outlined)
