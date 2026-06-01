#!/usr/bin/env python3
import sys
try:
    import pikepdf
except ImportError:
    sys.exit("pikepdf not available")

def extract_text(path):
    with pikepdf.open(path) as pdf:
        for page in pdf.pages:
            if "/Contents" in page:
                contents = page["/Contents"]
                if hasattr(contents, "read_bytes"):
                    data = contents.read_bytes()
                else:
                    data = bytes(contents)
                print(f"{path}: {data[:200]}")
                break

extract_text("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf")
extract_text("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf")
