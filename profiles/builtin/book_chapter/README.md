# BOOK_CHAPTER Profile

Book chapter with title, chapter number, author, section headings

## Match Criteria Summary

A document matches this profile when it displays the characteristic structure of a book chapter or excerpt. The classifier identifies chapter-specific terminology like "chapter" with Roman or Arabic numerals, "section" with numbers, and numbered section headings (e.g., "1. Introduction"). Structurally, chapters are recognized by running headers (often showing book title, chapter title, or page numbers), chapter headings, and sufficient length (5+ pages). Chapter boundaries are typically marked by large, centered chapter titles. Section headings within the chapter are extracted to provide a table of contents. This profile works best for professionally typeset books rather than scans.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| title | string | Extracted from page text using pattern matching | "example value" | region: first_page_top |
| chapter_number | string | Extracted from page text using pattern matching | "example value" | region: first_page_top |
| author | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| sections | array | Extracted from page text using pattern matching | [...] | region: headings |

## Known Limitations

- Chapter title extraction may confuse chapter title with book title if both appear on the first page
- Author extraction may fail if the author is not explicitly named on the chapter pages (e.g., listed in book front matter)
- Section heading extraction may capture sub-sections, sidebars, or pull quotes if they are formatted as headings
- Running headers with page numbers may interfere with section heading extraction
- Chapters with non-standard numbering (e.g., "Chapter One", "Part I") may not extract chapter numbers correctly
- Multi-chapter excerpts (e.g., chapters 3-4) may extract only the first chapter's information
- Books with complex layouts (multiple columns, marginal notes) may have reduced extraction quality
- Non-English books may not match due to English-only text patterns in match criteria

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/profiles/book_chapter/`.

*See the classifier corpus for representative documents.*

## Configuration Tips

To override this profile:

```bash
pdftract profiles export book_chapter > my-profile.yaml
# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-profile.yaml document.pdf
```

For chapters from specific publishers or series with consistent formatting, consider adding publisher-specific patterns to improve matching. For academic book chapters with different structure (e.g., contributed volumes with chapter authors), you may want to customize the `author` field extraction.

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
