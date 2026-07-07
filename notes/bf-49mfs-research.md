# Cargo Nextest Timing Output Formats Research

**Bead:** bf-49mfs (split from bf-35gxz)
**Date:** 2026-07-06
**Status:** Complete

## Overview

This document catalogs the available cargo nextest output formats that support timing data, as research prerequisite to selecting the right format for parseable timing data.

## Available Output Formats Supporting Timing Data

### 1. JUnit XML (Primary Format)

**Status:** Stable, production-ready

**Description:** The main mechanism for machine-readable test run output in nextest. Adheres to the Jenkins XML format for JUnit/XUnit.

**Configuration:** Add to `.config/nextest.toml`:
```toml
[profile.ci.junit]
path = "junit.xml"
```

**Timing fields:**
- `time` attribute on `<testsuites>` root: Total execution time in seconds
- `time` attribute on each `<testcase>`: Individual test execution time in seconds
- `timestamp` attribute on each `<testcase>`: ISO 8601 timestamp when test started

**Example:**
```xml
<testsuites name="nextest-run" tests="3" failures="1" errors="0" 
             uuid="45c50042-482e-477e-88a2-60cfcc3eaf95" 
             timestamp="2024-01-09T07:50:12.664+00:00" time="0.023">
    <testsuite name="fixture-project::basic" tests="3" disabled="0" errors="0" failures="1">
        <testcase name="test_cwd" classname="fixture-project::basic" 
                   timestamp="2024-01-09T07:50:12.665+00:00" time="0.004">
        </testcase>
        <testcase name="test_failure_assert" classname="fixture-project::basic" 
                   timestamp="2024-01-09T07:50:12.665+00:00" time="0.004">
            <failure type="test failure">...</failure>
        </testcase>
    </testsuite>
</testsuites>
```

**Additional configuration options:**
- `report-name`: Name of the report (default: "nextest-run")
- `store-success-output`: Include stdout/stderr for passing tests (default: false)
- `store-failure-output`: Include stdout/stderr for failing tests (default: true)
- `flaky-fail-status`: How to report flaky-fail tests ("failure" or "success", default: "failure")

**Output location:** `target/nextest/<profile-name>/junit.xml` within workspace root

**References:**
- [JUnit support - cargo-nextest](https://nexte.st/docs/machine-readable/junit/)

### 2. Libtest JSON (Experimental)

**Status:** Experimental, requires opt-in via environment variable

**Description:** Libtest-compatible JSON output for test runs, primarily for compatibility with existing test infrastructure that consumes this format.

**Enable with:** `NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1`

**Usage:**
```bash
NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run --message-format <format>
```

**Format variants:**
- `libtest-json`: Produce output similar to unstable libtest JSON
- `libtest-json-plus`: Same as libtest-json, with an extra `nextest` field containing additional metadata

**Versioning:**
- Specify version via `--message-format-version <version>`
- Current version: `0.1` (as of 2023-12)
- Version tracks the unstable upstream libtest JSON format

**Stability policy:**
- While experimental: Format may change to fix issues or track upstream changes
- After stabilization: Format changes will be accompanied by version number bumps (e.g., 0.1 → 0.2 → 1.0)

**Note:** The format specification is currently marked as TODO in the documentation, indicating it's a work in progress.

**References:**
- [Libtest JSON output - cargo-nextest](https://nexte.st/docs/machine-readable/libtest-json/)
- [Tracking issue #1152](https://github.com/nextest-rs/nextest/issues/1152)

### 3. Test List JSON (For Listing Only, Not Runs)

**Status:** Stable

**Description:** Machine-readable output for test *lists* (not test *runs*). Used to discover available tests without executing them.

**Usage:**
```bash
cargo nextest list --message-format json
```

**Timing support:** Does NOT include timing data (tests are not executed)

**References:**
- [Listing tests - cargo-nextest](https://nexte.st/docs/listing/)

## Human-Readable Timing Output

**Default terminal output:**
- Tests are marked PASS or FAIL
- Wall-clock execution time shown in square brackets after each test name
- Example: `tests/test_basic.rs:test_success ... ok (0.004s)`

**Not parseable:** This is for human consumption only; no stable machine-readable format.

## Recommendations

For parseable timing data:

1. **Use JUnit XML** for production use - stable, well-documented, widely understood by test analysis tools
2. **Consider Libtest JSON** only if compatibility with existing libtest JSON consumers is required, with acceptance of experimental status
3. **Avoid relying on human-readable terminal output** for programmatic consumption

## Sources

- [About output formats - cargo-nextest](https://nexte.st/docs/machine-readable/)
- [JUnit support - cargo-nextest](https://nexte.st/docs/machine-readable/junit/)
- [Libtest JSON output - cargo-nextest](https://nexte.st/docs/machine-readable/libtest-json/)
- [Running tests - cargo-nextest](https://nexte.st/docs/running/)
- [Path for stabilizing libtest's json output? - Rust Internals](https://internals.rust-lang.org/t/path-for-stabilizing-libtests-json-output/20163)
