# SCIENTIFIC_PAPER Profile

Academic paper with title, authors, abstract, DOI, references

## Match Criteria Summary

A document matches this profile when it displays the characteristic structure of an academic or scientific paper. The classifier identifies section headings like "abstract", "introduction", "keywords", and "references". Structurally, scientific papers are recognized by their two-column layout (common in journal publications) and the presence of a bibliography or references section. DOI identifiers are strong matching signals when present. Page counts typically range from 4-30 pages for conference papers and journal articles. The combination of author affiliations, abstract text, and structured sections distinguishes this profile from other document types.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| title | string | Extracted from page text using pattern matching | "example value" | regex patterns, region: first_page_top |
| authors | array | Extracted from page text using pattern matching | [...] | regex patterns, region: first_page_top_below_title |
| abstract | string | Extracted from page text using pattern matching | "example value" | regex patterns, region: after_abstract_heading |
| doi | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| journal | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| publication_date | date | Extracted from page text using pattern matching | 2024-01-15 | regex patterns |
| references | array | Extracted from page text using pattern matching | [...] | region: after_references_heading |

## Known Limitations

- DOIs in footnotes or page headers are not extracted; only first-page DOIs are picked up
- Papers with non-standard author formats (e.g., very long author lists, "et al." handling) may truncate author lists
- Abstract extraction may include section heading text if abstract boundaries are ambiguous
- Two-column layout detection may fail for single-column format papers (e.g., some arXiv preprints)
- References extraction captures numbered citations but may not handle unstructured reference formats
- Non-English papers may not match due to English-only section heading patterns
- Papers with complex figure/table layouts interrupting text flow may have extraction errors
- Conference proceedings vs. journal distinctions are not made; both match this profile

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/profiles/scientific_paper/`.

*See the classifier corpus for representative documents.*

## Configuration Tips

To override this profile:

```bash
pdftract profiles export scientific_paper > my-profile.yaml
# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-profile.yaml document.pdf
```

For papers from specific venues (e.g., ACM, IEEE), consider adding venue-specific patterns to the `journal` field extraction. For preprints or conference papers, you may want to adjust the `match.structural` signals to not require two-column layout.

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
