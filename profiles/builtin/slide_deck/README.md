# SLIDE_DECK Profile

Presentation slides with title, presenter, date, slide titles

## Match Criteria Summary

A document matches this profile when it exhibits the visual and structural characteristics of a presentation slide deck. The classifier identifies presentation-specific terminology like "slide" with numbers, "table of contents", and "presentation". Structurally, slide decks are recognized by their landscape aspect ratio (16:9 or 4:3), page counts of 3 or more, and large centered text typical of slide titles. Each page is treated as a slide, and the profile extracts title and presenter information from the title slide while capturing slide titles from subsequent pages. Extraction quality depends heavily on how the slides were exported to PDF.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| title | string | Extracted from page text using pattern matching | "example value" | region: first_page_centre |
| presenter | string | Extracted from page text using pattern matching | "example value" | region: first_page_below_title |
| date | date | Extracted from page text using pattern matching | 2024-01-15 | region: first_page_bottom |
| slide_titles | array | Extracted from page text using pattern matching | [...] | region: top_left_or_centre, per-page |

## Known Limitations

- Slide-deck PDFs vary enormously in quality; extraction depends on the exporter (PowerPoint, Keynote, Google Slides all export differently)
- Slides with complex graphics or image-based text will not extract slide titles correctly
- Presenter extraction may fail for non-standard name formats or institutional affiliations
- Slide title extraction may capture bullet points or body text if slide layout is non-standard
- Slides with multiple title candidates (e.g., subtitles, taglines) may extract the wrong text
- Presenter photos or logos on the title slide can confuse text extraction
- Hidden slides or notes pages (if included in the PDF) may be incorrectly processed
- Non-English presentations may not match due to English-only text patterns

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/profiles/slide_deck/`.

*See the classifier corpus for representative documents.*

## Configuration Tips

To override this profile:

```bash
pdftract profiles export slide_deck > my-profile.yaml
# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-profile.yaml document.pdf
```

For presentations from specific conferences or templates, consider adding template-specific patterns to improve slide title extraction. For corporate slide decks with branded title slides, you may need to customize the `presenter` and `date` region hints.

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
