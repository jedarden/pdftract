#!/usr/bin/env python3
"""
Generate hybrid-007: Scanned form with vector textbox overlays.

Creates a PDF with:
- Scanned tax form background
- Vector fillable textbox overlays (rectangles with labels)
"""

import zlib

def create_scanned_tax_form_pattern(width, height):
    """Create a 1-bit image pattern simulating a scanned tax form."""
    bytes_per_row = (width + 7) // 8
    img_data = b""

    for y in range(height):
        row = bytearray(bytes_per_row)
        # Create form structure
        # Horizontal dividers every 60 pixels
        is_divider = (y % 60) < 2

        if is_divider:
            # Full horizontal line
            for x in range(width):
                byte_offset = x // 8
                bit_offset = x % 8
                if byte_offset < len(row):
                    row[byte_offset] |= (0x80 >> bit_offset)
        else:
            # Text labels and vertical dividers
            in_label_row = (y % 60) < 15
            if in_label_row:
                # Labels at left side
                for x in range(0, 200, 12):
                    byte_offset = x // 8
                    bit_offset = x % 8
                    if byte_offset < len(row):
                        row[byte_offset] |= (0x80 >> bit_offset)

            # Vertical divider every 200 pixels
            for vx in [200, 400]:
                byte_offset = vx // 8
                bit_offset = vx % 8
                if byte_offset < len(row):
                    row[byte_offset] |= (0x80 >> bit_offset)

        img_data += bytes(row)

    return img_data

def create_hybrid_007_pdf():
    """Create hybrid-007: scanned tax form with vector textbox overlays."""

    width, height = 612, 792

    # Full-page scanned tax form
    img_data = create_scanned_tax_form_pattern(width, height)
    compressed_img = zlib.compress(img_data)

    # Vector textbox overlays at key positions
    content = f"""q
/Im1 Do
Q
BT
/F1 10 Tf
0 0 0 rg
210 750 Td
(Form 1040 - U.S. Individual Income Tax Return) Tj
ET
q
0.5 0.5 0.5 rg
1 w
210 680 150 20 re
S
Q
BT
/F1 9 Tf
0 0 0 rg
215 685 Td
(First name) Tj
ET
q
0.5 0.5 0.5 rg
1 w
210 650 150 20 re
S
Q
BT
/F1 9 Tf
0 0 0 rg
215 655 Td
(Last name) Tj
ET
q
0.5 0.5 0.5 rg
1 w
420 680 150 20 re
S
Q
BT
/F1 9 Tf
0 0 0 rg
425 685 Td
(SSN) Tj
ET
q
0.5 0.5 0.5 rg
1 w
210 560 360 60 re
S
Q
BT
/F1 9 Tf
0 0 0 rg
215 590 Td
(Home address) Tj
0 -12 Td
(City, State, ZIP) Tj
ET
q
0.5 0.5 0.5 rg
1 w
210 480 150 20 re
S
Q
BT
/F1 9 Tf
0 0 0 rg
215 485 Td
(Total income) Tj
ET
q
0.5 0.5 0.5 rg
1 w
420 480 150 20 re
S
Q
BT
/F1 9 Tf
0 0 0 rg
425 485 Td
(Tax withheld) Tj
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

    # Content stream: image + textboxes
    objects[4] = f"4 0 obj\n<< /Length {len(compressed_content)} /Filter /FlateDecode >>\nstream\n".encode() + compressed_content + b"\nendstream\nendobj\n"

    # Font
    objects[5] = b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n"

    # Image (full-page tax form)
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
    output_path = "tests/fixtures/hybrid/hybrid-007-textbox-overlay.pdf"
    pdf_bytes = create_hybrid_007_pdf()

    with open(output_path, 'wb') as f:
        f.write(pdf_bytes)

    print(f"Created {output_path} ({len(pdf_bytes)} bytes)")

if __name__ == "__main__":
    main()
