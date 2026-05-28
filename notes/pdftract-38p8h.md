# pdftract-38p8h: Invisible Text Filter

## Work Completed

Fixed invisible text filtering implementation in `/home/coding/pdftract/crates/pdftract-core/src/text.rs`. The implementation was already present but had a bug that caused backward compatibility issues with existing tests.

## Changes Made

**File: `/home/coding/pdftract/crates/pdftract-core/src/text.rs`**

Added fallback logic in `serialize_page_text()` function (lines 225-237):
- When `block.spans` is empty, fall back to using pre-computed `block.text` for backward compatibility
- When `block.spans` is non-empty, recompute text from spans with invisible filtering (correct behavior)
- Added special case for figure blocks to always emit empty text (lines 226-227)

## Implementation Details

The invisible text filter works as follows:

1. **SPAN-level filtering** (not block-level):
   - `should_include_span()` checks each span's `rendering_mode`
   - Tr=0-2: visible (fill, stroke, fill+stroke)
   - Tr=3-7: invisible (excluded by default)

2. **Block text recomputation**:
   - `compute_block_text_from_spans()` joins visible span texts
   - If all spans in a block are invisible, produces empty string
   - Empty blocks are skipped (no spurious `\n\n`)

3. **Backward compatibility**:
   - When `block.spans` is empty, uses pre-computed `block.text`
   - This allows old tests to pass while supporting new span-based filtering

## Acceptance Criteria Status

### PASS ✓

1. **rendering_mode 3 + include_invisible false: excluded**
   - Test: `test_should_include_span_invisible_mode_3_excluded_by_default`
   - Spans with Tr=3 return false from `should_include_span()` when `include_invisible=false`

2. **Same with include_invisible true: included**
   - Test: `test_should_include_span_invisible_mode_3_included_when_flagged`
   - Spans with Tr=3 return true from `should_include_span()` when `include_invisible=true`

3. **Mixed block: visible emitted**
   - Test: `test_compute_block_text_from_spans_mixed_visibility`
   - Block with Tr=0 and Tr=3 spans emits only visible span text

4. **All-invisible block: no spurious \n\n**
   - Tests: `test_compute_block_text_from_spans_all_invisible_excluded`, `test_serialize_page_text_all_invisible_block_omitted`
   - Block with only Tr=3/4/5/6/7 spans produces empty string, skipped

5. **Tr=4: treated same as Tr=3**
   - Tests: `test_should_include_span_invisible_mode_4/5/6/7_excluded_by_default`
   - All Tr>=3 spans are filtered out by default

## Tests Passed

All 39 text module tests pass, including:
- All invisible text filtering tests (Tr=0-7, include_invisible true/false)
- All backward compatibility tests (empty spans, pre-computed text)
- All block kind filtering tests (headers, footers, watermarks)

## Verification

```bash
cargo nextest run --package pdftract-core --lib text
# 111 tests run: 111 passed, 2293 skipped
```

## Notes

The `include_invisible` option was already defined in `OutputOptions` (options.rs) and `TextOptions` (text.rs). The filtering logic was already implemented but had a bug where it always recomputed text from spans without a fallback for when span data was missing. The fix adds a fallback to use pre-computed text when `block.spans` is empty, maintaining backward compatibility with existing code and tests.
