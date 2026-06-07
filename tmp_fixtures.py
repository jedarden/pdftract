#!/usr/bin/env python3
"""Generate content_edit fixtures using pikepdf."""

import pikepdf
from pathlib import Path


def create_simple_pdf(content, output_path):
    """Create a simple PDF with minimal text content."""
    pdf = pikepdf.new()
    pdf.add_blank_page(page_size=(612, 792))
    page = pdf.pages[0]

    content_stream = f"""
BT
/F1 12 Tf
50 700 Td
({content}) Tj
ET
"""
    stream = pikepdf.Stream(pdf, content_stream.encode())
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
    pdf.save(output_path)


def main():
    # Use absolute path since we may run from different directories
    import os
    script_dir = Path(os.path.dirname(os.path.abspath(__file__)))
    fixtures_dir = script_dir / "tests" / "fingerprint" / "fixtures"

    # Generate content_edit_one_glyph fixtures
    dir = fixtures_dir / "content_edit_one_glyph"
    dir.mkdir(exist_ok=True)
    create_simple_pdf("Hello World", dir / "v1.pdf")
    create_simple_pdf("Hello Worl", dir / "v2.pdf")
    print("Generated content_edit_one_glyph fixtures")

    # Generate content_edit_one_paragraph fixtures
    dir = fixtures_dir / "content_edit_one_paragraph"
    dir.mkdir(exist_ok=True)
    original_text = "This is the first paragraph. " * 5
    variant_text = "This is the second paragraph. " + "This is the first paragraph. " * 4
    create_simple_pdf(original_text, dir / "v1.pdf")
    create_simple_pdf(variant_text, dir / "v2.pdf")
    print("Generated content_edit_one_paragraph fixtures")


if __name__ == "__main__":
    main()
