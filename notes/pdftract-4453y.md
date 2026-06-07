# Phase 4.6: Output Serialization Plain Text (Coordinator) - Verification Note

## Summary

Phase 4.6 coordinator bead `pdftract-4453y` is complete. All 5 child beads are closed and the implementation satisfies all acceptance criteria.

## Children Closed

All 5 dependencies of this coordinator are closed:

1. **pdftract-56txm**: Phase 4.5: Reading Order (coordinator) - CLOSED
2. **pdftract-2bpzs**: OutputOptions struct + block-kind filter - CLOSED
3. **pdftract-38p8h**: Invisible text filter - CLOSED
4. **pdftract-3bgxq**: Document-level serializer (form feed) - CLOSED
5. **pdftract-529te**: Per-page block serializer - CLOSED

## Acceptance Criteria Status

| Criterion | Status | Evidence |
|-----------|--------|----------|
| All children closed | **PASS** | All 5 children verified closed via `bf show` |
| 10-page doc: 9 form-feed characters | **PASS** | `test_serialize_document_text_ten_pages` verifies exactly 9 form feeds |
| Header block excluded by default; included with flag | **PASS** | `test_serialize_page_text_header_excluded_by_default` and `test_serialize_page_text_header_included_when_flagged` |
| Invisible Tr=3 excluded by default | **PASS** | `test_should_include_span_invisible_mode_3_excluded_by_default` and related tests |
| Text round-trips with join | **PASS** | All serialization tests pass; proper `\n\n` and `\f` joining |

## Implementation Files

### Core Implementation
- `crates/pdftract-core/src/text.rs` - Per-page and document-level serialization
- `crates/pdftract-core/src/options.rs` - `OutputOptions` struct with filtering

### CLI Integration
- `crates/pdftract-cli/src/main.rs` - CLI flags wired to options:
  - `--include-headers` / `--include-footers` / `--include-headers-footers`
  - `--include-invisible-text`
  - `--include-watermarks`

## Key Invariants Verified

### Form Feed Invariant (INV)
- **N pages → N-1 form feeds**: `serialize_document_text` uses `join("\u{000C}")` which produces exactly N-1 form feeds
- No leading form feed
- No trailing form feed
- Empty page in middle: form feed before AND after

Test evidence:
- `test_serialize_document_text_one_page`: 0 form feeds
- `test_serialize_document_text_two_pages`: 1 form feed
- `test_serialize_document_text_ten_pages`: 9 form feeds
- `test_serialize_document_text_empty_page_in_middle`: 2 form feeds with empty middle page

### Block-Kind Filtering
- Header/Footer excluded by default (controlled via `OutputOptions`)
- Watermark excluded (no-op until Phase 7)
- Filtering at block level via `OutputOptions::include_block_kind`

Test evidence:
- `test_serialize_page_text_header_excluded_by_default`: header not in output
- `test_serialize_page_text_header_included_when_flagged`: header included with flag
- `test_serialize_page_text_watermark_excluded_by_default`: watermark not in output

### Invisible Text Filtering (SPAN-level)
- Filtering at SPAN level, not block level
- Tr=3 excluded by default (invisible)
- Tr=4-7 treated same as Tr=3 (invisible variants)
- Mixed-visibility blocks: visible emitted, invisible dropped
- All-invisible blocks: no spurious `\n\n`

Test evidence:
- `test_should_include_span_invisible_mode_3_excluded_by_default`: Tr=3 filtered
- `test_should_include_span_invisible_mode_4_excluded_by_default` through `test_should_include_span_invisible_mode_7`: Tr=4-7 filtered
- `test_compute_block_text_from_spans_mixed_visibility`: mixed block emits only visible
- `test_serialize_page_text_all_invisible_block_omitted`: all-invisible produces empty (no `\n\n`)

### Paragraph/Block Joining
- Blocks separated by `\n\n`
- Paragraph/Heading/Caption/Quote: space-joined lines
- List/Code: newline-joined lines
- Figure: empty string (no text content)

Test evidence:
- `test_serialize_page_text_three_paragraphs`: "Foo\n\nBar\n\nBaz"
- `test_serialize_page_text_heading_and_paragraphs`: "Title\n\nP1\n\nP2"
- `test_serialize_page_text_list`: lines joined with `\n`
- `test_serialize_page_text_figure_emits_empty`: figure produces ""

## Test Results

### text.rs tests (160 passed)
```
cargo test -p pdftract-core --lib text
test result: ok. 160 passed; 0 failed; 0 ignored
```

Key tests verifying acceptance criteria:
- `test_serialize_document_text_ten_pages`: 9 form feeds ✓
- `test_serialize_page_text_header_excluded_by_default` ✓
- `test_serialize_page_text_header_included_when_flagged` ✓
- `test_should_include_span_invisible_mode_3_excluded_by_default` ✓
- All rendering mode tests (Tr=0 through Tr=7) ✓

### options.rs tests (41 passed)
```
cargo test -p pdftract-core --lib options
test result: ok. 41 passed; 0 failed; 0 ignored
```

Key tests:
- `test_output_options_default`: all exclude defaults ✓
- `test_output_options_include_block_kind`: header/footer/watermark filtering ✓
- `test_output_options_include_span`: invisible/hidden layer filtering ✓

## References

- Plan section: Phase 4.6 Output Serialization (lines 1760-1776)
- Bead: pdftract-4453y
- Children: pdftract-56txm, pdftract-2bpzs, pdftract-38p8h, pdftract-3bgxq, pdftract-529te

## Conclusion

Phase 4.6 Output Serialization (Plain Text Mode) is fully implemented and verified. All acceptance criteria PASS. The implementation correctly:
1. Joins blocks in reading order with `\n\n`
2. Joins pages with form feed `\f` (exactly N-1 for N pages)
3. Excludes headers/footers by default with CLI flags to include
4. Excludes invisible text (Tr=3+) by default with CLI flag to include
5. Filters at SPAN level for invisible text, BLOCK level for kinds
