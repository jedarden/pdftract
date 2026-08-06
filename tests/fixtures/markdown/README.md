# Markdown Structure Test Fixtures

This directory contains test fixtures for testing `extract_text()` vs `extract_markdown()` behavior.

## Fixtures

### markdown-structures.pdf

A test PDF with clear Markdown-style structural elements for testing text extraction.

**Contents:**
- Multiple heading levels (H1, H2, H3)
- Link text (URLs in various formats)
- Bullet lists
- Numbered lists
- Tables

**Purpose:** Tests that `extract_markdown()` properly formats output with Markdown syntax while `extract_text()` returns plain text.

**Generator:** `tests/fixtures/markdown_test_fixture.py`

**Ground truth files:**
- `markdown-structures-expect-text.txt` - Expected output from `extract_text()`
- `markdown-structures-expect-markdown.txt` - Expected output from `extract_markdown()`

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
