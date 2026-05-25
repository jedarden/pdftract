# Verification Note: pdftract-529te - Per-page block serializer

## Bead ID
pdftract-529te - Per-page block serializer (joins block texts in reading order)

## Implementation Summary

Implemented `serialize_page_text()` function in `crates/pdftract-core/src/text.rs` that:
- Iterates blocks in reading order (as ordered in the blocks array)
- Filters by block-kind (Header/Footer/Watermark) via TextOptions
- For each block: computes block_text from the pre-computed `text` field
- Paragraph/Heading/Caption/Quote/Code/List/Table: use pre-computed block text
- Figure: emits empty string
- Concatenates blocks with `\n\n` separator
- Empty blocks emit nothing (no spurious newlines)

## Files Changed

### New Files
- `crates/pdftract-core/src/text.rs` - New module with plain text serialization logic

### Modified Files
- `crates/pdftract-core/src/lib.rs` - Added `pub mod text;` and exported `serialize_page_text, TextOptions`

## Acceptance Criteria Status

### PASS
- ✅ 3 Paragraph blocks "Foo Bar Baz": "Foo\n\nBar\n\nBaz"
- ✅ 1 Heading + 2 Paragraphs: "Title\n\nP1\n\nP2"
- ✅ Header excluded: not in output (default behavior)
- ✅ List: lines join with \n (pre-computed in block.text)
- ✅ Empty blocks emit nothing (no spurious \n\n)
- ✅ Footer excluded by default
- ✅ Header/Footer included when `with_headers_footers()` is set
- ✅ Watermark excluded by default
- ✅ Watermark included when `with_watermarks()` is set
- ✅ Figure emits empty string
- ✅ Code blocks preserve newlines
- ✅ Table blocks use pre-computed text
- ✅ Caption and Quote blocks work correctly
- ✅ TextOptions builder pattern works correctly

### WARN
- None

### FAIL
- None

## Test Results

All 22 tests in the `text` module pass:
```
text::tests::test_serialize_page_text_three_paragraphs - PASS
text::tests::test_serialize_page_text_heading_and_paragraphs - PASS
text::tests::test_serialize_page_text_header_excluded_by_default - PASS
text::tests::test_serialize_page_text_header_included_when_flagged - PASS
text::tests::test_serialize_page_text_footer_excluded_by_default - PASS
text::tests::test_serialize_page_text_list - PASS
text::tests::test_serialize_page_text_code - PASS
text::tests::test_serialize_page_text_figure_emits_empty - PASS
text::tests::test_serialize_page_text_empty_block_omitted - PASS
text::tests::test_serialize_page_text_watermark_excluded_by_default - PASS
text::tests::test_serialize_page_text_watermark_included_when_flagged - PASS
text::tests::test_serialize_page_text_caption - PASS
text::tests::test_serialize_page_text_quote - PASS
text::tests::test_serialize_page_text_table - PASS
text::tests::test_serialize_page_text_empty_blocks - PASS
text::tests::test_text_options_default - PASS
text::tests::test_text_options_builder_pattern - PASS
text::tests::test_is_header_or_footer - PASS
text::tests::test_is_watermark - PASS
text::tests::test_get_block_text_figure - PASS
text::tests::test_get_block_text_paragraph - PASS
text::tests::test_get_block_text_heading - PASS
```

## Compilation Status

- ✅ `cargo check --all-targets` - Passes
- ✅ `cargo clippy --all-targets -- -D warnings` - No text module issues (pre-existing errors elsewhere)
- ✅ `cargo fmt` - All formatted

## Notes

The implementation uses the pre-computed `block.text` field which already contains the joined text for the block. This aligns with the existing architecture where text computation happens earlier in the pipeline.

The `reading_order_rank` field mentioned in the plan is not yet present in the `BlockJson` structure; the function relies on the order of blocks in the array as the reading order (which is the current behavior).

The bead references plan lines 1747-1750 (Phase 4.6 Output Serialization). The implementation correctly handles:
- Blocks serialized in reading order
- Paragraphs separated by `\n\n`
- Headers/footers excluded by default
- Watermark blocks excluded
- Invisible text filtering (structure ready for span-level filtering)

The next step would be integrating this function into the CLI's `--text` output mode, which currently just dumps span texts one per line.
