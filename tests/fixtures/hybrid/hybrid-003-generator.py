#!/usr/bin/env python3
"""
Generate a hybrid PDF with mixed column layout.

Creates a PDF with:
- Left column: Vector text content (selectable)
- Right column: Scanned/image content (simulated newspaper article)
"""

import struct
import zlib
import base64

def create_scanned_image_pattern(width, height):
    """Create a simple 1-bit image pattern representing scanned text."""
    bytes_per_row = (width + 7) // 8
    img_data = b""

    for y in range(height):
        row = bytearray(bytes_per_row)
        # Create horizontal lines every 16 pixels to simulate text
        if (y % 16) < 8:
            # Fill with pattern to represent text lines
            for x in range(0, width, 60):
                byte_offset = x // 8
                bit_offset = x % 8
                if byte_offset < len(row):
                    # Create varied pattern to simulate words
                    word_width = 40
                    for w in range(word_width):
                        if byte_offset + (w // 8) < len(row):
                            row[byte_offset + (w // 8)] |= (0x80 >> ((bit_offset + w) % 8))
        img_data += bytes(row)

    return img_data

def create_mixed_column_pdf():
    """Create a hybrid PDF with vector text in left column and scanned content in right column."""

    width, height = 612, 792  # Letter size in points

    # Column layout: left 45% for vector text, right 55% for scanned content
    left_col_width = int(width * 0.45)
    right_col_width = width - left_col_width

    # Create image data for right column (scanned content)
    img_width = right_col_width
    img_height = height - 100  # Leave some margin at top/bottom
    img_data = create_scanned_image_pattern(img_width, img_height)
    compressed_img = zlib.compress(img_data)

    # Vector content stream: left column text + right column image
    content = f"""BT
/F1 10 Tf
50 750 Td
(LEFT COLUMN - VECTOR TEXT) Tj
0 -20 Td
(This content is selectable) Tj
0 -18 Td
(vector text that can be) Tj
0 -18 Td
(copy/pasted from PDF) Tj
ET
BT
/F1 9 Tf
50 700 Td
(Column 1 - Article Content) Tj
0 -20 Td
(Recent developments in the) Tj
0 -16 Td
(field of document processing) Tj
0 -16 Td
(have led to significant) Tj
0 -16 Td
(improvements in OCR) Tj
0 -16 Td
(technology. Modern systems) Tj
0 -16 Td
(can now accurately extract) Tj
0 -16 Td
(text from scanned documents) Tj
0 -16 Td
(with high precision.) Tj
0 -20 Td
(Research indicates that) Tj
0 -16 Td
(hybrid approaches combining) Tj
0 -16 Td
(vector and raster content) Tj
0 -16 Td
(provide optimal results.) Tj
ET
BT
/F1 9 Tf
50 560 Td
(The key challenge lies) Tj
0 -16 Td
(in accurate classification) Tj
0 -16 Td
(of content types. Systems) Tj
0 -16 Td
(must distinguish between) Tj
0 -16 Td
(pure vector, pure scanned,) Tj
0 -16 Td
(and hybrid documents.) Tj
ET
BT
/F1 9 Tf
50 420 Td
(Industrial applications) Tj
0 -16 Td
(require robust handling) Tj
0 -16 Td
(of mixed layouts.) Tj
0 -16 Td
(These include invoices,) Tj
0 -16 Td
(forms, reports, and) Tj
0 -16 Td
(newspaper formats.) Tj
ET
q
{left_col_width + 20} 50 {img_width} {img_height} re
W n
{left_col_width + 20} 50 translate
/Im1 Do
Q
BT
/F1 8 Tf
{left_col_width + 25} 40 Td
(Right column - scanned) Tj
ET
"""

    compressed_content = zlib.compress(content.encode())

    # Build PDF with proper structure
    parts = []

    # PDF Header
    parts.append("%PDF-1.4\n")
    parts.append("%\xC2\xB5\xC2\xB6\n")  # Comment with binary chars

    # Collect all object data
    objects = {}

    # Catalog
    objects[1] = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n"

    # Pages
    objects[2] = b"2 0 obj\n<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>\nendobj\n"

    # Page
    page_obj = f"""3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 {width} {height} ] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> /XObject << /Im1 6 0 R >> >> >>
endobj
"""
    objects[3] = page_obj.encode()

    # Content stream
    content_obj = f"""4 0 obj
<< /Length {len(compressed_content)} /Filter /FlateDecode >>
stream
"""
    objects[4] = content_obj.encode() + compressed_content + b"\nendstream\nendobj\n"

    # Font
    objects[5] = b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n"

    # Image (scanned right column)
    image_obj = f"""6 0 obj
<< /Type /XObject /Subtype /Image /Width {img_width} /Height {img_height} /BitsPerComponent 1 /ColorSpace /DeviceGray /Length {len(compressed_img)} /Filter /FlateDecode >>
stream
"""
    objects[6] = image_obj.encode() + compressed_img + b"\nendstream\nendobj\n"

    # Write PDF with offsets
    pdf_bytes = b"%PDF-1.4\n%\xB5\xB6\n"

    offsets = {}
    current_offset = len(pdf_bytes)

    for obj_num in range(1, 7):
        offsets[obj_num] = current_offset
        pdf_bytes += objects[obj_num]
        current_offset += len(objects[obj_num])

    # XRef
    xref_start = current_offset
    xref = f"xref\n0 7\n0000000000 65535 f \n"
    for i in range(1, 7):
        xref += f"{offsets[i]:010d} 00000 n \n"

    pdf_bytes += xref.encode()
    pdf_bytes += b"\ntrailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n"
    pdf_bytes += str(xref_start).encode()
    pdf_bytes += b"\n%%EOF\n"

    return pdf_bytes

def main():
    output_path = "tests/fixtures/hybrid/hybrid-003-mixed-column-layout.pdf"
    pdf_bytes = create_mixed_column_pdf()

    with open(output_path, 'wb') as f:
        f.write(pdf_bytes)

    print(f"Created {output_path} ({len(pdf_bytes)} bytes)")

if __name__ == "__main__":
    main()
