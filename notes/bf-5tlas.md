# bf-5tlas: Test basic pdftract CLI execution on JavaScript PDF fixture

## Summary
Successfully tested basic pdftract CLI execution on the JavaScript PDF fixture (`tests/fixtures/security/embedded-js.pdf`).

## Findings

### JavaScript PDF Fixture (embedded-js.pdf)
- **Path**: `tests/fixtures/security/embedded-js.pdf`
- **Status**: Valid fixture with embedded JavaScript
- **JavaScript detected**: 1 action (`app.alert("pwn")`) at catalog.openaction
- **Extraction**: SUCCESSFUL - CLI detects JavaScript safely without executing it

### Command Used
```bash
pdftract extract tests/fixtures/security/embedded-js.pdf
```

### Exit Code
0 (successful)

### Execution Time
- Real: 0.003s
- User: 0.001s
- Sys: 0.002s
**Well under the 30-second requirement.**

### Output Summary
The CLI executed successfully and produced JSON output with the following key elements:
- **Fingerprint**: `pdftract-v1:cd0fe35a93a7949a27a24ce0af7d13292ea0a40ba65b01a805b93ca583b71ce8`
- **JavaScript Actions Detected**: 1
  - Code excerpt: `app.alert("pwn")`
  - Location: `catalog.openaction`
- **Diagnostics**: `"Detected 1 JavaScript action(s) in PDF document. JavaScript was NOT executed."`
- **Schema Version**: 1.0
- **Pages**: `[]` (empty - this is a minimal PDF with JavaScript but no pages)

### Sample Output
```json
{
  "attachments": [],
  "fingerprint": "pdftract-v1:cd0fe35a93a7949a27a24ce0af7d13292ea0a40ba65b01a805b93ca583b71ce8",
  "form_fields": [],
  "javascript_actions": [
    {
      "code_excerpt": "app.alert(\"pwn\")",
      "location": "catalog.openaction"
    }
  ],
  "links": [],
  "metadata": {
    "block_count": 0,
    "cache_age_seconds": null,
    "cache_status": "skipped",
    "diagnostics": [
      "Detected 1 JavaScript action(s) in PDF document. JavaScript was NOT executed."
    ],
    "page_count": 0,
    "reading_order_algorithm": "xy_cut",
    "span_count": 0
  },
  "pages": [],
  "schema_version": "1.0",
  "signatures": [],
  "threads": []
}
```
```

## Acceptance Criteria Status

- ✅ **pdftract CLI executes without errors**: YES (no errors encountered)
- ✅ **Basic output is produced to stdout**: YES (valid JSON output)
- ✅ **CLI completes successfully (exit code 0)**: YES (exit code 0)
- ✅ **Execution time is reasonable (< 30 seconds)**: YES (0.003s - excellent)
- ✅ **Command used is documented**: YES (documented above)

## Notes
- The JavaScript PDF fixture (`tests/fixtures/security/embedded-js.pdf`) contains a simple JavaScript action that triggers `app.alert("pwn")`
- pdftract correctly detects the JavaScript and reports it in the `javascript_actions` field
- The CLI output confirms that JavaScript was **NOT executed** (safe behavior)
- No warnings or errors in basic output
- The fixture appears to be a minimal PDF with JavaScript but no actual pages/content
- The binary is located at `/home/coding/.local/bin/pdftract`
- System: Linux 6.12.63
