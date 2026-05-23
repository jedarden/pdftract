# pdftract-1x2: StructTree depth-first walker + /RoleMap resolution

## Summary

Implemented the depth-first walker over the PDF structure tree (/StructTreeRoot) with complete /RoleMap resolution support. This is Phase 7.1.1 of the plan.

## Implementation

### Files Modified/Created
- `crates/pdftract-core/src/parser/struct_tree.rs` (new, 1215 lines)
- `crates/pdftract-core/src/parser/mod.rs` (added exports)

### Core Types
- `StructureType`: Enum covering all 40+ PDF 1.7 standard structure types (Document, Part, Art, Sect, Div, P, H1..H6, Table, Figure, etc.)
- `Kid`: Enum for /K array entries (Element, Mcid, Mcr, ObjRef)
- `StructElemNode`: Tree node with resolved type, inherited lang/actual_text, and children
- `StructTreeRoot`: Root container with kids array and RoleMap
- `RoleMap`: Mapping from non-standard to standard types with chain resolution

### Key Features
1. **Depth-first traversal** via /K array handling all four entry types:
   - StructElem dictionary (recursive)
   - Integer MCID (direct marked content reference)
   - MCR dictionary (marked content reference with explicit page)
   - OBJR dictionary (annotation/XObject reference)

2. **RoleMap resolution** with:
   - Chain following (A -> B -> H1)
   - Cycle detection (A -> B -> A → NonStruct with diagnostic)
   - Standard type detection (no lookup needed for "P", "H1", etc.)

3. **Attribute inheritance**:
   - /Lang inherits from parent if not present on node
   - /ActualText inherits from parent (overrides all descendant glyph text)
   - /Alt (alternative text) extracted per-node
   - /ID, /T, /E, /Pg also extracted

## Verification

### Unit Tests (17 tests, all PASS)
```
test parser::struct_tree::tests::test_structure_type_from_name ... ok
test parser::struct_tree::tests::test_structure_type_is_heading ... ok
test parser::struct_tree::tests::test_structure_type_heading_level ... ok
test parser::struct_tree::tests::test_role_map_parse ... ok
test parser::struct_tree::tests::test_role_map_resolve ... ok
test parser::struct_tree::tests::test_role_map_chaining ... ok
test parser::struct_tree::tests::test_role_map_cycle_detection ... ok
test parser::struct_tree::tests::test_role_map_self_mapping ... ok
test parser::struct_tree::tests::test_struct_elem_node_new ... ok
test parser::struct_tree::tests::test_struct_tree_root_new ... ok
test parser::struct_tree::tests::test_struct_tree_root_default ... ok
test parser::struct_tree::tests::test_struct_tree_word_rolemap_integration ... ok
test parser::struct_tree::tests::test_struct_tree_lang_inheritance ... ok
test parser::struct_tree::tests::test_struct_tree_actual_text_scope ... ok
test parser::struct_tree::tests::test_struct_tree_mcr_kid ... ok
test parser::struct_tree::tests::test_struct_tree_objr_kid ... ok
test parser::struct_tree::tests::test_struct_tree_mcid_kid ... ok
```

### Acceptance Criteria Status
- ✓ Walker handles all four /K element kinds (Element, MCID, MCR, OBJR) without crashing
- ✓ /RoleMap chains resolve to a standard type or NonStruct
- ✓ /Lang and /ActualText inherit correctly down the tree
- ✓ Unit tests: fixtures with Word RoleMap (Heading1 -> H1)
- ✓ Unit tests: nested /Lang, /ActualText scope
- ✓ Public type StructElemNode is documented in the core crate

## Commit
- Commit: `d41d47d`
- Message: `feat(pdftract-1x2): implement StructTree depth-first walker with RoleMap resolution`

## Retrospective

### What worked
- The implementation followed the PDF 1.7 spec closely, with clear separation between parsing and type resolution
- Test-driven approach worked well - each kid type, RoleMap feature, and inheritance pattern has dedicated tests
- Using `indexmap::IndexMap` for RoleMap preserves insertion order while providing O(1) lookups

### What didn't
- Initial compilation error: mismatched types in match arms when resolving RoleMap references. Fixed by restructuring the error handling to assign to `root.role_map` directly rather than trying to return `RoleMap::new()` from a `PdfObject`-returning match.

### Surprise
- The RoleMap can map to another non-standard name (chains), not directly to standard types. This required recursive resolution with cycle detection.

### Reusable pattern
- For recursive type resolution through a mapping: track visited keys in a HashSet, emit diagnostic on cycle, return a safe fallback type.
- For inheritance in tree walkers: pass inherited values as `Option<&str>` to recursive calls, use `node.lang = lang.or_else(|| parent_lang.map(|s| s.to_string()))` to prefer local over inherited.

## References
- Plan section 7.1 StructTree Exploitation (lines 2547-2549, 2552-2553)
- PDF 1.7 spec §14.7.4 (Structure Tree) and §14.8.4 (Standard Structure Types)
