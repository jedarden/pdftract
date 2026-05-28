# pdftract-37qim Verification Note

## Task
CLI parsing + validation for multi-format output flags

## Implementation Status
**ALREADY COMPLETE** - No code changes required.

## Verification Summary

The CLI parsing and validation for multi-output is fully implemented in:
- `crates/pdftract-cli/src/output.rs` - Core validation logic
- `crates/pdftract-cli/src/main.rs` - CLI integration

### Validation Rules Implemented

1. **At most one stdout** (`-`): `output.rs:236-240`
   - Tracks stdout_count across all format specifications
   - Errors with clear message if > 1 format targets stdout

2. **Duplicate format rejection**: `output.rs:147-199`
   - Tracks each format in `format_sources` HashMap
   - Errors on duplicate `--json`, `--md`, `--text`, or `--ndjson` flags
   - Errors on duplicate formats in `--format` list
   - Errors when a format appears both as flag and in `--format` list

3. **NDJSON exclusivity**: Two-layer protection
   - clap-level: `conflicts_with_all` on `--ndjson` flag (`main.rs:115`)
   - Validation-level: Check in `build_specs()` (`output.rs:243-247`)

4. **Auto-naming with `--format` + `-o`**: `output.rs:212-233`
   - Derives filenames from base + format extension
   - Extensions: `.json`, `.md`, `.txt`, `.ndjson`

### Acceptance Criteria Verified

| Test | Status | Location |
|------|--------|----------|
| `--json a.json --md b.md` → 2 specs | ✓ | `test_multiple_format_flags` |
| `--json a.json --json b.json` → error | ✓ | Manual verification |
| `--ndjson --md b.md` → error | ✓ | Manual verification |
| `--md - --json out.json` → 2 specs | ✓ | `test_stdout_with_file` |
| `--md - --json -` → error | ✓ | Manual verification |
| `--format json,md -o out` → 2 specs | ✓ | `test_format_with_base` |

### Test Results
```bash
$ cargo test -p pdftract-cli --lib output::tests
test result: ok. 23 passed; 0 failed
```

### Manual CLI Verification
```bash
# Duplicate format rejection
$ ./target/release/pdftract extract --json a.json --json b.json blank.pdf
Error: duplicate format: --json and --json both specify json output
Exit code: 2

# NDJSON exclusivity
$ ./target/release/pdftract extract --ndjson --md b.md blank.pdf
error: the argument '--ndjson' cannot be used with '--md <PATH>'
Exit code: 2

# Multiple stdout rejection
$ ./target/release/pdftract extract --md - --json - blank.pdf
Error: at most one output may be stdout (-); multiple formats cannot all write to stdout
Exit code: 2

# Auto-naming
$ ./target/release/pdftract extract --format json,md -o out blank.pdf
Producing 2 outputs:
  json -> out.json
  markdown -> out.md
```

## Key Implementation Details

### OutputSpec Structure
```rust
pub struct OutputSpec {
    pub format: Format,
    pub dest: Destination,  // File(PathBuf) | Stdout
}
```

### Validation Flow
1. Parse CLI flags with clap
2. Build `OutputConfig` from parsed values
3. Call `build_specs()` which validates and returns `Vec<OutputSpec>`
4. Exit with code 2 on validation error

### Error Messages
All error messages are clear and point to the offending flag:
- "duplicate format: --json and --json both specify json output"
- "--ndjson cannot be combined with other output formats"
- "at most one output may be stdout (-)"
- "--format requires -o (output base path)"

## Conclusion
The implementation fully satisfies all acceptance criteria for bead pdftract-37qim. No code changes are required.
