#!/usr/bin/env python3
"""Inspect the content_edit fixtures to debug."""

import pikepdf
import zlib

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
            print(f"Filter: {contents1.get('/Filter')}")

            # Try decompressing
            try:
                text1 = zlib.decompress(data1, -15).decode("latin-1")
                print(f"Decompressed text: {text1}")
            except Exception as e:
                print(f"Decompress error: {e}")
                print(f"Raw stream (hex): {data1.hex()}")

        print("\n=== v2.pdf ===")
        contents2 = page2.get("/Contents")

        if isinstance(contents2, pikepdf.Stream):
            data2 = contents2.read_bytes()
            print(f"Stream length: {len(data2)}")
            print(f"Filter: {contents2.get('/Filter')}")

            # Try decompressing
            try:
                text2 = zlib.decompress(data2, -15).decode("latin-1")
                print(f"Decompressed text: {text2}")
            except Exception as e:
                print(f"Decompress error: {e}")
                print(f"Raw stream (hex): {data2.hex()}")

# Now check the paragraph ones
print("\n\n=== Paragraph fixtures ===")
with pikepdf.open("tests/fingerprint/fixtures/content_edit_one_paragraph/v1.pdf") as pdf1:
    with pikepdf.open("tests/fingerprint/fixtures/content_edit_one_paragraph/v2.pdf") as pdf2:
        page1 = pdf1.pages[0]
        page2 = pdf2.pages[0]

        print("=== v1.pdf ===")
        contents1 = page1.get("/Contents")

        if isinstance(contents1, pikepdf.Stream):
            data1 = contents1.read_bytes()
            try:
                text1 = zlib.decompress(data1, -15).decode("latin-1")
                print(f"Decompressed text: {text1[:200]}...")
            except Exception as e:
                print(f"Error: {e}")

        print("\n=== v2.pdf ===")
        contents2 = page2.get("/Contents")

        if isinstance(contents2, pikepdf.Stream):
            data2 = contents2.read_bytes()
            try:
                text2 = zlib.decompress(data2, -15).decode("latin-1")
                print(f"Decompressed text: {text2[:200]}...")
            except Exception as e:
                print(f"Error: {e}")
