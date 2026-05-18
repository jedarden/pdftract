# SCIENTIFIC_PAPER Profile

Academic paper with title, authors, abstract, DOI, references

## Match Criteria Summary

Documents matching this profile typically contain:

- **Strong text signals**: Words like "abstract", "introduction", "keywords:", "doi 10.", "references", "bibliography", "acknowledgments"
- **Structural signals**: Two-column layout (common in academic papers), bibliography section at end
- **Page count**: Usually 4-30 pages (academic papers have length constraints)
- **Layout patterns**: Title centered at top, authors below, abstract early, numbered sections, references at end

The classifier looks for academic paper terminology combined with two-column layout. Papers with "abstract" AND "references" AND two-column layout match with highest confidence.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| title | string | Paper title | "Machine Learning for Protein Folding" | First page, top, large font |
| authors | array | Author names | `["J. Smith", "A. Jones", "et al."]` | First page, below title |
| abstract | string | Abstract text | "We present a novel approach..." | After "abstract" heading |
| doi | string | Digital Object Identifier | "10.1234/example.5678" | "doi:" pattern or URL |
| journal | string | Journal name | "Nature" | "published in", "journal", or "proceedings" fields |
| publication_date | date | Publication date | 2024-01-15 | "received", "accepted", "published", or copyright date |
| references | array | Bibliographic references | `["[1] Smith et al..."]` | After "references" heading, numbered list |

## Known Limitations

- **DOI location**: Only DOIs on the first page are extracted; DOIs in footnotes or headers may be missed
- **Multi-page abstracts**: Abstracts spanning multiple columns or pages may be truncated
- **Complex author lists**: Papers with dozens of authors (e.g., high-energy physics) may truncate or miss some authors
- **Non-standard layouts**: Single-column journals or arXiv preprints may not match two-column heuristics
- **References**: Only numbered reference formats ([1], [2]) are detected; author-year formats may be missed
- **Supplementary materials**: Supplementary sections are not distinguished from main content
- **Non-English papers**: Papers in languages other than English may not match pattern lists
- **Hybrid layouts**: Papers with mixed one- and two-column sections may confuse the column-aware reading order
- **Figure captions**: Captions are extracted as body text; no separate figure extraction is performed

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/classifier/scific_paper/`.

The corpus includes 50 scientific paper documents covering various journals and layouts.

## Configuration Tips

To override this profile for custom scientific paper formats:

```bash
pdftract profiles export scientific_paper > my-paper.yaml
# Edit my-paper.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-paper.yaml document.pdf
```

Common customizations:
- Add field-specific DOI patterns to `doi.extraction.patterns`
- For author-year reference formats, update `references.extraction.patterns`
- Adjust `reading_order` for single-column journals: change `column_aware` to `line_dominant`

---

*This README documents the built-in `scientific_paper` profile. See `docs/research/document-classification-and-zone-labeling.md` for classifier theory.*
