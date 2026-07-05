# bf-632yt: Create corpus validation script

## Summary

Created `scripts/validate-corpus.sh` to verify corpus integrity against `manifest.csv`.

## Implementation

The script validates all 5 required manifest fields:
1. **File existence** - Verifies all files listed in manifest.csv exist
2. **File size** - Checks file sizes match manifest
3. **SHA256 checksums** - Validates checksums using `sha256sum`
4. **License information** - Verifies license field is present and valid
5. **Counts** - Reports total files, pages, and size

## Testing Results

Ran validation against `tests/fixtures/grep-corpus`:

- **Total files in manifest:** 1,260
- **Valid files:** 603 (with known licenses)
- **Missing files:** 0
- **Size mismatches:** 0
- **Checksum mismatches:** 0
- **Missing licenses:** 657 (PDFium test files with "unknown" license)

The 657 files with "unknown" licenses are from the PDFium test corpus, which is expected behavior. These files correctly fail the license validation check as designed.

## Exit Codes

- `0` - Validation passed (all files valid)
- `1` - Validation failed (errors found)
- `2` - Usage error or manifest not found

## Usage

```bash
./scripts/validate-corpus.sh [corpus_directory]
```

Default corpus directory: `tests/fixtures/grep-corpus`

## Acceptance Criteria Status

- ✅ scripts/validate-corpus.sh exists and is executable (chmod +x)
- ✅ Script validates all 5 manifest fields (existence, size, checksum, count, license)
- ✅ Returns 0 on success, non-zero on validation failure
- ✅ Outputs summary: total files, total pages, total size, validation status

## Commit

- Commit: `c189bfd2`
- Files changed: `scripts/validate-corpus.sh` (1 file, 114 insertions, 147 deletions)
