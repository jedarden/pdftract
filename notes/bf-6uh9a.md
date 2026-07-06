# bf-6uh9a: Clean tests/fixtures and Relocate Useful Generators

## Summary

Verified that the tests/fixtures cleanup and generator relocation work was already completed by previous beads (bf-1iefu, bf-xqib3, bf-2yhak, bf-620xp) on 2026-07-05. This bead completed final cleanup of remaining artifacts that had accumulated since then.

## Previous Cleanup Work (2026-07-05)

### bf-1iefu: Generator Categorization
- Categorized all 17 generator scripts into KEEP (10), DELETE (5), RELOCATE (2)
- Established rationale for each category

### bf-xqib3: Remove Obsolete Generators and Compiled Artifacts
- Removed 5 DELETE-category generators:
  - `scanned/generate_scanned_fixtures.rs` (Rust stub, use Python version)
  - `security/generate_sensitive_fixture.py` (duplicate, use Rust version)
  - `encoding/generate_unmapped_glyphs.rs` (superseded by Python version)
  - `malformed/gen-bomb-10k-2g.sh` (superseded by Python version)
  - `forms/generate_form_fixtures.d` (orphaned Makefile dependency)
- Verified no compiled object files or executable binaries present
- Test data `.bin` files preserved (LZW compression fixtures, not compiled artifacts)

### bf-2yhak: Relocate Useful Generators to Tools
- Relocated `convert_to_scanned.sh` to `tools/convert_pdf_to_scanned.sh`
- Deleted incomplete `regenerate.sh` stub
- Created comprehensive `tools/README.md` with all generators cataloged
- Added documentation headers and usage examples

### bf-620xp: Verify and Document Cleaned Structure
- Created `tests/fixtures/STRUCTURE.md` documenting final state
- Updated `tools/README.md` with cleanup summary section
- Verified all acceptance criteria met

## Current Cleanup (This Bead)

### Artifacts Removed:
1. **`tests/fixtures/scanned/__pycache__/`** - Python bytecode cache directory
   - File: `generate_scanned_fixtures.cpython-312.pyc`
   - Reason: Compiled bytecode artifact, not fixture data

2. **`-.json`** - Stray output file in repo root
   - Content: Empty pdftract JSON output (fingerprint: pdftract-v1:ab24a95f...)
   - Reason: Accidental output file, not a fixture or code

## Final State

### 10 KEEP Generators Remaining in tests/fixtures/
Per STRUCTURE.md, these generators are actively maintained and co-located with their fixtures:

| Script | Category | Purpose |
|--------|----------|---------|
| `encoding/generate_unmapped_glyphs.py` | KEEP | Unmapped glyph test generation |
| `encoding/create_unmapped_comprehensive.py` | KEEP | Comprehensive encoding tests |
| `forms/generate_form_fixtures.rs` | KEEP | AcroForm/XFA fixtures |
| `malformed/gen_bomb.py` | KEEP | Decompression bomb fixtures |
| `scanned/generate_scanned_fixtures.py` | KEEP | OCR scan simulation fixtures |
| `scanned/run_gen.sh` | KEEP | Nix-shell dependency wrapper |
| `scanned/calculate_wer.py` | KEEP | WER/CER calculation utility |
| `scanned/wer_gate_stub.rs` | KEEP | WER gate test stub |
| `security/generate_sensitive_fixture.rs` | KEEP | TH-08 log audit fixtures |
| `vector/generate_vector_cer_corpus.py` | KEEP | CER testing corpus |

### Binary Test Data (Legitimate Fixtures)
- `lzw_*.bin` - LZW compression stream test data (14 files)
- `malformed/random_bytes.bin` - Random corruption data
- `malformed/compression-bomb.bin` - Decompression bomb test fixture

These are **not compiled artifacts** - they are PDF stream test fixtures.

## Acceptance Criteria Status

✅ **tests/fixtures/ contains only fixture data, not generator binaries or drafts**
- 10 KEEP generators remain (actively maintained, co-located with fixtures)
- No gen_encoding_fixtures v2-v9 drafts (removed in bf-xqib3)
- No compiled artifacts (removed in bf-xqib3, verified clean here)

✅ **Useful generators relocated with documentation**
- RELOCATE-category generators moved in bf-2yhak
- tools/README.md comprehensive documentation created
- tests/fixtures/STRUCTURE.md documents final state

✅ **git rm executed for all removed files**
- bf-xqib3: 5 files removed with git rm
- This bead: 1 directory and 1 file removed with git rm

✅ **Single commit with descriptive message**
- Commit: `chore(bf-6uh9a): clean tests/fixtures and relocate useful generators`

## Files Created
- `notes/bf-6uh9a.md` (this verification note)

## Files Deleted
- `tests/fixtures/scanned/__pycache__/` (directory with 1 .pyc file)
- `-.json` (stray output file)

## Documentation References
- [`tests/fixtures/STRUCTURE.md`](../tests/fixtures/STRUCTURE.md) - Complete fixtures directory organization
- [`tools/README.md`](../tools/README.md) - General-purpose tools catalog
- [`notes/bf-1iefu.md`](bf-1iefu.md) - Generator categorization rationale
- [`notes/bf-xqib3.md`](bf-xqib3.md) - Obsolete generator removal
- [`notes/bf-2yhak.md`](bf-2yhak.md) - Tool relocation documentation
- [`notes/bf-620xp.md`](bf-620xp.md) - Structure verification

## Verification

- ✅ No gen_encoding_fixtures v2-v9 drafts present
- ✅ No compiled artifacts (.o, .so, .dll, .dylib, .pyc) present
- ✅ No executable binaries present (only scripts with execute permissions)
- ✅ 10 KEEP generators remain (actively maintained, per bf-1iefu categorization)
- ✅ tools/README.md comprehensive and up-to-date
- ✅ tests/fixtures/STRUCTURE.md documents final state

## Conclusion

The cleanup and generator relocation work was comprehensively completed on 2026-07-05 by beads bf-1iefu, bf-xqib3, bf-2yhak, and bf-620xp. This bead removed the only remaining artifacts that had accumulated since then (Python bytecode cache and an accidental output file), bringing the repository to a fully clean state.
