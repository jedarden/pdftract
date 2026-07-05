# bf-3f8ua: TH-04 JavaScript Detection Fixture

## Summary

Verified that `tests/fixtures/security/embedded-js.pdf` exists and the TH-04 security test suite passes.

## What Was Done

### Fixture Verification
- Confirmed `tests/fixtures/security/embedded-js.pdf` exists (1.1K)
- Regenerated fixture using `tests/fixtures/security/generate_embedded_js.py`
- Fixture contains 3 JavaScript actions:
  1. Catalog /OpenAction: `app.alert("pwn")`
  2. Page 0 /AA /O: `app.alert('page_open')`
  3. Page 1 annotation /A: `app.alert('annot_action')`

### Test Results
All 4 tests in `crates/pdftract-core/tests/TH-04-js-presence.rs` pass:

1. ✅ `test_javascript_detection` - Verifies:
   - Extraction succeeds (exit 0)
   - Exactly 3 JavaScript actions detected
   - Each action has correct location (catalog.openaction, page.0.aa.o, page.1.annot.0.a)
   - Each action has code excerpt truncated to 200 chars
   - JAVASCRIPT_PRESENT diagnostic emitted

2. ✅ `test_json_output_includes_javascript_actions` - Verifies JSON output includes javascript_actions array

3. ✅ `test_no_javascript` - Negative test: PDF without JavaScript has empty javascript_actions array

4. ✅ `test_no_js_engine_in_deps` - Ensures no JS engine (boa, deno_core, v8, quickjs) in dependencies

### Command Output
```bash
cd /home/coding/pdftract/crates/pdftract-core && cargo test --test TH-04-js-presence --no-fail-fast -- --nocapture

running 4 tests
test test_javascript_detection ... ok
test integration_tests::test_json_output_includes_javascript_actions ... ok
test test_no_javascript ... ok
test test_no_js_engine_in_deps ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Acceptance Criteria Status

- ✅ `tests/fixtures/security/embedded-js.pdf` exists with embedded JS
- ✅ TH-04 security test passes (all 4 tests)
- ✅ No JS execution occurs during extraction (verified by test_javascript_detection)

## References
- TH-04 Threat Model (plan lines 893)
- Test file: `crates/pdftract-core/tests/TH-04-js-presence.rs`
- Fixture: `tests/fixtures/security/embedded-js.pdf`
- Generator: `tests/fixtures/security/generate_embedded_js.py`
