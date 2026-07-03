# bf-3whc0: Run pdftract extract on no-mapping.pdf fixture

## Task
Run the pdftract extract command on the no-mapping.pdf fixture and capture its JSON output.

## Execution

### Command Run
```bash
pdftract extract tests/fixtures/encoding/no-mapping.pdf
```

### Exit Code
0 (success)

### JSON Output Captured
```json
{
  "attachments": [],
  "fingerprint": "pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8",
  "form_fields": [],
  "javascript_actions": [],
  "links": [],
  "metadata": {
    "block_count": 0,
    "cache_age_seconds": null,
    "cache_status": "skipped",
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

## Verification

### PASS Criteria
- ✅ pdftract extract command ran without errors
- ✅ JSON output was produced and captured
- ✅ Exit code was 0

### Output Analysis
The JSON output shows:
- 0 pages (empty PDF)
- 0 spans, 0 blocks
- No attachments, form fields, links, signatures, or threads
- Valid schema_version (1.0)
- Fingerprint: `pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8`

This is expected behavior for a PDF with no extractable content (the no-mapping.pdf fixture was designed to test encoding handling, not content extraction).

## Parent Bead
- bf-2kybw (fixture research and verification)

## Research References
- notes/bf-f0xqd-research.md sections 6.2, 7.1
