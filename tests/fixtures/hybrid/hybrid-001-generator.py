#!/usr/bin/env python3
"""
Generate a minimal hybrid PDF with vector header over scanned content.

Creates a PDF with:
- Vector text header (top 15%)
- Simulated scanned body (bottom 85%)
"""

import struct
import zlib
import base64

def create_simple_pdf():
    """Create a minimal PDF with vector and image content."""

    # PDF structure constants
    width, height = 612, 792  # Letter size in points
    header_height = 120  # Top 15% approx

    # Create a simple 1-bit image for the scanned body (simulate text)
    # This will be a simple pattern representing scanned text lines
    img_width = 612
    img_height = height - header_height
    img_data = create_scanned_image_pattern(img_width, img_height)

    # Flate encode the image data
    compressed_img = zlib.compress(img_data)

    # Build the PDF content
    pdf = []

    # PDF header
    pdf.append("%PDF-1.4\n")

    # Catalog
    pdf.append("1 0 obj\n")
    pdf.append("<< /Type /Catalog\n")
    pdf.append("   /Pages 2 0 R\n")
    pdf.append(">>\n")
    pdf.append("endobj\n")

    # Pages
    pdf.append("2 0 obj\n")
    pdf.append("<< /Type /Pages\n")
    pdf.append("   /Kids [ 3 0 R ]\n")
    pdf.append("   /Count 1\n")
    pdf.append(">>\n")
    pdf.append("endobj\n")

    # Page
    pdf.append("3 0 obj\n")
    pdf.append("<< /Type /Page\n")
    pdf.append("   /Parent 2 0 R\n")
    pdf.append("   /MediaBox [ 0 0 %d %d ]\n" % (width, height))
    pdf.append("   /Contents 4 0 R\n")
    pdf.append("   /Resources << /Font << /F1 5 0 R >> /XObject << /Im1 6 0 R >> >>\n")
    pdf.append(">>\n")
    pdf.append("endobj\n")

    # Contents (stream with drawing commands)
    # First draw vector text header, then overlay scanned image
    content = f"BT\n/F1 12 Tf\n50 750 Td\n(VECTOR HEADER - Company Name) Tj\n0 -20 Td\n(123 Main Street) Tj\n0 -20 Td\n(City, State 12345) Tj\nET\nq\n0 0 {img_width} {img_height} re\nW n\n50 0 translate\n/Im1 Do\nQ\n"

    # Make the image appear below the header (at y=0 in translated coords)
    content = f"BT\n/F1 12 Tf\n50 750 Td\n(VECTOR HEADER - Company Name) Tj\n0 -20 Td\n(123 Main Street) Tj\n0 -20 Td\n(City, State 12345) Tj\nET\nq\n50 0 {img_width-100} {img_height} re\nW n\n50 0 translate\n/Im1 Do\nQ\n"

    compressed_content = zlib.compress(content.encode())

    pdf.append("4 0 obj\n")
    pdf.append(f"<< /Length {len(compressed_content)} /Filter /FlateDecode >>\n")
    pdf.append("stream\n")
    pdf.append(compressed_content)
    pdf.append("\nendstream\n")
    pdf.append("endobj\n")

    # Font
    pdf.append("5 0 obj\n")
    pdf.append("<< /Type /Font\n")
    pdf.append("   /Subtype /Type1\n")
    pdf.append("   /BaseFont /Helvetica\n")
    pdf.append(">>\n")
    pdf.append("endobj\n")

    # Image XObject
    pdf.append("6 0 obj\n")
    pdf.append(f"<< /Type /XObject\n")
    pdf.append(f"   /Subtype /Image\n")
    pdf.append(f"   /Width {img_width}\n")
    pdf.append(f"   /Height {img_height}\n")
    pdf.append(f"   /BitsPerComponent 1\n")
    pdf.append(f"   /ColorSpace /DeviceGray\n")
    pdf.append(f"   /Length {len(compressed_img)}\n")
    pdf.append(f"   /Filter /FlateDecode\n")
    pdf.append(f">>\n")
    pdf.append("stream\n")
    pdf.append(compressed_img)
    pdf.append("\nendstream\n")
    pdf.append("endobj\n")

    # Cross-reference table (simplified)
    xref_offset = len(b"".join(pdf))
    pdf.append(f"xref\n")
    pdf.append(f"0 7\n")
    pdf.append(f"0000000000 65535 f \n")
    pdf.append(f"{sum([len(p.encode()) for p in pdf[:1]])} 00000 n \n")
    # ... this would need proper offset calculation

    # For simplicity, let's create a different approach
    return create_minimal_hybrid_pdf()

def create_scanned_image_pattern(width, height):
    """Create a simple 1-bit image pattern representing scanned text lines."""
    # Create alternating black and white horizontal lines to simulate text
    bytes_per_row = (width + 7) // 8
    img_data = b""

    for y in range(height):
        row = bytearray(bytes_per_row)
        # Create horizontal lines every 20 pixels to simulate text
        if (y % 20) < 10:
            # Fill with some pattern to represent text
            for x in range(0, width, 80):
                byte_offset = x // 8
                bit_offset = x % 8
                if byte_offset < len(row):
                    row[byte_offset] |= (0x80 >> bit_offset)
        img_data += bytes(row)

    return img_data

def create_minimal_hybrid_pdf():
    """Create a minimal hybrid PDF with vector text and image overlay."""

    width, height = 612, 792
    header_height = 120

    # Create image data for scanned body
    img_width = width
    img_height = height - header_height
    img_data = create_scanned_image_pattern(img_width, img_height)
    compressed_img = zlib.compress(img_data)

    # Vector content stream: draw text header, then image below
    content = f"""BT
/F1 14 Tf
50 750 Td
(VECTOR HEADER - ACME Corporation) Tj
0 -25 Td
(Financial Report 2024) Tj
0 -25 Td
(Confidential - Do Not Distribute) Tj
ET
q
50 0 {width-100} {img_height} re
W n
0 -{header_height+100} translate
/Im1 Do
Q
"""

    compressed_content = zlib.compress(content.encode())

    # Build PDF with proper structure
    parts = []

    # PDF Header
    parts.append("%PDF-1.4\n")
    parts.append("%\xC2\xB5\xC2\xB6\n")  # Comment with binary chars

    # Object 1: Catalog
    catalog = """1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
"""
    parts.append(catalog)

    # Object 2: Pages
    pages = """2 0 obj
<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>
endobj
"""
    parts.append(pages)

    # Object 3: Page
    page = f"""3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 {width} {height} ] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> /XObject << /Im1 6 0 R >> >> >>
endobj
"""
    parts.append(page)

    # Object 4: Content stream
    content_obj = f"""4 0 obj
<< /Length {len(compressed_content)} /Filter /FlateDecode >>
stream
{compressed_content}
endstream
endobj
"""
    parts.append(content_obj)

    # Object 5: Font
    font = """5 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
"""
    parts.append(font)

    # Object 6: Image
    image = f"""6 0 obj
<< /Type /XObject /Subtype /Image /Width {img_width} /Height {img_height} /BitsPerComponent 1 /ColorSpace /DeviceGray /Length {len(compressed_img)} /Filter /FlateDecode >>
stream
{compressed_img}
endstream
endobj
"""
    parts.append(image)

    # Now build with proper xref
    pdf_content = "".join(parts)
    pdf_bytes = pdf_content.encode()

    # Calculate object offsets
    obj_offsets = {}
    offset = 0
    i = 0
    while i < len(pdf_bytes):
        # Look for object numbers like "1 0 obj"
        match = None
        for j in range(i, min(i + 20, len(pdf_bytes))):
            if pdf_bytes[j:j+1] == b'\n' or pdf_bytes[j:j+1] == b' ':
                break
        # Scan for "N 0 obj" pattern
        for num in range(1, 20):
            pattern = f"{num} 0 obj".encode()
            if pdf_bytes[i:i+len(pattern)] == pattern:
                obj_offsets[num] = offset
                break

        offset += 1
        i += 1

    # Rebuild properly
    return create_hybrid_pdf_simple()

def create_hybrid_pdf_simple():
    """Create a simple hybrid PDF using minimal structure."""

    # Very simple approach: create PDF step by step
    pdf = []

    # Header
    pdf.append("%PDF-1.4\n")

    # Collect all object data first
    objects = {}

    # Catalog
    objects[1] = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n"

    # Pages
    objects[2] = b"2 0 obj\n<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>\nendobj\n"

    # Page
    objects[3] = b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 612 792 ] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> /XObject << /Im1 6 0 R >> >> >>\nendobj\n"

    # Content stream with vector text + image
    content = b"""BT
/F1 14 Tf
50 750 Td
(VECTOR HEADER - ACME Corp) Tj
0 -25 Td
(Annual Report 2024) Tj
ET
q
0 0 612 672 re
W n
/Im1 Do
Q
"""
    compressed = zlib.compress(content)
    objects[4] = f"4 0 obj\n<< /Length {len(compressed)} /Filter /FlateDecode >>\nstream\n".encode() + compressed + b"\nendstream\nendobj\n"

    # Font
    objects[5] = b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n"

    # Image (scanned body)
    img_data = create_scanned_image_pattern(612, 672)
    compressed_img = zlib.compress(img_data)
    objects[6] = f"6 0 obj\n<< /Type /XObject /Subtype /Image /Width 612 /Height 672 /BitsPerComponent 1 /ColorSpace /DeviceGray /Length {len(compressed_img)} /Filter /FlateDecode >>\nstream\n".encode() + compressed_img + b"\nendstream\nendobj\n"

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
    output_path = "tests/fixtures/hybrid/hybrid-001-vector-header-over-scan.pdf"
    pdf_bytes = create_hybrid_pdf_simple()

    with open(output_path, 'wb') as f:
        f.write(pdf_bytes)

    print(f"Created {output_path} ({len(pdf_bytes)} bytes)")

if __name__ == "__main__":
    main()
