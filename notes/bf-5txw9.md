# bf-5txw9: Add path normalization for fixture discovery

## Summary
Path normalization was already implemented in the fixture_discovery.rs module as part of the basic discovery implementation (bf-6cc3z).

## Implementation Status
The `normalize_path()` function (lines 56-114 in fixture_discovery.rs) provides comprehensive normalization:

### Features Implemented
1. **Symlink Resolution**: Uses `Path::canonicalize()` to resolve symlinks and get absolute paths
2. **Relative Component Handling**: Component-based normalization removes `.` and `..` components
3. **Absolute Path Conversion**: Converts relative paths to absolute paths based on current directory
4. **Fallback for Non-existent Paths**: Handles paths that don't exist gracefully with component-based normalization

### Code Coverage
The normalization is applied in:
- `discover_fixtures_flat()` - line 195
- `discover_fixtures_in_dir()` - line 233
- All discovery functions transitively through these

## Verification - Test Results
All 13 fixture discovery tests pass:

```
running 13 tests
test tests::test_category_discovery_returns_normalized_paths ... ok
test tests::test_discover_fixtures_by_category ... ok
test tests::test_discover_fixtures_flat ... ok
test tests::test_discover_all_fixtures ... ok
test tests::test_fixture_categories ... ok
test tests::test_fixture_sorting ... ok
test tests::test_fixture_paths_are_absolute ... ok
test tests::test_fixtures_root_exists ... ok
test tests::test_nonexistent_category ... ok
test tests::test_fixture_statistics ... ok
test tests::test_normalized_paths_are_consistent ... ok
test tests::test_normalized_paths_no_relative_components ... ok
test tests::test_normalized_paths_work_in_test_context ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Acceptance Criteria Status
- ✅ **Discovered paths are absolute or consistently test-relative**: `test_fixture_paths_are_absolute` verifies all paths are absolute
- ✅ **Symlinks and relative components are resolved**: `test_normalized_paths_no_relative_components` verifies no `/./` or `/../` in paths
- ✅ **Paths work in test context**: `test_normalized_paths_work_in_test_context` verifies all normalized paths exist and are readable

## Files
- `crates/pdftract-cli/tests/fixture_discovery.rs` - Lines 56-114 (normalize_path function), applied at lines 195, 233
- Tests: Lines 438-514 cover normalization verification

## Conclusion
The path normalization requirement for bf-5txw9 is already fully implemented and tested. No additional code changes are required.
