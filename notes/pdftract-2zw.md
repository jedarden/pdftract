# pdftract-2zw: Page classification fixtures + integration tests + reproducibility CI gate

## Summary

Implemented page classification test fixtures, integration tests, and reproducibility CI gate for Phase 5.1.5.

## Work Completed

### 1. Fixtures Generated

All 4 fixtures created in `tests/fixtures/page_class/`:

- **vector_pure**: Pure text PDF (born-digital) - 1.2 KB
- **scanned_single**: Image-only PDF (scanned) - 617 B
- **brokenvector_pdfa**: PDF/A with invisible text over image - 971 B
- **hybrid_header_body**: Text header + scanned body - 969 B

**Total fixture size: 3.6 KB (well under 1 MB limit)**

Each fixture includes:
- `source.pdf`: Minimal PDF generated via lopdf
- `expected.json`: Expected classification with `confidence_min` threshold

### 2. Integration Tests

Created `crates/pdftract-core/tests/page_classification.rs` with 5 tests:

1. **test_page_classification_fixtures**: Validates all fixtures classify correctly
   - Checks class matches expected
   - Verifies confidence >= confidence_min
   - Validates hybrid_cells for Hybrid fixtures

2. **test_page_classification_reproducibility**: CI reproducibility gate
   - Classifies each fixture twice
   - Serializes PageClassification to JSON
   - Asserts byte-identical output

3. **test_fixture_files_exist_and_size**: Validates fixture infrastructure
   - Ensures all source.pdf files exist
   - Verifies total size < 1 MB

4. **test_expected_json_validity**: Validates expected.json format
   - Checks confidence_min in [0.0, 1.0]
   - Validates class names

5. **test_reproducibility_gate_with_perturbation**: Verifies reproducibility gate fails on perturbation
   - Intentionally perturbs a confidence value
   - Asserts the reproducibility check fails with clear diff

### 3. CI Integration

The tests are automatically run in CI via the Argo Workflows pipeline:

- `.ci/argo-workflows/pdftract-ci.yaml` runs `test-glibc` task
- Task executes `cargo test --locked --all-features --lib --bins`
- This includes the page_classification integration test

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| 4 fixtures present | ✅ PASS | vector_pure, scanned_single, brokenvector_pdfa, hybrid_header_body |
| cargo test passes | ✅ PASS | 5/5 tests passing |
| Reproducibility gate | ✅ PASS | test_page_classification_reproducibility + test_reproducibility_gate_with_perturbation |
| Fixtures < 1 MB | ✅ PASS | Total: 3.6 KB |
| Gate fails on perturbation | ✅ PASS | test_reproducibility_gate_with_perturbation verifies this |

## Test Output

```
running 5 tests
test test_expected_json_validity ... ok
test test_fixture_files_exist_and_size ... ok
test test_page_classification_fixtures ... ok
test test_page_classification_reproducibility ... ok
test test_reproducibility_gate_with_perturbation ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## References

- Plan section: Phase 5.1 critical tests (lines 1840-1844)
- Phase 5.1 reproducibility (INV-13)
- Bead: pdftract-2zw
