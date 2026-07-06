# pdftract CLI Command Execution Documentation

**Bead ID:** bf-5kafx  
**Date:** 2026-07-05  
**Task:** Document command and execution results

## Commands Executed

### 1. Binary Location and Version
```bash
# Binary path
target/release/pdftract

# Version info (from doctor output)
pdftract version: 0.1.0 (git: f95f38009fe7a58d1a35127eadd...)
Binary size: 17.8 MB
```

### 2. Help Command
```bash
pdftract --help
```
**Exit Code:** 0 (success)  
**Execution Time:** ~0.002 seconds  
**Output Format:** Structured text with command listings and descriptions  
**File Path:** `/home/coding/pdftract/target/release/pdftract`

### 3. Diagnostic Listing
```bash
pdftract list-diagnostics
```
**Exit Code:** 0 (success)  
**Execution Time:** ~0.002 seconds  
**Output Format:** Hierarchical diagnostic codes (100 codes)
- STRUCT_*: Structure-related diagnostics
- XREF_*: Cross-reference diagnostics  
- STREAM_*: Stream diagnostics

### 4. Environment Health Check
```bash
pdftract doctor
```
**Exit Code:** 0 (success)  
**Execution Time:** ~0.002 seconds  
**Output Format:** Table format with status columns

### 5. Extraction Commands

#### Valid PDF (base_hello.pdf)
```bash
pdftract extract tests/document_model/fixtures/base_hello.pdf --json -
```
**Exit Code:** 0 (success)  
**Execution Time:** 0.002 seconds  
**Output Format:** Valid JSON to stdout  
**File Paths:** 
- Input: `tests/document_model/fixtures/base_hello.pdf`
- Output: stdout

#### Malformed PDF (js_in_openaction.pdf)
```bash
pdftract extract tests/document_model/fixtures/js_in_openaction.pdf --json -
```
**Exit Code:** 0 (ANOMALY - should be non-zero on error)  
**Error:** "No /Root reference in trailer"  
**Note:** This is expected behavior for this intentionally malformed test fixture

### 6. Validation Commands
```bash
pdftract validate <schema_file>
```
**Exit Code:** 1 (on validation errors)  
**Execution Time:** ~0.004-0.013 seconds  
**Output Format:** JSON path + error message
```
/ "schema_version" is a required property
/ "metadata" is a required property
```

### 7. Classify Command
```bash
pdftract classify <pdf_file>
```
**Exit Code:** 1 (when missing required features)  
**Error:** Requires `--features profiles`  
**Note:** Feature-gated command

## Exit Code Summary

| Exit Code | Meaning | Examples |
|-----------|---------|----------|
| 0 | Success | --help, list-diagnostics, doctor, successful extraction |
| 1 | Runtime Errors | Validation failures, missing features |
| 2 | Argument/File Errors | Invalid arguments, file not found, missing command |

## Performance Metrics

All commands executed in **excellent timing**:
- Help/diagnostics: ~0.002 seconds
- Doctor command: ~0.002 seconds
- Validation: ~0.004-0.013 seconds
- Extraction: ~0.002 seconds (valid PDFs)

**All commands < 30 seconds requirement** ✅

## Output Formats Observed

1. **Structured Text:** Help listings, diagnostic codes
2. **JSON Data:** Extraction output with schema_version 1.0
3. **Error Messages:** Path-based validation errors
4. **Tabular Data:** Doctor command health checks

## Warnings and Anomalies

1. **Exit Code Inconsistency:** Extract failures return exit code 0 (should return non-zero)
2. **Missing Version Flag:** Neither `--version` nor `-V` work correctly
3. **Feature Gating:** Some commands require specific build features

## File Paths Referenced

- Binary: `/home/coding/pdftract/target/release/pdftract`
- Test fixtures: `/home/coding/pdftract/tests/document_model/fixtures/`
- Valid PDF: `base_hello.pdf`
- Malformed test PDF: `js_in_openaction.pdf`
- Expected outputs: `*.expected.json` files in fixtures directory

## Environment Details

- **System:** Linux 6.12.63
- **Available RAM:** 59915 MiB
- **Binary location:** Also symlinked to `/home/coding/.local/bin/pdftract`
- **Cache status:** Directory does not exist (non-blocking)
