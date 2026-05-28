# Scientific Paper Profile Fixtures

This directory contains test fixtures for the scientific paper document profile.

## Fixture Types

1. **arxiv_paper** - arXiv preprint with CC-BY license, typical academic structure with Abstract, Introduction, Methods, Results, Discussion, References
2. **plos_one_paper** - PLOS ONE journal article with DOI, open access formatting, single-column layout
3. **ieee_paper** - IEEE-style 2-column journal article with mathematical equations, numbered references
4. **nature_paper** - Nature-style single-column article with sidebar layout, Received/Accepted dates
5. **conference_paper** - ACM/IEEE conference proceedings with DOI, author affiliations, structured references

## Expected Output Format

Each fixture should have a corresponding `*-expected.json` file with the following structure:

```json
{
  "metadata": {
    "document_type": "scientific_paper",
    "document_type_confidence": 0.XX,
    "document_type_reasons": [...],
    "profile_name": "scientific_paper",
    "profile_version": "1.0.0",
    "profile_fields": {
      "title": "...",
      "authors": ["..."],
      "abstract": "...",
      "doi": "...",
      "journal": "...",
      "publication_date": "YYYY-MM-DD",
      "references": "..."
    }
  }
}
```

## Profile Fields

The scientific paper profile extracts the following fields:

- **title**: Paper title (region: top_quarter, pick: largest_font)
- **authors**: Author list (region: top_quarter, pick: nearest_below)
- **abstract**: Abstract text (near: "Abstract", region: top_half)
- **doi**: Digital Object Identifier (regex match)
- **journal**: Journal or publication name (region: top_eighth)
- **publication_date**: Publication date (near: "Published", "Received", "Accepted")
- **references**: References section (region: bottom_half, after "References" heading)

## Provenance

All fixtures should be sourced from publicly available academic papers with appropriate licenses or created synthetically with clear provenance documentation. See PROVENANCE.md for details on each fixture.

## TODO

- [ ] Acquire or create PDF files for each fixture type
- [ ] Validate extraction accuracy against expected outputs
- [ ] Document extraction limitations (e.g., 3-column layouts, unusual author formats)
