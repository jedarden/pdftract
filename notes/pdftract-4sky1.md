# pdftract-4sky1: Exit code policy implementation

## Summary
Implemented the exit code policy for pdftract doctor per plan section 6.10 lines 2520-2521.

## Changes made

### 1. Updated doctor command help text (`crates/pdftract-cli/src/main.rs`)
- Added exit code policy documentation to the doctor command description
- Updated `--exit-on-fail` flag help text to clarify it's the default behavior and provided for CI script readability

### 2. Added code comment (`crates/pdftract-cli/src/doctor/mod.rs`)
- Added explanatory comment about the exit code policy and why `--exit-on-fail` is a no-op (default policy already exits 1 on any FAIL)

## Exit code behavior (per plan)
- Exit 0: all checks OK or WARN (no FAIL)
- Exit 1: at least one check is FAIL (regardless of --exit-on-fail flag)
- Exit 2: CLI parse error (clap default)

## Acceptance criteria verification

| Criterion | Result | Test output |
|-----------|--------|-------------|
| pdftract doctor with OK + WARN → exit 0 | **PASS** | `5 OK, 1 WARN, 0 FAIL` → exit 0 |
| pdftract doctor with FAIL → exit 1 | **PASS** | `4 OK, 1 WARN, 1 FAIL` → exit 1 |
| pdftract doctor --exit-on-fail with FAIL → exit 1 | **PASS** | `4 OK, 1 WARN, 1 FAIL` → exit 1 |
| pdftract doctor --exit-on-fail with OK/WARN → exit 0 | **PASS** | `5 OK, 1 WARN, 0 FAIL` → exit 0 |
| pdftract doctor --bogus-flag → exit 2 | **PASS** | `error: unexpected argument` → exit 2 |
| pdftract doctor --help → exit 0 | **PASS** | Help displayed → exit 0 |

## Notes
- The `--exit-on-fail` flag is accepted but does NOT change behavior (the default policy already exits 1 on any FAIL)
- The flag is retained for explicit CI-script readability and forward-compatibility (future `--warn-as-fail` or `--no-exit-on-fail` would invert)
- N/A status does NOT affect exit code
- WARN status does NOT affect exit code (calibrated: WARN = "watch this but don't block deployment")
