# pdftract-5omc: Per-Language Conformance Test Runner Pattern

## Summary

Implemented the conformance test runner pattern for all 10 SDKs as specified in the plan (line 3547). Each SDK now has a dedicated conformance test runner that:

1. Loads the shared `tests/sdk-conformance/cases.json` test suite
2. Executes test cases using language-native method invocations
3. Compares results against expected values with numeric tolerances
4. Emits a machine-readable `conformance-report.json` artifact
5. Exits non-zero on failures/errors for CI gating

## Files Created

### Core Infrastructure
- `tests/sdk-conformance/report-schema.json` - JSON schema for conformance reports
- `docs/notes/sdk-conformance-runner.md` - Pattern documentation and reference

### Per-Language Runners
1. **Rust**: `crates/pdftract-cli/tests/conformance.rs` - cargo test target
2. **Python**: `tests/conformance/test_conformance.py` - pytest harness
3. **Node.js**: `tests/conformance/conformance.test.ts` - vitest
4. **Go**: `tests/conformance/conformance_test.go` - go test
5. **Java**: `tests/conformance/ConformanceTest.java` - JUnit 5
6. **.NET**: `tests/conformance/ConformanceTests.cs` - xUnit
7. **C**: `tests/conformance/conformance.c` - standalone binary
8. **Ruby**: `tests/conformance/conformance_test.rb` - minitest
9. **PHP**: `tests/conformance/ConformanceTest.php` - PHPUnit
10. **Swift**: `tests/conformance/ConformanceTests.swift` - XCTest

### Updated CLI
- `crates/pdftract-cli/src/main.rs` - Contains `compare` and `conformance` subcommands

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Each SDK ships a conformance runner | ✅ PASS | All 10 SDKs have language-specific runners |
| Runner consumes `tests/sdk-conformance/cases.json` | ✅ PASS | All runners load from the shared suite path |
| Runner produces `conformance-report.json` | ✅ PASS | All runners emit JSON reports matching the schema |
| Runner exits non-zero on failure/error | ✅ PASS | Exit code 1 on failures, 0 on success |
| README links to published report | ⚠️ WARN | Skeleton runners only - not yet in SDK repos |
| 100% pass on every published SDK | ⚠️ WARN | Stub implementations return placeholder results |

## Implementation Details

### Shared Comparison Logic

All runners implement identical comparison semantics:

- **Numeric tolerances**: Both absolute (`abs`) and relative (`rel`) tolerance support
- **Wildcard path matching**: JSONPath-style `pages[*].blocks[*].bbox` patterns
- **Constraint fields**: `min`, `max`, `min_length`, `contains` for flexible assertions
- **Nested object/array comparison**: Recursive comparison with detailed failure paths

### Test Status Values

Each test case result has one of four statuses:
- `pass`: Actual matches expected within tolerances
- `fail`: Actual does not match expected
- `skip`: Feature unavailable or schema version too low
- `error`: Exception thrown or unexpected failure

### Report Structure

```json
{
  "sdk": "pdftract-<lang>",
  "sdk_version": "0.1.0",
  "suite_version": "1.0.0",
  "schema_version": "1.0",
  "timestamp": "2026-05-18T...",
  "results": [
    {
      "id": "extract-vector-scientific-paper",
      "status": "pass",
      "actual": {...},
      "expected": {...},
      "duration_ms": 123
    }
  ],
  "summary": {
    "total": 32,
    "passed": 30,
    "failed": 0,
    "skipped": 2,
    "errors": 0,
    "duration_ms": 5000
  },
  "environment": {
    "os": "linux",
    "arch": "x86_64",
    "binary_version": "0.1.0",
    "runtime_version": "..."
  }
}
```

## Known Limitations

1. **Stub Implementations**: All runners currently use stub `executeMethod()` functions that return placeholder values. These must be replaced with actual SDK calls when the SDKs are implemented.

2. **SDK Repository Placement**: The runners are currently in the main `pdftract` repository. Per the plan (line 3579), each SDK lives in its own git repository. These runners will need to be moved to their respective SDK repositories when those are created.

3. **README Integration**: The acceptance criterion for README "Conformance" sections linking to published reports cannot be verified until the SDK repositories exist and have their first published reports.

4. **CI/Argo Integration**: The runners produce reports that can be uploaded as Argo artifacts, but the actual Argo workflow templates that consume these reports are deferred to future beads (SDK publish workflows).

## Verification Commands

To verify the Rust runner (which can be run immediately):
```bash
cargo test --test conformance -- --nocapture
```

To verify other runners (requires respective runtimes):
```bash
# Python
pytest tests/conformance/test_conformance.py -v

# Node.js (requires TypeScript)
vitest test/conformance/conformance.test.ts

# Go
go test -v ./tests/conformance/conformance_test.go
```

## Next Steps

1. When SDK repositories are created, move each runner to its SDK repo
2. Replace stub `executeMethod()` with actual SDK bindings
3. Run each runner against the full conformance suite
4. Upload reports as Argo artifacts in publish workflows
5. Add "Conformance" sections to each SDK's README

## References

- Plan line 3547: "Every SDK has a `pdftract-sdk-conformance` test runner"
- Plan line 3589: "Conformance suite results published as an Argo artifact"
- `tests/sdk-conformance/cases.json`: The shared test suite (32 cases)
- `tests/sdk-conformance/report-schema.json`: Report JSON schema
