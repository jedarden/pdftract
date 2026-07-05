# JavaScript Detection Points in pdftract

## Summary

JavaScript detection in pdftract is implemented across multiple files. The system **detects but NEVER executes** embedded JavaScript per TH-04.

## Primary Detection Files

### 1. `crates/pdftract-core/src/detection.rs` (lines 15-93)

**Main entry point:** `detect_javascript()` function

**Detection mechanism:**
- Walks document tree checking for JavaScript actions in:
  - Catalog `/OpenAction` (line 48)
  - Catalog `/AA` (Additional Actions) (line 53)
  - Page-level `/AA` dictionaries (line 60)
  - AcroForm field `/AA` dictionaries (line 86)
  - Annotation `/A` and `/AA` entries (lines 65-82)

**Helper functions:**
- `has_js_action()` (lines 95-130): Checks if object is a JavaScript action
  - Detects `/S == /JavaScript` or `/S == /JS` (line 118)
  - Detects `/JS` entry containing JavaScript code (line 124)
- `has_js_in_aa()` (lines 132-166): Checks `/AA` dictionaries for JavaScript
- `has_js_in_acroform()` (lines 168-224): Recursively checks AcroForm fields

**Return value:** `bool` - true if any JavaScript action is found

### 2. `crates/pdftract-core/src/javascript.rs` (lines 25-95)

**Enhanced detection:** `detect_javascript()` function that returns detailed action information

**Detection mechanism:**
- Similar tree walk as `detection.rs` but returns structured data
- Returns `Vec<JavascriptAction>` with:
  - `location`: String describing where JS was found (e.g., "catalog.openaction", "page.0.aa.O")
  - `code_excerpt`: First 200 characters of JavaScript code

**Helper functions:**
- `check_object_for_js()` (lines 97-131): Checks individual objects for `/JS` entries
- `check_aa_for_js()` (lines 133-162): Checks `/AA` dictionaries
- `check_annotations_for_js()` (lines 164-199): Checks annotation arrays
- `extract_js_code()` (lines 201-241): Extracts JavaScript code from `/JS` entries
  - Handles both string and stream-based JavaScript
  - Truncates to 200 characters for security reporting

**Return value:** `(Vec<JavascriptAction>, Vec<Diagnostic>)` - detected actions + diagnostics

### 3. `crates/pdftract-core/src/extract.rs` (lines 971-986)

**Integration point:** JavaScript detection is called during PDF extraction

**Code location:**
```rust
// TH-04: Detect JavaScript actions in the document
use crate::javascript::detect_javascript;

let (js_actions, js_diagnostics) =
    detect_javascript(&catalog, &pages_for_js_detection, &resolver_arc);
```

**Result stored in:** `PdfResult.javascript_actions: Vec<JavascriptActionJson>` (line 274)

### 4. `crates/pdftract-core/src/diagnostics.rs` (lines 1068-1072, 2362)

**Diagnostic reporting:** Emits warning when JavaScript is detected

**Diagnostic code:** `DiagCode::SecurityJavascriptPresent`

**Message format:**
```
"Detected {} JavaScript action(s) in PDF document. JavaScript was NOT executed."
```

**Suggested action:**
"The PDF contains embedded JavaScript. Review the document metadata.javascript_actions array for details. pdftract never executes embedded JS."

## Detection Mechanism Details

### How JavaScript is Detected

1. **Action type detection:** Checks for `/S (subtype) == /JavaScript` or `/S == /JS`
2. **Code presence detection:** Checks for `/JS` entry containing JavaScript code
3. **Location tracking:** Records where JavaScript was found (catalog, pages, annotations, forms)

### No Execution Path

**Explicitly stated in code:**
- `detection.rs` line 24: "JavaScript is NEVER EXECUTED; only its presence is flagged."
- `javascript.rs` line 4: "Per TH-04, pdftract NEVER executes embedded JavaScript"
- `diagnostics.rs` line 1071: "The JavaScript is NEVER executed by pdftract"

### JavaScript Entry Points Checked

1. **Catalog level:**
   - `/OpenAction`: Actions to execute when document opens
   - `/AA`: Additional actions (O=open, C=close, etc.)

2. **Page level:**
   - `/AA`: Page-specific additional actions

3. **Annotation level:**
   - `/A`: Primary action for annotation
   - `/AA`: Additional actions for annotation

4. **Form field level:**
   - `/AA`: Field-specific additional actions
   - Recursively checks `/Kids` for nested fields

## Key Code Locations

| File | Lines | Purpose |
|------|-------|---------|
| `detection.rs` | 41-93 | Boolean JavaScript detection |
| `detection.rs` | 95-130 | `has_js_action()` - checks `/S == /JavaScript` and `/JS` |
| `detection.rs` | 132-166 | `has_js_in_aa()` - checks `/AA` dictionaries |
| `detection.rs` | 168-224 | `has_js_in_acroform()` - checks form fields |
| `javascript.rs` | 25-95 | Structured JavaScript detection with code extraction |
| `javascript.rs` | 201-241 | `extract_js_code()` - extracts JS code, truncates to 200 chars |
| `extract.rs` | 971-986 | Integration point calling JavaScript detection |
| `diagnostics.rs` | 1068-1072 | Diagnostic code definition |
| `diagnostics.rs` | 2362 | Suggested action text |

## Conclusion

The pdftract codebase has a robust two-layer JavaScript detection system:
1. **Simple boolean detection** in `detection.rs` for quick checks
2. **Detailed extraction** in `javascript.rs` for security reporting

**No execution path exists** - the code explicitly prevents JavaScript execution and only reports its presence for downstream security review.
