# JSON Schema Validation Test Fixtures

This directory contains PDF files used for JSON schema validation testing
per bead pdftract-3jm4n (Phase 6.1.4).

Each PDF in this directory is extracted and validated against the published
JSON schema at `docs/schema/v1.0/pdftract.schema.json`. Any validation error
fails CI.

## Adding Fixtures

To add a new fixture:
1. Place the PDF file in this directory
2. Name it descriptively (e.g., `simple-text.pdf`, `multi-page.pdf`)
3. The test harness will automatically pick it up

## Current Fixtures

- `simple-text.pdf` - Minimal text-only PDF for basic validation
