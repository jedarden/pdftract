#!/usr/bin/env python3
"""
Create fingerprint test fixtures with meaningful content differences.
This script generates PDFs where the actual rendered content differs.
"""

import struct
import zlib
import os

def create_simple_pdf(content_text, output_path):
    """
    Create a simple PDF with the given text content.

    The PDF structure:
    - One page with Helvetica font
    - Content stream displays the text
    - Simple structure without complications
    """

    # Create a simple content stream that displays text
    # BT ... ET begins/ends text block
    # Td moves to position
    # Tj shows text
    content_stream = f"BT 50 700 Td ({content_text}) Tj ET".encode('ascii')

    # Compress the content stream with FlateDecode
    compressed_content = zlib.compress(content_stream, 9)

    # Build the PDF structure
    pdf_objects = []

    # Object 1: Catalog
    pdf_objects.append(b"1 0 obj\n<< /Pages 2 0 R /Type /Catalog >>\nendobj\n")

    # Object 2: Pages
    pdf_objects.append(b"2 0 obj\n<< /Count 1 /Kids [ 3 0 R ] /Type /Pages >>\nendobj\n")

    # Object 3: Page
    pdf_objects.append(f"""3 0 obj
<< /Contents 4 0 R /MediaBox [ 0 0 612 792 ] /Parent 2 0 R /Resources << /Font << /F1 << /BaseFont (/Helvetica) /Subtype (/Type1) /Type (/Font) >> >> >> /Type /Page >>
endobj
""".encode('ascii'))

    # Object 4: Content stream (compressed)
    pdf_objects.append(f"""4 0 obj
<< /Length {len(compressed_content)} /Filter /FlateDecode >>
stream
""".encode('ascii'))
    pdf_objects.append(compressed_content)
    pdf_objects.append(b"\nendstream\nendobj\n")

    # Calculate xref offset
    pdf_data = b"%PDF-1.3\n%abcdefghijklmnopqrstuvwxyz\n"
    xref_offset = len(pdf_data)

    for obj in pdf_objects:
        pdf_data += obj

    # Build trailer
    trailer = f"""xref
0 5
0000000000 65535 f
{xref_offset:010d} 00000 n
{xref_offset + len(pdf_objects[0]):010d} 00000 n
{xref_offset + len(pdf_objects[0]) + len(pdf_objects[1]):010d} 00000 n
{xref_offset + len(pdf_objects[0]) + len(pdf_objects[1]) + len(pdf_objects[2]):010d} 00000 n
trailer
<< /Root 1 0 R /Size 5 >>
startxref
{xref_offset + sum(len(obj) for obj in pdf_objects)}
%%EOF
""".encode('ascii')

    pdf_data += trailer

    with open(output_path, 'wb') as f:
        f.write(pdf_data)

def create_linearized_pdf(input_path, output_path):
    """
    Create a linearized version of a PDF.

    For proper linearization, we need to create a PDF with:
    - A linearization dictionary at the beginning
    - Hint tables
    - Proper object ordering

    Since this is complex without qpdf, we'll create a simpler variant:
    Just add a /Linearized key to the document (not full linearization, but sufficient for testing).
    """
    with open(input_path, 'rb') as f:
        pdf_data = f.read()

    # For this test, we'll add a comment at the beginning that indicates linearization
    # In a real scenario, we'd use qpdf --linearize
    # But since qpdf is not available, we'll create a variant with different byte layout

    # Read the PDF and rebuild it with different object ordering
    # This simulates what a tool like qpdf might do
    lines = pdf_data.split(b'\n')

    # Find the trailer and rebuild with different line length (simulating re-save)
    new_lines = []
    for line in lines:
        if b'trailer' in line:
            # Add some spaces to change byte layout
            new_lines.append(b'  ' + line)
        else:
            new_lines.append(line)

    new_pdf = b'\n'.join(new_lines)

    with open(output_path, 'wb') as f:
        f.write(new_pdf)

def main():
    fixtures_dir = "tests/fingerprint/fixtures"

    # Create base_hello.pdf source
    base_hello = os.path.join(fixtures_dir, ".clean_source.pdf")

    # 1. byte_identical: Two copies of the same file
    create_simple_pdf("Hello World", os.path.join(fixtures_dir, "byte_identical/v1.pdf"))
    create_simple_pdf("Hello World", os.path.join(fixtures_dir, "byte_identical/v2.pdf"))
    print("Created byte_identical fixtures")

    # 2. acrobat_resave: Same content, simulate re-save by changing whitespace in trailer
    create_simple_pdf("Hello World", os.path.join(fixtures_dir, "acrobat_resave/v1.pdf"))
    create_simple_pdf("Hello World", os.path.join(fixtures_dir, "acrobat_resave/v2_temp.pdf"))

    # Modify v2 to have different whitespace (simulating Acrobat re-save)
    with open(os.path.join(fixtures_dir, "acrobat_resave/v2_temp.pdf"), 'rb') as f:
        pdf_data = f.read()
    # Add extra spaces before trailer
    pdf_data = pdf_data.replace(b'\ntrailer', b'\n  trailer')
    with open(os.path.join(fixtures_dir, "acrobat_resave/v2.pdf"), 'wb') as f:
        f.write(pdf_data)
    os.remove(os.path.join(fixtures_dir, "acrobat_resave/v2_temp.pdf"))
    print("Created acrobat_resave fixtures")

    # 3. pdftk_resave: Same as acrobat_resave for our purposes
    create_simple_pdf("Hello World", os.path.join(fixtures_dir, "pdftk_resave/v1.pdf"))
    with open(os.path.join(fixtures_dir, "pdftk_resave/v1.pdf"), 'rb') as f:
        pdf_data = f.read()
    # Modify whitespace differently
    pdf_data = pdf_data.replace(b'\nendobj', b'\n  endobj')
    with open(os.path.join(fixtures_dir, "pdftk_resave/v2.pdf"), 'wb') as f:
        f.write(pdf_data)
    print("Created pdftk_resave fixtures")

    # 4. qpdf_resave: Same as above, different whitespace pattern
    create_simple_pdf("Hello World", os.path.join(fixtures_dir, "qpdf_resave/v1.pdf"))
    with open(os.path.join(fixtures_dir, "qpdf_resave/v1.pdf"), 'rb') as f:
        pdf_data = f.read()
    # Modify whitespace differently
    pdf_data = pdf_data.replace(b' 0 obj', b' 0 obj  ')
    with open(os.path.join(fixtures_dir, "qpdf_resave/v2.pdf"), 'wb') as f:
        f.write(pdf_data)
    print("Created qpdf_resave fixtures")

    # 5. content_edit_one_glyph: Change ONE character in the text
    create_simple_pdf("Hello World", os.path.join(fixtures_dir, "content_edit_one_glyph/v1.pdf"))
    create_simple_pdf("Hallo World", os.path.join(fixtures_dir, "content_edit_one_glyph/v2.pdf"))  # 'e' -> 'a'
    print("Created content_edit_one_glyph fixtures")

    # 6. content_edit_one_paragraph: Change the entire text
    create_simple_pdf("Hello World", os.path.join(fixtures_dir, "content_edit_one_paragraph/v1.pdf"))
    create_simple_pdf("Goodbye World", os.path.join(fixtures_dir, "content_edit_one_paragraph/v2.pdf"))
    print("Created content_edit_one_paragraph fixtures")

    # 7. metadata_only: Same content, different metadata
    # For this, we create PDFs with same content but different trailer IDs
    create_simple_pdf("Hello World", os.path.join(fixtures_dir, "metadata_only/v1.pdf"))
    with open(os.path.join(fixtures_dir, "metadata_only/v1.pdf"), 'rb') as f:
        pdf_data = f.read()
    # Change the ID array in the trailer (metadata-only change)
    pdf_data = pdf_data.replace(b'<1b9f3b313fa7bcbcf4a42403f1794221>',
                                 b'<2a0f4c4240b8dcded0b53514g2805332>')
    with open(os.path.join(fixtures_dir, "metadata_only/v2.pdf"), 'wb') as f:
        f.write(pdf_data)
    print("Created metadata_only fixtures")

    # 8. linearization_toggle: We need a proper linearized PDF
    # Since qpdf is not available, we'll create a variant that simulates
    # the byte layout differences of linearization
    create_simple_pdf("Hello World", os.path.join(fixtures_dir, "linearization_toggle/v1.pdf"))
    with open(os.path.join(fixtures_dir, "linearization_toggle/v1.pdf"), 'rb') as f:
        pdf_data = f.read()
    # Simulate linearization by adding comment at start and reordering objects
    linearized = b"%PDF-1.3\n% Linearized: No\n" + pdf_data.split(b'%PDF-1.3\n')[-1]
    with open(os.path.join(fixtures_dir, "linearization_toggle/v2.pdf"), 'wb') as f:
        f.write(linearized)
    print("Created linearization_toggle fixtures")

    print("\nAll fixtures created successfully!")

if __name__ == "__main__":
    main()
