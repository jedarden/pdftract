#!/usr/bin/env python3
"""
Generate hybrid-004: Scanned page with vector watermark overlay.

Creates a PDF with:
- Full-page scanned document background
- Vector watermark overlay (diagonal text, semi-transparent effect)
"""

import zlib
import struct

def create_scanned_document_pattern(width, height):
    """Create a 1-bit image pattern simulating a scanned document page."""
    bytes_per_row = (width + 7) // 8
    img_data = b""

    for y in range(height):
        row = bytearray(bytes_per_row)
        # Create horizontal text lines throughout
        line_spacing = 18
        line_height = 10
        in_text_line = (y % line_spacing) < line_height

        if in_text_line:
            # Fill with pattern to simulate document text
            for x in range(0, width, 60):
                byte_offset = x // 8
                bit_offset = x % 8
                if byte_offset < len(row):
                    # Create word-like patterns (3-5 chars then space)
                    word_pattern = [1, 1, 1, 1, 0, 1, 1, 0]
                    if (x // 10) % 8 < len(word_pattern):
                        if word_pattern[(x // 10) % 8]:
                            row[byte_offset] |= (0x80 >> bit_offset)

        img_data += bytes(row)

    return img_data

def create_hybrid_004_pdf():
    """Create hybrid-004: scanned document with vector watermark overlay."""

    width, height = 612, 792

    # Full-page scanned document
    img_data = create_scanned_document_pattern(width, height)
    compressed_img = zlib.compress(img_data)

    # Vector watermark overlay (diagonal text across page)
    content = f"""q
/Im1 Do
Q
BT
/F1 28 Tf
0.5 0.5 0.5 rg
306 396 Td
-45 -25 Tm
(DRAFT - WATERMARK - CONFIDENTIAL) Tj
ET
BT
/F1 10 Tf
0 0 0 rg
50 750 Td
(Page Header - Document Title) Tj
0 -20 Td
(Author: John Doe) Tj
0 -20 Td
(Date: August 6, 2026) Tj
ET
"""

    compressed_content = zlib.compress(content.encode())

    # Build PDF
    pdf = b"%PDF-1.4\n%\xB5\xB6\n"

    objects = {}

    # Catalog
    objects[1] = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n"

    # Pages
    objects[2] = b"2 0 obj\n<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>\nendobj\n"

    # Page
    objects[3] = f"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 {width} {height} ] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> /XObject << /Im1 6 0 R >> >> >>\nendobj\n".encode()

    # Content stream: image + watermark
    objects[4] = f"4 0 obj\n<< /Length {len(compressed_content)} /Filter /FlateDecode >>\nstream\n".encode() + compressed_content + b"\nendstream\nendobj\n"

    # Font
    objects[5] = b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n"

    # Image (full-page scanned document)
    objects[6] = f"6 0 obj\n<< /Type /XObject /Subtype /Image /Width {width} /Height {height} /BitsPerComponent 1 /ColorSpace /DeviceGray /Length {len(compressed_img)} /Filter /FlateDecode >>\nstream\n".encode() + compressed_img + b"\nendstream\nendobj\n"

    # Calculate offsets
    offsets = {}
    current_offset = len(pdf)

    for obj_num in range(1, 7):
        offsets[obj_num] = current_offset
        pdf += objects[obj_num]
        current_offset += len(objects[obj_num])

    # XRef and trailer
    xref_start = current_offset
    xref = f"xref\n0 7\n0000000000 65535 f \n"
    for i in range(1, 7):
        xref += f"{offsets[i]:010d} 00000 n \n"

    pdf += xref.encode()
    pdf += b"\ntrailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n"
    pdf += str(xref_start).encode()
    pdf += b"\n%%EOF\n"

    return pdf

def main():
    output_path = "tests/fixtures/hybrid/hybrid-004-watermark-over-scan.pdf"
    pdf_bytes = create_hybrid_004_pdf()

    with open(output_path, 'wb') as f:
        f.write(pdf_bytes)

    print(f"Created {output_path} ({len(pdf_bytes)} bytes)")

if __name__ == "__main__":
    main()
