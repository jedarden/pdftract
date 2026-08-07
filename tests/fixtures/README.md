# PDF Fixtures

This directory contains test fixture PDFs used for validating pdftract's extraction behavior.

## markdown_structure.pdf

### Purpose
Tests the extraction of text with visible Markdown structure markers. This fixture validates the difference between `extract_text()` (which returns raw text content) and `extract_markdown()` (which processes structural elements).

### What it contains
The PDF contains the following structural elements with **visible** Markdown syntax markers:

- **Headings**: `# Main Document Title`, `## Section Subtitle`, `### Subsection Header`
- **Links**: `[link to example.com]`, `[GitHub]`, `[the documentation]`, `[Example.org]`, `[Test Site]`
- **Bullet lists**: `• First bullet point item`, `• Second bullet with more detail`, etc.
- **Numbered lists**: `1. First numbered item`, `2. Second numbered item`, `3. Third numbered item`
- **Inline code**: `var x = 42;` (in a code-like context)
- **Code blocks**: Multi-line code wrapped in ` ``` ` markers
- **Mixed content**: Paragraphs combining links, inline code, and regular text

### How it was generated
Two generation scripts exist:

1. **Primary script**: `tools/generate_markdown_structure_fixture.py`
   - Uses ReportLab with custom paragraph styles
   - Includes clickable hyperlinks (blue `<a>` tags)
   - More polished rendering with proper spacing

2. **Fallback script**: `tests/fixtures/create_markdown_structure_fixture.py`
   - Simpler approach using normal styling
   - Uses bold (`<b>`) for link-like text
   - More consistent "visible syntax" approach

To regenerate the fixture:

```bash
# Using the primary script
python3 tools/generate_markdown_structure_fixture.py

# Or specify a custom output path
python3 tools/generate_markdown_structure_fixture.py -o /tmp/test.pdf

# Or use the fallback script
python3 tests/fixtures/create_markdown_structure_fixture.py
```

### Expected behavior

#### `extract_text()`
Should return the raw text content **exactly as it appears** in the PDF, including all Markdown markers:

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

#### `extract_markdown()`
May process the structural elements differently. The exact behavior depends on implementation, but the key test is that:
- The method **recognizes** structural patterns (headings, lists, links)
- The output preserves **semantic meaning** while potentially formatting it differently

### Known limitations / quirks

1. **Links are not real hyperlinks**: In the text content, links appear as `[text]` format, not as actual clickable URLs. The `extract_markdown()` method would need to reconstruct URLs from the PDF's annotation layer if they exist.

2. **Code blocks are simulated**: The ` ``` ` markers are literal text, not PDF formatting. A real Markdown parser would need to detect this pattern.

3. **Font rendering differences**: The fixture uses ReportLab's default fonts. If the rendering environment differs, spacing or character appearance may vary slightly.

### Related tests
- Tests that validate `extract_text()` preserves all characters verbatim
- Tests that validate `extract_markdown()` correctly identifies structural patterns
- Conformance tests that check round-trip behavior

### File info
- **Size**: ~2.2 KB
- **Created**: 2026-08-06
- **Last updated**: 2026-08-06
