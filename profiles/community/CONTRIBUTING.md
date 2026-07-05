# Contributing Document Profiles

Thank you for your interest in contributing document profiles to pdftract! Community profiles help make pdftract more useful for everyone by supporting specialized document types and industry-specific formats.

This guide covers how to create, test, and submit document profiles for inclusion in the pdftract community profile collection.

## Overview

Document profiles (pdftract) define how to recognize and extract structured data from specific types of documents (invoices, receipts, contracts, scientific papers, etc.). Each profile specifies:

- **Match criteria**: How to recognize this document type
- **Extraction strategy**: How to read and parse the document content
- **Field definitions**: What structured data to extract (dates, amounts, line items, etc.)

Community profiles are stored in `profiles/community/` and can be used by anyone with pdftract.

## Quick Start

If you're new to profile authoring, start with an existing profile:

```bash
# Export a built-in profile to use as a template
pdftract profiles export invoice > my-profile.yaml

# Edit it to match your document type
vim my-profile.yaml

# Validate your profile
pdftract profiles validate my-profile.yaml

# Test it on a sample document
pdftract extract --profile my-profile sample.pdf
```

## File Naming Convention

Profile directories in `profiles/community/` should follow these conventions:

### Directory Structure

Each profile should be organized as a directory:

```
profiles/community/
├── .gitkeep                    # Placeholder file (empty)
├── CONTRIBUTING.md             # This file
└── <profile-name>/
    ├── profile.yaml            # Required: Profile definition
    ├── README.md               # Required: Profile documentation
    └── fixtures/               # Optional: Test documents
        ├── sample-01.pdf
        └── sample-02.pdf
```

### Profile Name Requirements

- **Use lowercase, hyphen-separated names**: `vendor-invoice`, `w2-form`, `packing-slip`
- **Be specific**: If a profile is for a specific region or industry, include it in the name
  - ✅ `eu-invoice-vat`, `medical-billing-statement`
  - ❌ `invoice`, `statement`
- **Avoid trademarked terms**: Don't use company names or product names in profile names
  - ✅ `saas-subscription-invoice`
  - ❌ `salesforce-invoice`, `aws-invoice`

### File Format

- **profile.yaml**: YAML format (see [Profile Authoring Guide](../../../docs/research/profile-authoring.md))
- **README.md**: Markdown format with specific sections (see below)
- **fixtures/**: Sample PDFs for testing (optional but recommended)

## Required Profile Fields

Every `profile.yaml` must include these top-level fields:

```yaml
name: profile-name              # Unique identifier (lowercase, hyphens)
description: One-line summary   # What this profile extracts
priority: 50                    # 1-100, higher = more specific

match:                          # Document recognition rules
  all:
    - ...                       # All conditions must match
  any:
    - ...                       # At least one must match
  none:
    - ...                       # None of these may match

extraction:                     # Extraction strategy
  reading_order: line_dominant
  table_detection: strict_borders
  # ... (see docs for full schema)

fields:                         # Fields to extract
  field_name:
    type: string|date|decimal|int|bool|array
    extraction:
      # ... extraction strategy
```

### Schema Requirements

- **name**: Must match directory name
- **description**: Clear, concise description (50-100 characters)
- **priority**: 1-100, where higher values indicate more specific profiles
  - 10-30: Generic/broad profiles (e.g., `document`)
  - 40-60: Standard document types (e.g., `invoice`, `receipt`)
  - 70-100: Highly specific profiles (e.g., `w2-tax-form-2024`)
- **match**: Must define at least one condition
- **fields**: Must define at least one field

### Validation Rules

Profiles are automatically validated on submission. Common validation failures:

- ❌ YAML syntax errors (indentation, unescaped characters)
- ❌ Missing required fields (name, description, priority, match, fields)
- ❌ Forbidden keys (api_key, password, token, secret, credential)
- ❌ Invalid field type combinations (type: decimal with parse: bool)
- ❌ Malformed regex patterns or YAML structure

See [Profile Authoring Guide](../../../docs/research/profile-authoring.md) for common YAML pitfalls.

## Local Testing Workflow

Before submitting a profile, test it thoroughly:

### 1. Validate the Profile

```bash
pdftract profiles validate profiles/community/<profile-name>/profile.yaml
```

Fix any validation errors before proceeding.

### 2. Test Against Sample Documents

```bash
# Test extraction on a sample document
pdftract extract --profile profiles/community/<profile-name>/profile.yaml sample.pdf

# Inspect the output
pdftract extract --profile profiles/community/<profile-name>/profile.yaml sample.pdf | jq .
```

### 3. Test Edge Cases

Test your profile against:
- ✅ Documents that should match (positive cases)
- ❌ Documents that should NOT match (negative cases)
- 📄 Variations (different layouts, regions, fonts)
- 🚫 Edge cases (missing fields, corrupted pages, blank pages)

### 4. Performance Check

Ensure extraction completes in reasonable time:
```bash
time pdftract extract --profile profiles/community/<profile-name>/profile.yaml sample.pdf
```

Typical targets:
- Simple profiles: < 2 seconds per page
- Complex profiles (tables, multi-page): < 5 seconds per page

## README.md Template

Every profile should include a `README.md` with these sections:

```markdown
# <Profile Name> Profile

<One-line description>

## Match Criteria Summary

A document matches this profile when... (describe the key indicators)

## Extracted Fields

| Field | Type | Description | Example Value | Source Hint |
|-------|------|-------------|----------------|-------------|
| field1 | string | Description | "example" | regex/near/region |
| field2 | decimal | Description | 123.45 | table: largest_table |

## Known Limitations

- Limitation 1
- Limitation 2

## Sample Input

Example fixtures demonstrating this profile are available in `fixtures/`.

## Configuration Tips

(Optional) Tips for customizing or overriding this profile.

---

*This README was auto-generated from `profile.yaml`.*
```

See [invoice README](../../../profiles/builtin/invoice/README.md) for a complete example.

## PR Submission Process

### 1. Fork and Branch

```bash
# Fork the repository and create a feature branch
git checkout -b add-profile-<profile-name>
```

### 2. Add Your Profile

```bash
# Create profile directory
mkdir -p profiles/community/<profile-name>

# Add profile.yaml and README.md
# ... (edit files)

# Commit
git add profiles/community/<profile-name>
git commit -m "Add profile: <profile-name>"
```

### 3. Submit Pull Request

Submit a PR to the main repository with:
- **Title**: `Add profile: <profile-name>`
- **Description**: Brief summary of what the profile extracts
- **Linked issues**: Reference any related issues or discussions

### 4. Review Process

Maintainers will review your profile for:
- ✅ Functional correctness (extraction accuracy)
- ✅ YAML validity and schema compliance
- ✅ Documentation completeness
- ✅ Edge case handling
- ✅ Performance characteristics

Review typically takes 1-3 business days.

## License Requirements for Fixtures

If you include sample PDFs in `fixtures/`, ensure:

- **Public domain or permissive license**: PDFs must be usable under Apache-2.0 or MIT
- **No personal information**: Remove or redact PII (names, addresses, account numbers)
- **No confidential data**: Don't include real business data, trade secrets, or proprietary information
- **Synthetic data preferred**: Use generated/fabricated sample documents where possible

### Redaction Examples

❌ **Don't include unredacted documents**:
```
Invoice #: INV-2024-001
Customer: John Doe
Account: 1234-5678-9012
```

✅ **Use synthetic/redacted data**:
```
Invoice #: INV-DEMO-001
Customer: [REDACTED]
Account: XXXX-XXXX-XXXX
```

## Review Checklist

Before submitting, confirm:

- [ ] **Validation passes**: `pdftract profiles validate` returns success
- [ ] **README.md is complete**: Includes all required sections
- [ ] **Field documentation**: Every field has a description and example
- [ ] **Positive cases work**: Profile extracts correctly from intended document types
- [ ] **Negative cases reject**: Profile doesn't match unrelated documents
- [ ] **Edge cases handled**: Gracefully handles missing/empty/malformed fields
- [ ] **Performance acceptable**: Extraction completes in reasonable time
- [ ] **Fixtures licensed**: Sample PDFs use permissive licenses or are synthetic
- [ ] **No secrets**: Profile contains no API keys, passwords, or credentials
- [ ] **Name follows conventions**: Lowercase, hyphen-separated, non-trademarked

## Profile Quality Standards

Profiles should meet these quality standards:

### Accuracy
- Extract correct values in 95%+ of test cases
- Handle common layout variations (centered, left-aligned, multi-column)
- Gracefully degrade when fields are missing

### Robustness
- Don't crash on malformed input
- Provide sensible defaults when extraction fails
- Log warnings for ambiguous cases

### Clarity
- Profile name clearly indicates purpose
- Field names are self-documenting
- README explains match criteria and limitations

### Performance
- No O(n²) or worse algorithms in match logic
- Reasonable memory usage (< 100MB per page)
- Fast enough for batch processing

## Getting Help

- **Documentation**: [Profile Authoring Guide](../../../docs/research/profile-authoring.md)
- **Examples**: [Built-in profiles](../../../profiles/builtin/)
- **Issues**: Open a GitHub issue for questions or problems
- **Discussions**: Use GitHub Discussions for design feedback

## Profile Maintenance

Profile authors are encouraged to:
- Respond to feedback during review
- Fix bugs reported by users
- Update profiles for new pdftract releases
- Add test fixtures for edge cases

Maintainers may update profiles for compatibility or correctness.

---

Thank you for contributing to pdftract! 🎉
