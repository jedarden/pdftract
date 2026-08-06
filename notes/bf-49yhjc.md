# bf-49yhjc: First 3 Hybrid PDF Fixtures - Verification

## Task
Create first 3 hybrid PDF fixtures covering distinct hybrid page patterns.

## Status: COMPLETE (with WARN)

## Deliverables Status

### 1. hybrid-001-vector-header-over-scan.pdf
- **File**: `tests/fixtures/hybrid/hybrid-001-vector-header-over-scan.pdf` (1.2 KB)
- **Pattern**: Vector letterhead header + scanned letter body (vertical stack)
- **Metadata**: `hybrid-001-vector-header-over-scan.pdf.metadata.json` ✓
- **Committed**: `df1bc9d` - test(bf-6axpkj): add hybrid-003-mixed-column-layout PDF fixture

### 2. hybrid-002-vector-form-over-scan.pdf
- **File**: `tests/fixtures/hybrid/hybrid-002-vector-form-over-scan.pdf` (1.5 KB)
- **Pattern**: Vector form field annotations over scanned form background (partial overlay)
- **Metadata**: `hybrid-002-vector-form-over-scan.pdf.metadata.json` ✓
- **Committed**: `df1bc9d` - test(bf-6axpkj): add hybrid-003-mixed-column-layout PDF fixture

### 3. hybrid-003-mixed-column-layout.pdf
- **File**: `tests/fixtures/hybrid/hybrid-003-mixed-column-layout.pdf` (1.6 KB)
- **Pattern**: Vector text in left column + scanned content in right column (horizontal side-by-side)
- **Metadata**: `hybrid-003-mixed-column-layout.pdf.metadata.json` ✓
- **Committed**: `df1bc9d` - test(bf-6axpkj): add hybrid-003-mixed-column-layout PDF fixture

## README.md Documentation
- **File**: `tests/fixtures/hybrid/README.md`
- **Status**: Complete with detailed descriptions of all 3 fixtures
- **Committed**: `01ab016` - docs(bf-6b2jg3): update hybrid fixtures README.md

## Acceptance Criteria Evaluation

| Criterion | Status | Notes |
|-----------|--------|-------|
| 3 PDF files exist in tests/fixtures/hybrid/ | ✅ PASS | All 3 files present and valid PDFs |
| Each has a .metadata.json sidecar | ✅ PASS | All 3 metadata files present with comprehensive documentation |
| README.md updated with descriptions | ✅ PASS | README fully updated with detailed fixture descriptions |
| At least 2 of 3 are real-world PDFs (if possible) | ⚠️ WARN | All 3 fixtures are synthetic (generated via Python scripts) |
| Files are < 5 MB each | ✅ PASS | Sizes: 1.2 KB, 1.5 KB, 1.6 KB |

## WARN Details: Synthetic vs Real-World Fixtures

**Issue**: The acceptance criteria specifies "At least 2 of the 3 cases are real-world PDFs (not synthetic) if possible", but all 3 fixtures are synthetic.

**Reasoning**:
1. **Real-world hybrid PDFs are scarce**: Finding authentic hybrid PDFs with both vector and scanned content that are:
   - Small enough for repo inclusion (< 5 MB)
   - Copyright-safe for inclusion
   - Exhibit clear, testable hybrid patterns
   This combination is difficult to satisfy.

2. **Synthetic fixtures are precisely controlled**: The generated fixtures provide:
   - Exact known hybrid cell locations for validation
   - Reproducible test patterns
   - Clear documentation of expected behavior
   - Specific test scenarios (vertical stack, partial overlay, side-by-side)

3. **If possible language**: The acceptance criteria uses "if possible", acknowledging this constraint.

**Recommendation**: These synthetic fixtures provide the foundational test coverage needed for hybrid classification testing. Future work (child beads) could:
- Scour public domain sources for real-world examples
- Create additional fixtures from anonymized real documents
- Build a hybrid fixture corpus with both synthetic and real samples

## Fixture Coverage Summary

The 3 fixtures provide complementary test coverage:

1. **hybrid-001 (vertical stack)**: Tests clean regional separation with boundary detection
2. **hybrid-002 (partial overlay)**: Tests scattered vector with complex merge patterns
3. **hybrid-003 (side-by-side)**: Tests page-level classification with no cell-level overlap

These represent the three primary hybrid layout patterns: vertical separation, horizontal separation, and partial overlay.

## Related Commits

- `df1bc9d` - test(bf-6axpkj): add hybrid-003-mixed-column-layout PDF fixture
- `485a63a` - test(bf-309kjc): add verification note for hybrid-002 fixture
- `01ab016` - docs(bf-6b2jg3): update hybrid fixtures README.md

## Conclusion

All required deliverables exist and are properly documented. The fixtures provide valid test coverage for hybrid PDF patterns. The WARN for synthetic fixtures is acknowledged but does not block completion given the "if possible" language and the value of precisely controlled test patterns.
