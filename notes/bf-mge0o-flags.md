# Candidate cargo nextest Timing Flags

**Bead:** bf-4wz6v  
**Task:** Document candidate cargo nextest timing flags identified in bf-52v1t  
**Date:** 2026-07-07  
**Source:** notes/bf-52v1t.md

## Summary

This document summarizes the candidate timing-related flags from cargo nextest research (bf-52v1t), ready for local testing.

## Primary Candidates (Per-Test Timing)

### 1. `--message-format=libtest-json-plus`

**Purpose:** Output test results in structured JSON format including detailed execution time per test  
**Status:** Experimental  
**Syntax:**
```bash
cargo nextest run --message-format=libtest-json-plus
```

**Expected output:** JSON stream with one object per test event, including `duration` field for execution time.

**Use case:** Programmatic consumption of per-test timing data.

---

### 2. `--message-format=libtest-json`

**Purpose:** Output in libtest-compatible JSON format (includes timing)  
**Status:** Experimental  
**Syntax:**
```bash
cargo nextest run --message-format=libtest-json
```

**Expected output:** Similar to libtest-json-plus but without nextest-specific metadata extensions.

**Use case:** Compatibility with libtest JSON consumers.

---

### 3. `--message-format-version=<VERSION>`

**Purpose:** Pin structured output format version for stable parsing  
**Status:** Experimental  
**Syntax:**
```bash
cargo nextest run --message-format=libtest-json-plus --message-format-version=1
```

**Expected behavior:** Ensures JSON schema remains consistent across nextest versions.

**Use case:** Production parsing pipelines needing stability guarantees.

---

## Secondary Candidates (Slow Test Detection)

### 4. `--status-level=slow`

**Purpose:** Display tests that exceed the `slow-timeout` threshold during execution  
**Status:** Stable  
**Syntax:**
```bash
cargo nextest run --status-level=slow
```

**Expected behavior:** Prints test events when tests are marked "slow" (exceeding configured threshold).

**Requires:** `slow-timeout` configured in `.config/nextest.toml` profile section.

---

### 5. `--final-status-level=slow`

**Purpose:** Show slow test summary at end of run  
**Status:** Stable  
**Syntax:**
```bash
cargo nextest run --final-status-level=slow
```

**Expected behavior:** Prints summary of all tests that exceeded `slow-timeout` after completion.

**Requires:** `slow-timeout` configured in `.config/nextest.toml` profile section.

---

## Configuration (Not a Flag, But Critical)

### `slow-timeout` (config file only)

**Purpose:** Define what constitutes a "slow" test  
**File:** `.config/nextest.toml`  
**Syntax:**
```toml
[profile.ci]
slow-timeout = { period = "60s", terminate-after = 3 }
```

**Components:**
- `period` - Threshold before marking test as "slow"
- `terminate-after` - Hard timeout multiplier (period × N)

**Default:** Tests exceeding `period` are tagged as "slow" for status-level flags.

---

## Excluded Flag (Not Per-Test Timing)

### `--timings[=<FMTS>]`

**Purpose:** Generate build timing reports  
**Status:** Unstable  
**Syntax:**
```bash
cargo nextest run --timings=json
cargo nextest run --timings=html
```

**Why excluded:** Despite its name, this flag generates **build compilation timing** (how long cargo spent compiling), NOT per-test execution timing. This is a common point of confusion.

**Scope:** Reports on crate/target compilation duration, not test runtime.

---

## Testing Priority Order

Recommended testing sequence:

1. **`--message-format=libtest-json-plus`** - Primary candidate for per-test timing data
2. **`--message-format-version=1`** - Schema stability verification
3. **`--status-level=slow`** - Slow test detection (requires config setup)
4. **`--final-status-level=slow`** - End-of-run slow test summary
5. **`--message-format=libtest-json`** - Compatibility verification

---

## Acceptance Criteria Status

- ✅ **List of at least 3 candidate timing flags from bf-52v1t** (5 documented)
- ✅ **Each flag has brief description of expected purpose** (provided)
- ✅ **Document saved to notes/bf-mge0o-flags.md** (this file)
