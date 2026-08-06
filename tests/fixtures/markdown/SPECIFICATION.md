# Markdown Structure PDF Content Specification

**Purpose:** This document specifies the exact content requirements for the `markdown-structures.pdf` test fixture. It serves as the source of truth for what structural elements should be present in the PDF.

**Target PDF:** `tests/fixtures/markdown/markdown-structures.pdf`

**Version:** 1.0

---

## Document Structure Overview

The PDF shall contain the following structural elements, organized as a single-page test document:

1. **Headings** (3 levels)
2. **Links** (bare URLs in various formats)
3. **Bullet lists** (unordered)
4. **Numbered lists** (ordered)
5. **Tables** (with headers and multiple rows)

---

## Exact Content Specification

### 1. Document Header

**Heading Level 1 (H1):**
```
Markdown Test Document
```

**Heading Level 2 (H2):**
```
Testing extract_markdown() vs extract_text()
```

**Heading Level 3 (H3):**
```
By Test Author
```

---

### 2. Introduction Section

**Paragraph (plain text):**
```
This document tests the difference between extract_text() and extract_markdown().
The extract_text() function should return plain text without Markdown formatting.
The extract_markdown() function should return properly formatted Markdown with
headings marked with #, links with [text](url) syntax, and lists with - or 1. syntax.
```

---

### 3. Links Section

**Section Heading (H2):**
```
Links Section
```

**Paragraph with bare URLs:**
```
Visit our website at https://example.com for more information.
You can also email us at support@example.com or check our documentation at
https://docs.example.com/guide.
```

**URLs included:**
- `https://example.com`
- `support@example.com`
- `https://docs.example.com/guide`

---

### 4. Bullet List Section

**Section Heading (H2):**
```
Key Features
```

**Bullet list items (unordered):**
```
• Feature one: Text extraction from PDFs
• Feature two: Markdown formatting support
• Feature three: Link preservation
• Feature four: List detection
• Feature five: Table extraction
```

**Format:** Each item should start with a bullet point (•) or similar marker.

---

### 5. Numbered List Section

**Section Heading (H2):**
```
Implementation Steps
```

**Numbered list items (ordered):**
```
1. Parse the PDF content stream
2. Extract text spans with metadata
3. Detect block types (heading, paragraph, list)
4. Format blocks as Markdown
5. Return the complete Markdown output
```

**Format:** Each item should start with a number followed by a period.

---

### 6. Table Section

**Section Heading (H2):**
```
Sample Table
```

**Table structure:**

| Column 1: Name | Column 2: Type | Column 3: Description |
| --- | --- | --- |
| extract_text | Function | Returns plain text from PDF |
| extract_markdown | Function | Returns Markdown formatted text |
| Markdown | Format | Lightweight markup language |
| PDF | Format | Portable Document Format |

**Table rows:**
1. Header: `Name | Type | Description`
2. Row 1: `extract_text | Function | Returns plain text from PDF`
3. Row 2: `extract_markdown | Function | Returns Markdown formatted text`
4. Row 3: `Markdown | Format | Lightweight markup language`
5. Row 4: `PDF | Format | Portable Document Format`

---

## Markdown Output Mapping

When the PDF is processed by `extract_markdown()`, the content should map to Markdown syntax as follows:

### Headings
- `Markdown Test Document` → `# Markdown Test Document`
- `Testing extract_markdown() vs extract_text()` → `## Testing extract_markdown() vs extract_text()`
- `By Test Author` → `### By Test Author`
- `Links Section` → `## Links Section`
- `Key Features` → `## Key Features`
- `Implementation Steps` → `## Implementation Steps`
- `Sample Table` → `## Sample Table`

### Links
- `https://example.com` → `<https://example.com>` or `[https://example.com](https://example.com)`
- `support@example.com` → `<support@example.com>` (email link)
- `https://docs.example.com/guide` → `<https://docs.example.com/guide>`

### Bullet Lists
```
- Feature one: Text extraction from PDFs
- Feature two: Markdown formatting support
- Feature three: Link preservation
- Feature four: List detection
- Feature five: Table extraction
```

### Numbered Lists
```
1. Parse the PDF content stream
2. Extract text spans with metadata
3. Detect block types (heading, paragraph, list)
4. Format blocks as Markdown
5. Return the complete Markdown output
```

### Tables
```markdown
| Name | Type | Description |
| --- | --- | --- |
| extract_text | Function | Returns plain text from PDF |
| extract_markdown | Function | Returns Markdown formatted text |
| Markdown | Format | Lightweight markup language |
| PDF | Format | Portable Document Format |
```

---

## Plain Text Output Mapping

When processed by `extract_text()`, the same content should return as plain text without Markdown formatting:
- Headings appear as plain text (no `#` prefixes)
- URLs appear as plain text (no `<angle brackets>` or `[text](url)` syntax)
- Lists appear as plain text (bullet points as is, numbers as is)
- Tables appear as space-separated text or a simplified representation

---

## Validation Criteria

A generated `markdown-structures.pdf` is valid if it contains:

1. **All specified text strings** exactly as written above
2. **Visual hierarchy** (headings in larger/bold font, paragraphs in normal font)
3. **Lists** (bullet or numbered items visually distinguished)
4. **Table** (with visible cell structure and borders)
5. **Single page** (all content fits on one page)

---

## Generation Implementation Note

The specification is implemented in `tests/fixtures/markdown_test_fixture.py` using ReportLab. The script should reference this specification to ensure the generated PDF matches these requirements.

---

## Change History

- **2024-08-06:** Initial specification created for bf-5e4912 task
