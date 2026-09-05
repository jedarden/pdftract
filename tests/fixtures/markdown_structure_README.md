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
- **Generator:** `tools/generate_markdown_structure_fixture.py` (run with no arguments writes exactly this path)

> **Drift note:** the checked-in 2,265-byte PDF was produced by an earlier revision of
> the generator. The current script emits a two-page superset — it adds the
> `## Resources Section`, `### Conclusion`, and four further links wrapped in clickable
> `<a href>` annotations — so regenerating is *not* byte-identical. The sections below
> distinguish what the checked-in fixture contains from what the generator design covers.

## Structural Elements

The fixture contains the following Markdown-style structural elements:

### 1. Headings with # Markers
- **Level 1:** `# Main Document Title`
- **Level 2:** `## Section Subtitle`
- **Level 3:** `### Subsection Header`

### 2. Links with ``[text](url)`` syntax

Design contract: the PDF carries the link **text** in square brackets as visible
characters — `[text]` — and never prints the URL. `extract_markdown()` is expected to
re-attach the target and emit `[text](url)`; `extract_text()` must emit the bare
`[text]`. The generator supplies targets through ReportLab's `<a href="...">` markup,
which becomes a `/URI` link annotation where present.

In the checked-in fixture — 3 links, plain visible text, no annotations:

| Visible text | Designed target |
|---|---|
| `[link to example.com]` | `https://example.com` |
| `[GitHub]` | `https://github.com` |
| `[the documentation]` | `https://docs.example.com` |

Additional links that exist only in the generator's extended two-page design
(annotation-backed):

| Visible text | Designed target |
|---|---|
| `[Example.org]` | `https://example.org` |
| `[Test Site]` | `https://test.example` |
| `[a link]` | `https://mixed.content` |
| `[final link]` | `https://final.link` |

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

## PDF Generation Tool

**ReportLab** (platypus API), driven by `tools/generate_markdown_structure_fixture.py`.

Chosen because:

1. **Already available in this workspace.** ReportLab 4.x is installed (verified 4.5.1)
   and is the generator behind the repo's other hand-authored PDF fixtures, so the test
   environment gains no new dependency.
2. **Markup-literal rendering.** platypus `Paragraph` draws its input as opaque glyphs:
   `# Main Document Title` and `[GitHub]` are rendered verbatim and are never parsed as
   Markdown. That is the whole point of a "Markdown-native" fixture — the syntax markers
   must survive as *text*, not be converted into formatting by the generator.
3. **Optional real hyperlinks.** `<a href="...">` yields a genuine `/URI` link
   annotation, so the `[text](url)` design can be exercised at the annotation layer
   without changing what is visibly printed.
4. **Deterministic and small.** Single-pass, offline, no GUI and no network; base-14
   fonts (Helvetica / Helvetica-Bold / Courier, WinAnsiEncoding) keep the artifact at
   roughly 2 KB and reproducible across machines.
5. **Explicit layout control.** Per-element `ParagraphStyle` (font size, leading,
   `spaceAfter`, `Spacer`) fixes the vertical rhythm of the page rather than inheriting
   tool defaults, which keeps the extraction order stable.

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

✓ **Re-validated:** 2026-09-05
- Size on disk matches the documented 2,265 bytes; header declares PDF 1.4, `/Producer` is ReportLab, one page (`/Count 1`), `MediaBox [0 0 612 792]`
- Content stream (`/ASCII85Decode` + `/FlateDecode`) decoded directly: all heading, link, list, inline-code and code-block strings from the "Text Content" section are present verbatim, in that order, across fonts F1 (Helvetica), F2 (Helvetica-Bold) and F3 (Courier)
- No `/Annots` on the page — the checked-in links are plain visible text, as documented above

## Text Content (checked-in fixture)

Extraction order of the checked-in PDF, top to bottom:

````
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
````

(The generator source writes the `return` line with four leading spaces; the checked-in
PDF's content stream carries it unindented. Code block markers are literal triple
backticks in the visible text.)

## Related Files

- **Generator script:** `tools/generate_markdown_structure_fixture.py` - Generates this fixture (run with no arguments writes `tests/fixtures/markdown_structure.pdf`)
- **Alternative fixture:** `tests/fixtures/markdown/markdown-structures.pdf` - A more formal test fixture with tables and additional structural elements (its generator, `tests/fixtures/markdown_test_fixture.py`, is cited by `markdown/SPECIFICATION.md` but is not currently in-tree)
- **Specification:** `tests/fixtures/markdown/SPECIFICATION.md` - Detailed specification for the markdown/ fixture
- **Consumers:** `crates/pdftract-core/src/sdk.rs` (integration test), `tests/sdk/test_extract_smoke.py`, `tests/sdk/test_python_sdk.py`

## Notes

This fixture is designed to be "Markdown-native" - the structural elements are visible in the raw text extraction, making it ideal for testing Markdown structure detection algorithms. The # markers in headings, [brackets] in links, and code markers are intentionally visible in the PDF content.
