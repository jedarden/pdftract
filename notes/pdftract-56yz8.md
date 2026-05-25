# Bead pdftract-56yz8: Inline Span Styling (Phase 6.5)

## Summary

Implemented `span_to_markdown` function that translates span flag bitmask values to Markdown inline syntax per Phase 6.5 of the plan (lines 2188-2195).

## Changes Made

### File: `crates/pdftract-core/src/markdown.rs`

1. Added `SpanJson` import to the module
2. Implemented `span_to_markdown(span: &SpanJson) -> String`:
   - Reads span flags vector (`Vec<String>`) for style indicators
   - Emits appropriate Markdown syntax based on flags
   - Handles combinations: bold+italic → `***text***`
   - Handles script nesting: `**<sub>text</sub>**` (scripts inside bold/italic)
   - Handles smallcaps+script: `**<span><sup>text</sup></span>**` (scripts inside smallcaps)
   - Skips whitespace-only spans (no point styling whitespace)
   - Color-only differences: no styling emitted

3. Implemented `escape_markdown_inline(s: &str) -> String`:
   - Escapes CommonMark special characters: `\` `` ` `` `*` `_` `[` `]` `(` `)` `#` `!` `+` `<` `>`
   - Does NOT escape `-` `.` `=` (not special in inline context per CommonMark)

4. Added comprehensive test coverage (20+ tests):
   - Bold, italic, bold+italic combinations
   - Subscript, superscript, smallcaps individually
   - Combined styling (bold+subscript, italic+superscript, all flags)
   - Special character escaping
   - Whitespace-only edge cases

### File: `crates/pdftract-core/src/lib.rs`

- Exported `span_to_markdown` from the markdown module for public API

## Acceptance Criteria Status

| Criterion | Test | Status |
|-----------|------|--------|
| Bold + italic → ***text*** | `test_span_to_markdown_bold_italic` | PASS |
| Subscript → `<sub>2</sub>` | `test_span_to_markdown_subscript` | PASS |
| Superscript → `<sup>th</sup>` | `test_span_to_markdown_superscript` | PASS |
| Smallcaps → `<span style="font-variant: small-caps">CAPS</span>` | `test_span_to_markdown_smallcaps` | PASS |
| Color-only difference: no styling | `test_span_to_markdown_no_flags` | PASS |
| Special chars escaped: "1*2" → "1\*2" | `test_span_to_markdown_special_chars_escaped` | PASS |

## Test Results

```
cargo test --package pdftract-core --lib markdown
test result: ok. 43 passed; 0 failed
```

All acceptance criteria tests pass.

## Implementation Notes

1. **Nesting order**: Following plan guidance "emit **<sub>text</sub>** not <sub>**text**</sub>", script tags are placed inside bold/italic wrappers. For smallcaps+script combinations, smallcaps wraps scripts (e.g., `<span><sup>text</sup></span>`).

2. **Escaping**: Implemented per CommonMark spec - only escapes characters that have special meaning in inline Markdown context. Characters like `-` and `.` are NOT escaped because they're only special at line start (for lists/HR), not inline.

3. **Edge cases**: Whitespace-only spans skip styling entirely to avoid emitting empty formatting like `**   **`.

## Commits

- `pdftract-core`: Add span_to_markdown function with inline span styling (Phase 6.5)
