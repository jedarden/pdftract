# bf-5for4: Design manifest schema for grep-corpus

## Summary
Designed and documented the CSV manifest schema for the grep-corpus benchmark corpus.

## Changes Made

### 1. Created `tests/fixtures/grep-corpus/manifest.csv`
- Added CSV header row with fields: `filename,source_url,page_count,file_size,checksum,license`
- Included inline documentation explaining each field
- Left placeholder for population during corpus generation (blocked on 7.8.x grep implementation)

### 2. Updated `tests/fixtures/grep-corpus/README.md`
- Updated "Manifest Format" section with new schema
- Added detailed field descriptions:
  - `filename`: Relative path from corpus/ directory
  - `source_url`: Download source URL
  - `page_count`: Number of pages
  - `file_size`: Size in bytes
  - `checksum`: SHA256 for integrity verification
  - `license`: License identifier (public-domain, cc-by-4.0, cc-by-sa-4.0, mit, other)
- Added example CSV rows

## Acceptance Criteria Status
- ✅ `tests/fixtures/grep-corpus/manifest.csv` exists with proper header
- ✅ Schema documented in README.md
- ✅ Header row includes: filename,source_url,page_count,file_size,checksum,license

## References
- Parent: bf-4zuss (Phase 7.8 grep-corpus infrastructure)
- Plan lines 2796-2824 (Phase 7.8 grep-corpus benchmark)
