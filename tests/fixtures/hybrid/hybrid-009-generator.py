#!/usr/bin/env python3
"""
Generate hybrid-009: Scanned page with semi-transparent vector overlay.

Creates a PDF with:
- Full-page scanned document background
- Semi-transparent vector text overlay (using transparency group)
"""

import zlib

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
                    # Create word-like patterns
                    word_pattern = [1, 1, 1, 1, 0, 1, 1, 0]
                    if (x // 10) % 8 < len(word_pattern):
                        if word_pattern[(x // 10) % 8]:
                            row[byte_offset] |= (0x80 >> bit_offset)

        img_data += bytes(row)

    return img_data

def create_hybrid_009_pdf():
    """Create hybrid-009: scanned document with semi-transparent vector overlay."""

    width, height = 612, 792

    # Full-page scanned document
    img_data = create_scanned_document_pattern(width, height)
    compressed_img = zlib.compress(img_data)

    # Semi-transparent vector overlay using graphics state with opacity
    # Note: PDF transparency requires proper ExtGState usage
    content = f"""q
/Im1 Do
Q
BT
/F1 10 Tf
0 0 0 rg
50 750 Td
(Document with Semi-Transparent Overlay) Tj
ET
q
/GS1 gs
BT
/F1 24 Tf
0.7 0.7 0.7 rg
306 396 Td
(OVERLAY TEXT) Tj
ET
Q
BT
/F1 14 Tf
0.5 0.5 0.5 rg
50 400 Td
(Secondary overlay text) Tj
0 -25 Td
(Another transparent line) Tj
0 -25 Td
(Third transparency test) Tj
ET
q
/GS2 gs
BT
/F1 18 Tf
0.8 0.2 0.2 rg
450 600 Td
(Note) Tj
ET
Q
"""

    compressed_content = zlib.compress(content.encode())

    # Build PDF with ExtGState for transparency
    pdf = b"%PDF-1.4\n%\xB5\xB6\n"

    objects = {}

    # Catalog
    objects[1] = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n"

    # Pages
    objects[2] = b"2 0 obj\n<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>\nendobj\n"

    # Page with ExtGState resource
    objects[3] = f"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 {width} {height} ] /Contents 9 0 R /Resources << /Font << /F1 5 0 R >> /ExtGState << /GS1 6 0 R /GS2 7 0 R >> /XObject << /Im1 8 0 R >> >> >>\nendobj\n".encode()

    # ExtGState with 0.5 opacity (50% transparent)
    objects[6] = b"6 0 obj\n<< /Type /ExtGState /ca 0.5 /CA 0.5 >>\nendobj\n"

    # ExtGState with 0.7 opacity (30% transparent)
    objects[7] = b"7 0 obj\n<< /Type /ExtGState /ca 0.7 /CA 0.7 >>\nendobj\n"

    # Font
    objects[5] = b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n"

    # Image (full-page scanned document)
    objects[8] = f"8 0 obj\n<< /Type /XObject /Subtype /Image /Width {width} /Height {height} /BitsPerComponent 1 /ColorSpace /DeviceGray /Length {len(compressed_img)} /Filter /FlateDecode >>\nstream\n".encode() + compressed_img + b"\nendstream\nendobj\n"

    # Content stream
    objects[9] = f"9 0 obj\n<< /Length {len(compressed_content)} /Filter /FlateDecode >>\nstream\n".encode() + compressed_content + b"\nendstream\nendobj\n"

    # Calculate offsets
    offsets = {}
    current_offset = len(pdf)

    obj_nums = [1, 2, 3, 5, 6, 7, 8, 9]
    for obj_num in obj_nums:
        offsets[obj_num] = current_offset
        pdf += objects[obj_num]
        current_offset += len(objects[obj_num])

    # XRef and trailer
    xref_start = current_offset
    xref = f"xref\n0 10\n0000000000 65535 f \n"
    for i in obj_nums:
        xref += f"{offsets[i]:010d} 00000 n \n"

    pdf += xref.encode()
    pdf += b"\ntrailer\n<< /Size 10 /Root 1 0 R >>\nstartxref\n"
    pdf += str(xref_start).encode()
    pdf += b"\n%%EOF\n"

    return pdf

def main():
    output_path = "tests/fixtures/hybrid/hybrid-009-transparent-vector.pdf"
    pdf_bytes = create_hybrid_009_pdf()

    with open(output_path, 'wb') as f:
        f.write(pdf_bytes)

    print(f"Created {output_path} ({len(pdf_bytes)} bytes)")

if __name__ == "__main__":
    main()
