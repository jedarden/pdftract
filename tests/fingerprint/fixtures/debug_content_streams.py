#!/usr/bin/env python3
"""Debug content stream extraction without decompression."""

import pikepdf

# Check the content of the two PDFs
with pikepdf.open("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf") as pdf1:
    with pikepdf.open("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf") as pdf2:
        # Get the content stream
        page1 = pdf1.pages[0]
        page2 = pdf2.pages[0]

        print("=== v1.pdf ===")
        contents1 = page1.get("/Contents")

        if isinstance(contents1, pikepdf.Stream):
            data1 = contents1.read_bytes()
            print(f"Stream length: {len(data1)}")
            print(f"Raw stream (bytes): {data1}")
            print(f"Raw stream (text): {data1.decode('latin-1')}")
            print(f"MD5: {data1.hex()}")

        print("\n=== v2.pdf ===")
        contents2 = page2.get("/Contents")

        if isinstance(contents2, pikepdf.Stream):
            data2 = contents2.read_bytes()
            print(f"Stream length: {len(data2)}")
            print(f"Raw stream (bytes): {data2}")
            print(f"Raw stream (text): {data2.decode('latin-1')}")
            print(f"MD5: {data2.hex()}")

        print("\n=== Difference ===")
        print(f"Streams are identical: {data1 == data2}")
        print(f"v1 has 'World': {b'World' in data1}")
        print(f"v2 has 'World': {b'World' in data2}")
