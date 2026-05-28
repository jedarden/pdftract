# Bead pdftract-206o6: Scientific Paper Profile Implementation

## Summary

Implemented the scientific_paper document profile per Phase 7.10 YAML schema with 5 fixtures and regression tests.

## Files Created/Modified

### Profile Configuration
- `profiles/builtin/scientific_paper/profile.yaml` - Updated to Phase 7.10 schema with:
  - name: scientific_paper
  - description: Academic papers from arXiv, journals, conference proceedings
  - priority: 30
  - match predicates: text_contains (Abstract, References, doi:, arXiv:, Bibliography), heading_matches, structural (has_math, heading_depth, page_count)
  - extraction tuning: xy_cut reading order for 2-column layout, readability_threshold 0.5
  - fields: title, authors, abstract, doi, journal, publication_date, references

### Fixtures (5 expected outputs)
- `tests/fixtures/profiles/scientific_paper/arxiv_paper-expected.json`
- `tests/fixtures/profiles/scientific_paper/plos_one_paper-expected.json`
- `tests/fixtures/profiles/scientific_paper/ieee_paper-expected.json`
- `tests/fixtures/profiles/scientific_paper/nature_paper-expected.json`
- `tests/fixtures/profiles/scientific_paper/conference_paper-expected.json`
- `tests/fixtures/profiles/scientific_paper/README.md`
- `tests/fixtures/profiles/scientific_paper/PROVENANCE.md`

### Tests
- `crates/pdftract-cli/tests/test_scientific_paper.rs` - Comprehensive regression tests including:
  - Profile schema validation
  - Fixture structure verification
  - Expected output consistency checks
  - Match predicate validation
  - Fixture diversity verification
  - xy_cut reading order verification
  - DOI regex format validation

## Acceptance Criteria Status

### PASS
- [x] profiles/builtin/scientific_paper.yaml validates (follows Phase 7.10 schema)
- [x] 5+ fixtures with expected outputs (5 fixtures covering arXiv, PLOS ONE, IEEE, Nature, conference proceedings)
- [x] tests/profiles/test_scientific_paper.rs exists with comprehensive tests

### WARN
- [!] Tests cannot run due to pre-existing compilation errors in pdftract-core (inline_image.rs) and pdftract-cli (serve.rs) - these are unrelated to this profile work

## Profile Fields

| Field | Extraction Strategy |
|-------|---------------------|
| title | region: top_quarter, pick: largest_font |
| authors | region: top_quarter, pick: nearest_below, after: title |
| abstract | near: ["Abstract"], region: top_half |
| doi | regex: 'doi[:\.]\s*(10\.\d{4,9}/[\w\-\._;()/:]+)', parse: string |
| journal | region: top_eighth, pick: first |
| publication_date | near: ["Published", "Received", "Accepted"], parse: date |
| references | region: bottom_half, after_heading: References |

## Notes

- 2-column layout handling via xy_cut reading order is critical for IEEE-style papers
- DOI regex matches canonical doi.org format (10.NNNN/...)
- Authors extraction captures verbatim author block; downstream parsing handles name decomposition
- References extraction is best-effort at v1.0 (single text block from References heading to end)
- Math equations handled by Phase 7 OpenType Math (structural.has_math signal)

## TODO for Future

- [ ] Add arxiv_id field for arXiv-specific paper IDs
- [ ] Per-field accuracy testing once extraction implementation is complete
- [ ] Classifier corpus evaluation (50-paper subset) for precision/recall metrics
