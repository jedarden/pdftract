#!/usr/bin/env python3
"""Create unmapped-comprehensive.pdf fixture for testing unmapped glyph handling.

This fixture tests the 4-level Unicode fallback chain failure path:
- Level 1 (ToUnicode CMap): Not present
- Level 2 (AGL lookup): 7 glyphs not in AGL
- Level 3 (Font fingerprint): Font not embedded
- Level 4 (Shape recognition): Disabled by default

Fixture design: notes/bf-68f9i-design.md
Glyph selection: notes/bf-68f9i-glyphs.md
"""

import os

def create_unmapped_comprehensive_pdf(output_path):
    """Create the unmapped-comprehensive.pdf fixture."""

    # Object 1: Catalog
    obj1 = "1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n"

    # Object 2: Pages
    obj2 = "2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj\n"

    # Object 3: Page dictionary
    obj3 = """3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Resources <<
/Font <<
/F1 5 0 R
>>
>>
/Contents 4 0 R
>>
endobj
"""

    # Object 4: Content stream
    # Line 1 (y=700): <000102> Tj → /g001, /g002, /g003 (PUA unmapped)
    # Line 2 (y=680): <03040506> Tj → /CustomA, /CustomB, /NotAGlyph, /glyph_0041 (unmapped)
    # Line 3 (y=660): <070809> Tj → /A, /B, /space (AGL mapped)
    content = """BT
/F1 12 Tf
50 700 Td
<000102> Tj
50 680 Td
<03040506> Tj
50 660 Td
<070809> Tj
ET
"""
    content_length = len(content)
    obj4 = f"4 0 obj\n<<\n/Length {content_length}\n>>\nstream\n{content}endstream\nendobj\n"

    # Object 5: Font dictionary with custom encoding
    # Codes 0-9 map to glyph names as specified in the design
    obj5 = """5 0 obj
<<
/Type /Font
/Subtype /Type1
/BaseFont /UnmappedTestFont
/Encoding <<
/Type /Encoding
/Differences [0 /g001 /g002 /g003 /CustomA /CustomB /NotAGlyph /glyph_0041 /A /B /space]
>>
>>
endobj
"""

    # Calculate offsets
    objects = [obj1, obj2, obj3, obj4, obj5]
    pdf_data = b"%PDF-1.4\n"
    offsets = [0]  # Offset 0 is for the "free" entry

    for obj in objects:
        offsets.append(len(pdf_data))
        pdf_data += obj.encode('latin-1')

    xref_offset = len(pdf_data)

    # Build xref table
    xref = "xref\n"
    xref += f"0 {len(offsets)}\n"
    xref += "0000000000 65535 f \n"
    for i, offset in enumerate(offsets[1:], start=1):
        xref += f"{offset:010d} 00000 n \n"

    # Build trailer
    trailer = "trailer\n"
    trailer += "<<\n"
    trailer += f"/Size {len(offsets)}\n"
    trailer += "/Root 1 0 R\n"
    trailer += ">>\n"
    trailer += f"startxref\n{xref_offset}\n"
    trailer += "%%EOF\n"

    pdf_data += xref.encode('latin-1')
    pdf_data += trailer.encode('latin-1')

    with open(output_path, 'wb') as f:
        f.write(pdf_data)

def create_ground_truth(output_path):
    """Create the ground truth text file for unmapped-comprehensive.pdf.

    Expected extraction output:
    - Line 1: ��� (3 U+FFFD for /g001, /g002, /g003)
    - Line 2: ��� (4 U+FFFD for /CustomA, /CustomB, /NotAGlyph, /glyph_0041)
    - Line 3: AB  (U+0041, U+0042, U+0020 for /A, /B, /space)
    """
    with open(output_path, 'w', encoding='utf-8') as f:
        # Line 1: 3 replacement characters
        f.write('���\n')
        # Line 2: 4 replacement characters
        f.write('����\n')
        # Line 3: A, B, space
        f.write('AB \n')

def main():
    fixtures_dir = os.path.dirname(os.path.abspath(__file__))
    pdf_path = os.path.join(fixtures_dir, "unmapped-comprehensive.pdf")
    txt_path = os.path.join(fixtures_dir, "unmapped-comprehensive.txt")

    create_unmapped_comprehensive_pdf(pdf_path)
    create_ground_truth(txt_path)

    print(f"Created unmapped-comprehensive fixtures:")
    print(f"  {pdf_path} ({os.path.getsize(pdf_path)} bytes)")
    print(f"  {txt_path} ({os.path.getsize(txt_path)} bytes)")
    print()
    print("Fixture structure:")
    print("  Character codes 0-2: /g001, /g002, /g003 (PUA unmapped)")
    print("  Character codes 3-6: /CustomA, /CustomB, /NotAGlyph, /glyph_0041 (unmapped)")
    print("  Character codes 7-9: /A, /B, /space (AGL mapped)")
    print()
    print("Expected extraction output:")
    print("  Line 1 (y=700): ���")
    print("  Line 2 (y=680): ���")
    print("  Line 3 (y=660): AB ")
    print()
    print("Expected diagnostics: 7 GLYPH_UNMAPPED warnings (one per unmapped glyph)")

if __name__ == '__main__':
    main()
