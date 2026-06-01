#!/usr/bin/env python3
import pikepdf

# Check content_edit_one_glyph
print("=== content_edit_one_glyph ===")
for fname in ["v1.pdf", "v2.pdf"]:
    path = f"tests/fingerprint/fixtures/content_edit_one_glyph/{fname}"
    with pikepdf.open(path) as pdf:
        page = pdf.pages[0]
        contents = page.get("/Contents")
        print(f"\n{fname}:")
        print(f"  Type: {type(contents)}")
        if hasattr(contents, "get"):
            print(f"  /Filter: {contents.get('/Filter')}")
        # Get raw bytes
        if hasattr(contents, "read_bytes"):
            raw = contents.read_bytes()
        else:
            raw = bytes(contents._data)
        print(f"  Length: {len(raw)}")
        print(f"  First 100 bytes: {raw[:100]}")

# Try a different approach - create PDFs with NO compression
print("\n=== Creating uncompressed fixtures ===")
pdf = pikepdf.new()

# Add page
pdf.add_blank_page(page_size=(612, 792))
page = pdf.pages[0]

# Add content WITHOUT compression
content_stream = b"BT /F1 12 Tf 50 700 Td (Hello World) Tj ET"
stream = pikepdf.Stream(pdf, content_stream)
page["/Contents"] = stream
page["/Resources"] = pikepdf.Dictionary({
    "/Font": pikepdf.Dictionary({
        "/F1": pikepdf.Dictionary({
            "/Type": "/Font",
            "/Subtype": "/Type1",
            "/BaseFont": "/Helvetica"
        })
    })
})

# Save WITHOUT compression
pdf.save("tests/fingerprint/fixtures/content_edit_one_glyph/v1_uncompressed.pdf",
         compress_streams=False,
         stream_decode_level=pikepdf.StreamDecodeLevel.none)

# Create v2 with different content
pdf2 = pikepdf.new()
pdf2.add_blank_page(page_size=(612, 792))
page2 = pdf2.pages[0]
content_stream2 = b"BT /F1 12 Tf 50 700 Td (Hello Worl) Tj ET"
stream2 = pikepdf.Stream(pdf2, content_stream2)
page2["/Contents"] = stream2
page2["/Resources"] = pikepdf.Dictionary({
    "/Font": pikepdf.Dictionary({
        "/F1": pikepdf.Dictionary({
            "/Type": "/Font",
            "/Subtype": "/Type1",
            "/BaseFont": "/Helvetica"
        })
    })
})

pdf2.save("tests/fingerprint/fixtures/content_edit_one_glyph/v2_uncompressed.pdf",
         compress_streams=False,
         stream_decode_level=pikepdf.StreamDecodeLevel.none)

print("Created uncompressed fixtures")
