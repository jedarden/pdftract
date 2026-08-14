# Cleanup Manifest for tests/fixtures/

Generated during bead bf-24po9b (parent: bf-6uh9a)

## Files to Remove - Generator Debris

### Root level
- `mod.rs` - Rust module fragment, not fixture data
- `create_markdown_structure_fixture.py` - Generator script, not fixture data
- `markdown_test_fixture.py` - Generator script, not fixture data

### hybrid/ subdirectory
- `hybrid-001-generator.py` - Generator script
- `hybrid-002-generator.py` - Generator script
- `hybrid-003-generator.py` - Generator script
- `hybrid-004-generator.py` - Generator script
- `hybrid-005-generator.py` - Generator script
- `hybrid-006-generator.py` - Generator script
- `hybrid-007-generator.py` - Generator script
- `hybrid-008-generator.py` - Generator script
- `hybrid-009-generator.py` - Generator script
- `hybrid-010-generator.py` - Generator script
- `create_hybrid_001.py` - Generator script
- `generate_hybrid_fixtures.py` - Generator script
- `mod.rs` - Rust module fragment, not fixture data

## Verification

No compiled ELF binaries found in tests/fixtures/ (good - already clean per repo hygiene).

All identified files are generator scripts (.py, .rs) that should be removed to keep tests/fixtures/ containing only fixture data, not generator code.
