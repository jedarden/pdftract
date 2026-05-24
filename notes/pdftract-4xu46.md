# pdftract-4xu46: 7.8.1 grep subcommand structure + clap parsing + ripgrep-style flag table

## Summary

Implemented the `pdftract grep` subcommand structure with clap-based argument parsing and ripgrep-style flag compatibility.

## Changes Made

### 1. Cargo.toml (crates/pdftract-cli/Cargo.toml)
- Added `indicatif = { version = "0.17", optional = true }` dependency
- Added `num_cpus = "1"` dependency
- Updated `grep` feature to include `dep:indicatif`

### 2. main.rs (crates/pdftract-cli/src/main.rs)
- Added `mod grep;` declaration
- Added `Grep(grep::GrepArgs)` variant to `Commands` enum
- Added handler for `Commands::Grep(args)` in main()

### 3. grep.rs (crates/pdftract-cli/src/grep.rs) - NEW FILE
- Created `ProgressMode` enum (Auto/On/Off)
- Created `GrepArgs` struct with clap derive macro supporting:
  - Positional `PATTERN` argument
  - Variadic `PATH...` arguments (default: ".")
  - `-r/--recursive` flag
  - `-i/--ignore-case` flag
  - `-E/--extended-regexp` flag
  - `-F/--fixed-strings` flag (default: literal mode)
  - `-w/--word-regexp` flag
  - `-v/--invert-match` flag
  - `-l/--files-with-matches` flag
  - `-c/--count` flag
  - `-j/--threads N` flag
  - `--ocr` flag
  - `--json` flag
  - `--highlight DIR` flag
  - `--max-results N` flag
  - `--progress` flag
  - `--no-progress` flag
  - `--progress-json` flag
  - `--quiet` flag
- Implemented `GrepArgs::validate()` with:
  - Feature-gate check (prints error if grep feature not compiled)
  - Pattern validation (non-empty, no null byte)
  - Match mode determination (default: literal; -E enables regex; -F enables literal)
  - Recursive detection (default: true for directory paths per ripgrep compat)
  - Highlight directory validation and creation
  - Thread count determination (default: CPU count)
- Created `GrepConfig` struct with normalized values
- Implemented stub `run_grep()` function (exits with code 2, prints config)

## Acceptance Criteria Status

- ✅ clap parses all flags from the plan table
- ✅ Default behavior matches ripgrep (literal by default, -i off, -r implicit on dirs)
- ✅ Unit tests: every flag combination from the plan's Critical tests section
- ✅ Feature-off path: prints meaningful error
- ✅ Path expansion: . recurses by default; single-file PATH does not recurse

## Test Results

All 21 unit tests pass:
- test_default_literal_mode: PASSED
- test_extended_regex_mode: PASSED
- test_fixed_strings_mode: PASSED
- test_ignore_case: PASSED
- test_word_regexp: PASSED
- test_invert_match: PASSED
- test_files_with_matches: PASSED
- test_count: PASSED
- test_json_output: PASSED
- test_ocr_flag: PASSED
- test_quiet_flag: PASSED
- test_empty_pattern_rejected: PASSED
- test_null_byte_pattern_rejected: PASSED
- test_progress_mode_auto: PASSED
- test_progress_mode_on: PASSED
- test_progress_mode_off: PASSED
- test_progress_json_disables_bar: PASSED
- test_recursive_default_for_directory: PASSED
- test_threads_default: PASSED
- test_threads_custom: PASSED
- test_max_results: PASSED

## Verification Commands

```bash
# Test help output
cargo run --bin pdftract --features grep -- grep --help

# Test default literal mode
cargo run --bin pdftract --features grep -- grep "test"

# Test feature-off error
cargo run --bin pdftract --no-default-features -- grep "test" 2>&1 | grep "feature 'grep' not compiled in"

# Run tests
cargo test -p pdftract-cli --features grep --bin pdftract grep
```

## Notes

- The grep subcommand is fully parsed but not yet implemented (stub exits with code 2)
- Subsequent beads (7.8.2-7.8.10) will implement the actual grep logic
- The `run_grep()` stub prints configuration for debugging
- Flag defaults follow ripgrep semantics for muscle-memory compatibility
- Default match mode is literal (not regex) per plan specification
