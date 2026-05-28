# pdftract-37wcw: Table emission verification

## Bead: 6.5.4 Table emission (GFM pipe + HTML fallback for merged cells) + caption italic

### Implementation Summary

The table emission functionality was already implemented in `/home/coding/pdftract/crates/pdftract-core/src/markdown.rs`:

1. **`emit_table`** (line 1042-1055): Main function that decides between GFM and HTML
2. **`emit_gfm_table`** (line 1064-1140): Emits GFM pipe tables for simple tables
3. **`emit_html_table`** (line 1147-1185): Emits HTML tables for complex tables
4. **`escape_pipe`** (line 1194-1219): Escapes pipes and handles newlines

### Acceptance Criteria Status

| Criterion | Status | Evidence |
|-----------|--------|----------|
| **Critical test: merged-cell table -> HTML fallback** | ✅ PASS | `test_emit_table_merged_cells_html_fallback` passes |
| **Simple 3x3 table: GFM pipe format** | ✅ PASS | `test_emit_table_simple_3x3` passes |
| **Caption appears as italic line below table** | ✅ PASS | Handled in `block_to_markdown` (line 270-272): `*{text}*\n` |
| **Cell with pipe character: escaped as `\|`** | ✅ PASS | `test_escape_pipe` and `test_emit_table_with_pipe_in_cell` pass |
| **Cell with newline: rendered with `<br>`** | ✅ PASS | `test_escape_pipe_newline_to_br` and `test_emit_table_with_newline_in_cell` pass |
| **Nested-block cell: HTML fallback** | ⚠️ N/A | Schema doesn't support nested blocks in cells (only `text` + `spans`) |

### Test Results

```bash
$ cargo test -p pdftract-core --lib 'markdown::'
running 65 tests
test result: ok. 65 passed; 0 failed; 0 ignored
```

All table emission tests pass:
- `test_emit_table_empty` - Empty table returns empty string
- `test_emit_table_merged_cells_html_fallback` - Merged cells trigger HTML fallback
- `test_emit_table_rowspan_html_fallback` - Rowspan triggers HTML fallback
- `test_emit_table_no_header` - Tables without header row use first row as header
- `test_emit_table_simple_3x3` - Simple table uses GFM pipe format
- `test_emit_table_with_newline_in_cell` - Newlines become `<br>` tags
- `test_emit_table_single_row` - Single row tables work correctly
- `test_emit_table_with_pipe_in_cell` - Pipes escaped as `\|`

### Implementation Details

**Simple table detection (GFM):**
```rust
let is_simple = table.rows.iter().all(|row| {
    row.cells.iter().all(|cell| cell.rowspan == 1 && cell.colspan == 1)
});
```

**GFM pipe table format:**
```markdown
| Header 1 | Header 2 | Header 3 |
| --- | --- | --- |
| Data 1 | Data 2 | Data 3 |
```

**HTML fallback for merged cells:**
```html
<table>
  <tr>
    <th colspan="2">Merged Header</th>
    <th>Header 2</th>
  </tr>
  ...
</table>
```

**Caption handling:**
Captions are separate blocks (kind: "caption") emitted as italic text:
```markdown
*Table caption*
```

### Notes

1. **Nested blocks in cells**: The current `CellJson` schema only has `text` (String) and `spans` (Vec<SpanRef>). There's no support for nested block elements like paragraphs within cells. This appears to be a forward-looking requirement or something that doesn't exist in the current data model.

2. **Header-less tables**: GFM requires a header row. The implementation synthesizes an empty header row for tables with `is_header=false` on all rows (the first row becomes the header).

3. **Column padding**: The implementation correctly handles variable-width rows by padding with empty cells to match the maximum column count.

### Files Modified

No files were modified - the implementation was already complete. All tests pass.

### Commits

N/A - No changes made, implementation already exists.
