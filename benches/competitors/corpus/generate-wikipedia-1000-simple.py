#!/usr/bin/env python3
"""
Generate a minimal Wikipedia-like PDF for the grep benchmark.

This creates a simple PDF with 100 pages, each containing the word "the"
multiple times for grep benchmarking. Uses only standard library.
"""

import struct
import zlib

def create_simple_pdf(output_path, num_pages=100):
    """Create a minimal PDF with multiple pages."""

    # Content stream with "the" repeated
    text_content = b""
    for i in range(50):  # 50 lines per page
        text_content += b"BT /F1 12 Tf 50 %d Td (The quick brown fox jumps over the lazy dog. The word the appears many times. The the the. ) Tj ET\n" % (700 - i * 12)

    # Compress the content
    compressed_content = zlib.compress(text_content)

    # Build PDF objects
    pdf_objects = []

    # Object 1: Catalog
    pdf_objects.append(b"1 0 obj\n<< /Type /Catalog /Outlines 2 0 R /Pages 3 0 R >>\nendobj\n")

    # Object 2: Outlines (empty)
    pdf_objects.append(b"2 0 obj\n<< /Type /Outlines /Count 0 >>\nendobj\n")

    # Object 3: Pages
    kids = b" ".join([f"{4 + i} 0 R".encode() for i in range(num_pages)])
    pdf_objects.append(b"3 0 obj\n<< /Type /Pages /Kids [ " + kids + b" ] /Count " + str(num_pages).encode() + b" /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /MediaBox [ 0 0 612 792 ] >>\nendobj\n")

    # Page objects (4 to 4+num_pages-1)
    page_content_obj = 4 + num_pages  # Object number for content stream

    for i in range(num_pages):
        pdf_objects.append(f"{4 + i} 0 obj\n<< /Type /Page /Parent 3 0 R /Contents {page_content_obj} 0 R >>\nendobj\n".encode())

    # Content stream object
    pdf_objects.append(str(page_content_obj).encode() + b" 0 obj\n<< /Length " + str(len(compressed_content)).encode() + b" /Filter /FlateDecode >>\nstream\n" + compressed_content + b"\nendstream\nendobj\n")

    # Build PDF
    pdf_data = b"%PDF-1.4\n"

    # Calculate offsets
    offsets = [len(pdf_data)]
    for obj in pdf_objects:
        pdf_data += obj
        offsets.append(len(pdf_data))

    # Remove the last offset (it's after all objects)
    offsets = offsets[:-1]

    # Cross-reference table
    xref_offset = len(pdf_data)
    pdf_data += b"xref\n"
    pdf_data += b"0 " + str(len(pdf_objects) + 1).encode() + b"\n"
    pdf_data += b"0000000000 65535 f \n"

    for offset in offsets:
        pdf_data += b"%010d 00000 n \n" % offset

    # Trailer
    pdf_data += b"trailer\n"
    pdf_data += b"<< /Size " + str(len(pdf_objects) + 1).encode() + b" /Root 1 0 R >>\n"
    pdf_data += b"startxref\n"
    pdf_data += str(xref_offset).encode() + b"\n"
    pdf_data += b"%%EOF\n"

    # Write to file
    with open(output_path, 'wb') as f:
        f.write(pdf_data)

    print(f"Generated {output_path} with {num_pages} pages")

if __name__ == "__main__":
    import sys
    output_path = sys.argv[1] if len(sys.argv) > 1 else "wikipedia-1000.pdf"
    create_simple_pdf(output_path, num_pages=100)
