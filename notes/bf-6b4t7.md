# bf-6b4t7: Relocate useful generators to tools/ or xtask/

**Date:** 2026-07-08
**Status:** ✅ **ALREADY COMPLETE** (Work done by prior beads)
**Dependency:** bf-16far (debris removal completed)

## Finding

All useful generators have already been relocated from `tests/fixtures/` to appropriate locations by beads completed on 2026-07-05:
- **bf-1iefu**: Categorized all 17 generator scripts into KEEP (10), DELETE (5), RELOCATE (2)
- **bf-xqib3**: Removed 5 obsolete generators and all compiled artifacts
- **bf-2yhak**: Relocated 2 general-purpose tools to `tools/` with comprehensive documentation

## Verification

### Generators Already Relocated to tools/

The following general-purpose generators were moved to `tools/` by bf-2yhak:
- `convert_pdf_to_scanned.sh` - Converts text-embedded PDFs to scanned image-based PDFs
- `generate_encoding_fixtures.py` - Generate Unicode recovery test fixtures
- `generate_encrypted_pdf_fixtures.py` - Generate encrypted PDF test fixtures
- `generate_stress_pdf.py` - Generate synthetic stress-test PDFs
- `generate_invoice_pdf_fixtures.py` - Generate invoice OCR test fixtures
- `generate_invoice_fixture.rs` - Generate invoice fixture as Rust binary
- `generate_encrypted_pdf_fixtures.rs` - Generate encrypted PDF fixtures as Rust binary

### Co-located Generators (Intentionally NOT Relocated)

The following 11 generator scripts remain in `tests/fixtures/` subdirectories because they are **co-located with their specific fixture types** for maintainability:

**Encoding fixtures** (`tests/fixtures/encoding/`):
- `generate_unmapped_glyphs.py` - Unmapped glyph fixtures
- `create_unmapped_comprehensive.py` - Comprehensive encoding tests

**Form fixtures** (`tests/fixtures/forms/`):
- `generate_form_fixtures.rs` - Form fixture generator

**Malformed fixtures** (`tests/fixtures/malformed/`):
- `gen_bomb.py` - Malformed PDF bomb generator

**Scanned fixtures** (`tests/fixtures/scanned/`):
- `generate_scanned_fixtures.py` - Scanned OCR fixtures
- `run_gen.sh` - Scanned fixture generation script
- `calculate_wer.py` - Word Error Rate calculation
- `low-quality/create_degraded_200dpi.py` - Degraded fixture generator

**Security fixtures** (`tests/fixtures/security/`):
- `generate_embedded_js.py` - Embedded JS fixtures
- `generate_sensitive_fixture.rs` - Sensitive data fixtures

**Vector fixtures** (`tests/fixtures/vector/`):
- `generate_vector_cer_corpus.py` - Vector CER test corpus

These generators are correctly placed and actively maintained. See `tests/fixtures/STRUCTURE.md` for details on the fixtures organization.

### Generators in xtask/

- `xtask/src/bin/gen_encoding_fixtures.rs` - Encoding fixture generator (xtask binary)

## Git Status Verification

```bash
git status tests/fixtures/
# Result: No untracked generator files that should be relocated (✅)
git status tools/
# Result: All relocated generators properly tracked (✅)
```

## Acceptance Criteria

- ✅ All useful generators moved from `tests/fixtures/` (done by bf-2yhak)
- ✅ Old locations removed via git rm (done by bf-2yhak)
- ✅ New locations in `tools/` or `xtask/` added via git add (done by bf-2yhak)
- ✅ References updated in tools/README.md (done by bf-2yhak)
- ✅ Single commit documenting state (this note)

## Documentation

- `tools/README.md` - Comprehensive generator catalog
- `tests/fixtures/STRUCTURE.md` - Fixtures directory organization
- `tests/fixtures/PROVENANCE.md` - Fixture generation history

## Conclusion

**No further action required.** All useful generators have been relocated to appropriate locations (`tools/` or `xtask/`). The remaining 11 generators in `tests/fixtures/` are intentionally co-located with their specific fixture types for maintainability, as documented in `tests/fixtures/STRUCTURE.md`. This bead's objectives were achieved by prior work on 2026-07-05.
