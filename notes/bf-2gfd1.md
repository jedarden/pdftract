# pdftract CLI Text Extraction Flags - Verification Note

## Task
Identify pdftract CLI text extraction flags for OCR mode and text extraction operations.

## Findings

### Primary Text Extraction Flags

1. **`--text <PATH>`**
   - **Purpose**: Output plain text to PATH (use '-' for stdout)
   - **Type**: Output file path flag
   - **Usage**: `pdftract extract input.pdf --text -` (for stdout)
   - **Usage**: `pdftract extract input.pdf --text output.txt` (for file)

2. **`--ocr`**
   - **Purpose**: Enable OCR for scanned pages (requires 'ocr' feature)
   - **Type**: Boolean flag
   - **Requirement**: Requires 'ocr' feature to be enabled during build
   - **Usage**: `pdftract extract input.pdf --ocr --text -`

3. **`--ocr-language <OCR_LANGUAGE>`**
   - **Purpose**: Specify OCR language codes (comma-separated)
   - **Type**: Comma-separated language codes
   - **Default**: 'eng' (English)
   - **Examples**: 'eng,fra,deu' (English, French, German)
   - **Usage**: `pdftract extract input.pdf --ocr --ocr-language eng,fra --text -`

### Related Output Format Flags

- **`--json <PATH>`**: Output JSON to PATH
- **`--md <PATH>`**: Output Markdown to PATH  
- **`--ndjson`**: Output NDJSON to stdout (mutually exclusive with other formats)
- **`--format <FORMATS>`**: Comma-separated output formats (json,markdown,text,ndjson)

## CLI Help Verification

Ran `cargo run --bin pdftract -- extract --help` and confirmed all flags exist in the CLI help output.

## Acceptance Criteria Status

✅ **PASS**: Correct CLI subcommand identified - `extract` subcommand handles text extraction
✅ **PASS**: CLI help confirms flags exist - all flags documented in help output  
✅ **PASS**: Flag syntax is documented - each flag has clear usage syntax in help

## Example Commands

```bash
# Basic text extraction to stdout
pdftract extract document.pdf --text -

# OCR-enabled text extraction
pdftract extract scanned.pdf --ocr --text -

# Multi-language OCR
pdftract extract document.pdf --ocr --ocr-language eng,fra,deu --text output.txt

# Combined format output (text + JSON)
pdftract extract document.pdf --text - --json result.json
```

## Parent Bead Reference

This task (bf-2gfd1) supports parent bead bf-677eo for understanding pdftract CLI capabilities in text extraction mode.
