# Bead bf-2ngt6: Source Document for Degraded OCR Fixture

## Task Completed
Research and identify source document for degraded OCR fixture

## Source Document Identified

**Document Title:** "Abraham Lincoln: The People's Leader in the Struggle for National Existence"

**Author:** George Haven Putnam (1909)

**Source:** Project Gutenberg eBook #11728

**Public Domain Status:** ✅ CONFIRMED
- Project Gutenberg states: "This eBook is for the use of anyone anywhere in the United States and most other parts of the world at no cost and with almost no restrictions whatsoever."
- Released: March 1, 2004
- Most recently updated: October 28, 2024
- Public domain in the USA

**Document URL:** https://www.gutenberg.org/ebooks/11728
**Direct Text Download:** https://www.gutenberg.org/cache/epub/11728/pg11728.txt

## Document Characteristics for OCR Testing

### Text Structure
- **Clear paragraph organization** with standard formatting
- **Multiple sections**: Introduction, 9 chapters, Appendix with Cooper Institute Address
- **Consistent structure** suitable for OCR evaluation

### Content Variety
- **Dates**: 1809-1865 era (e.g., "February 12, 1909", "April 12, 1861")
- **Names**: Historical figures (Lincoln, Douglas, Grant, Lee, etc.)
- **Numbers**: Statistics, counts, measurements
- **Places**: Geographic locations, cities, states
- **Vocabulary**: 19th-century American English with formal and narrative styles
- **Mixed formats**: Narrative text, speeches, correspondence

### Content Size
- **File size:** ~270KB of plain text
- **Word count:** Approximately 50,000+ words
- **Sufficient for comprehensive OCR testing**

## Acceptance Criteria Verification

### ✅ Source document saved in workspace
**Location:** `/home/coding/pdftract/tests/fixtures/scanned/low-quality/source-document-abraham-lincoln-public-domain.txt`

### ✅ Document confirmed public-domain
- Project Gutenberg public domain license
- No copyright restrictions in the USA
- Free to use, copy, and distribute

### ✅ Sufficient text content for OCR testing
- 270KB of structured text
- Varied content (dates, numbers, names, places)
- Clear paragraph and section structure
- Historical vocabulary provides good OCR challenge

## Additional Resources from Project Gutenberg

Also identified other public-domain government documents suitable for OCR testing:
1. **Government Documents in Small Libraries** (eBook #26551) - 38KB
2. **Messages and Papers of the Presidents** (eBook #14137) - 1.2MB
3. **Parks for the People** (eBook #26084) - Government publication

## Existing Fixture Note
Current low-quality fixture already exists:
- `degraded-200dpi.pdf` with timesheet document
- Ground truth: `degraded-200dpi-ground-truth.txt`
- Contains tables, numbers, dates suitable for OCR

The new source document provides **additional variety** for creating more degraded OCR test cases with:
- Longer-form narrative text
- Historical language patterns
- Speech and address formats
- Correspondence samples

## Sources
- Project Gutenberg: https://www.gutenberg.org/
- eBook #11728: https://www.gutenberg.org/ebooks/11728
- Plain text UTF-8: https://www.gutenberg.org/cache/epub/11728/pg11728.txt
