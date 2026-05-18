# SDK Conformance Test Runner Pattern

This document describes the pattern that every pdftract SDK must implement for conformance testing.

## Overview

Every SDK ships a `pdftract-sdk-conformance` test runner that:
1. Loads `tests/sdk-conformance/cases.json` (the shared test suite)
2. Iterates through test cases
3. Invokes the SDK's native method with the case's options
4. Compares the result against `expected` with tolerances
5. Reports per-case pass/fail/skip/error status
6. Emits `conformance-report.json`

The runner is a TEST target, not production code. It lives in the SDK's test tree.

## Test Case Structure

Each test case in `cases.json` has:

```json
{
  "id": "extract-vector-scientific-paper",
  "fixture": "scientific_paper/01.pdf",
  "method": "extract",
  "options": {
    "ocr_language": "eng",
    "ocr_threshold": 0.7,
    "preserve_layout": false,
    "extract_images": false
  },
  "expected": {
    "schema_version": "1.0",
    "metadata.page_count": 1,
    "pages.length": 1,
    "pages[0].page_index": 0,
    "pages[0].width": {"min": 500, "max": 700},
    "pages[0].height": {"min": 700, "max": 900},
    "pages[0].rotation": 0,
    "pages[0].spans.length": {"min": 1},
    "pages[0].blocks.length": {"min": 1},
    "pages[0].blocks[0].kind": "heading",
    "errors.length": 0
  },
  "tolerances": {
    "pages[*].blocks[*].bbox": {"abs": 0.5},
    "pages[*].spans[*].bbox": {"abs": 0.5}
  },
  "feature": "vector",
  "min_schema_version": "1.0"
}
```

## Expected Value Constraints

The `expected` field supports several constraint types:

### Exact Value Match
```json
{"pages[0].rotation": 0}
```

### Min/Max Ranges
```json
{"pages[0].width": {"min": 500, "max": 700}}
```

### Minimum Length (arrays/strings)
```json
{"pages[0].spans.length": {"min": 1}}
{"value": {"min_length": 50}}
```

### Contains (strings)
```json
{"value": {"contains": ["Abstract", "Introduction"]}}
```

### Boolean/Null Checks
```json
{"metadata.is_encrypted": true}
{"metadata.title": null}
```

## Tolerances

Tolerances allow for numeric imprecision in comparisons:

```json
{
  "tolerances": {
    "pages[*].blocks[*].bbox": {"abs": 0.5},
    "pages[*].spans[*].confidence": {"abs": 0.2, "rel": 0.1}
  }
}
```

- `abs`: Absolute tolerance - values pass if `|actual - expected| <= abs`
- `rel`: Relative tolerance - values pass if `|actual - expected| / average <= rel`

Wildcard patterns (`*`) in tolerance paths match any array index or field name.

## Skip Conditions

A test case should be skipped (status: `"skip"`) if:

1. **Feature unavailable**: The SDK doesn't support the required feature
   - Check: `case.feature` is not in the SDK's available features
   - Example: C SDK without OCR support skips all `feature: "ocr"` tests

2. **Schema version too old**: The SDK's binary schema version is older than required
   - Check: `sdk.schema_version < case.min_schema_version`
   - Example: SDK with schema 1.0 skips tests requiring 1.1

3. **Explicit skip**: The case has `skip_reason` set
   - Check: `case.skip_reason` is not null

## Report Format

The runner must emit `conformance-report.json`:

```json
{
  "sdk": "pdftract-python",
  "sdk_version": "1.0.0",
  "suite_version": "1.0.0",
  "timestamp": "2026-05-18T12:00:00Z",
  "results": [
    {
      "id": "extract-vector-scientific-paper",
      "status": "pass",
      "actual": {...},
      "expected": {...},
      "duration_ms": 150
    },
    {
      "id": "extract-scanned-receipt",
      "status": "fail",
      "actual": {...},
      "expected": {...},
      "error": "pages[0].page_type: expected 'scanned', got 'vector'",
      "duration_ms": 200
    },
    {
      "id": "extract-remote-pdf",
      "status": "skip",
      "error": "Feature 'remote' not supported by this SDK",
      "duration_ms": 0
    }
  ],
  "summary": {
    "total": 32,
    "passed": 28,
    "failed": 1,
    "skipped": 3,
    "errors": 0
  }
}
```

Status values: `"pass"`, `"fail"`, `"skip"`, `"error"`

## Exit Codes

The runner must exit with:
- `0` if all non-skip tests passed
- `1` if any test failed or had an error

## Comparison Logic (Pseudocode)

```
function compare(actual, expected, tolerances, path):
    match (actual, expected):
        case (Number, Object with min/max):
            if actual < expected.min: return FAIL("value below minimum")
            if actual > expected.max: return FAIL("value above maximum")
            if expected.value exists:
                return compare_with_tolerance(actual, expected.value, tolerances, path)
            return PASS

        case (String, Object with constraints):
            if actual.length < expected.min_length: return FAIL("string too short")
            for substring in expected.contains:
                if substring not in actual: return FAIL("missing required substring")
            return PASS

        case (Array, Object with min/max):
            if actual.length < expected.min: return FAIL("array too short")
            if actual.length > expected.max: return FAIL("array too long")
            return PASS

        case (_, _):
            if actual == expected: return PASS
            return FAIL("value mismatch")

function compare_with_tolerance(actual, expected, tolerances, path):
    tolerance = find_tolerance(tolerances, path)
    if tolerance == null:
        return exact_compare(actual, expected)

    diff = abs(actual - expected)
    if tolerance.abs exists and diff <= tolerance.abs:
        return PASS
    if tolerance.rel exists:
        avg = (actual + expected) / 2
        if diff / avg <= tolerance.rel:
            return PASS
    return FAIL("numeric mismatch")

function find_tolerance(tolerances, path):
    // Try exact match first
    if tolerances[path] exists: return tolerances[path]

    // Try wildcard patterns
    for key in tolerations:
        if key contains '*':
            pattern = key.replace('*', '.*')
            if path matches pattern: return tolerations[key]

    return null
```

## Using the CLI Compare Subcommand

For SDKs that prefer not to reimplement the comparison logic, the `pdftract` CLI provides a `compare` subcommand:

```bash
pdftract compare actual.json expected.json --tolerances tolerances.json --format json
```

This outputs a JSON report of pass/fail for each expected field, with detailed failure reasons.

## Per-Language Runner Locations

| SDK | Runner Path | Test Framework |
|-----|-------------|----------------|
| Python | `tests/test_conformance.py` | pytest |
| Rust | `crates/pdftract-cli/tests/conformance.rs` | cargo test |
| Node.js | `test/conformance.test.ts` | vitest |
| Go | `conformance_test.go` | go test |
| Java | `src/test/java/.../ConformanceTest.java` | JUnit 5 |
| .NET | `tests/Pdftract.Tests/ConformanceTests.cs` | xUnit |
| C | `tests/conformance.c` | standalone binary |
| Ruby | `test/conformance_test.rb` | minitest |
| PHP | `tests/ConformanceTest.php` | PHPUnit |
| Swift | `Tests/PdftractTests/ConformanceTests.swift` | XCTest |

## CI Integration

Each SDK's Argo publish workflow must:
1. Run the conformance runner
2. Parse the report JSON
3. Fail the workflow if `summary.failed > 0` or `summary.errors > 0`
4. Upload the report as an Argo artifact
5. Link the artifact from the SDK's README "Conformance" section

## Milestone Gates

Before publishing any SDK milestone tag:
- 100% of applicable (non-skip) tests must pass
- The conformance report must be included in the release notes
- The README must link to the published report artifact
