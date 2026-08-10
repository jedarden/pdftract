# bead bf-1djtvm: Run all negative fraction tests individually

## Summary
Ran all 5 negative fraction tests individually to identify any specific failing tests. All tests passed successfully with no orphaned processes.

## Tests Executed

All tests run with `cargo test <test_name> --lib --package pdftract-core`:

| # | Test Name | Result | Duration |
|---|-----------|--------|----------|
| 1 | `test_intersection_x_negative_fraction` | ✅ PASS | 0.00s |
| 2 | `test_round_x_negative_fraction_rounds_down` | ✅ PASS | 0.00s |
| 3 | `test_round_x_negative_fractions_round_down` | ✅ PASS | 0.00s |
| 4 | `test_round_x_small_negative_fraction_rounds_down` | ✅ PASS | 0.00s |
| 5 | `test_round_x_very_small_negative_fraction_rounds_down` | ✅ PASS | 0.00s |

## Verification

### Test Execution
- Each test was run individually using cargo test
- All tests completed successfully with exit code 0
- No test hangs or timeouts observed
- Test execution time: ~0.00s per test

### Orphaned Process Check
- Executed: `pgrep -af 'pdftract.*mcp|TH-0|TH_0'`
- Result: No orphaned pdftract MCP or TH-0 processes found
- Only stale cargo processes from other projects (sigil-core) were running, unrelated to pdftract

## Acceptance Criteria Status

1. ✅ **Each negative fraction test runs successfully in isolation** - All 5 tests ran and passed individually
2. ✅ **All individual tests pass** - 5/5 tests passed (100% success rate)
3. ✅ **No test hangs or times out** - All tests completed in ~0.00s each
4. ✅ **No orphaned processes after each test run** - Process check confirmed no pdftract-related orphans

## Conclusion

All negative fraction tests are functioning correctly when run in isolation. No specific test failures were identified. All tests pass quickly and cleanly without orphaning processes.

## References

- Bead ID: bf-1djtvm
- Parent bead: bf-5nq9ga
- Test source: `crates/pdftract-core/src/font/type3_rasterizer.rs` (lines 861-1017)
