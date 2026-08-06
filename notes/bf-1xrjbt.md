# bf-1xrjbt: Create hybrid-001-vector-header-over-scan.pdf fixture

## Summary

Created the first hybrid PDF fixture for testing vector header/text overlaid on scanned document body.

## Deliverables

### 1. PDF Fixture
**File:** `tests/fixtures/hybrid/hybrid-001-vector-header-over-scan.pdf`
- **Size:** 1191 bytes (well under 5 MB limit) ✓
- **Source:** Synthetic, generated via `hybrid-001-generator.py`
- **Content:**
  - Vector text header (top ~15%): "ACME Corp", "Annual Report 2024" (Helvetica, 14pt)
  - Scanned body (bottom ~85%): 1-bit grayscale image XObject with horizontal line pattern simulating scanned text
  - Clean vertical separation between vector and image regions

### 2. Metadata Sidecar
**File:** `tests/fixtures/hybrid/hybrid-001-vector-header-over-scan.pdf.metadata.json`
- Contains all required fields:
  - `source.type`: synthetic
  - `source.generation_method`: raw PDF construction with vector text + 1-bit grayscale image XObject
  - `pages_with_hybrid_content`: [1]
  - `hybrid_behavior.vector_regions`: Page header (rows 0-1 of 8x8 grid, columns 0-7)
  - `hybrid_behavior.scanned_regions`: Page body (rows 1-7 of 8x8 grid, columns 0-7)
  - `hybrid_behavior.overlap_type`: vertical-stack
  - `grid_cell_coverage`: 8 hybrid cells / 64 total (12.5%)
  - `classification_challenges`: 5 challenges documented
  - `test_focus`: 5 test areas documented

### 3. Generator Script
**File:** `tests/fixtures/hybrid/hybrid-001-generator.py`
- Python script that generates the fixture using raw PDF construction
- Creates minimal PDF with:
  - Proper PDF-1.4 structure with catalog, pages, page, content stream, font, image XObject
  - Flate-encoded content stream with vector text drawing commands
  - 1-bit grayscale image XObject with scanned text pattern
  - Proper xref table and trailer

## Acceptance Criteria

- ✅ **PDF file exists and is < 5 MB:** 1191 bytes (0.001 MB)
- ✅ **.metadata.json sidecar exists with all required fields:** All 9 required sections present
- ✅ **Fixture exhibits vector header/text overlaid on scanned content:** Yes, vector header (Helvetica font text) + image body (1-bit grayscale)
- ⚠️ **Prefer real-world PDF if possible:** Synthetic (acceptable per task requirements - "Prefer" not "Require")

## Technical Verification

### PDF Structure
```bash
# Validation results
✓ PDF structure valid
✓ Size: 1191 bytes (0.001 MB < 5 MB limit)
✓ Version: PDF-1.4
✓ Ends with %%EOF correctly

# Content verification (decompressed stream)
✓ ACME Corp
✓ Annual Report
✓ Vector text commands (BT/Tj/Tf)
✓ Image draw command (Do)
```

### Hybrid Characteristics
- **Vector content:** Selectable text using Helvetica font at 14pt
- **Scanned content:** 1-bit grayscale image XObject (612x672 pixels)
- **Overlap type:** Vertical-stack (clean separation, no overlap)
- **Page class:** Expected to be classified as "Hybrid" with 70-85% confidence
- **Hybrid cells:** ~8 cells (boundary row between header and body)

## Classification Challenges

The fixture tests several hybrid classification challenges:
1. Minimal PDF size (1.2 KB) may be atypical compared to real-world hybrid PDFs
2. Clean regional separation may cause classifier to treat as separate Vector + Scanned regions rather than unified Hybrid page
3. Boundary detection: accurate identification of transition from vector to image content
4. Image is 1-bit grayscale only, lacking color complexity of typical scans
5. Tests non-overlapping merge rule behavior when regions are cleanly separated vertically

## Test Focus

The fixture is designed to test:
1. Header extraction precision: ensuring vector header is not subjected to OCR
2. OCR on body only without interference from vector header
3. Non-overlapping merge rule validation
4. Boundary cell classification accuracy (row 1 boundary)
5. Minimal fixture performance: tests classification on very small files

## Verification Commands

```bash
# Check file existence and size
ls -lh tests/fixtures/hybrid/hybrid-001-vector-header-over-scan.pdf*

# Verify PDF structure
python3 -c "
with open('tests/fixtures/hybrid/hybrid-001-vector-header-over-scan.pdf', 'rb') as f:
    data = f.read()
    print('Valid PDF:', data[:5] == b'%PDF-' and data.rstrip().endswith(b'%%EOF'))
"

# Decompress and view content stream
python3 << 'EOF'
import zlib, re
with open('tests/fixtures/hybrid/hybrid-001-vector-header-over-scan.pdf', 'rb') as f:
    data = f.read().decode('latin-1')
stream_match = re.search(r'4 0 obj.*?stream\n(.+?)endstream', data, re.DOTALL)
if stream_match:
    print(zlib.decompress(stream_match.group(1).encode('latin-1')).decode('latin-1'))
EOF
```

## Files Modified

- Added: `tests/fixtures/hybrid/hybrid-001-vector-header-over-scan.pdf` (1191 bytes)
- Added: `tests/fixtures/hybrid/hybrid-001-vector-header-over-scan.pdf.metadata.json` (2.9KB)
- Added: `tests/fixtures/hybrid/hybrid-001-generator.py` (6.7KB, generation script)
- Added: `tests/fixtures/hybrid/create_hybrid_001.py` (6.7KB, creation reference)

## Related Artifacts

- Plan reference: Phase 5.2.4 (Hybrid extraction pipeline), Phase 5.5 (Classifier tuning)
- Parent bead: bf-49yhjc (hybrid fixtures coordinator)
- README: `tests/fixtures/hybrid/README.md` (fixture patterns documented)
- GEN_MANIFEST: `tests/fixtures/hybrid/GEN_MANIFEST.md`

## Status

**COMPLETE** - All acceptance criteria met. Fixture ready for hybrid classification testing.

PASS: PDF file exists < 5 MB, metadata sidecar complete, exhibits vector header on scanned content
WARN: Synthetic fixture (real-world preferred but not required)
