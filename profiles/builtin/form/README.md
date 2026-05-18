# FORM Profile

Fillable form with fields; uses line_dominant reading order and form_fields from Phase 7.4

## Match Criteria Summary

A document matches this profile when it exhibits the structure of a fillable form or questionnaire. The classifier identifies form-specific terminology like "form" with alphanumeric identifiers, "application form", "questionnaire", and instructions like "please fill out" or "required fields". Structurally, forms are recognized by their field layout (labels followed by blank spaces or boxes) and the presence of colon-terminated field labels. Forms typically range from 1-10 pages and may include checkboxes, radio buttons, and lined or boxed areas for handwritten responses. This profile is a degenerate case: it has no profile field extractors, instead relying on the form_fields extraction from Phase 7.4.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| *(none)* | - | *This profile has no field extractors* | - | - |

**Note:** This profile does not define extracted fields in `profile_fields`. Instead, it uses `form_fields_integration: true` to leverage the generic form field extraction from Phase 7.4. Field names and values are extracted dynamically based on the form's layout (label-value pairs, checkboxes, etc.).

## Known Limitations

- Form field extraction depends on clear label-value relationships; poorly aligned forms may fail
- Handwritten responses are not transcribed; only field labels and pre-filled values are captured
- Forms with complex layouts (nested sections, conditional fields) may extract fields incorrectly
- Forms without colons or clear field delimiters may not be recognized as forms
- Multi-page forms with page continuations may have broken field extraction across page boundaries
- Checkboxes and radio buttons are detected but their checked/unchecked state may not be reliable
- Forms with tables or grids for data entry may not extract individual cell values correctly
- Non-English forms may not match due to English-only text patterns in match criteria

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

For specific form types (e.g., tax forms, government applications), consider creating a dedicated profile with form-specific `profile_fields` instead of using this generic form profile. The form_fields integration can be combined with custom field extractors for hybrid approaches.

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
