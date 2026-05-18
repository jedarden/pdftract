# SDK Conformance Test Runner Pattern

This document describes the conformance test runner pattern that every SDK implements for pdftract.

## Overview

The conformance test suite is the SDK API contract. Every SDK must implement a test runner that:

1. Loads the shared `tests/sdk-conformance/cases.json` file
2. Iterates through test cases
3. Invokes the SDK's native methods with the case's options
4. Compares the result against expected values with tolerances
5. Reports per-case pass/fail/skip/error status
6. Emits a machine-readable JSON summary (`conformance-report.json`)

## Conformance Report Schema

See `tests/sdk-conformance/report-schema.json` for the full JSON schema.

Key fields:
- `sdk`: SDK name (e.g., "pdftract-py", "pdftract-node")
- `sdk_version`: SDK version that produced the report
- `suite_version`: Version of the conformance suite run
- `results`: Array of per-case results with `id`, `status`, `actual`, `expected`, `error`, `reason`, `duration_ms`
- `summary`: Aggregate counts for `total`, `passed`, `failed`, `skipped`, `errors`
- `environment`: OS, arch, binary version, runtime version

## Per-Language Runners

| SDK | Path | Test Framework | CLI Command |
|-----|------|----------------|-------------|
| Rust | `crates/pdftract-cli/tests/conformance.rs` | cargo test | `cargo test --test conformance` |
| Python | `tests/conformance/test_conformance.py` | pytest | `pytest tests/conformance/test_conformance.py -v` |
| Node.js | `tests/conformance/conformance.test.ts` | vitest | `vitest test/conformance/conformance.test.ts` |
| Go | `tests/conformance/conformance_test.go` | go test | `go test -v ./conformance_test.go` |
| Java | `tests/conformance/ConformanceTest.java` | JUnit 5 | `mvn test -Dtest=ConformanceTest` |
| .NET | `tests/conformance/ConformanceTests.cs` | xUnit | `dotnet test --filter ConformanceTests` |
| C | `tests/conformance/conformance.c` | standalone binary | `./conformance [suite-path] [output-path]` |
| Ruby | `tests/conformance/conformance_test.rb` | minitest | `ruby test/conformance/conformance_test.rb` |
| PHP | `tests/conformance/ConformanceTest.php` | PHPUnit | `./vendor/bin/phpunit tests/ConformanceTest.php` |
| Swift | `tests/conformance/ConformanceTests.swift` | XCTest | `swift test --filter ConformanceTests` |

## Shared Comparison Logic

All runners implement the same comparison logic with tolerances:

### Numeric Comparison with Tolerance

```pseudocode
function compare_with_tolerance(actual, expected, tolerance):
    if tolerance is null:
        return abs(actual - expected) < EPSILON

    if tolerance.abs exists:
        if abs(actual - expected) <= tolerance.abs:
            return true

    if tolerance.rel exists:
        diff = abs(actual - expected)
        avg = (actual + expected) / 2.0
        if avg > 0.0 and diff / avg <= tolerance.rel:
            return true

    return false
```

### Wildcard Path Matching

Tolerances use JSONPath-like wildcard syntax:
- `pages[*].blocks[*].bbox` matches all bbox values
- `pages[0].spans[*].confidence` matches all confidence values in page 0

### Expected Value Constraints

The expected object supports special constraint fields:

| Field | Type | Description |
|-------|------|-------------|
| `min` | number | Minimum numeric value |
| `max` | number | Maximum numeric value |
| `value` | number | Exact value (with tolerance) |
| `min_length` | number | Minimum string/array length |
| `contains` | array | String must contain all substrings |
| `min` | number | Minimum array length |
| `max` | number | Maximum array length |

## Test Case Execution Flow

1. Load test case from suite
2. Check `min_schema_version` - skip if SDK schema is too old
3. Resolve fixture path (handle remote URLs)
4. Execute SDK method with options
5. Compare result against expected with tolerances
6. Record result with timing
7. Emit final report

## Exit Codes

- `0`: All tests passed (or all failures were skips)
- `1`: One or more tests failed or errored

## CI Integration

The per-SDK Argo publish workflow MUST run the conformance runner BEFORE publishing. A failed runner aborts the publish step.

Example Argo step:

```yaml
- name: conformance
  template: conformance-runner
  arguments:
    parameters:
    - name: sdk
      value: pdftract-py

- name: publish
  template: publish-to-pypi
  dependencies:
  - conformance
  when: "{{steps.conformance.exitCode}}"
```

## README Integration

Each SDK's README should have a "Conformance" section that links to the latest published report:

```markdown
## Conformance

This SDK passes the official pdftract conformance suite. Latest report: [conformance-pdftract-py-0.1.0.json](https://argoproj.example/artifacts/conformance-pdftract-py-0.1.0.json)
```

## Stub Implementation Notes

The current runners contain stub implementations for `executeMethod()` that return placeholder values. These must be replaced with actual SDK calls when:

1. The SDK's native methods are implemented
2. The binary interface is stable
3. The JSON output schema is finalized

Until then, the runners serve as:
- A reference implementation pattern
- A starting point for SDK development
- Documentation of expected behavior

## Adding New Test Cases

To add a new test case to the suite:

1. Add the case to `tests/sdk-conformance/cases.json`
2. Bump `version` in the suite (if cases changed)
3. Update all SDK runners to handle the new case (if needed)
4. Verify all SDKs pass the updated suite before publishing

## References

- Plan section: SDK Architecture / The Conformance Suite, line 3547
- Plan section: SDK Acceptance Criteria, line 3589
- Shared suite: `tests/sdk-conformance/cases.json`
- Report schema: `tests/sdk-conformance/report-schema.json`
