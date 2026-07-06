# Diagnostic Output Files in pdftract Build Directories

## Summary

Located and documented diagnostic output files stored in the project build and test output directories.

## Build Diagnostic Files (target/ directory)

### Location: `/home/coding/pdftract/target/debug/build/*/stderr`

Build stderr files are stored in build-specific subdirectories within `target/debug/build/`. These contain compiler warnings and build diagnostic information.

**Example file:** `/home/coding/pdftract/target/debug/build/pdftract-core-0d428a00850f9797/stderr`

- **File size:** 760 bytes
- **Last modified:** July 6, 2026 17:23
- **Contents:** Cargo compiler warnings about missing optional checksum files and unused code

**Sample content:**
```
cargo:warning=Checksum file not found (optional): std14-metrics.json
cargo:warning=Checksum file not found (optional): ../../../build/glyph-shapes.json
cargo:warning=Checksum file not found (optional): named-encodings.json
cargo:warning=Checksum file not found (optional): predefined-cmaps/adobe-japan1.json
```

**Pattern:** Each build crate has its own subdirectory with a `stderr` file (e.g., `pdftract-core-HASH/stderr`, `pdftract-cli-HASH/stderr`).

### File Count

- **Total stderr files found:** ~150+ (one per build crate dependency)
- **Non-empty stderr files:** ~10-15 (containing actual compiler warnings)
- **Most recent:** July 6, 2026 17:30

## Test Output Files (notes/ directory)

### Location: `/home/coding/pdftract/notes/`

The `notes/` directory contains diagnostic output files from test runs and CLI invocations. These are more comprehensive and user-readable than the build stderr files.

**Most recent diagnostic file:** `/home/coding/pdftract/notes/bf-677eo-output.txt`

- **File size:** 104K (104,000+ bytes)
- **Last modified:** July 6, 2026 16:36
- **Contents:** Full compiler output with warnings, dead code analysis, and cfg condition checks

**Other diagnostic files found:**
- `bf-5ucbr-pdftract-debug-output.json` - JSON debug output from pdftract CLI
- `bf-694ie-extract.log` - Extraction operation log
- `bf-694ie-help.log` - CLI help output log
- `bf-3j4ec-*.txt` - Multiple output format test files (text, ndjson, json)

## Key Findings

1. **Build stderr files** are small (usually 0 bytes, 760 bytes when non-empty) and contain compiler warnings
2. **Test output files** in `notes/` are much larger (up to 104K) and contain comprehensive diagnostic information
3. **File naming pattern:** Build stderr uses crate-HASH directory, test outputs use bead-ID prefix
4. **Most recent diagnostic:** `bf-677eo-output.txt` (104K, July 6, 2026 16:36)

## Access Patterns

### For build diagnostics:
```bash
# Find most recent non-empty stderr files
find /home/coding/pdftract/target/debug/build -name "stderr" -size +0c -exec ls -lth {} \; | head -10
```

### For test outputs:
```bash
# Find most recent diagnostic files
ls -lht /home/coding/pdftract/notes/*.log /home/coding/pdftract/notes/*output* 2>/dev/null | head -10
```

## Recommendations

1. The `notes/` directory appears to be the primary location for comprehensive test diagnostic outputs
2. Build stderr files are useful for tracking compiler warnings across build iterations
3. Test output files in `notes/` are substantially larger and more detailed than build diagnostics
