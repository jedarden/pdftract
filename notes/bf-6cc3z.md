# Bead bf-6cc3z: Implement basic glob PDF discovery

## Summary
Implemented glob-based PDF discovery functionality for finding all PDF files in the fixtures directory.

## Work Completed

### 1. Added glob dependency to workspace
- Added `glob = "0.3"` to workspace dependencies in `/home/coding/pdftract/Cargo.toml`
- Added `glob = "0.3"` to pdftract-cli dependencies in `/home/coding/pdftract/crates/pdftract-cli/Cargo.toml`

### 2. Enhanced fixture_discovery.rs with glob functions
Updated `/home/coding/pdftract/tests/fixture_discovery.rs` with new glob-based functions:
- `discover_pdf_fixtures_glob()` - Uses glob pattern `**/*.pdf` to recursively discover PDF files
- `discover_pdf_fixtures_glob_relative()` - Returns relative paths from fixtures directory
- Added comprehensive tests for glob-based discovery functionality

### 3. Created glob discovery test program
Created `/home/coding/pdftract/tests/test_glob_discovery.rs`:
- Standalone test program demonstrating glob-based PDF discovery
- Uses glob pattern matching: `fixtures/**/*.pdf`
- Returns sorted collection of PathBuf objects
- Added as binary target in pdftract-cli Cargo.toml

## Acceptance Criteria Status

✅ **PASS**: Code discovers all .pdf files in fixtures/
- Successfully discovered 3,353 PDF files in tests/fixtures/
- Verified all discovered files exist and are valid PDFs

✅ **PASS**: Uses glob pattern matching  
- Uses `glob::glob()` with pattern `**/*.pdf` for recursive discovery
- Pattern: `{fixtures_path}/**/*.pdf`

✅ **PASS**: Returns paths as a collection
- Returns `Vec<PathBuf>` containing discovered file paths
- Paths are sorted alphabetically for consistent ordering

## Verification

### Build Test
```bash
cargo build --package pdftract-cli --bin test_glob_discovery
```
✅ Compiled successfully

### Runtime Test
```bash
cargo run --package pdftract-cli --bin test_glob_discovery
```
✅ Output:
- Total PDF files discovered: 3,353
- All files exist: true
- All files are PDFs: true  
- Paths are sorted: true

### Test Output (first 20 files)
```
First 20 PDF files:
  1. tests/fixtures/cjk/cjk-chinese-gb18030.pdf
  2. tests/fixtures/cjk/cjk-japanese-shiftjis.pdf
  3. tests/fixtures/cjk/cjk-korean-euckr.pdf
  4. tests/fixtures/cjk/cjk-tc-big5.pdf
  5. tests/fixtures/classifier/contract/01.pdf
  ... (and 3333 more files)
```

## Files Changed
- `/home/coding/pdftract/Cargo.toml` - Added glob to workspace dependencies
- `/home/coding/pdftract/crates/pdftract-cli/Cargo.toml` - Added glob dependency and binary target
- `/home/coding/pdftract/tests/fixture_discovery.rs` - Added glob-based discovery functions and tests
- `/home/coding/pdftract/tests/test_glob_discovery.rs` - Created test program (NEW)

## Technical Details

The glob pattern `**/*.pdf` provides recursive matching:
- `**` matches zero or more directories
- `*.pdf` matches any file ending with .pdf extension
- Pattern is applied to the fixtures base path
- Results are sorted for deterministic ordering

This implementation uses the standard `glob` crate which is widely used and well-maintained in the Rust ecosystem.

## Related Beads
- Parent: bf-3k0i4
- Depends on: bf-4votp (verified directory structure first)
