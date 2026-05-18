# FORM Profile

Fillable form with fields; uses line_dominant reading order and form_fields from Phase 7.4

## Match Criteria Summary

Documents matching this profile typically contain:

- **Strong text signals**: Words like "form 1099", "application form", "questionnaire", "please fill out", "required fields"
- **Structural signals**: Form field layout (blanks, checkboxes, labeled input areas), blank lines with colons
- **Page count**: Usually 1-10 pages (forms are typically concise)
- **Layout patterns**: Labels followed by blanks/underlines, checkboxes, signature blocks, structured fields

The classifier looks for form-specific terminology combined with field layout patterns. Documents with "form" terminology AND blank fields match with highest confidence.

**Note**: This is a degenerate profile with **no field extractors**. It uses `line_dominant` reading order and surfaces all `form_fields` from Phase 7.4. The profile enables form-specific processing but does not extract named fields like other profiles.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| *(none)* | - | *This profile has no field extractors* | - | - |

Instead of named fields, this profile integrates with Phase 7.4's `form_fields` system, which extracts:
- Text input fields (labels + values)
- Checkbox/radio button states
- Signature blocks
- Date fields
- Multi-line text areas

See Phase 7.4 documentation for the `form_fields` schema.

## Known Limitations

- **No named extraction**: Unlike other profiles, this does not return named fields; users must process `form_fields` output
- **Handwritten forms**: Handwritten responses may not be OCRed correctly
- **Complex layouts**: Forms with non-standard layouts (e.g., grids, nested sections) may confuse field detection
- **Checkboxes and radio buttons**: Checkbox states may be unreliable depending on PDF encoding
- **Multi-page forms**: Fields spanning page boundaries may be split incorrectly
- **Non-English forms**: Forms in other languages may not match pattern lists
- **Scanned forms**: Poor scan quality can lead to missed fields or incorrect label-value pairing
- **Dynamic forms**: Forms with conditional fields (e.g., "if yes, go to section B") are not interpreted

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/classifier/misc/`.

Form fixtures are typically single-page documents with labeled fields and blanks for user input.

## Configuration Tips

To override this profile for custom form formats:

```bash
pdftract profiles export form > my-form.yaml
# Edit my-form.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-form.yaml document.pdf
```

Common customizations:
- Add form-specific patterns to `match.text_patterns` for proprietary form types
- If you need named field extraction, copy this profile and add `profile_fields` entries
- For government forms (e.g., IRS, USCIS), create specific profiles with known field mappings

**Integration with Phase 7.4**: This profile sets `form_fields_integration: true`, which enables the form field extraction pipeline. The extracted `form_fields` array is included in the output JSON.

---

*This README documents the built-in `form` profile. See `docs/research/document-classification-and-zone-labeling.md` for classifier theory and Phase 7.4 for `form_fields` schema.*
