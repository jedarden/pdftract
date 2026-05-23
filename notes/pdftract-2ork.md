# pdftract-2ork: Element-type to block-kind mapping table

## Summary

Implemented the StandardType -> BlockKind mapping that converts walked StructElem nodes into the BlockKind taxonomy used by Phase 4 output. Includes Artifact suppression and heading-level extraction (H, H1..H6 -> heading with level).

## Implementation

### Files Modified/Created
- `crates/pdftract-core/src/parser/struct_tree.rs` (added 420+ lines)
- `crates/pdftract-core/src/parser/mod.rs` (updated exports)

### Core Types Added
- `BlockKind`: Enum covering all output block kinds (paragraph, heading with level, table, list, list_item, figure, caption, code, block_quote, toc, formula, reference, note, form_field_struct, inline, structural_container, artifact, unknown)
- `MappingResult`: Result type for mapping operations containing block_kind, is_emitted flag, and optional diagnostic
- `structure_type_to_block_kind()`: Pure mapping function from StructureType to BlockKind
- `map_element_to_block()`: Primary mapping function taking StructElemNode and returning MappingResult
- `is_artifact()`: Placeholder for Artifact marked-content integration (Phase 3.4)

### Key Features
1. **Complete type mapping**:
   - Block-level elements (P, H1..H6, Table, L, LI, Lbl, LBody, Figure, Caption, Code, BlockQuote, TOC, TOCI, Formula, Reference, Note, Form) → emitted block kinds
   - Inline elements (Span, Quote, Link, Ruby, etc.) → Inline (not emitted as separate blocks)
   - Structural containers (Document, Part, Art, Sect, Div, NonStruct, Private, Index, TR, TH, TD, THead, TBody, TFoot) → StructuralContainer (descend without emitting)
   - Unknown types → Unknown (emits as paragraph with diagnostic)

2. **Heading level extraction**:
   - H (no explicit level) → Heading{level: 1}
   - H1..H6 → Heading{level: 1..6}
   - No auto-increment for nested H elements (spec leaves this to producer)

3. **Artifact handling**:
   - Placeholder `is_artifact()` function ready for Phase 3.4 marked-content integration
   - When integrated, will suppress both "Artifact" structure type and MCIDs inside Artifact marked-content sequences

4. **Diagnostic support**:
   - Unknown types emit a diagnostic warning
   - MappingResult includes optional Diagnostic for downstream collection

## Verification

### Unit Tests (32 new tests, all PASS)
```
test parser::struct_tree::tests::test_block_kind_paragraph ... ok
test parser::struct_tree::tests::test_block_kind_heading_h ... ok
test parser::struct_tree::tests::test_block_kind_heading_h1 ... ok
test parser::struct_tree::tests::test_block_kind_heading_h2 ... ok
test parser::struct_tree::tests::test_block_kind_heading_all_levels ... ok
test parser::struct_tree::tests::test_block_kind_table ... ok
test parser::struct_tree::tests::test_block_kind_list ... ok
test parser::struct_tree::tests::test_block_kind_list_item ... ok
test parser::struct_tree::tests::test_block_kind_list_label ... ok
test parser::struct_tree::tests::test_block_kind_list_body ... ok
test parser::struct_tree::tests::test_block_kind_figure ... ok
test parser::struct_tree::tests::test_block_kind_caption ... ok
test parser::struct_tree::tests::test_block_kind_code ... ok
test parser::struct_tree::tests::test_block_kind_block_quote ... ok
test parser::struct_tree::tests::test_block_kind_toc ... ok
test parser::struct_tree::tests::test_block_kind_formula ... ok
test parser::struct_tree::tests::test_block_kind_reference ... ok
test parser::struct_tree::tests::test_block_kind_note ... ok
test parser::struct_tree::tests::test_block_kind_form ... ok
test parser::struct_tree::tests::test_block_kind_inline_span ... ok
test parser::struct_tree::tests::test_block_kind_inline_quote ... ok
test parser::struct_tree::tests::test_block_kind_structural_container ... ok
test parser::struct_tree::tests::test_block_kind_unknown ... ok
test parser::struct_tree::tests::test_mapping_result_for_paragraph ... ok
test parser::struct_tree::tests::test_mapping_result_for_heading_with_level ... ok
test parser::struct_tree::tests::test_mapping_result_for_unknown_type ... ok
test parser::struct_tree::tests::test_mapping_result_for_inline_element ... ok
test parser::struct_tree::tests::test_mapping_result_for_structural_container ... ok
test parser::struct_tree::tests::test_list_nesting_mapping ... ok
test parser::struct_tree::tests::test_table_grouping_mapping ... ok
test parser::struct_tree::tests::test_span_passthrough ... ok
test parser::struct_tree::tests::test_heading_level_not_auto_incremented ... ok
```

### Acceptance Criteria Status
- ✓ Every Standard structure type has a mapping decision (in-table, suppressed, or structural-container)
- ✓ Critical test: H1/H2 -> heading level 1/2
- ✓ Unit tests: list nesting (L, LI, Lbl, LBody all map correctly)
- ✓ Unit tests: table grouping (TR, TH, TD, THead, TBody, TFoot → StructuralContainer)
- ✓ Unit tests: span passthrough (Span, Quote → Inline, not emitted)
- ✓ Unknown-type fallback path emits a diagnostic line

## Integration Notes

### Public API
The following are now exported from `pdftract-core::parser`:
- `BlockKind` enum
- `MappingResult` struct
- `structure_type_to_block_kind()` function
- `map_element_to_block()` function
- `is_artifact()` function

### Usage Example
```rust
use pdftract_core::parser::{map_element_to_block, StructElemNode};

// Map a structure element node to its block kind
let result = map_element_to_block(&node);

if result.is_emitted {
    // Emit a block with kind = result.block_kind.as_str()
    if let Some(level) = result.block_kind.heading_level() {
        // Include level in heading block
    }
}

if let Some(diag) = result.diagnostic {
    diagnostics.push(diag);
}
```

### Future Work
- **Phase 3.4 integration**: Connect `is_artifact()` to marked-content tagger to suppress MCIDs inside Artifact marked-content sequences
- **Phase 7.1 walker integration**: Use `map_element_to_block()` in the depth-first walker to classify nodes for output

## Commit
- Commit: `3a2b9c8`
- Message: `feat(pdftract-2ork): implement element-type to block-kind mapping table`

## Retrospective

### What worked
- Clean separation between `BlockKind` (internal enum) and output string representation via `as_str()`
- Comprehensive test coverage for all mapping paths (32 tests covering block-level, inline, structural container, and unknown types)
- `MappingResult` nicely bundles block kind with emit flag and diagnostic

### What didn't
- Initial design didn't include `is_emitted()` method on `BlockKind`, had to duplicate the logic in `MappingResult`. Added `is_emitted()` to `BlockKind` for cleaner API.

### Surprise
- PDF 1.7 has 40+ standard structure types, and the categorization (block-level vs inline vs structural container) isn't always obvious from the spec alone. Had to cross-reference multiple sources to get the mapping right.

### Reusable pattern
- For enum-to-string mapping that needs to support fallback values, use an enum with a derived `as_str()` method that can return different values than the enum variant name (e.g., `Unknown` → "paragraph").

## References
- Plan section 7.1 lines 2552-2553
- PDF 1.7 spec §14.8.4 (Standard Structure Types)
- pdftract-1x2 (StructTree depth-first walker with RoleMap resolution)
