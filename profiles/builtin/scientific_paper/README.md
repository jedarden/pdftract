# SCIENTIFIC_PAPER Profile

Academic paper with title, authors, abstract, DOI, references

## Match Criteria Summary

This profile matches academic papers, journal articles, and conference proceedings. Documents typically contain:

- **Section headings**: "Abstract", "Introduction", "Keywords:"
- **Bibliography markers**: "References", "Bibliography", "Acknowledgments"
- **Two-column layout**: Most academic papers use two-column formatting
- **Metadata patterns**: DOI numbers (10.xxxx/...), copyright notices, journal names

Papers are typically 4-30 pages. The profile expects standard academic formatting with sections and citations.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| title | string | Full title of the paper | "A Novel Approach to Machine Learning" | regex patterns, region: first_page_top |
| authors | array | List of author names | ["Jane Doe", "John Smith"] | regex patterns, region: first_page_top_below_title |
| abstract | string | Abstract paragraph text | "This paper presents a novel method..." | regex patterns, region: after_abstract_heading |
| doi | string | Digital Object Identifier | "10.1234/example.2024.001" | regex patterns |
| journal | string | Name of the journal or conference | "Journal of Computer Science" | regex patterns |
| publication_date | date | Publication or copyright date | 2024-01-15 | regex patterns |
| references | array | Bibliography entries | ["[1] Author et al., Title..."] | regex patterns, region: after_references_heading |

## Known Limitations

- **DOIs in footnotes**: Only first-page DOIs are picked up; DOIs in footnotes or first-page footers are not extracted
- **Multi-page abstracts**: Abstract extraction stops at double newline or "Keywords"; multi-paragraph abstracts are truncated
- **Complex author lists**: "et al." is captured literally; full author lists with affiliations are not parsed
- **Reference parsing**: Only captures bracketed references ([1], [2]); numbered formats without brackets are missed
- **Single-column papers**: Papers without two-column layout may still match but extraction quality is lower
- **Non-English papers**: Pattern matching is optimized for English section headings
- **Supplementary materials**: Attached supplementary data files are not analyzed
- **ArXiv preprints**: Preprints without journal metadata may have incomplete extraction

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/classifier/scientific_paper/` (50+ representative papers).

*See the classifier corpus for representative documents.*

## Configuration Tips

To override this profile:

```bash
pdftract profiles export scientific_paper > my-profile.yaml
# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-profile.yaml document.pdf
```

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
