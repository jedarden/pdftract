#!/usr/bin/env python3
"""
Generate sensitive.pdf for TH-08 log audit test.

This script creates a password-protected PDF with unique, distinctive markers:
- Body text contains "UNIQUE-MARKER-IN-BODY-TEXT-7f9a"
- Password value is "UNIQUE-PASSWORD-FOR-TH08-7f9a"

These markers are specifically designed to be unlikely to appear
in normal log output, making substring-based leak detection reliable.
"""

import pikepdf
import io

# Constants for unique markers
BODY_TEXT = "UNIQUE-MARKER-IN-BODY-TEXT-7f9a"
PASSWORD = "UNIQUE-PASSWORD-FOR-TH08-7f9a"

# Minimal PDF content with the unique marker
MINIMAL_PDF = f"""%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Count 1
/Kids [3 0 R]
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Resources <<
/Font <<
/F1 <<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
>>
>>
>>
/Contents 4 0 R
>>
endobj
4 0 obj
<<
/Length {len(BODY_TEXT) + 30}
>>
stream
BT
/F1 12 Tf
100 700 Td
({BODY_TEXT}) Tj
ET
endstream
endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000350 00000 n
trailer
<<
/Size 5
/Root 1 0 R
>>
startxref
450
%%EOF
"""

def create_sensitive_pdf():
    """Create a password-protected PDF with unique markers."""
    # Load the minimal PDF from bytes
    base_pdf = pikepdf.open(io.BytesIO(MINIMAL_PDF.encode()))

    # Save with password protection
    output_path = "tests/fixtures/security/sensitive.pdf"
    base_pdf.save(
        output_path,
        encryption=pikepdf.Encryption(
            owner="",
            user=PASSWORD,
            R=2,  # RC4-40 (widest compatibility)
            aes=False,  # RC4 encryption for R=2
            allow=pikepdf.Permissions(
                accessibility=True,
                extract=True,
                modify_annotation=True,
                modify_assembly=False,
                modify_form=True,
                modify_other=True,
                print_lowres=True,
                print_highres=True
            ),
            metadata=False  # Can't encrypt metadata with R < 4
        )
    )

    print(f"Created {output_path}")
    print(f"  Password: {PASSWORD}")
    print(f"  Body text marker: {BODY_TEXT}")

if __name__ == "__main__":
    import os

    # Create security fixtures directory if it doesn't exist
    os.makedirs("tests/fixtures/security", exist_ok=True)

    try:
        create_sensitive_pdf()
        print("\nSensitive fixture created successfully for TH-08 log audit test!")
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        print("\nNote: This script requires pikepdf.")
        print("Install with: pip install pikepdf")
