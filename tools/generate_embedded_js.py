#!/usr/bin/env python3
"""Generate embedded-js.pdf fixture for TH-04 JavaScript detection testing.

This creates a minimal PDF with 3 JavaScript actions at different locations:
1. Catalog /OpenAction -> /JS containing app.alert("pwn")
2. Page 0 /AA -> /O (open action) -> /JS containing a second alert
3. Page 1 annotation /A -> /JS containing a third snippet

Generated PDF is valid and can be parsed by pdftract, but the JavaScript
is NEVER executed - only detected and reported.

Usage:
    python3 generate_embedded_js.py [--output PATH]

Options:
    --output PATH    Output PDF path (default: tests/fixtures/security/embedded-js.pdf)
    --help           Show this help message
"""

import argparse
import struct

def write_pdf(output_path):
    """Generate a minimal PDF with embedded JavaScript actions.

    Args:
        output_path: Where to write the PDF file
    """

    # PDF version 1.4
    pdf_header = b"%PDF-1.4\n\n"

    # Object 0: Pages root (dummy, will be overwritten)
    # Object 1: Page 0 (with /AA /O JavaScript)
    obj1 = """1 0 obj
<<
/Type/Page
/MediaBox[0 0 612 792]
/Parent 0 0 R
/Resources<<
/Font<<
/F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>
>>
>>
/Contents 2 0 R
/AA<<
/O<</S/JavaScript/JS(app.alert('page_open'))>>
>>
>>
endobj

"""

    # Object 2: Content stream for Page 0 (simple text)
    obj2 = """2 0 obj
<<
/Length 44
>>
stream
BT
/F1 12 Tf
100 700 Td
(Page 0) Tj
ET
endstream
endobj

"""

    # Object 3: Page 1 (with annotation containing JavaScript)
    obj3 = """3 0 obj
<<
/Type/Page
/MediaBox[0 0 612 792]
/Parent 0 0 R
/Resources<<
/Font<<
/F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>
>>
>>
/Contents 4 0 R
/Annots[5 0 R]
>>
endobj

"""

    # Object 4: Content stream for Page 1
    obj4 = """4 0 obj
<<
/Length 44
>>
stream
BT
/F1 12 Tf
100 700 Td
(Page 1) Tj
ET
endstream
endobj

"""

    # Object 5: Annotation on Page 1 with JavaScript action
    obj5 = """5 0 obj
<<
/Type/Annot
/Subtype/Link
/Rect[100 600 200 620]
/A<</S/JavaScript/JS(app.alert('annot_action'))>>
>>
endobj

"""

    # Object 6: Pages catalog
    obj6 = """6 0 obj
<<
/Type/Pages
/Count 2
/Kids[1 0 R 3 0 R]
>>
endobj

"""

    # Object 7: Catalog with /OpenAction JavaScript
    obj7 = """7 0 obj
<<
/Type/Catalog
/Pages 6 0 R
/OpenAction<</S/JavaScript/JS(app.alert("pwn"))>>
>>
endobj

"""

    # Combine all objects
    pdf_content = obj1 + obj2 + obj3 + obj4 + obj5 + obj6 + obj7

    # Calculate offsets for xref table
    offsets = []
    current_offset = len(pdf_header)

    # Split into individual objects to calculate offsets
    objects = [obj1, obj2, obj3, obj4, obj5, obj6, obj7]

    for obj in objects:
        offsets.append(current_offset)
        current_offset += len(obj.encode('latin1'))

    # Build xref table
    xref = f"xref\n0 8\n0000000000 65535 f \n"
    for i, offset in enumerate(offsets, start=1):
        xref += f"{offset:010d} 00000 n \n"

    # Trailer
    trailer = f"""trailer
<<
/Size 8
/Root 7 0 R
>>
startxref
{current_offset}
%%
%%EOF
"""

    # Write complete PDF
    complete_pdf = pdf_header + pdf_content.encode('latin1') + xref.encode('latin1') + trailer.encode('latin1')

    with open(output_path, "wb") as f:
        f.write(complete_pdf)

    print(f"Generated {output_path}")
    print("JavaScript actions:")
    print("  1. Catalog /OpenAction: app.alert(\"pwn\")")
    print("  2. Page 0 /AA /O: app.alert('page_open')")
    print("  3. Page 1 annotation /A: app.alert('annot_action')")


def main():
    parser = argparse.ArgumentParser(
        description="Generate PDF fixture with embedded JavaScript for TH-04 testing",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )

    parser.add_argument(
        '--output',
        default='tests/fixtures/security/embedded-js.pdf',
        help='Output PDF path (default: tests/fixtures/security/embedded-js.pdf)'
    )

    args = parser.parse_args()

    write_pdf(args.output)


if __name__ == "__main__":
    main()
