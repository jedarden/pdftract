# CONTRACT Profile

Legal contract with parties, effective date, term, signatures

## Match Criteria Summary

This profile matches legal contracts and agreements. Documents typically contain:

- **Contract language**: "Agreement is made", "Contract agreement", "Terms and conditions", "Memorandum of understanding"
- **Legal boilerplate**: "Effective date", "Governing law", "Termination notice", "Indemnification"
- **Signature blocks**: Signatories at the bottom of pages (usually last page)
- **Multi-page structure**: Contracts are almost always 2+ pages

The profile expects formal legal language and signature blocks. It works for NDAs, employment agreements, service contracts, and MOUs.

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| parties | array | Contract parties (vendor/client, employer/employee) | ["Acme Corp Inc.", "John Smith"] | regex patterns |
| effective_date | date | Date when the contract becomes effective | 2024-01-15 | regex patterns |
| term | string | Duration of the contract (months or years) | "24 months" | regex patterns |
| governing_law | string | Jurisdiction governing the contract | "California" | regex patterns |
| signatures | array | Signatory names from signature blocks | ["Jane Doe", "Bob Johnson"] | regex patterns, region: bottom_20_percent |

## Known Limitations

- **Complex party structures**: Only extracts parties explicitly named in "Between X and Y" or "Party X:" format; complex corporate hierarchies may be missed
- **Multi-party agreements**: Only captures the first two parties; additional parties are not extracted
- **Amendments/addenda**: Treated as separate documents; cross-references between documents are not resolved
- **Handwritten signatures**: Signature blocks are extracted by pattern only; handwritten signatures are not validated
- **International formats**: Non-US date formats (DD/MM/YYYY) may parse incorrectly
- **Exhibits and schedules**: Attached exhibits are not analyzed; only the main agreement text is processed
- **Scanned contracts**: Poor-quality scans of signed contracts may have illegible signature text

## Sample Input

Example fixtures demonstrating this profile are available in `tests/fixtures/classifier/contract/` (50+ representative contracts).

*See the classifier corpus for representative documents.*

## Configuration Tips

To override this profile:

```bash
pdftract profiles export contract > my-profile.yaml
# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns
pdftract extract --profile my-profile.yaml document.pdf
```

---

*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*
