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
| title | string | Presentation title from first slide | "Q4 2024 Business Review" | regex patterns, region: first_page_centre |
| presenter | string | Presenter name from title slide | "Jane Smith" | regex patterns, region: first_page_below_title |
| date | date | Presentation date | 2024-01-15 | regex patterns, region: first_page_bottom |
| slide_titles | array | Title text from each slide | ["Overview", "Metrics", "Q&A"] | regex patterns, region: top_left_or_centre, per-page |

## Known Limitations

- **Exporter variability**: Slide-deck PDFs vary enormously depending on the presentation software (PowerPoint, Keynote, Google Slides) and PDF exporter; extraction quality depends heavily on how text was converted to PDF
- **Image-heavy slides**: Slides with minimal text (e.g., photo slides, diagrams) will not produce meaningful slide_titles
- **Non-standard layouts**: Slides without clear title regions (e.g., all-center layouts, artistic templates) may not extract slide_titles correctly
- **Presenter extraction**: Assumes the presenter name appears below the title on the first slide; alternative formats (e.g., title slide with no presenter) will miss this field
- **Date parsing**: Date extraction from first-page footer may fail if the presentation date is in a non-standard format
- **Handout formats**: PDF handouts with multiple slides per page are not supported
- **Slide notes**: Speaker notes (if exported) are not extracted
- **Non-English presentations**: Pattern matching is optimized for English presentation formats

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/classifier/misc/` (slide_deck samples: 24-30.pdf).

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
