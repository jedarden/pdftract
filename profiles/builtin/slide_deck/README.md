# SLIDE_DECK Profile

Presentation slides with title, presenter, date, slide titles

## Match Criteria Summary

Documents matching this profile typically contain:

- **Strong text signals**: Words like "presentation", "slide N", "table of contents"
- **Structural signals**: Landscape page orientation (width > height), 3+ pages, large centered text blocks (titles)
- **Page count**: Usually 3-200 pages (presentations vary widely)
- **Layout patterns**: Title slide with centered text, subsequent slides with headings at top or left, content below

The classifier looks for presentation-specific terminology combined with landscape orientation and multiple pages. Landscape documents with 3+ pages AND large centered text match with highest confidence.

**Note**: Slide deck PDFs vary enormously in quality depending on the export method (PowerPoint "Save as PDF", Keynote export, Google Slides PDF, etc.). Extraction quality depends heavily on the exporter's text rendering.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| title | string | Presentation title | "Q4 2024 Business Review" | First slide, centered, large font |
| presenter | string | Presenter name(s) | "Jane Doe, CEO" | First slide, below title |
| date | string | Presentation date | 2024-01-15 | First slide, bottom area |
| slide_titles | array | Title of each slide | `["Overview", "Financial Results", "Q&A"]` | Per-slide extraction, top or center |

## Known Limitations

- **Export quality**: Poor PDF exports (e.g., from old PowerPoint versions) may have garbled text or text-as-images
- **Image-only slides**: Slides that are pure images (e.g., photos, screenshots) have no extractable text
- **Complex layouts**: Slides with multiple text boxes, overlapping elements, or non-standard layouts may not extract correctly
- **Animations and transitions**: PDF exports capture final state only; animated builds are not represented
- **Non-title slides**: Slides without clear titles (e.g., photo-only slides) will have null entries in `slide_titles`
- **Speaker notes**: Speaker notes are typically not included in PDF exports and cannot be extracted
- **Non-English presentations**: Presentations in other languages may not match pattern lists
- **Handout formats**: 3-up or 6-up handout PDFs (multiple slides per page) are not supported
- **Portrait slides**: Rare portrait-orientation slides may not be detected correctly

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/classifier/misc/`.

Slide deck fixtures are typically landscape PDFs with clear title slides and multiple pages.

## Configuration Tips

To override this profile for custom slide deck formats:

```bash
pdftract profiles export slide_deck > my-slides.yaml
# Edit my-slides.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-slides.yaml document.pdf
```

Common customizations:
- Adjust `slide_titles.extraction.per_page` to false if you only want the first slide's title
- For handout formats (multiple slides per page), consider creating a custom profile with different extraction logic
- Add presenter-specific patterns if your organization uses consistent presenter naming conventions

---

*This README documents the built-in `slide_deck` profile. See `docs/research/document-classification-and-zone-labeling.md` for classifier theory.*
