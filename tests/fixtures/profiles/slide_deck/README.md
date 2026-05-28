# Slide Deck Profile Fixtures

This directory contains test fixtures for the slide deck document profile.

## Fixture Types

1. **pitch_deck** - Sales/product pitch deck (10 slides) with typical startup presentation structure
2. **academic_lecture** - Academic lecture slides (40 slides) with technical content and Q&A slides
3. **corporate_kickoff** - Corporate annual kickoff presentation (15 slides) with business metrics and roadmap
4. **bilingual_deck** - Bilingual English/Spanish presentation (12 slides) testing multilingual extraction
5. **googleslides_handout** - Google Slides handout mode export (3 slides per page, 4 pages total) testing multi-slide-per-page edge case

## Expected Output Format

Each fixture should have a corresponding `*-expected.json` file with the following structure:

```json
{
  "metadata": {
    "document_type": "slide_deck",
    "document_type_confidence": 0.XX,
    "document_type_reasons": [...],
    "profile_name": "slide_deck",
    "profile_version": "1.0.0",
    "profile_fields": {
      "title": "...",
      "presenter": "...",
      "date": "YYYY-MM-DD",
      "slide_titles": [...]
    }
  }
}
```

## Profile Fields

The slide deck profile extracts the following fields:

- **title**: Presentation title (region: middle_half, pick: largest_font)
- **presenter**: Presenter name (region: bottom_half, pick: largest_font)
- **date**: Presentation date (near: "Date", parse: date)
- **slide_titles**: Ordered list of slide titles (pick: largest_font, collected per page)

## Known Limitations

- Multi-slide-per-page PDFs (handout mode) are a known limitation: page_count no longer equals slide count
- Slides with image-based titles or icons will not extract slide titles correctly
- Presenter extraction often fails when slides include logos or affiliations with names
- Non-English presentations may have reduced extraction accuracy
- Google Slides exports vary in structure depending on export settings
- Beamer (LaTeX) exports have very different structural signals

## Provenance

All fixtures are sourced from synthetic templates created for testing purposes. See PROVENANCE.md for details on each fixture.
