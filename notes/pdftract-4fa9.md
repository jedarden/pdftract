# pdftract-4fa9: Object parser fixture corpus + proptest harness + critical-test suite

## Work Summary

This bead verifies that the object parser has:
1. A curated fixture corpus of 10 test cases
2. Proptest properties verifying core invariants
3. Golden output tests with BLESS=1 support

## Fix Applied

Fixed a duplicate `classify_page` function definition in `crates/pdftract-core/src/classify.rs` (lines 731-747). The function was defined twice (at line 564 and line 744), causing compilation errors. Removed the duplicate.

## Acceptance Criteria Status

### PASS: All 10 fixture files exist with sibling `.expected.json` goldens
- nested_dict.pdf.in + nested_dict.expected.json
- mixed_array.pdf.in + mixed_array.expected.json
- indirect_simple.pdf.in + indirect_simple.expected.json
- indirect_stream.pdf.in + indirect_stream.expected.json
- objstm_basic.pdf.in + objstm_basic.expected.json
- objstm_extends.pdf.in + objstm_extends.expected.json
- circular_self.pdf.in + circular_self.expected.json
- circular_three.pdf.in + circular_three.expected.json
- truncated_dict.pdf.in + truncated_dict.expected.json
- deep_nesting.pdf.in + deep_nesting.expected.json

Location: `/home/coding/pdftract/tests/object_parser/fixtures/`

### PASS: `cargo test -p pdftract-core --features proptest -- object_parser` passes
```
running 12 tests
test test_circular_self ... ok
test test_circular_self_with_64kb_stack ... ok
test test_circular_three ... ok
test test_all_fixtures ... ok
test test_indirect_simple ... ok
test test_deep_nesting ... ok
test test_indirect_stream ... ok
test test_mixed_array ... ok
test test_objstm_basic ... ok
test test_nested_dict ... ok
test test_objstm_extends ... ok
test test_truncated_dict ... ok
```

### PASS: Proptest properties implemented and passing
5 properties in `tests/proptest/object_parser.rs`:
1. `prop_parser_never_panics` - Verifies INV-8: parser is total over its input domain
2. `prop_resolve_terminates` - Verifies resolution terminates within 1000 operations
3. `prop_dict_order_preserved` - Verifies INV-3: dict order is deterministic for fingerprint stability
4. `prop_cache_consistency` - Verifies identical inputs produce identical outputs
5. `prop_inv8_no_panic` - Core INV-8 property: any input produces valid result or EOF, never panics

Test run:
```
running 5 tests
test prop_cache_consistency ... ok
test prop_dict_order_preserved ... ok
test prop_inv8_no_panic ... ok
test prop_resolve_terminates ... ok
test prop_parser_never_panics ... ok
```

### PASS: circular_self.pdf.in test runs with --stack-size 64KB
Test `test_circular_self_with_64kb_stack` in `crates/pdftract-core/tests/object_parser.rs` spawns a thread with 64KB stack and verifies that cycle detection works without stack overflow.

### PASS: proptest-regressions directories exist
- `/home/coding/pdftract/tests/proptest/proptest-regressions/`
- `/home/coding/pdftract/crates/pdftract-core/proptest-regressions/`

Note: Regression files are created by proptest when a failing test case is found. Since all properties currently pass, these directories are empty, which is expected.

## Files Modified

- `crates/pdftract-core/src/classify.rs` - Removed duplicate `classify_page` function definition

## Test Infrastructure

- Fixture test harness: `/home/coding/pdftract/tests/object_parser.rs`
- In-tree fixture test: `/home/coding/pdftract/crates/pdftract-core/tests/object_parser.rs`
- Proptest properties: `/home/coding/pdftract/tests/proptest/object_parser.rs`
- Additional proptest: `/home/coding/pdftract/crates/pdftract-core/tests/object_parser_proptest.rs`

## Related Plan Sections

- Phase 1.2 lines 1077-1081 (critical tests)
- INV-3 (fingerprint byte-stability)
- INV-8 (no panic)
- EC-08 (circular refs)
