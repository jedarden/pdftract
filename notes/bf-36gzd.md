# PDF Test Fixtures with JavaScript Actions

## Finding Summary

The repository already contains PDF test fixtures with JavaScript actions. No new fixture creation was needed.

## Existing Fixtures

### 1. `/home/coding/pdftract/tests/fixtures/security/embedded-js.pdf`

**Status**: ✅ Working - Successfully parsed by pdftract

**Location**: `tests/fixtures/security/embedded-js.pdf`

**Generator Script**: `tests/fixtures/security/generate_embedded_js.py`

**Documented JavaScript Actions** (from generator script):
1. **Catalog /OpenAction**: `app.alert("pwn")`
2. **Page 0 /AA /O** (open action): `app.alert('page_open')`
3. **Page 1 annotation /A**: `app.alert('annot_action')`

**Actual Extraction Results** (via `pdftract extract`):
```json
{
  "javascript_actions": [
    {
      "code_excerpt": "app.alert(\"pwn\")",
      "location": "catalog.openaction"
    }
  ],
  "metadata": {
    "diagnostics": [
      "Detected 1 JavaScript action(s) in PDF document. JavaScript was NOT executed."
    ]
  }
}
```

**Note**: The PDF structure shows 3 JavaScript actions in the source code, but extraction only reports 1. This may indicate that the additional JavaScript actions (Page 0 /AA /O and Page 1 annotation) are not being extracted, or the PDF structure differs from documentation.

### 2. `/home/coding/pdftract/tests/document_model/fixtures/js_in_openaction.pdf`

**Status**: ⚠️ Parsing Error

**Location**: `tests/document_model/fixtures/js_in_openaction.pdf`

**Issue**: Expected JSON shows parsing failure:
```json
{
  "error": "Failed to parse PDF: No /Root reference in trailer",
  "contains_javascript": false
}
```

**Hexdump Analysis**: The PDF appears structurally valid with:
- `%PDF-1.4` header
- JavaScript action: `app.alert('Hello')` at object 3
- Catalog `/OpenAction` pointing to JavaScript object 3
- Trailer with `/Root 6 0 R`

This fixture may need investigation for why it's not parsing correctly despite appearing structurally valid.

## JavaScript Detection Implementation

Confirmed that pdftract-core has JavaScript detection implemented in `crates/pdftract-core/src/detection.rs`:
- Function: `detect_javascript()`
- Checks catalog `/OpenAction` and `/AA` (Additional Actions)
- JavaScript is NEVER executed; only presence is flagged
- Threat model compliance: TH-04 (JavaScript detection without execution)

## Recommendations

1. Use `tests/fixtures/security/embedded-js.pdf` for JavaScript detection testing
2. Investigate why `js_in_openaction.pdf` fails to parse despite appearing structurally valid
3. Verify that all 3 documented JavaScript actions in `embedded-js.pdf` are properly extracted (current extraction shows only 1)

## Commit Information

This finding is documented as bead `bf-36gzd` - PDF test fixture with JavaScript actions already exists in the repository.
