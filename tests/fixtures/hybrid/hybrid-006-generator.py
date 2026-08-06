#!/usr/bin/env python3
"""
Generate hybrid-006: Scanned document with vector stamp/seal overlay.

Creates a PDF with:
- Full-page scanned contract document
- Vector circular stamp/seal overlay (bottom right corner)
"""

import zlib
import math

def create_scanned_contract_pattern(width, height):
    """Create a 1-bit image pattern simulating a scanned contract."""
    bytes_per_row = (width + 7) // 8
    img_data = b""

    for y in range(height):
        row = bytearray(bytes_per_row)
        # Create contract text lines
        line_spacing = 18
        line_height = 10
        in_text_line = (y % line_spacing) < line_height

        if in_text_line:
            # Fill with pattern to simulate contract text
            for x in range(0, width, 55):
                byte_offset = x // 8
                bit_offset = x % 8
                if byte_offset < len(row):
                    # Legal text pattern (dense, formal)
                    legal_pattern = [1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 0]
                    if (x // 7) % 12 < len(legal_pattern):
                        if legal_pattern[(x // 7) % 12]:
                            row[byte_offset] |= (0x80 >> bit_offset)

        img_data += bytes(row)

    return img_data

def create_hybrid_006_pdf():
    """Create hybrid-006: scanned contract with vector stamp overlay."""

    width, height = 612, 792

    # Full-page scanned contract
    img_data = create_scanned_contract_pattern(width, height)
    compressed_img = zlib.compress(img_data)

    # Vector stamp seal (circular, in bottom right)
    stamp_center_x = 500
    stamp_center_y = 100
    stamp_radius = 60

    content = f"""q
/Im1 Do
Q
BT
/F1 12 Tf
0 0 0 rg
50 750 Td
(CONTRACT AGREEMENT) Tj
0 -20 Td
(This Agreement is made on August 6, 2026) Tj
ET
q
1 0 0 1 {stamp_center_x} {stamp_center_y} cm
0 0 m
{stamp_radius} 0 l
{stamp_radius} 0 0 {stamp_radius} 0 0 re
W n
0 0 1 rg
3 w
S
Q
BT
/F1 10 Tf
1 0 0 rg
{stamp_center_x - 40} {stamp_center_y - 5} Td
(APPROVED) Tj
0 -12 Td
(Official Seal) Tj
ET
q
{stamp_center_x - stamp_radius} {stamp_center_y - stamp_radius} {stamp_radius * 2} {stamp_radius * 2} re
S
Q
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

    # Content stream: image + stamp
    objects[4] = f"4 0 obj\n<< /Length {len(compressed_content)} /Filter /FlateDecode >>\nstream\n".encode() + compressed_content + b"\nendstream\nendobj\n"

    # Font
    objects[5] = b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>\nendobj\n"

    # Image (full-page contract)
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
    output_path = "tests/fixtures/hybrid/hybrid-006-stamp-annotation.pdf"
    pdf_bytes = create_hybrid_006_pdf()

    with open(output_path, 'wb') as f:
        f.write(pdf_bytes)

    print(f"Created {output_path} ({len(pdf_bytes)} bytes)")

if __name__ == "__main__":
    main()
