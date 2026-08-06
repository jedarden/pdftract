# Markdown Structure Test Fixtures

This directory contains test fixtures for testing `extract_text()` vs `extract_markdown()` behavior.

## Content Specification

**IMPORTANT:** The authoritative content specification for these fixtures is documented in `SPECIFICATION.md`. That file defines the exact text strings, structural elements, and expected Markdown mappings for the PDF content. All fixture generation should reference that specification as the source of truth.

## Fixtures

### markdown-structures.pdf

A test PDF with clear Markdown-style structural elements for testing text extraction.

**Contents:** (See `SPECIFICATION.md` for exact content)
- Multiple heading levels (H1, H2, H3)
- Link text (URLs in various formats)
- Bullet lists
- Numbered lists
- Tables

**Purpose:** Tests that `extract_markdown()` properly formats output with Markdown syntax while `extract_text()` returns plain text.

**Generator:** `tests/fixtures/markdown_test_fixture.py` (implements `SPECIFICATION.md`)

**Ground truth files:**
- `markdown-structures-expect-text.txt` - Expected output from `extract_text()`
- `markdown-structures-expect-markdown.txt` - Expected output from `extract_markdown()`

**Validation Status:** ✓ Validated
- PDF opens successfully with `pdfinfo` (poppler-utils)
- Single-page letter-size document (612 x 792 pts)
- Created with ReportLab PDF Library, PDF version 1.4
- Uses Helvetica and Helvetica-Bold fonts
- Text extraction produces 57 lines of content
- Contains all expected structural elements: headings (H1-H3), bare URLs, bullet lists (•), numbered lists (1-5), and a table

## Generating Fixtures

To regenerate these fixtures (requires ReportLab):

```bash
# Using the pdftract-py virtual environment
crates/pdftract-py/.venv/bin/python3 tests/fixtures/markdown_test_fixture.py

# Or with system Python (if reportlab is installed)
python3 tests/fixtures/markdown_test_fixture.py
```

## Expected Behavior

When processed by pdftract:

- `extract_text()` should return plain text without Markdown formatting (see `markdown-structures-expect-text.txt`)
- `extract_markdown()` should return properly formatted Markdown with:
  - Headings marked with `#`, `##`, etc.
  - Links in `[text](url)` syntax
  - Lists with `-` or `1.` syntax
  - Tables in Markdown table format
