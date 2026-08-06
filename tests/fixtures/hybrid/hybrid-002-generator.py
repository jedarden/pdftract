#!/usr/bin/env python3
"""
Generate hybrid-002: Vector form fields/annotations overlaid on scanned form.

Creates a PDF with:
- Scanned form background (simulated employee information form)
- Vector form field annotations (fillable text boxes, checkboxes)
- Vector overlay elements (highlight boxes, field indicators)
"""

import struct
import zlib

def create_scanned_form_image_pattern(width, height):
    """Create a 1-bit grayscale image pattern simulating a scanned form with text."""
    bytes_per_row = (width + 7) // 8
    img_data = b""

    # Form layout - create text-like patterns in specific regions
    form_sections = [
        # (y_start, y_end, text_pattern)
        (50, 100, "EMPLOYEE INFORMATION FORM"),  # Title
        (130, 160, "Full Name: _______________________"),  # Field 1
        (180, 210, "Employee ID: _____________________"),  # Field 2
        (230, 260, "Department: _____________________"),  # Field 3
        (290, 320, "Email: _____________________________"),  # Field 4
        (350, 380, "Phone: ____________________________"),  # Field 5
        (420, 450, "SECTION A: Personal Information"),  # Section header
        (480, 510, "Date of Birth: ___________________"),  # Field 6
        (530, 560, "Address: __________________________"),  # Field 7
        (590, 620, "City: __________ State: ____ ZIP: _____"),  # Field 8
        (660, 700, "I certify the information provided is correct."),  # Certification
        (720, 750, "Signature: _________________  Date: _______"),  # Signature
    ]

    for y in range(height):
        row = bytearray(bytes_per_row)

        # Check if this row is within a form section
        for y_start, y_end, pattern in form_sections:
            if y_start <= y < y_end:
                # Create horizontal line pattern for text
                if (y % 15) < 10:  # Text lines
                    for x in range(50, min(width - 50, 550)):
                        byte_offset = x // 8
                        bit_offset = x % 8
                        if byte_offset < len(row):
                            # Create scattered pixels to simulate text
                            if (x + y) % 3 == 0:
                                row[byte_offset] |= (0x80 >> bit_offset)
                elif (y % 15) >= 12:  # Horizontal lines for underlines
                    for x in range(200, min(width - 50, 550)):
                        byte_offset = x // 8
                        bit_offset = x % 8
                        if byte_offset < len(row):
                            row[byte_offset] |= (0x80 >> bit_offset)
                break

        img_data += bytes(row)

    return img_data


def create_hybrid_002_pdf():
    """Create hybrid-002 PDF with vector form fields over scanned background."""

    width, height = 612, 792  # Letter size

    # Create scanned form background image
    img_data = create_scanned_form_image_pattern(width, height)
    compressed_img = zlib.compress(img_data)

    # Vector content stream: draw form field annotations over scanned background
    # This creates vector rectangles and text that overlay the scanned form
    content = f"""BT
/F1 10 Tf
70 750 Td
(EMPLOYEE INFORMATION FORM) Tj
ET
q
70 70 472 652 re
W n
/Im1 Do
Q
BT
/F1 9 Tf
70 135 Td
(Full Name:) Tj
/F1 8 Tf
-5 14 Td
(Name field annotation) Tj
ET
BT
/F1 9 Tf
70 185 Td
(Employee ID:) Tj
/F1 8 Tf
-5 14 Td
(ID field annotation) Tj
ET
BT
/F1 9 Tf
70 235 Td
(Department:) Tj
/F1 8 Tf
-5 14 Td
(Dept field annotation) Tj
ET
BT
/F1 9 Tf
70 285 Td
(Email:) Tj
/F1 8 Tf
-5 14 Td
(Email field annotation) Tj
ET
BT
/F1 9 Tf
70 335 Td
(Phone:) Tj
/F1 8 Tf
-5 14 Td
(Phone field annotation) Tj
ET
BT
/F1 9 Tf
70 385 Td
(Notes:) Tj
/F1 8 Tf
-5 14 Td
(Additional information field) Tj
ET
q
0.5 0.5 0.5 rg
200 120 300 20 re
S
200 170 300 20 re
S
200 220 300 20 re
S
200 270 300 20 re
S
200 320 300 20 re
S
200 370 300 100 re
S
Q
q
1 0 0 rg
50 50 10 10 re
f
Q
q
1 0 0 rg
50 70 10 10 re
f
Q
q
1 0 0 rg
50 90 10 10 re
f
Q
"""

    compressed_content = zlib.compress(content.encode())

    # Build PDF with proper structure
    parts = []

    # PDF Header
    parts.append("%PDF-1.4\n")
    parts.append("%\xC2\xB5\xC2\xB6\n")

    # Collect all object data first
    objects = {}

    # Catalog
    objects[1] = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n"

    # Pages
    objects[2] = b"2 0 obj\n<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>\nendobj\n"

    # Page
    objects[3] = b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 612 792 ] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> /XObject << /Im1 6 0 R >> >> >>\nendobj\n"

    # Content stream with vector form field annotations over scanned background
    objects[4] = f"4 0 obj\n<< /Length {len(compressed_content)} /Filter /FlateDecode >>\nstream\n".encode() + compressed_content + b"\nendstream\nendobj\n"

    # Font
    objects[5] = b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n"

    # Image (scanned form background)
    objects[6] = f"6 0 obj\n<< /Type /XObject /Subtype /Image /Width {width} /Height {height} /BitsPerComponent 1 /ColorSpace /DeviceGray /Length {len(compressed_img)} /Filter /FlateDecode >>\nstream\n".encode() + compressed_img + b"\nendstream\nendobj\n"

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
    output_path = "tests/fixtures/hybrid/hybrid-002-vector-form-over-scan.pdf"
    pdf_bytes = create_hybrid_002_pdf()

    with open(output_path, 'wb') as f:
        f.write(pdf_bytes)

    print(f"Created {output_path} ({len(pdf_bytes)} bytes)")
    print(f"File size: {len(pdf_bytes)} bytes")
    return output_path, len(pdf_bytes)


if __name__ == "__main__":
    main()
