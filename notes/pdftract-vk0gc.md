# Verification Note: pdftract-vk0gc (Markdown Anchors)

## Summary

Implemented `--md-anchors` positional HTML comment markers for Markdown output with parser regex.

## Changes Made

### 1. Core Implementation (crates/pdftract-core/src/markdown.rs)

Created new markdown module with:
- `Anchor` struct with `page`, `block`, `bbox`, `kind` fields
- `parse_anchors()` function with regex: `r"<!--\s*pdftract:\s*page=(\d+)\s+block=(\d+)\s+bbox=\[([\d.,]+)\]\s+kind=(\w+)\s*-->"`
- `block_to_markdown()` - converts single block to markdown with optional anchor
- `page_to_markdown()` - converts all blocks from a page with optional anchors and page breaks
- `Anchor::to_comment()` - formats anchor as HTML comment with 1 decimal place precision

### 2. Options (crates/pdftract-core/src/options.rs)

Added `markdown_anchors: bool` field to `ExtractionOptions` with default `false`.

### 3. CLI Integration (crates/pdftract-cli/src/main.rs)

- Added `--md-anchors` flag to Extract command
- Passed flag through to ExtractionOptions
- Updated markdown output to use `page_to_markdown()` when anchors enabled
- Added import for `page_to_markdown` and `block_to_markdown`

### 4. Documentation (docs/integrations/markdown-anchors.md)

Created comprehensive integration guide covering:
- Anchor format specification
- Regex schema
- CLI and Rust API usage
- Edge cases (code fences, empty blocks, per-page indexing)
- Integration examples for Python and JavaScript

## Acceptance Criteria

### PASS

- ✅ `--md-anchors` flag emits comment before every block
- ✅ Parser regex extracts page, block, bbox, kind from sample output
- ✅ Round-trip test: `test_roundtrip_extract_and_parse` passes
- ✅ Comment is ONE LINE (no embedded newline)
- ✅ bbox precision: 1 decimal place exact (verified in `test_anchor_to_comment_round_bbox`)
- ✅ kind matches block kind (heading, paragraph, etc.)
- ✅ Parser library `parse_anchors()` available
- ✅ Module exports: `Anchor`, `parse_anchors`, `block_to_markdown`, `page_to_markdown`
- ✅ 16 unit tests pass (including roundtrip, bbox parsing, multiple anchors)
- ✅ Regex is stable public API (documented in markdown-anchors.md)
- ✅ HTML comments are passthrough in major renderers (documented)
- ✅ Block index is per-page (0-based within page)

### WARN (Infrastructure limitations)

- None

## Testing

### Unit Tests (16/16 pass)

- `test_anchor_to_comment` - basic comment formatting
- `test_anchor_to_comment_round_bbox` - 1 decimal place precision
- `test_parse_anchors_single` - parse single anchor
- `test_parse_anchors_multiple` - parse multiple anchors
- `test_parse_anchors_invalid_format_skipped` - invalid formats skipped
- `test_parse_anchors_whitespace_tolerant` - whitespace tolerance
- `test_parse_bbox` - bbox parsing with various formats
- `test_block_to_markdown_heading_with_anchor` - heading with anchor
- `test_block_to_markdown_paragraph_without_anchor` - paragraph without anchor
- `test_block_to_markdown_list` - list block
- `test_block_to_markdown_table` - table block
- `test_block_to_markdown_figure` - figure block
- `test_page_to_markdown_with_page_break` - page break separator
- `test_page_to_markdown_without_page_break` - no page break
- `test_page_to_markdown_with_anchors` - anchors enabled
- `test_roundtrip_extract_and_parse` - full roundtrip

### Build Verification

- `cargo build -p pdftract-core` - ✅ Success
- `cargo build -p pdftract-cli` - ✅ Success
- `cargo test -p pdftract-core --lib markdown` - ✅ 16/16 tests pass

## Example Output

With `--md-anchors` enabled:

```markdown
<!-- pdftract: page=0 block=0 bbox=[72.0,640.5,540.0,672.0] kind=heading -->
# Chapter 1

<!-- pdftract: page=0 block=1 bbox=[72.0,600.0,540.0,630.0] kind=paragraph -->
This is the first paragraph.
```

## Files Modified

- `crates/pdftract-core/src/markdown.rs` (new)
- `crates/pdftract-core/src/lib.rs` (module export)
- `crates/pdftract-core/src/options.rs` (markdown_anchors field)
- `crates/pdftract-core/Cargo.toml` (regex dependency already present)
- `crates/pdftract-cli/src/main.rs` (CLI flag and output logic)
- `docs/integrations/markdown-anchors.md` (new documentation)

## References

- Plan section: Phase 6.5 positional anchors (lines 2183-2197)
- Bead: pdftract-vk0gc
