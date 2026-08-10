# bf-3vp9ku: Add page count validation to detect empty documents

## Summary
Bead verified as **already implemented**. The required page count validation to detect empty documents is already present in `validate_pages_structure()` function.

## Implementation Verified

### 1. Zero Page Count Detection
**Location:** `crates/pdftract-core/src/document.rs:946`
```rust
match count_pages_tree(resolver, catalog.pages_ref) {
    Ok(page_count) => {
        if page_count == 0 {
            return Err(DocumentError::EmptyDocument {
                source: source_identifier.to_string(),
            });
        }
```

### 2. Empty /Kids Array Detection
**Location:** `crates/pdftract-core/src/document.rs:904-908`
```rust
match kids_ref {
    Some(crate::parser::object::PdfObject::Array(kids_array)) if kids_array.is_empty() => {
        // /Kids array is explicitly empty - document has no pages
        return Err(DocumentError::EmptyDocument {
            source: source_identifier.to_string(),
        });
    }
```

### 3. Source Identifier in Error Messages
Both checks include `source: source_identifier.to_string()` to ensure error messages include the source identifier.

### 4. Correct Ordering
The implementation follows the required ordering per bead specification:
- **Check 0 (line 759):** Catalog dictionary validation
- **Check 1 (line 828):** Validate catalog.pages_ref is non-zero
- **Check 2 (line 837):** Resolve pages reference
- **Check 3 (line 848):** Verify resolved object is a dictionary
- **Check 3.5 (line 859):** Verify Pages dictionary Type field
- **Check 4 (line 892):** Validate /Kids array (empty/null check)
- **Check 5 (line 943):** Validate page count via count_pages_tree (zero check)

This ordering ensures catalog validation happens BEFORE any pages access, preventing panics on invalid references.

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Zero page count returns DocumentError::EmptyDocument | ✅ PASS | Line 946 |
| Empty /Kids array returns DocumentError::EmptyDocument | ✅ PASS | Line 904 |
| Error message includes source identifier | ✅ PASS | Both checks include source |
| No panic on zero page count | ✅ PASS | Check occurs before iteration |
| Test coverage for zero page variants | ✅ PASS | 16 tests pass |

## Test Coverage

All 16 tests in `catalog_emptiness_checks.rs` pass:
- Test 1-3: Catalog dictionary emptiness detection (Check 0)
- Test 4: Error message includes source identifier
- Test 5: Valid catalog passes through normally
- Test 6-8: Various None catalog types and detection order
- Test 9: No panic or hang on empty catalog
- Test 10-15: Catalog with /Pages key but null/wrong type
- **Test 16, test case 4:** Empty /Kids array detection (lines 636-668)

Test 16, test case 4 specifically validates empty /Kids array:
```rust
// Create a Pages node with empty /Kids array
let mut pages_dict = indexmap::IndexMap::new();
pages_dict.insert("Type".into(), PdfObject::Name("Pages".into()));
pages_dict.insert("Kids".into(), PdfObject::Array(Box::new(vec![])));
pages_dict.insert("Count".into(), PdfObject::Integer(0));
```

## No Code Changes Required

The implementation was completed previously. The bead requirements are fully satisfied by the existing code in `document.rs` and the comprehensive test suite in `catalog_emptiness_checks.rs`.

## Verification Commands

```bash
# Run all catalog emptiness checks
cargo test --test catalog_emptiness_checks

# Verify implementation
grep -n "page_count == 0" crates/pdftract-core/src/document.rs  # Line 946
grep -n "kids_array.is_empty()" crates/pdftract-core/src/document.rs  # Line 904
```

## References
- Plan lines 3880-3910 (Edge case validation)
- Parent bead: bf-34zi7m
- Depends on: bf-5xofrl (catalog check must come first)
