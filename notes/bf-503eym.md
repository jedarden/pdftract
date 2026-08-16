# Verification Note for bf-503eym

## Task
Add Span type assertion to smoke test

## Implementation

### Changes Made
Modified `crates/pdftract-py/tests/smoke_test.py` to add a specific type assertion for `doc.pages[0].spans[0]` as a `pdftract.Span` instance.

### Location
Added new verification section after Page-level type verification (line ~106):
- **Section title:** "Specific first-page span type check"
- **Purpose:** Validate the deepest level of the SDK type hierarchy

### Code Added
```python
# ===== Specific first-page span type check =====
# Verify the first page's first span is properly typed (deepest hierarchy level)

assert len(doc.pages[0].spans) > 0, \
    f"First page should contain at least one span for type verification, found {len(doc.pages[0].spans)} spans"
assert isinstance(doc.pages[0].spans[0], pdftract.Span), \
    f"doc.pages[0].spans[0] should be Span instance, got {type(doc.pages[0].spans[0]).__name__}"
print("✓ doc.pages[0].spans[0] is typed Span instance (deepest hierarchy level validated)")
```

### Acceptance Criteria
- ✅ **PASS:** Test checks `len(doc.pages[0].spans) > 0` before accessing `doc.pages[0].spans[0]`
- ✅ **PASS:** Test includes `isinstance(doc.pages[0].spans[0], pdftract.Span)` check
- ✅ **PASS:** Error message clearly states expected vs. received type
- ✅ **PASS:** Test structure matches existing smoke test pattern (uses same assertion style and print format)
- ✅ **PASS:** Verification note written to notes/bf-503eym.md

### Test Execution
```bash
$ python3 crates/pdftract-py/tests/smoke_test.py
```

**Result:** ✅ ALL SMOKE TESTS PASSED
- Document structure: 1 page(s), 2 span(s)
- Content verification: 2/2 spans with text
- Type contract verification: COMPLETE
- New assertion output: "✓ doc.pages[0].spans[0] is typed Span instance (deepest hierarchy level validated)"

### Why This Matters
Span-level objects are the leaf type in the SDK hierarchy. Ensuring they are properly typed validates the deepest level of the type contract, completing the full type hierarchy verification chain:
- Document (top-level) → Pages (mid-level) → Spans (deepest level)

This ensures end-to-end type integrity across the entire SDK object model.

### Files Modified
- `crates/pdftract-py/tests/smoke_test.py` (added 10 lines)

### Commit Details
To be committed with conventional commit message:
```
test(bf-503eym): add Span type assertion to smoke test

Add specific type verification for doc.pages[0].spans[0] as pdftract.Span
instance to validate deepest level of SDK type hierarchy.

Acceptance criteria:
- Checks len(doc.pages[0].spans) > 0 before accessing
- Includes isinstance(doc.pages[0].spans[0], pdftract.Span) check
- Error message clearly states expected vs. received type
- Test structure matches existing smoke test pattern

Verification: notes/bf-503eym.md, test execution PASS
```

## Related
- Parent bead: bf-ds6pdh (SDK type assertion coverage)
- Plan reference: Smoke test validates SDK type contract
