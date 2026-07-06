# pdftract CLI Exit Code and Output Format Verification

**Bead ID:** bf-698np  
**Date:** 2026-07-05  
**Command tested:** `target/release/pdftract` (17.8 MB binary)

## Exit Codes Discovered

### Exit Code 0 (Success)
- `pdftract --help` - Help display
- `pdftract list-diagnostics` - Diagnostic listing (100 codes)
- `pdftract doctor` - Environment health check

**ANOMALY:** Some error conditions return exit code 0:
- `pdftract extract --output /tmp/test_output <file>` returns 0 even when extraction fails

### Exit Code 1 (Runtime Errors)
- `pdftract validate` - JSON validation failures (missing required properties)
- `pdftract classify` - Missing feature errors (requires `--features profiles`)

### Exit Code 2 (Argument/File Errors)
- `pdftract --version` - Invalid argument
- `pdftract -V` - Invalid argument  
- `pdftract` (no args) - Missing command
- `pdftract invalid-command` - Unrecognized subcommand
- `pdftract hash <file>` - File operation failures

## Output Format Analysis

### Text-Based Output
- **Help text:** Structured command listing with descriptions
- **Diagnostics:** Hierarchical categorization (STRUCT_*, XREF_*, STREAM_*, etc.)
- **Doctor:** Table format with status columns

### Validation Output
- **Error format:** JSON path + error message
  ```
  / "schema_version" is a required property
  / "metadata" is a required property
  ```
- **Quiet mode:** `--quiet` suppresses output, exit code only

### Key Observations
1. **No version flag available** - Neither `--version` nor `-V` work
2. **Fast execution** - Most commands complete in 0.002-0.004 seconds
3. **Exit code inconsistency** - Some errors return 0 (should return non-zero)
4. **Feature gating** - Some commands require specific build features

## Execution Timing
- Help commands: ~0.002 seconds
- Diagnostic listing: ~0.002 seconds  
- Doctor command: ~0.002 seconds
- Validation: ~0.004-0.013 seconds
- All commands are very responsive (< 15ms)

## Recommendations
1. Add proper `--version` flag support
2. Fix exit code for extract failures (currently returns 0 on error)
3. Consider consistent exit code 1 for runtime errors vs 2 for argument errors
