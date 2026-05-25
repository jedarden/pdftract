# Verification Note: pdftract-653ah

## Bead: 6.10.4: Runbook integration (docs/operations/manual-platform-smoke.md references doctor)

## Implementation Summary

Implemented runbook integration for `pdftract doctor` by:

1. **Created `docs/operations/manual-platform-smoke.md`** - A comprehensive smoke test runbook for KU-12 quarterly manual platform testing, including:
   - Step 1: pdftract doctor as the first validation step
   - Expected output examples for TTY and JSON formats
   - Troubleshooting table with 22+ rows covering all FAIL/WARN scenarios for all 14 doctor checks
   - Platform-specific notes for macOS, Windows, and Linux
   - Completion criteria (PASS/FAIL conditions)

2. **Updated `docs/user-docs/src/installation.md`** - Added "Environment Health Check" section with `pdftract doctor` command and link to the operations runbook.

3. **Updated `docs/user-docs/src/quickstart.md`** - Added "Verify Your Environment" section as the first step in the quickstart, before any extraction examples.

4. **Created `tests/doctor_runbook_coverage.rs`** - CI gate test that:
   - Parses the troubleshooting table from the runbook
   - Verifies all 14 doctor checks are present
   - Detects orphaned checks (in table but not in registry)
   - Fails fast if runbook is out of sync with check registry

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| docs/operations/manual-platform-smoke.md has "Step 1: pdftract doctor" as the first section | ✅ PASS | Created with comprehensive runbook |
| Troubleshooting table has one row per FAIL-capable check | ✅ PASS | All 14 checks covered (22+ rows including FAIL/WARN variants) |
| docs/user-docs/src/install.md mentions pdftract doctor with link | ✅ PASS | Added "Environment Health Check" section |
| docs/user-docs/src/quickstart.md uses pdftract doctor as first command | ✅ PASS | Added "Verify Your Environment" before extraction |
| CI gate parses runbook and asserts all checks are present | ✅ PASS | Created doctor_runbook_coverage.rs test |
| mdBook build succeeds | ✅ PASS | Built successfully with new content |
| No broken internal links | ✅ PASS | mdbook found no broken links |

## Commit

- **Commit hash:** `d9d21df`
- **Commit message:** `docs(pdftract-653ah): add runbook integration for pdftract doctor`
- **Files changed:**
  - `docs/operations/manual-platform-smoke.md` (new)
  - `docs/user-docs/src/installation.md` (modified)
  - `docs/user-docs/src/quickstart.md` (modified)
  - `tests/doctor_runbook_coverage.rs` (new)

## Test Results

- **CI gate test:** ✅ PASS
  ```
  ✓ Runbook troubleshooting table covers all doctor checks
  ✓ No orphaned checks in table
  ```
- **mdBook build:** ✅ PASS
  ```
  INFO Book building has started
  INFO Running the html backend
  INFO HTML book written to `/home/coding/pdftract/docs/user-docs/build/user-docs`
  ```

## Doctor Checks Covered

The troubleshooting table covers all 14 doctor checks from the registry:

1. pdftract binary (FAIL only - corrupted binary)
2. tesseract install (FAIL/WARN/OK)
3. tesseract languages (FAIL/WARN/OK)
4. leptonica install (FAIL/WARN/OK)
5. libtiff (FAIL only - missing)
6. libopenjp2 (FAIL only - missing)
7. pdfium native lib (FAIL/WARN/OK)
8. network reachability (FAIL/WARN/OK)
9. cache directory (FAIL/WARN/OK)
10. profile search path (FAIL/WARN/OK)
11. ulimit -n (FAIL/WARN/OK)
12. available RAM (FAIL/WARN/OK)
13. system locale (FAIL/WARN/OK)
14. temp dir writable (FAIL/WARN/OK)

## Plan References

- Plan section: Phase 6.10 `pdftract doctor` (lines 2479–2528 in `/docs/plan/plan.md`)
- Sibling 6.10.1: check registry (implemented in `crates/pdftract-cli/src/doctor/checks/mod.rs`)
- Sibling 6.10.3: exit code contract (implemented in `crates/pdftract-cli/src/doctor/mod.rs` lines 190-200)

## Bead Status

✅ **CLOSED** - All acceptance criteria met with no WARN items.
