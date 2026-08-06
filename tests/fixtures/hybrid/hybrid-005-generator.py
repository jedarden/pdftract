#!/usr/bin/env python3
"""
Generate hybrid-005: Scanned body with vector footer/page numbers.

Creates a PDF with:
- Scanned document body (top 90%)
- Vector footer with page numbers (bottom 10%)
"""

import zlib

def create_scanned_body_pattern(width, height):
    """Create a 1-bit image pattern simulating a scanned document body."""
    bytes_per_row = (width + 7) // 8
    img_data = b""

    for y in range(height):
        row = bytearray(bytes_per_row)
        # Create text lines throughout the body
        line_spacing = 16
        line_height = 9
        in_text_line = (y % line_spacing) < line_height

        if in_text_line:
            # Fill with pattern to simulate paragraph text
            for x in range(0, width, 50):
                byte_offset = x // 8
                bit_offset = x % 8
                if byte_offset < len(row):
                    # Create text-like patterns
                    text_pattern = [1, 1, 1, 1, 1, 0, 1, 1, 1, 0]
                    if (x // 8) % 10 < len(text_pattern):
                        if text_pattern[(x // 8) % 10]:
                            row[byte_offset] |= (0x80 >> bit_offset)

        img_data += bytes(row)

    return img_data

def create_hybrid_005_pdf():
    """Create hybrid-005: scanned body with vector footer."""

    width, height = 612, 792
    footer_height = 80
    body_height = height - footer_height

    # Scanned body only (top portion)
    img_data = create_scanned_body_pattern(width, body_height)
    compressed_img = zlib.compress(img_data)

    # Content: scanned image + vector footer
    content = f"""BT
/F1 28 Tf
0 0 0 rg
200 760 Td
(Legal Document Header) Tj
ET
q
0 {footer_height} {width} {body_height} re
W n
/Im1 Do
Q
BT
/F1 9 Tf
0 0 0 rg
306 35 Td
(Page 1 of 3) Tj
0 -15 Td
(Confidential - Attorney Eyes Only) Tj
ET
q
50 15 {width-100} 1 re
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

    # Content stream
    objects[4] = f"4 0 obj\n<< /Length {len(compressed_content)} /Filter /FlateDecode >>\nstream\n".encode() + compressed_content + b"\nendstream\nendobj\n"

    # Font
    objects[5] = b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n"

    # Image (body only)
    objects[6] = f"6 0 obj\n<< /Type /XObject /Subtype /Image /Width {width} /Height {body_height} /BitsPerComponent 1 /ColorSpace /DeviceGray /Length {len(compressed_img)} /Filter /FlateDecode >>\nstream\n".encode() + compressed_img + b"\nendstream\nendobj\n"

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
    output_path = "tests/fixtures/hybrid/hybrid-005-vector-footer-over-scan.pdf"
    pdf_bytes = create_hybrid_005_pdf()

    with open(output_path, 'wb') as f:
        f.write(pdf_bytes)

    print(f"Created {output_path} ({len(pdf_bytes)} bytes)")

if __name__ == "__main__":
    main()
