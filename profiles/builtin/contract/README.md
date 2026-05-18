# CONTRACT Profile

Legal contract with parties, effective date, term, signatures

## Match Criteria Summary

A document matches this profile when it exhibits the formal structure and language of a legal agreement. The classifier identifies contract-specific terminology such as "agreement is made", "terms and conditions", "effective date", "governing law", and "indemnification". Structurally, contracts are multi-page documents (typically 2-50 pages) with signature blocks in the final pages. The presence of defined legal language patterns combined with signature block detection is the strongest matching signal. Contracts often use formal legal language and may include recitals, numbered sections, and definitions sections.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| parties | array | Extracted from page text using pattern matching | [...] | regex patterns |
| effective_date | date | Extracted from page text using pattern matching | 2024-01-15 | regex patterns |
| term | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| governing_law | string | Extracted from page text using pattern matching | "example value" | regex patterns |
| signatures | array | Extracted from page text using pattern matching | [...] | region: bottom_20_percent |

## Known Limitations

- Contracts with more than two parties may not extract all parties correctly
- Signature extraction depends on clear text signatures; typed signatures are extracted but handwritten signatures are not OCR'd
- Complex contract structures (e.g., exhibits, appendices) may not be fully captured
- Contracts with amendments or riders attached may extract only the primary agreement
- Non-English contracts may not match due to English-only text patterns
- Contracts with scanned signatures (images) will not extract signature names
- Term extraction may fail for contracts with complex duration formulas (e.g., "until completion of services")
- Governing law extraction may capture jurisdiction incorrectly for federal/international agreements

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/profiles/contract/`.

*See the classifier corpus for representative documents.*

## Configuration Tips

To override this profile:

```bash
pdftract profiles export contract > my-profile.yaml
# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-profile.yaml document.pdf
```

For specific contract types (e.g., NDAs, employment agreements), consider adding contract-type-specific text patterns to improve matching. For international contracts, add region-specific governing law patterns.

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
