# Verification Note: bf-5g04q - Examine diagnostic file format and structure

## Task
Understand the format and structure of the diagnostic output files found in the previous step (bf-3e0vl).

## Acceptance Criteria - ALL PASS

### ✅ PASS: File format is identified (text/JSON/structured/etc.)
Identified **four distinct diagnostic file formats** in the pdftract project:

#### 1. Compiler/Build Output Format (Human-Readable)
- **Example file**: `notes/bf-677eo-output.txt` (106KB, 2967 lines)
- **Format**: Plain text with structured compiler diagnostic messages
- **Purpose**: Rust compiler warnings, dead code analysis, and cfg condition checks

#### 1b. Compiler JSON-Lines Format (Machine-Readable)
- **Example file**: `target/debug/.fingerprint/pdftract-cli-8b8c0ec31cd61f5b/output-test-lib-pdftract_cli` (87KB, 50 lines)
- **Format**: JSON-Lines (JSONL) - one complete JSON object per line
- **Purpose**: Machine-readable Rust compiler diagnostics for tooling/analysis
- **Key difference**: Each line is a complete JSON diagnostic object with rich metadata

#### 2. Cargo Build Stderr Format
- **Example file**: `target/debug/build/pdftract-core-ffeb7d8a650f4c25/stderr` (11 lines)
- **Format**: Key-value pairs with `cargo:warning=` prefix
- **Purpose**: Build system warnings about missing optional checksum files

#### 3. JSON Debug Output Format
- **Example files**: `notes/bf-5ucbr-pdftract-debug-output.json` (663 bytes)
- **Format**: Structured JSON with schema version
- **Purpose**: Extraction results with metadata, diagnostics array, and structured data

#### 4. Expected Diagnostics Format (Test Fixtures)
- **Example files**: `tests/error_recovery/fixtures/*.expected_diagnostics.json`
- **Format**: JSON with test expectations
- **Purpose**: Define expected diagnostic codes and counts for conformance testing

### ✅ PASS: Overall structure is understood and documented

#### Compiler/Build Output Structure:
```
warning: <warning message>
  --> <file path>:<line>:<column>
   |
<line number> | <code context>
   |        <caret pointing to issue>
   |
   = note: <explanatory note>
   = help: <suggestion> (optional)
```

**Pattern characteristics**:
- Multi-line diagnostic messages with code context
- ASCII art-style pointers (--> and |)
- Structured metadata (= note, = help)
- File locations with line/column numbers

#### Cargo Build Stderr Structure:
```
cargo:warning=<message text>
```

**Pattern characteristics**:
- Flat list of key-value pairs
- Each line is a complete warning message
- No hierarchical structure
- Cargo build system integration format

#### JSON Debug Output Structure:
```json
{
  "schema_version": "1.0",
  "fingerprint": "pdftract-v1:<hash>",
  "metadata": {
    "page_count": 0,
    "block_count": 0,
    "cache_status": "skipped",
    "diagnostics": []
  },
  "pages": [],
  "attachments": [],
  "form_fields": [],
  "javascript_actions": [],
  "links": [],
  "signatures": [],
  "threads": []
}
```

**Pattern characteristics**:
- Schema versioned (1.0)
- Empty arrays for optional features
- Metadata object with diagnostic array
- Fingerprint for change detection

#### Compiler JSON-Lines Structure (Machine-Readable Format):

Located in `target/debug/.fingerprint/<crate-hash>/output-test-*`, these files contain Rust compiler diagnostics in JSON-Lines format (one JSON object per line).

```json
{
  "$message_type": "diagnostic",
  "message": "unused import: `PathBuf`",
  "code": {
    "code": "unused_imports",
    "explanation": null
  },
  "level": "warning",
  "spans": [
    {
      "file_name": "crates/pdftract-cli/src/cache_cmd.rs",
      "byte_start": 544,
      "byte_end": 551,
      "line_start": 13,
      "line_end": 13,
      "column_start": 23,
      "column_end": 30,
      "is_primary": true,
      "text": [{"text": "use std::path::{Path, PathBuf};", "highlight_start": 23, "highlight_end": 30}],
      "label": null,
      "suggested_replacement": null,
      "suggestion_applicability": null,
      "expansion": null
    }
  ],
  "children": [
    {"message": "`#[warn(unused_imports)]` on by default", "level": "note", ...},
    {"message": "remove the unused import", "level": "help", "suggestion_applicability": "MachineApplicable", ...}
  ],
  "rendered": "ANSI-formatted terminal output with escape codes"
}
```

**Key characteristics**:
- `$message_type` always `"diagnostic"` for Rust compiler output
- Rich span information: file paths, byte ranges, line/column numbers
- `children` array contains sub-diagnostics (notes, help messages)
- `rendered` field contains terminal-ready ANSI-formatted output
- `suggestion_applicability`: `MachineApplicable`, `MaybeIncorrect`, `HasPlaceholders`, or `null`

**Common diagnostic codes in JSON-Lines format**:
- `unused_imports` - Import never used
- `unused_variables` - Variable never read
- `dead_code` - Function/struct/field never used
- `unreachable_code` - Code after bail!/return
- `redundant_semicolons` - Unnecessary semicolon
- `unused_assignments` - Value assigned but never read
- `unused_mut` - Variable declared mutable but never mutated

#### Expected Diagnostics Structure:
```json
{
  "description": "<test scenario description>",
  "expected_diagnostics": [
    {
      "code": "STRUCT_MISSING_KEY",
      "min_count": 10,
      "description": "<explanation>"
    }
  ],
  "expected_pages": "10",
  "expected_default_mediabox": "612x792 letter size for all pages"
}
```

**Pattern characteristics**:
- Test scenario metadata
- Expected diagnostic codes with minimum counts
- Expected output values (page count, defaults)

### ✅ PASS: Encoding and line endings are noted

#### File Encoding:
- **All diagnostic files**: ASCII/UTF-8 compatible
- **First bytes**:
  - Compiler output: `77 61 72 6e 69 6e 67 3a 20` ("warning: ")
  - JSON output: `7b 0a` ("{\n")
  - Build stderr: `63 61 72 67 6f 3a 77 61 72 6e 69 6e 67 3d` ("cargo:warning=")

#### Line Endings:
- **All files use Unix line endings**: `\n` (0x0a)
- **No Windows line endings**: No `\r\n` sequences found
- **Hexdump verification**: All line breaks show as `0a` (LF only)

#### File Sizes:
- **Compiler output**: Large (106KB, 2967 lines) - comprehensive diagnostic data
- **Build stderr**: Small (11 lines) - minimal build warnings
- **JSON debug**: Small (663 bytes) - structured extraction results
- **Expected diagnostics**: Small (200-500 bytes) - test fixtures

### ✅ PASS: Note is created documenting the format findings

This verification note documents all identified diagnostic file formats, their structures, encoding, and line endings.

## File Location Patterns

### Build Directory Diagnostics:
- **Path**: `target/debug/build/<crate-hash>/stderr`
- **One per build crate** - ~150+ stderr files total
- **Only 10-15 are non-empty** (most builds have no warnings)
- **Naming**: Crate-specific directory with hash suffix

### Test Output Diagnostics:
- **Path**: `notes/` directory
- **Naming pattern**: `<bead-id>-<purpose>-output.<ext>`
- **Examples**:
  - `bf-677eo-output.txt` - Full compiler output
  - `bf-5ucbr-pdftract-debug-output.json` - JSON extraction results
  - `bf-694ie-extract.log` - Extraction operation logs
  - `bf-694ie-help.log` - CLI help output

### Test Fixture Diagnostics:
- **Path**: `tests/error_recovery/fixtures/`
- **Naming pattern**: `<test-case>.expected_diagnostics.json`
- **Purpose**: Define expected diagnostic codes for conformance testing

## Metadata and Wrapper Analysis

#### Compiler Output Wrapper:
- **No outer metadata** - raw compiler output
- **Internal structure**: Each warning is self-contained with file location
- **No header/footer** - starts immediately with first warning
- **No summary statistics** - just raw diagnostic stream

#### JSON Output Wrapper:
- **Schema version** in root object (`"schema_version": "1.0"`)
- **Fingerprint** for change detection (`"fingerprint": "pdftract-v1:<hash>"`)
- **Metadata section** with diagnostic messages array
- **Empty arrays** for unextracted features (attachments, form_fields, etc.)

#### Expected Diagnostics Wrapper:
- **Description field** - human-readable test scenario
- **Expected values section** - page counts, defaults
- **Diagnostic codes array** - structured test expectations

## Field Organization Patterns

#### Compiler Output:
- Hierarchical: warning → location → code context → notes
- Visual hierarchy using ASCII art (--> and |)
- Nested structure within each diagnostic message

#### JSON Output:
- Flat key-value structure at root level
- Nested metadata object
- Arrays for multi-value features (pages, attachments, etc.)

#### Expected Diagnostics:
- Flat structure with three main sections
- Array of expected diagnostic objects
- Key-value pairs for expectations

## Encoding Compatibility

All diagnostic files are **ASCII-compatible UTF-8**:
- Compiler output uses ASCII characters (7-bit clean)
- JSON output uses standard ASCII JSON syntax
- No BOM (Byte Order Mark) detected
- Safe to parse with standard text/JSON parsers

## Recommendations

1. **For parsing compiler output**: Use line-by-line parsing looking for "warning:" or "error:" prefixes
2. **For JSON output**: Parse with standard JSON parser, check schema_version first
3. **For build stderr**: Simple key-value parsing after "cargo:warning=" prefix
4. **For test fixtures**: Validate against expected_diagnostics schema

## Files Examined

1. `/home/coding/pdftract/notes/bf-677eo-output.txt` (106KB, 2967 lines)
2. `/home/coding/pdftract/notes/bf-5ucbr-pdftract-debug-output.json` (663 bytes)
3. `/home/coding/pdftract/target/debug/build/pdftract-core-ffeb7d8a650f4c25/stderr` (11 lines)
4. `/home/coding/pdftract/tests/error_recovery/fixtures/missing_mediabox_all_pages.expected_diagnostics.json`
5. `/home/coding/pdftract/notes/bf-694ie-help.log` (1.4KB)
6. `/home/coding/pdftract/notes/bf-3j4ec-json-output.txt` (451 bytes)

## Conclusion

All acceptance criteria PASS. Four distinct diagnostic file formats were identified, their structures analyzed and documented, encoding (ASCII/UTF-8) and line endings (Unix \n) verified, and this comprehensive note created documenting all findings.
