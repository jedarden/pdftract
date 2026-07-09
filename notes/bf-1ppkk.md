# bf-1ppkk: Document relocated generators with usage notes

**Date:** 2026-07-08
**Status:** ✅ **COMPLETE**

## Task

Add documentation for all relocated generators explaining their purpose and usage.

## Work Completed

### Created xtask/README.md

Created comprehensive documentation for all xtask generators (515 lines):

#### Documented xtask Commands
1. **Schema Generation**
   - `gen-schema` - Generate JSON Schema from Rust output types
   - `validate-schema` - Validate checked-in schema matches generated

2. **Documentation Generation**
   - `gen-cli_reference` - Generate CLI reference from clap tree
   - `rustdoc_coverage` - Calculate rustdoc coverage (target: 80%)
   - `doc-profile <name>` - Generate profile README from YAML
   - `doc-profiles` - Generate all profile READMEs

3. **PDF Fixture Generators**
   - `generate-tagged-fixtures` - PDF/UA and PDF/A tagged fixtures
   - `generate-stress-pdfs` - Memory ceiling test fixtures (100-page, 10k-page)
   - `generate-page-class-fixtures` - Page classification fixtures (Vector, Scanned, Hybrid, BrokenVector)
   - `generate-brokenvector-fixtures` - OCR test fixtures (aligned/misaligned)
   - `generate-sensitive-fixture` - TH-08 log audit test fixtures
   - `gen-shape-db <fonts>` - Glyph shape database for Level 4 Unicode recovery

4. **Testing & Quality**
   - `memory-ceiling` - Enforce Tier-1 memory targets (512MB buffered, 256MB streaming, 1GB adversarial)

5. **Binary-Specific Generators**
   - `gen_encoding_fixtures` - Unicode recovery Levels 2-4 fixtures
   - `gen_form_fixtures` - AcroForm and XFA fixtures
   - `gen_scanned_fixtures` - Scanned OCR fixtures
   - `gen_lzw_fixtures` - LZW compression fixtures
   - `gen_unmapped_fixtures` - Unmapped Unicode fixtures
   - `generate_document_json` - Schema validation fixtures
   - `migrate_schema` - Schema migration tool

### Documentation Structure

Each generator section includes:
- **Purpose:** What the generator does and why it exists
- **Output:** Files and directories created
- **When to run:** Guidance on when regeneration is needed
- **Usage:** Command-line examples
- **What it does:** Implementation details and process description

### Additional Sections

- **Running xtask Commands** - How to invoke xtask from workspace root
- **Development Workflow** - Typical development cycle with commands
- **Adding a New xtask Command** - Guide for extending xtask
- **Related Documentation** - Links to fixtures structure, provenance, tools README
- **Maintenance Notes** - Schema versioning, fixture idempotency, cross-platform notes
- **Conventions** - Output paths, error handling, workspace root location

## Acceptance Criteria

- ✅ Each generator has clear documentation explaining its purpose
- ✅ Usage examples provided for each generator
- ✅ README.md exists in xtask/
- ✅ Single commit: `chore(bf-1ppkk): document relocated generators with usage notes`

## Verification

### Commit Details

**Commit:** 765a3639
**Message:** chore(bf-1ppkk): document relocated generators with usage notes

**Files Added:**
- `xtask/README.md` (515 lines)

**Files Modified:** None

### Git Status

```bash
git status
# On branch main
# Your branch is ahead of 'origin/main' by 416 commits.
```

### Push Status

```bash
git push origin main
# To https://git.ardenone.com/jedarden/pdftract.git
#    47632cb8..765a3639  main -> main
```

### Documentation Coverage

- **tools/README.md:** Already comprehensive (created by bf-2yhak on 2026-07-05)
- **xtask/README.md:** Now comprehensive (created by this bead)

Both directories now have complete documentation for all generators.

## Related Documentation

- `tools/README.md` - Python and shell generator scripts
- `tests/fixtures/STRUCTURE.md` - Complete fixtures directory organization
- `tests/fixtures/PROVENANCE.md` - Generation history for each fixture
- `docs/schema/v1.0/pdftract.schema.json` - JSON Schema for extraction output
- `docs/user-docs/src/cli-reference.md` - CLI reference documentation

## Conclusion

All relocated generators in `tools/` and `xtask/` are now fully documented with:
- Clear purpose and motivation
- Usage examples and command-line options
- Fixture outputs and when-to-run guidance
- Development workflow and maintenance notes

The documentation provides a complete reference for developers working with pdftract's fixture generation and development tooling.
