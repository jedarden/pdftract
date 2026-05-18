# SLIDE_DECK Profile

Presentation slides with title, presenter, date, slide titles

## Match Criteria Summary

This profile matches presentation slides exported to PDF. Documents typically exhibit:

- **Landscape orientation**: Slides are almost always landscape (4:3 or 16:9 aspect ratio)
- **Large centred text**: Title slides have large, centered text
- **Multiple pages**: 3+ pages minimum; slide decks often run 10-200 pages
- **Slide numbering**: "Slide 1", "Slide 2", or table of contents

This is a degenerate profile with minimal field extraction (title, presenter, date, slide titles) because slide-deck PDFs vary enormously depending on the presentation software and exporter.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| date | date | Extracted from page text using pattern matching | 2024-01-15 | regex patterns, region: first_page_bottom |
| presenter | string | Extracted from page text using pattern matching | "example value" | regex patterns, region: first_page_below_title |
| slide_titles | array | Extracted from page text using pattern matching | [...] | regex patterns, region: top_left_or_centre, per-page |
| title | string | Extracted from page text using pattern matching | "example value" | regex patterns, region: first_page_centre |

## Known Limitations

*This section documents known edge cases and failure modes. Contributions to improve extraction quality are welcome.*

- **Exporter variability**: Slide-deck PDFs vary enormously depending on the presentation software (PowerPoint, Keynote, Google Slides) and PDF exporter; extraction quality depends heavily on how text was converted to PDF
- **Image-heavy slides**: Slides with minimal text (e.g., photo slides, diagrams) will not produce meaningful slide_titles
- **Non-standard layouts**: Slides without clear title regions (e.g., all-center layouts, artistic templates) may not extract slide_titles correctly
- **Presenter extraction**: Assumes the presenter name appears below the title on the first slide; alternative formats (e.g., title slide with no presenter) will miss this field
- **Date parsing**: Date extraction from first-page footer may fail if the presentation date is in a non-standard format

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

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
