#!/usr/bin/env python3
"""Check fixture content streams."""
import pikepdf
import sys

if len(sys.argv) > 1:
    path = sys.argv[1]
    pdf = pikepdf.open(path)
    page = pdf.pages[0]
    content = page.Contents.get_stream_bytes() if hasattr(page.Contents, 'get_stream_bytes') else page.Contents.getbytes()
    print(f"{path}: {content}")
else:
    # Check both content edit fixtures
    paths = [
        'tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf',
        'tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf',
        'tests/fingerprint/fixtures/content_edit_one_paragraph/v1.pdf',
        'tests/fingerprint/fixtures/content_edit_one_paragraph/v2.pdf',
    ]

    for path in paths:
        try:
            pdf = pikepdf.open(path)
            page = pdf.pages[0]
            if hasattr(page.Contents, 'get_stream_bytes'):
                content = page.Contents.get_stream_bytes()
            else:
                content = page.Contents.getbytes()
            print(f"{path}: {content}")
        except Exception as e:
            print(f"{path}: ERROR - {e}")
