#!/usr/bin/env python3
import pikepdf
import zlib

# Check v1.pdf
with pikepdf.open("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf") as pdf:
    page = pdf.pages[0]
    contents = page.get("/Contents")
    if contents:
        raw = contents.read_raw_bytes()
        print(f"v1 raw hex: {raw.hex()}")

        # Try with zlib header (78 9c)
        try:
            decompressed = zlib.decompress(raw)
            print(f"v1 decompressed: {decompressed}")
        except Exception as e:
            print(f"v1 decompress failed: {e}")

print()

# Check v2.pdf
with pikepdf.open("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf") as pdf:
    page = pdf.pages[0]
    contents = page.get("/Contents")
    if contents:
        raw = contents.read_raw_bytes()
        print(f"v2 raw hex: {raw.hex()}")

        try:
            decompressed = zlib.decompress(raw)
            print(f"v2 decompressed: {decompressed}")
        except Exception as e:
            print(f"v2 decompress failed: {e}")
