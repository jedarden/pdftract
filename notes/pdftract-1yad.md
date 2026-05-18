# Verification Note: pdftract-1yad

## Task
Implement traditional xref table parser (20-byte fixed-width entries, multi-subsection merge)

## Work Completed

### Implementation Status
The `parse_traditional_xref` function was already implemented in `/home/coding/pdftract/crates/pdftract-core/src/parser/xref.rs`. This task focused on:

1. **Test fixes**: Fixed two failing tests:
   - `test_parse_xref_entry_malformed`: Updated to use a proper 19-byte malformed entry
   - `test_parse_xref_missing_trailer`: Added tracking for trailer keyword and emit diagnostic when not found

2. **INV-8 compliance**: Replaced `unwrap()` calls on `RwLock` operations with graceful error handling:
   - `is_resolving`: Returns `false` on lock poisoning
   - `start_resolving`: Returns `false` on lock poisoning
   - `finish_resolving`: Silently ignores on lock poisoning
   - `resolve`: Handles cache lock poisoning gracefully
   - `cache_object`: Silently ignores on lock poisoning

3. **Proptest fix**: Removed incorrect `#[cfg(feature = "proptest")]` attribute since proptest dependency is always available (not behind a feature flag)

### Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| Simple test: well-formed single-subsection xref with 6 entries | **PASS** | `test_parse_simple_xref_space_newline` |
| Multi-subsection test: `0 3` then `100 2` produces 5 in-use entries | **PASS** | `test_parse_multi_subsection_xref` |
| Line-ending variant tests: ` \n` and `\r\n` both work | **PASS** | `test_parse_simple_xref_space_newline`, `test_parse_xref_carriage_return_newline` |
| `\n` alone detected as 19-byte stride | **PASS** | `test_parse_xref_lf_only_19_byte_entries` |
| Malformed entry test: single bad line skipped | **PASS** | `test_parse_xref_with_malformed_entry`, `test_parse_xref_entry_malformed` |
| proptest: random byte sequences never panic | **PASS** | `proptest_random_bytes_no_panic`, `proptest_random_offset_no_panic` |
| INV-8 maintained (no panic/unwrap/expect in production code) | **PASS** | All `unwrap()` calls replaced or in test code only |

### Implementation Details

The implementation follows the PDF spec 7.5.4 format:
- Reads `xref` keyword at `start_offset`
- Parses subsections with `obj_start obj_count` headers
- Handles 20-byte entries (10-digit offset + space + 5-digit generation + space + n/f + 2-byte line ending)
- Detects 19-byte stride for buggy producers (`\n` alone without leading space)
- Skips malformed entries with diagnostic emission
- Ignores free entries (they don't resolve to objects)
- Parses trailer dictionary after all subsections
- Emits `TrailerNotFound` diagnostic when trailer is missing

### Test Results

```
running 30 tests
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 103 filtered out; finished in 0.01s
```

Includes 2 proptest tests that verify random byte sequences never panic.

### Files Modified

- `crates/pdftract-core/src/parser/xref.rs`: Test fixes, INV-8 compliance improvements, proptest fix
- `notes/pdftract-1yad.md`: This verification note

### References

- Plan section: Phase 1.3 line 1088 (traditional xref)
- PDF spec 7.5.4 (Cross-Reference Table)
