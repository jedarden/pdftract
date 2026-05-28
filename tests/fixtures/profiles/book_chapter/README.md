# Book Chapter Profile Fixtures

This directory contains test fixtures for the book chapter document profile.

## Fixture Types

1. **novel_chapter** - Project Gutenberg-style novel chapter (public domain), narrative fiction with chapter number, author, and sections
2. **academic_chapter** - Academic book chapter (CC-BY license), scholarly content with structured sections and formal tone
3. **textbook_chapter** - Textbook chapter with figures, educational content with structured sections and figure references
4. **technical_manual_chapter** - Technical manual chapter, procedural content with numbered steps and warnings
5. **recipe_book_chapter** - Cookbook chapter, instructional content with ingredient lists and techniques

## Expected Output Format

Each fixture has a corresponding `*-expected.json` file with the following structure:

```json
{
  "metadata": {
    "document_type": "book_chapter",
    "document_type_confidence": 0.XX,
    "document_type_reasons": [...],
    "profile_name": "book_chapter",
    "profile_version": "1.0.0",
    "profile_fields": {
      "title": "...",
      "chapter_number": "...",
      "author": "...",
      "sections": [...]
    }
  }
}
```

## Profile Fields

The book chapter profile extracts the following fields:

- **title**: Chapter title (region: top_third, pick: largest_font, page: first)
- **chapter_number**: Chapter number (near: ['Chapter', 'Part'], regex: '\d+')
- **author**: Author name (region: top_quarter, pick: smallest_font, page: first)
- **sections**: List of section headings (per-page collection)

## Profile Characteristics

- **Priority**: 5 (lowest among built-in profiles - acts as catch-all for narrative text)
- **Reading Order**: line_dominant (for top-to-bottom narrative flow)
- **Readability Threshold**: 0.6 (higher threshold for narrative text quality)
- **Headers/Footers**: Excluded (page numbers are not body content)

## Provenance

All fixtures are created synthetically with clear provenance documentation. See PROVENANCE.md for details on each fixture.

## Known Limitations

- Multi-chapter PDFs (whole books) are not fully supported at v1.0 - the profile matches the first chapter only
- Un-numbered chapters (Prologue, Epilogue, Acknowledgements) will have null chapter_number
- Sections extraction is a best-effort table-of-contents based on heading-level-2+ headings
- Non-numeric chapter numbering (Roman numerals, words) may not be captured correctly
