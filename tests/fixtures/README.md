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

---

## no-mapping.pdf

### Purpose
Tests the **GLYPH_UNMAPPED diagnostic** and Level 4 Unicode recovery — the worst-case scenario for PDF text extraction. This fixture validates pdftract's behavior when encountering custom glyph names that cannot be mapped to Unicode characters through any standard method.

### What it contains
A minimal hand-written PDF (660 bytes) with three custom glyphs (`/g001`, `/g002`, `/g003`) that:
- Use a custom Type1 font (`CustomNoMap`) with non-standard encoding
- Have no ToUnicode CMap (Level 1 recovery fails)
- Are not in the Adobe Glyph List (Level 2 recovery fails)
- Have no embedded font for fingerprinting (Level 3 recovery fails)
- Have no embedded glyph outlines (Level 4 recovery fails)

### Expected behavior
When pdftract extracts text from this fixture, it should emit **three U+FFFD (REPLACEMENT CHARACTER)** codepoints (`���`), representing the three unmapped glyphs. This validates:
- Proper handling of completely unmappable content
- Correct GLYPH_UNMAPPED diagnostic emission
- Graceful fallback when all Unicode recovery levels fail

### Ground truth
The expected output is documented in `tests/fixtures/encoding/no-mapping.txt`:
```
���
```

### Related diagnostics
- **GLYPH_UNMAPPED**: Emitted when a glyph cannot be mapped to any Unicode character
- **ENCODING_RECOVERY_LEVEL_4**: Indicates Level 4 (glyph shape) recovery was attempted
- **UNICODE_REPLACEMENT_CHARACTER**: Confirms U+FFFD emission for unmapped content

### Comprehensive documentation
See **[`tests/fixtures/encoding/no-mapping.md`](encoding/no-mapping.md)** for complete details including:
- Raw PDF structure analysis
- Font encoding details
- Regeneration instructions
- Test coverage scenarios
- Troubleshooting guide

### Related tests
- `tests/encoding_recovery.rs` — Unicode recovery level tests
- `tests/test_glyph_unmapped_diagnostic.rs` — GLYPH_UNMAPPED diagnostic validation
- `tests/encoding_recovery_integration.rs` — Full extraction pipeline tests

### Related fixtures
- `agl-only.pdf` — Level 2 recovery (Adobe Glyph List mapping)
- `fingerprint-match.pdf` — Level 3 recovery (font fingerprinting)
- `shape-match.pdf` — Level 4 recovery (glyph shape database)
- `unmapped-glyphs.pdf` — Additional unmapped glyph test case

### File info
- **Location**: `tests/fixtures/encoding/no-mapping.pdf`
- **Size**: 660 bytes
- **Created**: 2026-06-09
- **Last updated**: 2026-07-03
- **SHA256**: `b24f88d3add958bfec1d6b134f2cd030cd41bb1932bedbe99405599bd01fa8f0`
