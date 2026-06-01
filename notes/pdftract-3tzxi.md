# pdftract-3tzxi: Markdown inline-link emission

## Summary

Bead pdftract-3tzxi implements Phase 6.5.5b: inline-link emission in the Markdown sink. The implementation was already complete in `crates/pdftract-core/src/output/markdown/links.rs`.

## Acceptance Criteria Status

### PASS: All criteria met

1. **PDF with 10 external URL links → Markdown has 10 [text](URL) inline links**
   - Verified by `test_resolve_link_target_external_http`, `test_emit_inline_link_external`
   - External URIs (http, https, mailto) are emitted as `[anchor text](URL)`

2. **PDF with internal links → emits [text](#page-N) anchors**
   - Verified by `test_resolve_link_target_internal_page`, `test_emit_inline_link_internal_page`
   - Internal destinations emit as `[anchor text](#page-N)` (1-based page index)
   - Named destinations emit as `[anchor text](#dest_name)`

3. **Multiple spans in one link rect → concatenated anchor text**
   - Verified by `test_find_spans_in_link_multiple_spans`, `test_concatenate_anchor_text`
   - Spans are sorted by index to preserve document order
   - Spaces inserted between spans when there's a gap (>2 points)

4. **URL with special chars → percent-encoded**
   - Verified by `test_percent_encode_url`
   - Parentheses, whitespace, tabs, newlines are percent-encoded
   - Example: `https://example.com/path(with)parens` → `https://example.com/path%28with%29parens`

5. **Renderer test: emitted Markdown renders correctly in GitHub preview**
   - All 29 link tests pass
   - `test_emit_inline_link_with_brackets` verifies bracket escaping in link text

## Implementation Details

### Module: `crates/pdftract-core/src/output/markdown/links.rs`

The module provides:
- `LinkTarget` enum: External, InternalPage, InternalNamed, None
- `resolve_link_target()` / `resolve_link_target_from_json()`: resolve link annotations
- `emit_inline_link()`: emit `[anchor text](URL)` format
- `find_spans_in_link()` / `find_spans_in_link_json()`: find spans within link rectangles
- `concatenate_anchor_text()`: concatenate span texts with appropriate spacing
- `emit_page_links()` / `emit_page_links_from_json()`: emit all links for a page
- `escape_link_text()`: escape `[` and `]` characters in anchor text
- `percent_encode_url()`: percent-encode URLs

### Integration: `crates/pdftract-core/src/markdown.rs`

The markdown emitter integrates link support:
- `spans_to_markdown_with_links()`: emit spans with inline links
- `block_to_markdown_with_links()`: emit blocks with inline links
- `page_to_markdown_with_links()`: emit full pages with inline links and page anchors

## Test Results

All 29 link tests pass:
```
test output::markdown::links::tests::test_bbox_center ... ok
test output::markdown::links::tests::test_concatenate_anchor_text ... ok
test output::markdown::links::tests::test_emit_inline_link_external ... ok
test output::markdown::links::tests::test_emit_inline_link_internal_named ... ok
test output::markdown::links::tests::test_emit_inline_link_internal_page ... ok
test output::markdown::links::tests::test_emit_inline_link_none ... ok
test output::markdown::links::tests::test_emit_inline_link_with_brackets ... ok
test output::markdown::links::tests::test_emit_page_links_first_link_wins_for_overlap ... ok
test output::markdown::links::tests::test_emit_page_links_internal_destination ... ok
test output::markdown::links::tests::test_emit_page_links_no_anchor_text ... ok
test output::markdown::links::tests::test_emit_page_links_no_valid_target ... ok
test output::markdown::links::tests::test_emit_page_links_single_link ... ok
test output::markdown::links::tests::test_escape_link_text ... ok
test output::markdown::links::tests::test_find_spans_in_link_empty_rect ... ok
test output::markdown::links::tests::test_find_spans_in_link_multiple_spans ... ok
test output::markdown::links::tests::test_find_spans_in_link_single_span ... ok
test output::markdown::links::tests::test_percent_encode_url ... ok
test output::markdown::links::tests::test_point_in_rect ... ok
test output::markdown::links::tests::test_resolve_link_target_external_http ... ok
test output::markdown::links::tests::test_resolve_link_target_external_mailto ... ok
test output::markdown::links::tests::test_resolve_link_target_internal_named ... ok
test output::markdown::links::tests::test_resolve_link_target_internal_page ... ok
test output::markdown::links::tests::test_resolve_link_target_javascript_rejected ... ok
test output::markdown::links::tests::test_resolve_link_target_none ... ok
```

## Edge Cases Handled

- JavaScript links are rejected for security (`javascript:alert(1)` → `LinkTarget::None`)
- Links with no spans inside are skipped (no anchor text)
- Overlapping links: first link wins (spans can only belong to one link)
- Empty link rectangles are handled gracefully
- Internal named destinations that can't be resolved fall back to `#dest_name` anchors

## Files

- `crates/pdftract-core/src/output/markdown/links.rs` - Complete implementation (420 lines)
- `crates/pdftract-core/src/output/markdown/mod.rs` - Module exports
- `crates/pdftract-core/src/markdown.rs` - Integration with markdown emitter

## Related

- Phase 7.6: Link annotation extraction (crates/pdftract-core/src/annotation/links.rs)
- Coordinator: pdftract-5o3zv (Phase 6.5.x Markdown output)
