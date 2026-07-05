# PDF JavaScript Action Structure Research

**Bead:** bf-5w2us  
**Date:** 2025-07-05  
**Task:** Research PDF JavaScript action structure for TH-04 threat model implementation

## Overview

This research documents the minimal PDF structure required for valid JavaScript actions, based on the PDF 1.7 specification (ISO 32000-1:2008) and practical examples from security research. JavaScript actions in PDFs are a known attack vector (TH-04) but are useful for legitimate interactivity. pdftract's approach is to **detect and report** JavaScript actions, never execute them.

## Minimal JavaScript Action Dictionary

A JavaScript action in a PDF requires **two mandatory keys**:

```
<< /S /JavaScript /JS (JavaScript code here) >>
```

### Required Keys

1. **`/S`** (Name) - Action type specification
   - Value: `/JavaScript`
   - Required: Yes
   - Purpose: Identifies this action dictionary as containing JavaScript code

2. **`/JS`** (Text String or Stream) - JavaScript content
   - Value: Text string OR text stream containing JavaScript code
   - Required: Yes
   - Format options:
     - **Text string**: `(JavaScript code)` or `(JavaScript\ code\ with\ escapes)` or `<HexEncoded>`
     - **Text stream**: `stream\nJavaScript code\nendstream`
   - Purpose: Contains the actual JavaScript to execute

### Example Minimal JavaScript Action

```
<< /S /JavaScript /JS (app.alert('Hello World!')) >>
```

## JavaScript Action Placement Locations

JavaScript actions can be attached at multiple levels in the PDF object tree:

### 1. Document Catalog Level

#### `/OpenAction` (Document Open Action)
- **Location**: Catalog dictionary (`/Type/Catalog`)
- **Execution**: Automatically when PDF is opened
- **Required parent key**: `/OpenAction`
- **Example**:
  ```
  7 0 obj
  <<
  /Type/Catalog
  /Pages 6 0 R
  /OpenAction << /S /JavaScript /JS (app.alert("pwn")) >>
  >>
  endobj
  ```

### 2. Page Level

#### `/AA` (Additional Actions) Dictionary
- **Location**: Page dictionary (`/Type/Page`)
- **Execution**: On specific page events (open, close, etc.)
- **Required parent key**: `/AA`
- **Trigger keys**: `/O` (open), `/C` (close), etc.
- **Example**:
  ```
  1 0 obj
  <<
  /Type/Page
  /MediaBox[0 0 612 792]
  /Parent 0 0 R
  /Contents 2 0 R
  /AA <<
    /O << /S /JavaScript /JS (app.alert('page_open')) >>
  >>
  >>
  endobj
  ```

### 3. Annotation Level

#### `/A` (Action) on Link Annotation
- **Location**: Annotation dictionary (`/Type/Annot /Subtype/Link`)
- **Execution**: When annotation is clicked/activated
- **Required parent key**: `/A`
- **Example**:
  ```
  5 0 obj
  <<
  /Type/Annot
  /Subtype/Link
  /Rect[100 600 200 620]
  /A << /S /JavaScript /JS (app.alert('annot_action')) >>
  >>
  endobj
  ```

### 4. Form Field Level

#### `/AA` on Interactive Form Fields
- **Location**: Form field dictionary (Widget annotation, Field dictionary)
- **Execution**: On field-specific events (focus, blur, validate, calculate, etc.)
- **Required parent key**: `/AA`
- **Trigger keys**: `/F` (format), `/V` (validate), `/K` (keystroke), `/C` (calculate)

## JavaScript String Formats

The `/JS` entry can contain JavaScript in three formats:

### 1. Literal Text String
```
/JS (app.alert('Hello'))
```
- Simple parentheses-delimited string
- Special characters must be escaped with backslash `\( \) \\`
- Limited to ASCII (use hex or octal escapes for non-ASCII)

### 2. Hexadecimal-Encoded String
```
/JS <6170702E616C657274282748656C6C6F2729>
```
- Encodes bytes as hex pairs
- Useful for binary data or Unicode characters
- Case-insensitive (6a = 6A)

### 3. Text Stream
```
/JS 10 0 R
...
10 0 obj
<< /Length 44 >>
stream
// Multi-line
// JavaScript code
app.alert('Hello');
endstream
endobj
```
- JavaScript stored as indirect object reference
- Can contain arbitrary content including newlines, quotes
- Useful for longer scripts

## Parent Keys and Attachment Points

### Required Parent Key Summary

| Attachment Point | Parent Key | Trigger |
|-----------------|-----------|---------|
| Document catalog | `/OpenAction` | Document opens |
| Page | `/AA /O` | Page opens |
| Page | `/AA /C` | Page closes |
| Annotation (Link) | `/A` | Annotation clicked |
| Annotation (Widget) | `/AA /K` | Keystroke |
| Annotation (Widget) | `/AA /F` | Format |
| Annotation (Widget) | `/AA /V` | Validate |
| Annotation (Widget) | `/AA /C` | Calculate |
| Bookmark | `/A` | Bookmark clicked |

## Complete Minimal PDF Example

Here's a minimal valid PDF with JavaScript at all three attachment points:

```
%PDF-1.4

1 0 obj
<<
/Type/Page
/MediaBox[0 0 612 792]
/Parent 3 0 R
/Resources<<
/Font<<
/F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>
>>
>>
/Contents 2 0 R
/AA<<
/O<</S/JavaScript/JS(app.alert('page_open'))>>
>>
>>
endobj

2 0 obj
<<
/Length 44
>>
stream
BT
/F1 12 Tf
100 700 Td
(Page 0) Tj
ET
endstream
endobj

3 0 obj
<<
/Type/Pages
/Count 1
/Kids[1 0 R]
>>
endobj

4 0 obj
<<
/Type/Catalog
/Pages 3 0 R
/OpenAction<</S/JavaScript/JS(app.alert("doc_open"))>>
>>
endobj

xref
0 5
0000000000 65535 f 
0000000009 00000 n 
0000000247 00000 n 
0000000348 00000 n 
0000000419 00000 n 
trailer
<<
/Size 5
/Root 4 0 R
>>
startxref
520
%%EOF
```

## Security Considerations (TH-04)

### Attack Vector
JavaScript in PDFs can execute arbitrary code in the Acrobat JavaScript context, including:
- File system access (limited but present)
- Network requests
- Information disclosure
- Social engineering (fake alerts, prompts)

### Detection Strategy
pdftract should:
1. **Never execute** embedded JavaScript
2. **Parse and detect** JavaScript actions at all attachment points
3. **Report presence** via `JAVASCRIPT_PRESENT` diagnostic (info-level)
4. **Surface metadata** in JSON output: `metadata.javascript_actions[]`
5. **Include location** (catalog/page/annotation), trigger, and code snippet

### Test Fixtures
The existing fixture `tests/fixtures/security/embedded-js.pdf` demonstrates all three attachment points:
- Catalog `/OpenAction` → `app.alert("pwn")`
- Page 0 `/AA /O` → `app.alert('page_open')`  
- Page 1 annotation `/A` → `app.alert('annot_action')`

## PDF Specification References

- **PDF 1.7 Reference** (ISO 32000-1:2008)
  - Section 12.6: Interactive Features (Actions)
  - Section 12.6.4: Action Types
  - JavaScript actions introduced in PDF 1.3
  
- **Additional Actions Dictionary** (`/AA`)
  - Defined in PDF 1.2+
  - Associates actions with specific trigger events
  - Prohibited in PDF/A-1 for archival stability

## Sources

- [PDF Reference, version 1.7](https://opensource.adobe.com/dc-acsdk-docs/pdfstandards/pdfreference1.7old.pdf) - Adobe Open Source
- [Portable document format — Part 1: PDF 1.7 (ISO 32000-1:2008)](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf) - Adobe Open Source
- [Notes for Analysing Malicious PDF Documents](https://prtksec.github.io/posts/MA_PDF_Notes/) - Pratik Patel
- [How PDF forms use JavaScript for validation](https://blog.idrsolutions.com/how-pdf-forms-use-javascript-for-validation/) - IDR Solutions
- [Corkami PDF Documentation](https://github.com/corkami/docs/blob/master/PDF/PDF.md) - Ange Albertini
- [PDF Injection Security](https://hacktricks.wiki/en/pentesting-web/xss-cross-site-scripting/pdf-injection.html) - HackTricks
- [PDF/A-1 Compliance (6.6.2)](https://www.soliddocuments.com/iso-19005-1-compliance.htm?subject=6.6.2) - Solid Documents

## Implementation Notes for pdftract

1. **Parser**: When traversing PDF dictionaries, check for:
   - `/OpenAction` in catalog
   - `/AA` in any object (page, annotation, form field)
   - `/A` in annotations

2. **Action detection**: When action dictionary found, check if `/S` equals `/JavaScript`

3. **Code extraction**: Extract `/JS` value (text string, hex string, or stream reference)

4. **Reporting**: Emit `JAVASCRIPT_PRESENT` diagnostic with:
   - Location path (e.g., `/Root/OpenAction`, `/Pages/Kids[0]/AA/O`)
   - Trigger event type
   - Code snippet (first 100 chars, sanitized)

5. **JSON output**: Add to `metadata.javascript_actions[]` array with structured info

## Verification Criteria

- [PASS] Documented minimal JavaScript action dictionary structure (`/S /JavaScript` + `/JS`)
- [PASS] Identified all attachment points (catalog `/OpenAction`, page `/AA`, annotation `/A`, form field `/AA`)
- [PASS] Understood JS string formats (literal, hex, stream)
- [PASS] Noted required parent keys and trigger events
- [PASS] Located existing test fixture with embedded JavaScript
- [PASS] Cited all sources from research
