# BOOK_CHAPTER Profile

Book chapter with title, chapter number, author, section headings

## Match Criteria Summary

Documents matching this profile typically contain:

- **Strong text signals**: Words like "chapter 1", "section 1.1", numbered headings (1., 2., etc.)
- **Structural signals**: Running headers (book title, chapter title, page numbers), chapter heading structures
- **Page count**: Usually 5-50 pages (chapters vary by book type)
- **Layout patterns**: Chapter title at top, chapter number, author (if different from book), numbered sections, running headers

The classifier looks for book chapter terminology combined with running headers and chapter heading structures. Documents with "chapter" terminology AND running headers match with highest confidence.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| title | string | Chapter title | "The Origins of Language" | First page, after chapter number |
| chapter_number | string | Chapter identifier | "3" or "III" | "chapter" field or first heading number |
| author | string | Chapter author (if applicable) | "Dr. Jane Smith" | "by" or "author" fields near title |
| sections | array | Section headings within chapter | `["Introduction", "Historical Context", "Analysis"]` | Headings throughout the chapter |

## Known Limitations

- **Books without chapter numbers**: Books with unnumbered chapters (e.g., named chapters only) may not match correctly
- **Multi-author books**: Chapters by different authors may not extract author if not explicitly labeled
- **Complex numbering**: Non-standard chapter numbering (e.g., "Chapter 3A") may not parse correctly
- **Front/back matter**: Prefaces, introductions, and conclusions without "chapter" labels may not match
- **Non-English books**: Books in other languages may not match pattern lists
- **Scanned books**: Poor OCR quality can lead to missed headings, especially in decorative fonts
- **Ebook exports**: Ebook PDF exports may have unusual layouts (e.g., flowing text) that confuse heading detection
- **Running header variations**: Books with alternating running headers (recto/verso) may not parse consistently
- **Boxed or sidebar sections**: Sidebars or boxed text may be incorrectly identified as sections

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/classifier/misc/`.

Book chapter fixtures are typically multi-page documents with clear chapter headings and running headers.

## Configuration Tips

To override this profile for custom book chapter formats:

```bash
pdftract profiles export book_chapter > my-chapter.yaml
# Edit my-chapter.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-chapter.yaml document.pdf
```

Common customizations:
- Add book-specific section numbering patterns to `sections.extraction.patterns`
- For numbered sections (e.g., "1.1", "1.2"), adjust patterns to capture hierarchical numbering
- If chapters use roman numerals (I, II, III), ensure patterns include these formats

---

*This README documents the built-in `book_chapter` profile. See `docs/research/document-classification-and-zone-labeling.md` for classifier theory.*
