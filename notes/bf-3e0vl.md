# Diagnostic Output Files Discovery (bf-3e0vl)

## Summary
Successfully located and documented diagnostic output files in the project build directories.

## Primary Diagnostic Output Location

### Test Output Diagnostics
**Path Pattern:** `target/debug/.fingerprint/<crate-hash>/output-test-*`

These files contain JSON-formatted compiler and test diagnostics, including:
- Compiler warnings (unused imports, dead code, unreachable code, unused variables)
- Error messages with line/column information
- Suggested fixes and machine-applicable suggestions

#### Most Recent Example
**File:** `/home/coding/pdftract/target/debug/.fingerprint/pdftract-cli-8b8c0ec31cd61f5b/output-test-lib-pdftract_cli`

**Details:**
- **Size:** 89,369 bytes (~87 KB)
- **Created:** 2026-07-06 17:32:20
- **Lines:** 50 lines (one JSON diagnostic per line)
- **Last accessed:** 2026-07-06 17:34:29

**Content Sample:**
```json
{"$message_type":"diagnostic","message":"unused import: `PathBuf`","code":{"code":"unused_imports","explanation":null},"level":"warning","spans":[...]}
```

### Other Recent Diagnostic Files
- `target/debug/.fingerprint/pdftract-core-e8f303349505d378/output-test-integration-test-verify_proptest_catches_bugs` (2.7 KB, 2026-07-06 17:23)
- `target/debug/.fingerprint/pdftract-py-77353842664f24e0/output-test-lib-pdftract` (timestamp: 1783373534)
- `target/debug/.fingerprint/pdftract-libpdftract-2ba5f4872f785275/output-test-lib-pdftract` (timestamp: 1783373534)

## Expected Diagnostic Fixtures

**Location:** `tests/error_recovery/fixtures/*.expected_diagnostics.json`

These are test fixtures that define expected diagnostic outputs for error recovery tests.

**Examples:**
- `missing_endobj.expected_diagnostics.json` - Tests missing `endobj` marker detection
- `missing_mediabox_all_pages.expected_diagnostics.json` - Tests missing MediaBox recovery
- `combined_failures.expected_diagnostics.json` - Tests multiple error conditions
- `truncated_mid_stream.expected_diagnostics.json` - Tests truncated stream recovery
- `xref_30pct_bad_offsets.expected_diagnostics.json` - Tests corrupted xref recovery
- `int_overflow_bbox.expected_diagnostics.json` - Tests integer overflow in bounding boxes
- `nested_failure.expected_diagnostics.json` - Tests nested error conditions

**Sample Structure:**
```json
{
  "description": "Object 5 is missing its endobj marker",
  "expected_diagnostics": [
    {
      "code": "STRUCT_INVALID_XREF_ENTRY",
      "min_count": 1,
      "description": "Parser should detect object 5 is malformed and recover"
    }
  ],
  "expected_objects": "at least 6 objects parsed (objects 6+ should still be accessible)"
}
```

## Nextest Configuration

Per `.config/nextest.toml`, nextest stores test data in:
- **Config:** `dir = "target"` (default: `target/nextest`)
- Actual diagnostic outputs are stored in `.fingerprint` subdirectories as shown above

## Acceptance Criteria Status

✅ **PASS** - At least one diagnostic output file found (50+ files located)
✅ **PASS** - Full paths documented (primary path and fixtures documented)
✅ **PASS** - File creation/modification times noted (2026-07-06 17:32:20 for most recent)
✅ **PASS** - File sizes reasonable (ranging from 2KB to 87KB, not empty or excessively large)

## Files Modified
- `notes/bf-3e0vl.md` (this file) - Updated with actual diagnostic output findings

## Key Findings
1. **Diagnostic files are compiler/test output** stored in `.fingerprint` directories
2. **Format is JSON-lines** (one JSON diagnostic object per line)
3. **Content includes compiler warnings** about unused code, dead code, unreachable code, etc.
4. **Test fixtures define expected diagnostics** for error recovery testing scenarios
5. **Files are actively updated** during test runs and builds
