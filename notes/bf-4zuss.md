# bf-4zuss: Corpus manifest and validation

## Summary
Verified that the grep-corpus manifest and validation infrastructure is fully operational.

## Acceptance Criteria Verification

### PASS: `tests/fixtures/grep-corpus/manifest.csv` exists with proper schema
- **Location**: `/home/coding/pdftract/tests/fixtures/grep-corpus/manifest.csv`
- **Schema**: filename,source_url,page_count,file_size,checksum,license
- **Sample entry**: `synthetic_1000.pdf,synthetic-generation,5,3751,<sha256>,public-domain`

### PASS: `scripts/validate-corpus.sh` exists and is executable
- **Location**: `/home/coding/pdftract/scripts/validate-corpus.sh`
- **Permissions**: `-rwxr-xr-x` (executable)
- **Lines**: 183 lines of bash validation logic

### PASS: `make validate-corpus` runs validation successfully
```bash
$ make validate-corpus
Validating grep-corpus...
[INFO] Validated 200 files...
[INFO] Validated 400 files...
[INFO] Validated 600 files...
[INFO] Validated 800 files...
[INFO] Validated 1000 files...

=== Corpus Validation Summary ===
Total files in manifest: 1000
Valid files:             1000

Corpus metrics (for valid files):
  Total pages:  10590
  Total size:   6870643 bytes

[✓] VALIDATION PASSED
```

### PASS: Manifest contains ≥1000 entries with all required fields
- **Entry count**: 1000 PDF files
- **All fields present**: filename, source_url, page_count, file_size, checksum, license
- **All entries validated**: 0 missing files, 0 size mismatches, 0 checksum mismatches

### PASS: Validation script confirms corpus meets size/count targets
- **Files validated**: 1000/1000
- **Total pages**: 10,590 pages
- **Total size**: 6,870,643 bytes (~6.5 MB)
- **Licenses**: All synthetic PDFs marked as `public-domain`

## Related Files

### Core Implementation
- `tests/fixtures/grep-corpus/manifest.csv` - Corpus manifest with 1000 entries
- `tests/fixtures/grep-corpus/corpus/` - Directory containing 1000 synthetic PDFs
- `scripts/validate-corpus.sh` - Validation script that verifies:
  - File existence
  - File size matching
  - SHA256 checksum verification
  - License presence
  - Corpus quality metrics

### Supporting Scripts
- `scripts/download-grep-corpus.sh` - Downloads PDFs and generates manifest entries
- `scripts/grep-corpus-generate-manifest.sh` - Bootstraps manifest for existing corpus

### Makefile Integration
- `Makefile:22-24` - `validate-corpus` target that calls the validation script

## Corpus Quality Metrics
- **Corpus type**: Synthetic PDFs generated via ReportLab (public-domain)
- **Page count distribution**: 1-20 pages per PDF
- **Average file size**: ~6.9 KB per PDF
- **Total corpus size**: ~6.5 MB for 1000 PDFs
- **Validation status**: All 1000 files valid with correct checksums

## Implementation Notes
The corpus generation uses synthetic PDFs rather than downloaded content to ensure:
1. **Determinism**: Same script produces identical corpus
2. **Known license**: Public domain (no licensing concerns)
3. **Controlled variety**: Mix of page counts (1-20) and content
4. **Fast regeneration**: No network dependencies

## References
- Parent bead: bf-38sa3
- Phase 7.8 (plan lines 2796–2824)
