# Verification Note: pdftract-37qim

## Task: CLI parsing + validation (multi-format flags, --ndjson exclusivity, stdout uniqueness)

## Summary
The CLI parsing + validation for multi-output was already implemented in `crates/pdftract-cli/src/output.rs`. This verification confirms that the implementation meets all acceptance criteria.

## Pre-existing Work
The implementation was already present in the codebase. This task primarily verified that:
1. The `OutputConfig` struct and `build_specs()` method correctly validate output configurations
2. All validation rules from the plan (lines 2261-2265) are enforced
3. The CLI integration in `main.rs` uses the output configuration correctly

## Fixes Made
- Fixed compilation errors in `crates/pdftract-core/src/span/mod.rs` by adding missing `column: None` field to two constructors (`new()` and `empty()`)

## Verification Results

### Acceptance Criteria - ALL PASS

1. **`--json a.json --md b.md -> 2 OutputSpecs built`** - PASS
   - Test: `test_multiple_format_flags`
   - Verified: `cargo nextest run -p pdftract-cli --lib output::tests::test_multiple_format_flags`

2. **`--json a.json --json b.json -> CLI error "duplicate format"`** - PASS
   - CLI test: `./target/debug/pdftract extract --json a.json --json b.json tests/fixtures/empty.pdf`
   - Output: `Error: duplicate format: --json and --json both specify json output`

3. **`--ndjson --md b.md -> CLI error "--ndjson cannot be combined"`** - PASS (critical test line 2302)
   - CLI test: `./target/debug/pdftract extract --ndjson --md b.md tests/fixtures/empty.pdf`
   - Output: `error: the argument '--ndjson' cannot be used with '--md <PATH>'`
   - Note: clap's `conflicts_with_all` catches this at parse time

4. **`--md - --json out.json -> 2 specs, MD=Stdout, JSON=File`** - PASS
   - Test: `test_stdout_with_file`
   - Verified: MD goes to stdout, JSON goes to file

5. **`--md - --json - -> CLI error "at most one stdout"`** - PASS
   - CLI test: `./target/debug/pdftract extract --md - --json - tests/fixtures/empty.pdf`
   - Output: `Error: at most one output may be stdout (-); multiple formats cannot all write to stdout`

6. **`--format json,md -o out -> 2 specs, out.json + out.md`** - PASS
   - Test: `test_format_with_base`
   - CLI test: `./target/debug/pdftract extract --format json,md -o out tests/fixtures/empty.pdf`
   - Output: `Producing 2 outputs: json -> out.json, markdown -> out.md`

### Additional Verification

- **Default behavior (no output flags)** - PASS
  - Per line 2242-2243: Single output to stdout (default)
  - `test_output_config_default` confirms JSON to stdout when no flags specified

- **`--format without -o` error** - PASS
  - CLI test: `./target/debug/pdftract extract --format json tests/fixtures/empty.pdf`
  - Output: `Error: --format requires -o (output base path)`

- **Cross-format duplication detection** - PASS
  - Tests: `test_duplicate_format_json_flag_and_format_list`, `test_duplicate_format_md_flag_and_format_list`, `test_duplicate_format_text_flag_and_format_list`
  - Validates that `--json` and `--format json` cannot both specify JSON output

## Implementation Details

### OutputConfig Structure
Located in `crates/pdftract-cli/src/output.rs`:
- `OutputConfig` struct stores parsed CLI flags
- `build_specs()` method validates and builds `Vec<OutputSpec>`
- Validation rules:
  1. Each format can appear at most once
  2. At most one output can be stdout
  3. `--ndjson` cannot be combined with other formats
  4. `--format` requires `-o`

### CLI Integration
Located in `crates/pdftract-cli/src/main.rs`:
- `cmd_extract()` creates `OutputConfig` from CLI args
- Calls `build_specs()` and reports errors with `exit(2)`
- Iterates over output specs and writes each to its destination
- Uses `AtomicFileWriter` for file outputs (atomic writes)

### Test Coverage
All 23 tests in `output::tests` pass:
- Format parsing (`test_format_from_str`)
- Extension mapping (`test_format_extension`)
- Destination handling (`test_destination_from_path`)
- Single format flags (`test_single_format_flag_json`, `test_single_format_flag_md`, `test_single_format_flag_text`)
- Multiple format flags (`test_multiple_format_flags`)
- Stdout handling (`test_stdout_with_file`, `test_multiple_stdout_rejected`)
- NDJSON exclusivity (`test_ndjson_exclusive_with_json`, `test_ndjson_exclusive_with_md`, `test_ndjson_exclusive_with_text`)
- Format auto-naming (`test_format_with_base`, `test_format_with_all_formats`, `test_output_spec_auto_named`)
- Duplicate detection (`test_duplicate_format_json_flag_and_format_list`, etc.)

## References
- Plan section: Phase 6.6 CLI design + validation rules (lines 2221-2247, 2261-2303)
- Critical test: Line 2302 - `--ndjson --md b.md` → rejected at CLI parse time

## PASS/WARN/FAIL Summary
- **PASS**: All 6 acceptance criteria
- **WARN**: None
- **FAIL**: None

## Files Modified
- `crates/pdftract-core/src/span/mod.rs` - Fixed compilation errors (added `column: None` to constructors)

## Files Verified
- `crates/pdftract-cli/src/output.rs` - Core validation logic
- `crates/pdftract-cli/src/main.rs` - CLI integration
