#!/usr/bin/env python3
"""Generate minimal stub PDF files for conformance testing."""

import struct
import zlib

def create_minimal_pdf(path, text="Test", title="Test Document"):
    """Create a minimal valid PDF file."""
    # Minimal PDF with text content
    pdf = f"""%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 4 0 R
/Resources <<
/Font <<
/F1 5 0 R
>>
>>
>>
endobj
4 0 obj
<<
/Length {len(text) + 50}
>>
stream
BT
/F1 12 Tf
50 700 Td
({text}) Tj
ET
endstream
endobj
5 0 obj
<<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
>>
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000274 00000 n
0000000389 00000 n
trailer
<<
/Size 6
/Root 1 0 R
>>
startxref
470
%%EOF
"""
    with open(path, 'wb') as f:
        f.write(pdf.encode('latin-1'))

def create_multi_page_pdf(path, num_pages, title="Multi-Page Document"):
    """Create a PDF with multiple pages."""
    pages = []
    objects = []
    xref_offset = 0

    # Create page objects
    for i in range(num_pages):
        page_num = 3 + i
        content_num = 3 + num_pages + i
        pages.append(f"{page_num} 0 R")

        objects.append(f"""{page_num} 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents {content_num} 0 R
/Resources <<
/Font <<
/F1 5 0 R
>>
>>
>>
endobj
""")

        objects.append(f"""{content_num} 0 obj
<<
/Length 50
>>
stream
BT
/F1 12 Tf
50 700 Td
(Page {i+1}) Tj
ET
endstream
endobj
""")

    # Build PDF
    pdf = f"""%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
/Title ({title})
>>
endobj
2 0 obj
<<
/Type /Pages
/Kids [{' '.join(pages)}]
/Count {num_pages}
>>
endobj
"""
    pdf += '\n'.join(objects)

    # Font object
    pdf += f"""5 0 obj
<<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
>>
endobj
"""

    xref_start = len(pdf.encode('latin-1'))
    pdf += f"xref\n0 {6 + num_pages * 2}\n0000000000 65535 f\n"

    # Simplified xref (offsets are approximate for stub PDFs)
    offset = 9
    for i in range(6 + num_pages * 2 - 1):
        pdf += f"{offset:010d} 00000 n\n"
        offset += 100

    pdf += f"""trailer
<<
/Size {6 + num_pages * 2}
/Root 1 0 R
>>
startxref
{xref_start}
%%EOF
"""

    with open(path, 'wb') as f:
        f.write(pdf.encode('latin-1'))

if __name__ == '__main__':
    import os
    import sys

    fixture_dir = os.path.dirname(os.path.abspath(__file__))

    # Create stub PDFs for missing fixtures
    stubs = [
        ('encrypted/encrypted.pdf', 'Encrypted PDF', 'test123'),
        ('fillable-form/form.pdf', 'Fillable Form'),
        ('mixed/mixed.pdf', 'Mixed Content'),
        ('large/50pages.pdf', 50),
        ('large/100pages.pdf', 100),
        ('vertical/vertical.pdf', 'Vertical Text'),
        ('code/code.pdf', 'Code Sample'),
        ('xmp/xmp-metadata.pdf', 'XMP Metadata'),
        ('receipts/valid-receipt.pdf', 'Valid Receipt'),
        ('receipts/valid-receipt.receipt.json', '{}'),
        ('receipts/tampered-receipt.pdf', 'Tampered Receipt'),
        ('receipts/tampered-receipt.receipt.json', '{}'),
        ('broken/corrupt.pdf', 'Broken PDF'),
    ]

    for stub in stubs:
        path = os.path.join(fixture_dir, stub[0])
        os.makedirs(os.path.dirname(path), exist_ok=True)

        if len(stub) == 2 and isinstance(stub[1], int):
            # Multi-page PDF
            create_multi_page_pdf(path, stub[1])
        elif len(stub) == 3 and isinstance(stub[2], str):
            # PDF with password placeholder (note: real encryption requires more)
            create_minimal_pdf(path, stub[1])
        elif stub[0].endswith('.json'):
            # Receipt file
            with open(path, 'w') as f:
                f.write('{"fingerprint": "stub", "signature": "stub"}')
        else:
            # Regular PDF
            create_minimal_pdf(path, stub[1])

        print(f"Created {stub[0]}")
