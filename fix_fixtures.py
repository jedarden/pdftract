#!/usr/bin/env python3
"""Fix malformed PDF fixtures with incorrect startxref offsets."""
import re
import subprocess

fixtures = [
    "tests/document_model/fixtures/ocg_default_off.pdf",
    "tests/document_model/fixtures/tagged_3_level_outline.pdf",
    "tests/document_model/fixtures/multi_revision_3.pdf",
    "tests/document_model/fixtures/inheritance_grandparent_mediabox.pdf",
    "tests/document_model/fixtures/missing_mediabox.pdf",
    "tests/document_model/fixtures/partial_resource_override.pdf",
    "tests/document_model/fixtures/js_in_openaction.pdf",
    "tests/document_model/fixtures/xfa_form.pdf",
    "tests/document_model/fixtures/pdfa_1b_conformance.pdf",
    "tests/document_model/fixtures/page_labels_roman_arabic.pdf",
    "tests/document_model/fixtures/encrypted_unknown_handler.pdf",
]

for fixture_path in fixtures:
    try:
        # Read the file
        with open(fixture_path, 'rb') as f:
            data = f.read()

        # Find the first "xref" (the correct one)
        xref_match = re.search(b'xref\n', data)
        if not xref_match:
            print(f"Skipping {fixture_path}: no xref found")
            continue

        correct_offset = xref_match.start()

        # Fix the startxref value
        new_data = re.sub(rb'startxref\n\d+', f'startxref\n{correct_offset}'.encode(), data)

        # Write back
        with open(fixture_path, 'wb') as f:
            f.write(new_data)

        print(f"Fixed {fixture_path}: startxref now points to {correct_offset}")
    except Exception as e:
        print(f"Error fixing {fixture_path}: {e}")
