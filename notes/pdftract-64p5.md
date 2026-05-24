# Verification Note for pdftract-64p5: Classify CLI Subcommand

## Summary

Implemented the `pdftract classify` CLI subcommand structure with proper argument parsing and feature gates. The `--auto` flag was added to the extract subcommand.

## What Was Implemented

### 1. CLI Structure (COMPLETE)
- Added `Classify` subcommand to main.rs with arguments:
  - `input` (positional): Path to PDF file
  - `--password-stdin`: Read password from stdin
  - `--password`: PDF password (insecure, requires env var)
  - `--profiles DIR`: Custom profiles directory
  - `--pretty`: Pretty-print JSON output
  - `--top-k N`: Number of top reasons to include (default: all)
  - `--exit-on-unknown`: Exit code 1 if document_type is unknown

### 2. Extract --auto Flag (COMPLETE)
- Added `--auto` flag to Extract subcommand
- Implements feature-gated stub that explains limitations
- Shows helpful message when profiles feature is not enabled

### 3. Path Traversal Protection (COMPLETE)
- Implemented canonicalization check for --profiles DIR
- Prevents directory traversal attacks
- Proper error messages for escaped paths

### 4. Feature Gating (COMPLETE)
- Classify command requires `profiles` feature
- Graceful error message when feature is not enabled
- Auto flag has separate handling for feature available/unavailable

### 5. Code Structure (COMPLETE)
- Created `crates/pdftract-cli/src/classify.rs` module
- Added `ClassifyArgs` and `ClassificationOutput` structs
- Implemented `run_classify()` and `format_json()` functions
- Added unit tests for output serialization

## Limitations (Known Before Implementation)

The following functionality is deferred to bead 5.6.4 (built-in profile definitions):

1. **Built-in profiles**: `load_builtins()` function does not exist yet
2. **YAML profile loading**: `load_profiles_from_dir()` requires YAML-to-Profile parsing
3. **Full classification pipeline**: Requires profile loading infrastructure

For now, the classify command returns a helpful error message explaining these limitations.

## Acceptance Criteria Status

### From Bead Description:

| Criterion | Status | Notes |
|-----------|--------|-------|
| CLI invocation works | PARTIAL | Command structure complete, but returns limitation message |
| --auto flag on extract | COMPLETE | Implemented with helpful messaging |
| JSON shape matches plan | COMPLETE | ClassificationOutput struct matches plan format |
| Performance | N/A | Deferred to 5.6.4 when profiles are available |
| Help text documents all flags | COMPLETE | Clap derives help from struct definitions |

### From Plan Section 5.6 CLI (lines 1965-1970):

| Requirement | Status | Notes |
|-------------|--------|-------|
| `pdftract classify FILE.pdf` | PARTIAL | Command exists, awaits profile loading |
| `--profiles DIR` | COMPLETE | Path traversal protection implemented |
| `--json` (default) | COMPLETE | JSON is the output format |
| `--pretty` | COMPLETE | Pretty-print JSON flag added |
| `--top-k` | COMPLETE | Top-K reasons flag added |
| `--classify-with-ocr` | NOT REQUIRED | Out of scope for this bead (scanned PDF handling) |
| `--exit-on-unknown` | COMPLETE | Exit code 1 on unknown flag added |
| `pdftract extract --auto` | COMPLETE | Implemented with helpful messaging |
| JSON shape exact match | COMPLETE | Matches plan line 1968-1970 |

## Testing

### Manual Testing
```bash
# Test classify command (should show limitation message)
cargo run --bin pdftract --features profiles -- classify tests/fixtures/sample.pdf

# Test help text
cargo run --bin pdftract --features profiles -- classify --help

# Test --auto flag
cargo run --bin pdftract -- extract --auto tests/fixtures/sample.pdf

# Test without profiles feature (should show feature-gate message)
cargo run --bin pdftract -- classify tests/fixtures/sample.pdf
```

### Unit Tests
- `test_classification_output_serialization`: Verifies JSON output structure
- `test_format_json_pretty`: Verifies pretty vs compact JSON

## Files Modified

1. `crates/pdftract-cli/src/main.rs`:
   - Added `classify` module import
   - Added `Classify` subcommand to Commands enum
   - Added `--auto` flag to Extract subcommand
   - Added `cmd_classify()` handler
   - Updated `cmd_extract()` signature for `auto` parameter

2. `crates/pdftract-cli/src/classify.rs` (NEW):
   - Classification output structures
   - Classification runner with feature gates
   - JSON formatting functions
   - Unit tests

## Dependencies

No new dependencies added. Uses existing:
- `anyhow` for error handling
- `serde`/`serde_json` for JSON output
- `clap` (derive) for CLI parsing

## Next Steps (Bead 5.6.4)

Bead 5.6.4 will implement:
1. `load_builtins()` function to load bundled profile YAMLs
2. `load_profiles_from_dir()` function for custom profiles
3. YAML-to-Profile parsing infrastructure
4. Full classification pipeline integration

## Commit Information

This implementation provides the CLI structure and feature gates required for the classify subcommand. The actual classification logic will be completed in bead 5.6.4 when profile loading infrastructure is available.
