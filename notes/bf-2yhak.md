# bf-2yhak: Relocate Useful Generators to Tools with Documentation

## Summary

Successfully relocated useful generators from `tests/fixtures/` to `tools/` with comprehensive documentation as specified in bead bf-2yhak.

## Changes Made

### 1. Relocated `convert_to_scanned.sh` to `tools/`

**Source:** `tests/fixtures/scanned/convert_to_scanned.sh`  
**Destination:** `tools/convert_pdf_to_scanned.sh`

**Enhancements:**
- Added comprehensive documentation header following existing tools pattern
- Documented fixtures generated (invoice, letter, form at 300 DPI)
- Documented requirements (pdftoppm, ImageMagick, nix-shell)
- Documented conversion process (backup → PPM → PDF)
- Renamed to `convert_pdf_to_scanned.sh` for clarity

### 2. Deleted incomplete `regenerate.sh` stub

**Removed:** `tests/fixtures/grep-corpus/regenerate.sh`

**Reason:** Incomplete stub with TODOs as identified in bf-1iefu categorization. This script was marked as RELOCATE but recommended for deletion unless prioritized, since it only contained placeholder structure.

### 3. Created comprehensive `tools/README.md`

**Content:**
- Categorized documentation for all generators:
  - Python generators (encoding, encrypted, stress, invoice, docs)
  - Rust generators (invoice, encrypted)
  - Shell scripts (convert, count, release notes)
  - Debug tools (fingerprint, diff)
  - Specialized builders (objstm, xref)
- Usage examples for each generator
- Requirements and dependencies
- Development workflow examples
- Contributing guidelines

## Acceptance Criteria Status

✅ **All RELOCATE-category generators handled:**
- `convert_to_scanned.sh` → Relocated to `tools/` with documentation
- `regenerate.sh` → Deleted (incomplete stub)

✅ **Each generator has clear documentation:**
- `convert_pdf_to_scanned.sh` includes comprehensive header comment
- All existing generators documented in `tools/README.md`

✅ **`tools/README.md` created:**
- Complete catalog of all 18 tools/generators
- Usage examples for each tool
- Dependency information
- Development workflow section

✅ **Changes committed:**
- Commit: `chore(bf-2yhak): relocate useful generators to tools/`

## Verification

**Files Created:**
- `tools/convert_pdf_to_scanned.sh` (3093 bytes)
- `tools/README.md` (6296 bytes)
- `notes/bf-2yhak.md` (this verification note)

**Files Deleted:**
- `tests/fixtures/grep-corpus/regenerate.sh`

**Files Moved:**
- `tests/fixtures/scanned/convert_to_scanned.sh` → `tools/convert_pdf_to_scanned.sh`

**Documentation Quality:**
- ✅ Comprehensive header in `convert_pdf_to_scanned.sh`
- ✅ Complete `tools/README.md` with all generators cataloged
- ✅ Usage examples provided for each tool
- ✅ Dependencies documented

**Committed Work:**
- Single commit covering relocation, deletion, and documentation
- Follows conventional commit format: `chore(bf-2yhak): ...`

## Notes

- Followed existing pattern from `generate_encoding_fixtures.py` for documentation style
- Maintained executable permissions on shell script
- `tools/README.md` serves as central documentation for all 18 tools/generators
- Contributing guidelines added for future generator additions
