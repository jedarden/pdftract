#!/usr/bin/env python3
"""
Generate encrypted PDF test fixtures for pdftract.

This script creates four test PDFs with different encryption levels:
- EC-04: RC4-40 encrypted PDF (V=1, R=2)
- EC-05: AES-128 encrypted PDF (V=4, R=4)
- EC-06: AES-256 encrypted PDF (V=5, R=6)
- EC-empty-password: PDF with empty password (decrypts without --password)

All PDFs use user password "test" and contain the same simple content.
"""

import pikepdf

# Simple minimal PDF content
MINIMAL_PDF = b"""%PDF-1.4
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
/Length 83
>>
stream
BT
/F1 12 Tf
100 700 Td
(Hello, World!) Tj
100 680 Td
(This is a test PDF for encryption.) Tj
100 660 Td
(Page 1 content) Tj
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
465
%%EOF
"""

def create_base_pdf():
    """Create a simple base PDF with known content."""
    # Load the minimal PDF from bytes
    import io
    return pikepdf.open(io.BytesIO(MINIMAL_PDF))

def create_rc4_encrypted_pdf(password="test"):
    """Create RC4-40 encrypted PDF (V=1, R=2)."""
    pdf = create_base_pdf()

    # Encrypt with RC4-40 (V=1, R=2)
    pdf.save(
        "tests/fixtures/EC-04-rc4-encrypted.pdf",
        encryption=pikepdf.Encryption(
            owner="",
            user=password,
            R=2,  # RC4-40
            allow=None
        )
    )

    print("Created EC-04-rc4-encrypted.pdf (RC4-40, V=1, R=2, user password: 'test')")

def create_aes128_encrypted_pdf(password="test"):
    """Create AES-128 encrypted PDF (V=4, R=4)."""
    pdf = create_base_pdf()

    # Encrypt with AES-128 (V=4, R=4)
    pdf.save(
        "tests/fixtures/EC-05-aes128-encrypted.pdf",
        encryption=pikepdf.Encryption(
            owner="",
            user=password,
            R=4,  # AES-128
            allow=None
        )
    )

    print("Created EC-05-aes128-encrypted.pdf (AES-128, V=4, R=4, user password: 'test')")

def create_aes256_encrypted_pdf(password="test"):
    """Create AES-256 encrypted PDF (V=5, R=6)."""
    pdf = create_base_pdf()

    # Encrypt with AES-256 (V=5, R=6)
    pdf.save(
        "tests/fixtures/EC-06-aes256-encrypted.pdf",
        encryption=pikepdf.Encryption(
            owner="",
            user=password,
            R=6,  # AES-256 (PDF 2.0)
            allow=None
        )
    )

    print("Created EC-06-aes256-encrypted.pdf (AES-256, V=5, R=6, user password: 'test')")

def create_empty_password_pdf():
    """Create PDF with empty owner password (decrypts without --password)."""
    pdf = create_base_pdf()

    # Encrypt with empty passwords - should decrypt with empty string
    pdf.save(
        "tests/fixtures/EC-empty-password.pdf",
        encryption=pikepdf.Encryption(
            owner="",
            user="",
            R=2,
            allow=None
        )
    )

    print("Created EC-empty-password.pdf (empty password, decrypts without --password)")

if __name__ == "__main__":
    import io
    import os

    # Create fixtures directory if it doesn't exist
    os.makedirs("tests/fixtures", exist_ok=True)

    try:
        create_rc4_encrypted_pdf("test")
        create_aes128_encrypted_pdf("test")
        create_aes256_encrypted_pdf("test")
        create_empty_password_pdf()
        print("\nAll encrypted fixtures created successfully!")
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        print("\nNote: This script requires pikepdf.")
        print("Install with: pip install pikepdf")
