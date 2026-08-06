# Markdown Structure Test Fixture

## Overview

`markdown_structure.pdf` is a test fixture PDF designed to test Markdown structure extraction from PDF documents. It contains clear Markdown-style structural elements that should map to Markdown syntax when processed by `extract_markdown()`.

## Location

- **File:** `tests/fixtures/markdown_structure.pdf`
- **Size:** 2,265 bytes
- **Pages:** 1
- **Page Size:** Letter (612 x 792 pts)
- **PDF Version:** 1.4
- **Created with:** ReportLab PDF Library
- **Generator:** `tests/fixtures/create_markdown_structure_fixture.py`

## Structural Elements

The fixture contains the following Markdown-style structural elements:

### 1. Headings with # Markers
- **Level 1:** `# Main Document Title`
- **Level 2:** `## Section Subtitle`
- **Level 3:** `### Subsection Header`

### 2. Links with [text] Format
- `[link to example.com]`
- `[GitHub]`
- `[the documentation]`
- `[Example.org]`
- `[Test Site]`
- `[a link]`
- `[final link]`

### 3. Lists
**Bullet lists (unordered):**
- `• First bullet point item`
- `• Second bullet with more detail`
- `• Third bullet point that spans a bit more text`

**Numbered lists (ordered):**
- `1. First numbered item`
- `2. Second numbered item`
- `3. Third numbered item`

### 4. Code Elements
**Inline code:**
- `var x = 42;`

**Code blocks:**
```
def example_function():
    return 'hello world'
```

## Purpose

This fixture serves to test the difference between:
- `extract_text()` - should return plain text without Markdown formatting
- `extract_markdown()` - should return properly formatted Markdown with:
  - Headings marked with `#`, `##`, `###`
  - Links in `[text](url)` syntax
  - Lists with `-` or `1.` syntax
  - Code blocks with backticks

## Validation

✓ **Validated:** 2026-08-06
- Opens successfully with `pdfinfo` (poppler-utils)
- 1-page letter-size document (612 x 792 pts)
- Contains all expected structural elements
- Text extraction produces readable output with visible Markdown syntax markers (`#`, `[text]`, code blocks)

## Text Content

```
# Main Document Title
## Section Subtitle
This is a paragraph with a [link to example.com] and some regular text following it.
Visit [GitHub] for more information or check [the documentation].
### Subsection Header
• First bullet point item
• Second bullet with more detail
• Third bullet point that spans a bit more text
1. First numbered item
2. Second numbered item
3. Third numbered item
Here is some inline code: var x = 42; within a paragraph of text.
```
def example_function():
return 'hello world'
```
```

## Related Files

- **Generator script:** `tests/fixtures/create_markdown_structure_fixture.py` - Generates this fixture
- **Alternative fixture:** `tests/fixtures/markdown/markdown-structures.pdf` - A more formal test fixture with tables and additional structural elements
- **Alternative generator:** `tests/fixtures/markdown_test_fixture.py` - Generates the markdown/ subdirectory fixture
- **Specification:** `tests/fixtures/markdown/SPECIFICATION.md` - Detailed specification for the markdown/ fixture

## Notes

This fixture is designed to be "Markdown-native" - the structural elements are visible in the raw text extraction, making it ideal for testing Markdown structure detection algorithms. The # markers in headings, [brackets] in links, and code markers are intentionally visible in the PDF content.
