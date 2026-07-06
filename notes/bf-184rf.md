# bf-184rf: Forms Integration Test Scaffold

## Summary

Verified and documented the existing forms integration test scaffold. The test infrastructure was already complete and functional.

## Acceptance Criteria Status

✅ **PASS**: Test file `tests/forms_integration.rs` exists and compiles
- Location: `crates/pdftract-cli/tests/forms_integration.rs`
- File size: 10,459 bytes (314 lines)
- Compiles without errors

✅ **PASS**: Test discovers all PDF fixtures in `tests/fixtures/forms/`
- Uses `walkdir` crate for recursive directory traversal
- Function `discover_pdf_fixtures()` walks fixtures directory
- Filters for `.pdf` extension
- Returns sorted `Vec<PathBuf>` of discovered fixtures

✅ **PASS**: Test can be run via `cargo test forms_integration`
- Command: `cargo test --test forms_integration`
- Result: All 6 tests pass
- Test names:
  - `test_discover_pdf_fixtures`
  - `test_forms_fixtures_discovery`
  - `test_extract_all_discovered_pdfs`
  - `test_form_field_structure`
  - `test_acroform_features`
  - `test_xfa_detection`

✅ **PASS**: Test skeleton calls pdftract extract
- Uses `Command::new(&bin).arg("extract").arg("--json").arg(pdf_path)`
- Invokes built pdftract binary via `pdftract_bin()` helper
- Captures stdout/stderr for validation
- Reports success/failure with detailed output

## Files Verified

1. **`crates/pdftract-cli/tests/forms_integration.rs`** (existing, verified)
   - 314 lines of test infrastructure
   - Helper functions for PDF discovery
   - 6 test functions covering forms extraction
   - Well-documented with module-level comments

2. **`crates/pdftract-cli/tests/fixtures/forms/`** (directory exists, empty)
   - Currently 0 PDF files
   - Ready for fixture population

## Test Infrastructure Details

### Helper Functions
- `pdftract_bin()`: Locates the compiled pdftract binary
- `fixtures_dir()`: Returns path to fixtures directory
- `find_pdf_fixtures()`: Non-recursive PDF discovery
- `discover_pdf_fixtures()`: Recursive walkdir-based discovery

### Test Functions
1. **`test_discover_pdf_fixtures`**: Validates fixture discovery and prints found files
2. **`test_forms_fixtures_discovery`**: Tests each fixture with pdftract extract --json
3. **`test_extract_all_discovered_pdfs`**: Comprehensive extraction test with output capture
4. **`test_form_field_structure`**: Skeleton for form field structure validation (TODO)
5. **`test_acroform_features`**: Skeleton for AcroForm-specific validation (TODO)
6. **`test_xfa_detection`**: Skeleton for XFA detection validation (TODO)

## Integration Points

- **Binary location**: Uses `CARGO_MANIFEST_DIR` to find `target/debug/pdftract` or `target/release/pdftract`
- **Fixtures path**: `tests/fixtures/forms/` relative to CLI crate root
- **Dependencies**: `walkdir` crate for recursive PDF discovery
- **CLI invocation**: Executes `pdftract extract --json <pdf>` via `std::process::Command`

## Next Steps (Future Beads)

The scaffold is complete. Subsequent beads should:
1. Populate `tests/fixtures/forms/` with real form PDF fixtures
2. Implement TODO items for form field structure validation
3. Implement AcroForm-specific validation logic
4. Implement XFA detection validation
5. Add JSON output structure verification
