# Document Model Test Fixtures

This directory contains curated PDF fixtures for testing the document model integration.

## Fixture Passwords

**IMPORTANT:** The passwords for encrypted fixtures are NOT secret. They are test fixtures:

- `encrypted_rc4_test.pdf`: RC4-40, password "test"
- `encrypted_aes128_test.pdf`: AES-128, password "test"
- `encrypted_aes256_test.pdf`: AES-256 (PDF 2.0), password "test"
- `encrypted_empty_password.pdf`: RC4-40, empty password

## Fixture List

### Encrypted Files (EC-04, EC-05, EC-06)

- `encrypted_rc4_test.pdf` — RC4-encrypted, user password "test" (EC-04)
- `encrypted_aes128_test.pdf` — AES-128, password "test" (EC-05)
- `encrypted_aes256_test.pdf` — AES-256 (PDF 2.0), password "test" (EC-06)
- `encrypted_empty_password.pdf` — RC4-encrypted, empty owner password
- `encrypted_unknown_handler.pdf` — Custom handler (Adobe Public Key, /Filter /Adobe.PubSec)

### Tagged PDFs

- `tagged_3_level_outline.pdf` — 3 levels of bookmarks with mixed UTF-16BE/PDFDocEncoded titles

### Optional Content (EC-16)

- `ocg_default_off.pdf` — Single OCG with /D /BaseState /OFF (EC-16)

### Multi-Revision

- `multi_revision_3.pdf` — 3 incremental revisions, page count differs across revisions

### Page Tree Inheritance (EC-09)

- `inheritance_grandparent_mediabox.pdf` — page 0 has no MediaBox; inherits from grandparent /Pages node
- `missing_mediabox.pdf` — page with no MediaBox anywhere (EC-09)

### Resource Merging

- `partial_resource_override.pdf` — page overrides /Resources /Font partially; merged result expected

### JavaScript Detection

- `js_in_openaction.pdf` — /OpenAction /S /JavaScript

### XFA Forms

- `xfa_form.pdf` — /AcroForm /XFA present

### Conformance Detection

- `pdfa_1b_conformance.pdf` — XMP metadata declaring PDF/A-1B conformance

### Page Labels

- `page_labels_roman_arabic.pdf` — pages 0..3 roman, pages 4..end arabic

## Fixture Generation

Fixtures are generated using `qpdf` and hand-crafted PDF construction.

See `scripts/generate_document_model_fixtures.sh` for generation scripts.
