# pdftract-57o4: ParentTree-based MCID-to-StructElem resolver

## Summary

Implemented the ParentTree resolver that assigns each MCID-tagged marked-content sequence on a page to its owning StructElem. The implementation walks the `/StructTreeRoot /ParentTree` (a number tree keyed by structParents) and produces a per-page map `MCID -> StructElemRef` that the block builder consumes.

## Work Completed

### 1. Core Implementation (already in place)

The following types and functions were already implemented in `crates/pdftract-core/src/parser/struct_tree.rs`:

- **`ParentTreeEntry` enum**: Represents either an array of StructElem refs (for pages, indexed by MCID) or a single StructElem ref (for annotations with `/StructParent`)

- **`ParentTreeResolver` struct**: Caches the resolved ParentTree and provides per-page MCID-to-StructElem mapping
  - `entries: HashMap<i32, ParentTreeEntry>` - Map from /StructParents key to ParentTree entry
  - `diagnostics: Vec<Diagnostic>` - Diagnostics emitted during parsing
  - `struct_elems: HashMap<ObjRef, Rc<StructElemNode>>` - Map from object reference to parsed StructElem node

- **`ParentTreeResolver::parse()`**: Parses a ParentTree from a StructTreeRoot dictionary
  - Extracts `/ParentTree` entry (handles indirect references)
  - Walks the number tree via `walk_number_tree()`
  - Returns a `ParentTreeResolver` with all entries parsed

- **`walk_number_tree()` function**: Walks a number tree (PDF 1.7 7.9.7)
  - Handles both leaf nodes (with `/Nums`) and intermediate nodes (with `/Kids` + `/Limits`)
  - Processes `/Nums` arrays containing alternating key-value pairs
  - Emits diagnostics for malformed nodes

- **`process_nums_array()` function**: Processes a `/Nums` array from a number tree leaf node
  - Extracts integer keys and array/ref values
  - Preserves null entries as `ObjRef { object: 0 }` to mark orphan MCIDs
  - Emits diagnostics for non-integer keys and odd-length arrays

- **`resolve_page()` method**: Resolves MCIDs for a page to their owning StructElem nodes
  - Takes `/StructParents` value from page dictionary
  - Returns `(HashMap<u32, Rc<StructElemNode>>, Vec<u32>)` - MCID map and orphan MCIDs
  - Handles both `ParentTreeEntry::Array` (pages) and `ParentTreeEntry::Single` (annotations)

- **`resolve_annotation()` method**: Resolves an annotation's `/StructParent` to its owning StructElem ref
  - Takes `/StructParent` value from annotation dictionary
  - Returns `Option<ObjRef>` if found

### 2. Test Fixes

Fixed 8 failing tests that were incorrectly structured:

**Problem**: The tests were passing the ParentTree dictionary directly (with `/Nums`) to `ParentTreeResolver::parse()`, but the function expects a StructTreeRoot dictionary containing `/ParentTree`.

**Solution**: Wrapped each test's ParentTree in a StructTreeRoot-like structure:
```rust
// Before (incorrect):
let mut dict = PdfDict::new();
dict.insert(intern("Nums"), nums_array);
let root_obj = PdfObject::Dict(Box::new(dict));
let parent_resolver = ParentTreeResolver::parse(&resolver, &root_obj);

// After (correct):
let mut parent_tree_dict = PdfDict::new();
parent_tree_dict.insert(intern("Nums"), nums_array);
let mut root_dict = PdfDict::new();
root_dict.insert(intern("ParentTree"), PdfObject::Dict(Box::new(parent_tree_dict)));
let root_obj = PdfObject::Dict(Box::new(root_dict));
let parent_resolver = ParentTreeResolver::parse(&resolver, &root_obj);
```

**Tests fixed**:
- `test_parent_tree_leaf_nums` - Simple leaf number tree with /Nums array
- `test_parent_tree_single_ref` - Single ref for annotations
- `test_parent_tree_null_entry` - Null entries in arrays (orphan MCIDs)
- `test_parent_tree_intermediate_kids` - Intermediate nodes with /Kids + /Limits
- `test_parent_tree_malformed_nums_non_integer_key` - Diagnostic for non-integer keys
- `test_parent_tree_malformed_nums_odd_length` - Diagnostic for odd-length arrays
- `test_parent_tree_malformed_unsupported_value_type` - Diagnostic for unsupported value types
- `test_parent_tree_empty_struct_tree_root` - Integration with parse_struct_tree

### 3. Bug Fix: Null Entry Preservation

**Problem**: The `process_nums_array()` function was using `filter_map(|o| o.as_ref())` which filtered out `PdfObject::Null` entries. This caused orphan MCIDs to be lost.

**Solution**: Changed the array processing to preserve null entries as `ObjRef { object: 0, generation: 0 }`:
```rust
// Before (incorrect):
let refs: Vec<ObjRef> = arr.as_ref()
    .iter()
    .filter_map(|o| o.as_ref())
    .collect();

// After (correct):
let refs: Vec<ObjRef> = arr.as_ref()
    .iter()
    .map(|o| match o {
        PdfObject::Ref(r) => *r,
        PdfObject::Null => ObjRef { object: 0, generation: 0 },
        _ => ObjRef { object: 0, generation: 0 }, // Invalid ref treated as null
    })
    .collect();
```

The `resolve_page()` function already checks for `elem_ref.object == 0` as a null marker, so this fix ensures orphan MCIDs are correctly reported.

## Acceptance Criteria Status

- [x] **PASS**: ParentTree walked correctly for both numeric tree shapes (Kids+Limits, leaf Names)
- [x] **PASS**: Per-page map built; orphan MCIDs recorded
- [x] **PASS**: Unit tests: synthetic ParentTree with valid + malformed + missing entries
- [x] **PASS**: Test fixture: Integration with parse_struct_tree (empty StructTreeRoot with ParentTree)
- [x] **PASS**: Annotations with /StructParent point INTO the structure tree
- [x] **PASS**: Malformed ParentTree handling (off-by-one indexing, missing entries) - emits diagnostics without crashing

## Files Modified

- `crates/pdftract-core/src/parser/struct_tree.rs`:
  - Fixed `process_nums_array()` to preserve null entries as `ObjRef { object: 0 }`
  - Fixed 8 tests to correctly wrap ParentTree in StructTreeRoot structure

## Test Results

All 65 struct_tree tests pass:
```bash
$ cargo test -p pdftract-core --lib struct_tree
test result: ok. 65 passed; 0 failed; 0 ignored; 0 measured; 886 filtered out
```

## Integration Points

- **`parse_struct_tree()`**: Calls `ParentTreeResolver::parse()` and sets the struct_elems map via `set_struct_elems()`
- **Phase 7.1.4 (coverage check)**: Will consume the per-page MCID map and orphan list from `resolve_page()`
- **Block builder**: Will use the MCID-to-StructElem map to reconstruct blocks

## References

- Plan section: 7.1 line 2550 (MCID-to-StructElem mapping)
- PDF 1.7 spec 14.7.4.4 ParentTree
- PDF 1.7 spec 7.9.7 Number Tree
- Phase 3.4 marked-content tagger (MCID source)
