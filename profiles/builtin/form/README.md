# FORM Profile

Fillable form with fields; uses line_dominant reading order and form_fields from Phase 7.4

## Match Criteria Summary

This profile matches fillable forms and questionnaires. Documents typically contain:

- **Explicit form markers**: "Form 1234", "Application form", "Questionnaire", "Please fill out", "Required fields"
- **Field layout**: Repeated label-value pairs with colons or underscores (e.g., "Name: ______", "Date: __/__/__")
- **Blank input areas**: Lines, boxes, or underscored areas for user input

This is a degenerate profile with **no field extractors** — it only identifies documents as forms and relies on the `form_fields` integration from Phase 7.4 for field extraction. Forms are typically 1-10 pages.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| *(none)* | - | *This profile has no field extractors* | - | - |

## Known Limitations

*This section documents known edge cases and failure modes. Contributions to improve extraction quality are welcome.*

- **No field extraction**: This profile only classifies documents as forms; actual field extraction is handled by the `form_fields` integration (Phase 7.4), which must be run separately
- **Pre-filled forms**: Forms with already-filled handwritten or typed responses may confuse the classifier's field layout detection
- **Complex layouts**: Forms with non-standard layouts (e.g., grids, nested tables, multi-column designs) may not be recognized
- **Scanned forms**: Poor scan quality may cause field labels to be missed or misclassified
- **Non-English forms**: Pattern matching is optimized for English terminology like "form", "application", "questionnaire"

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/profiles/form/`.

*See the classifier corpus for representative documents.*

## Configuration Tips

To override this profile:

```bash
pdftract profiles export form > my-profile.yaml
# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-profile.yaml document.pdf
```

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
