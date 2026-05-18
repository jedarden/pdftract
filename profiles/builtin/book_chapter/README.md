# BOOK_CHAPTER Profile

Book chapter with title, chapter number, author, section headings

## Match Criteria Summary

This profile matches book chapters and book excerpts. Documents typically contain:

- **Chapter headings**: "Chapter XIV", "Chapter 3", or numbered sections like "3.1 Introduction"
- **Section numbering**: Hierarchical section headings (e.g., "1.2", "3.4.1") or all-caps headings
- **Running headers**: Book title, author name, or chapter title in page headers
- **Multi-page structure**: Book chapters are almost always 5+ pages

The profile expects formal book formatting with clear chapter/section headings. It works for fiction non-fiction chapters, textbook excerpts, and technical book chapters.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| author | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| chapter_number | string | Extracted from page text using pattern matching | "example value" | regex patterns, region: first_page_top |
| sections | array | Extracted from page text using pattern matching | [...] | regex patterns, region: headings |
| title | string | Extracted from page text using pattern matching | "example value" | regex patterns, region: first_page_top |

## Known Limitations

*This section documents known edge cases and failure modes. Contributions to improve extraction quality are welcome.*

- **Author extraction**: Assumes author is explicitly listed with "by:" or "author:" markers; books without explicit author attribution may miss this field
- **Section heading parsing**: Only captures top-level headings; nested subsections may be missed
- **Short chapters**: Chapters under 5 pages may not match (page_count_gte: 5)
- **Prefaces/introductions**: Front matter without clear chapter numbering may not match
- **Multi-chapter excerpts**: Excerpts containing multiple chapters may only extract the first chapter number
- **Non-English books**: Pattern matching is optimized for English terminology like "Chapter" and "Section"

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

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
