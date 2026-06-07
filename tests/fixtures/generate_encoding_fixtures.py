#!/usr/bin/env python3
"""
Generate encoding test fixtures for Phase 2.2–2.5 Unicode recovery.

Creates four PDF fixtures exercising Level 2–4 Unicode recovery:
- no-mapping.pdf: Font with no ToUnicode and no standard encoding (worst case)
- agl-only.pdf: Font with only AGL glyph names (Level 2 recovery)
- fingerprint-match.pdf: Font embedded for fingerprint matching (Level 3)
- shape-match.pdf: Font for shape-based recognition (Level 4)

Each fixture has a paired .txt ground truth file.
"""

import os
import struct

def create_no_mapping_pdf():
    """
    Create PDF with no ToUnicode CMap and custom encoding.

    This PDF uses a Type1 font with custom glyph names that don't map to AGL.
    Expected behavior: All glyphs fail Levels 1-3, only Level 4 shape recognition
    might recover some content (U+FFFD otherwise).
    """
    pdf = b"""%PDF-1.4
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
/Resources <<
/Font <<
/F1 4 0 R
>>
>>
/Contents 5 0 R
>>
endobj
4 0 obj
<<
/Type /Font
/Subtype /Type1
/BaseFont /CustomFont
/Encoding <<
/Type /Encoding
/Differences [0 /g00 /g01 /g02 /g03 /g04 /g05]
>>
>>
endobj
5 0 obj
<<
/Length 65
>>
stream
BT
/F1 12 Tf
50 700 Td
/g00 /g01 /g02 /g03 Tj
50 680 Td
/g04 /g05 Tj
ET
endstream
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000348 00000 n
0000000509 00000 n
trailer
<<
/Size 6
/Root 1 0 R
>>
startxref
645
%%EOF
"""
    return pdf

def create_agl_only_pdf():
    """
    Create PDF with AGL-compatible glyph names but no ToUnicode.

    This PDF uses standard Type1 font with glyph names from the Adobe Glyph List.
    Expected behavior: Level 2 AGL lookup successfully recovers all content.
    Glyph names used: /H /e /l /o (Hello), /W /o /r /l /d (World)
    """
    pdf = b"""%PDF-1.4
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
/Resources <<
/Font <<
/F1 4 0 R
>>
>>
/Contents 5 0 R
>>
endobj
4 0 obj
<<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
>>
endobj
5 0 obj
<<
/Length 60
>>
stream
BT
/F1 12 Tf
100 700 Td
(Hello) Tj
100 680 Td
(World) Tj
ET
endstream
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000329 00000 n
0000000379 00000 n
trailer
<<
/Size 6
/Root 1 0 R
>>
startxref
512
%%EOF
"""
    return pdf

def create_fingerprint_match_pdf():
    """
    Create PDF with embedded font program for fingerprint matching.

    This PDF embeds a font program (BaseFont) that can be SHA-256 hashed.
    Expected behavior: Level 3 fingerprint lookup matches the embedded font
    and recovers content from the fingerprint database.
    """
    # This uses a minimal embedded font program (would be larger in production)
    pdf = b"""%PDF-1.4
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
/Resources <<
/Font <<
/F1 4 0 R
>>
>>
/Contents 5 0 R
>>
endobj
4 0 obj
<<
/Type /Font
/Subtype /Type1
/BaseFont /TestFingerprintFont
/FontDescriptor 6 0 R
>>
endobj
5 0 obj
<<
/Length 47
>>
stream
BT
/F1 12 Tf
100 700 Td
(Test) Tj
ET
endstream
endobj
6 0 obj
<<
/Type /FontDescriptor
/FontName /TestFingerprintFont
/Flags 4
/FontBBox [0 0 100 100]
/ItalicAngle 0
/Ascent 100
/Descent 0
/CapHeight 100
/StemV 80
/FontFile3 7 0 R
>>
endobj
7 0 obj
<<
/Length1 52
/Length2 28
/Length3 0
/Subtype /Type1C
/Length 80
>>
stream
%!PS-AdobeFont-1.0: TestFingerprintFont
%%CreationDate: Mon Jun 6 00:00:00 2026
% Minimal font program for fingerprint testing
endstream
endobj
xref
0 8
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000329 00000 n
0000000438 00000 n
0000000497 00000 n
0000000625 00000 n
trailer
<<
/Size 8
/Root 1 0 R
>>
startxref
765
%%EOF
"""
    return pdf

def create_shape_match_pdf():
    """
    Create PDF with subset font for shape-based recognition.

    This PDF uses a subset font (ABCDEF+Helvetica) with no ToUnicode.
    Expected behavior: Level 4 glyph shape recognition compares rendered
    glyph shapes against the shape database.
    """
    pdf = b"""%PDF-1.4
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
/Resources <<
/Font <<
/F1 4 0 R
>>
>>
/Contents 5 0 R
>>
endobj
4 0 obj
<<
/Type /Font
/Subtype /TrueType
/BaseFont /ABCDEF+Helvetica
/FontDescriptor 6 0 R
>>
endobj
5 0 obj
<<
/Length 42
>>
stream
BT
/F1 12 Tf
100 700 Td
(Shape) Tj
ET
endstream
endobj
6 0 obj
<<
/Type /FontDescriptor
/FontName /ABCDEF+Helvetica
/Flags 4
/FontBBox [0 0 100 100]
/ItalicAngle 0
/Ascent 100
/Descent 0
/CapHeight 100
/StemV 80
/FontFile2 7 0 R
>>
endobj
7 0 obj
<<
/Length 60
>>
stream
Minimal TrueType font program for shape testing
endstream
endobj
xref
0 8
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000329 00000 n
0000000477 00000 n
0000000536 00000 n
0000000664 00000 n
trailer
<<
/Size 8
/Root 1 0 R
>>
startxref
768
%%EOF
"""
    return pdf

def main():
    """Generate all encoding fixtures."""
    os.makedirs("tests/fixtures/encoding", exist_ok=True)

    # Fixture 1: no-mapping.pdf
    # Ground truth: mostly U+FFFD replacement chars, minimal recovery
    pdf1 = create_no_mapping_pdf()
    with open("tests/fixtures/encoding/no-mapping.pdf", "wb") as f:
        f.write(pdf1)
    # Ground truth: expected to be mostly U+FFFD with current implementation
    with open("tests/fixtures/encoding/no-mapping.txt", "w") as f:
        f.write("����\n��")
    print("Created: tests/fixtures/encoding/no-mapping.pdf")

    # Fixture 2: agl-only.pdf
    # Ground truth: "Hello\nWorld" (AGL successfully maps glyph names)
    pdf2 = create_agl_only_pdf()
    with open("tests/fixtures/encoding/agl-only.pdf", "wb") as f:
        f.write(pdf2)
    with open("tests/fixtures/encoding/agl-only.txt", "w") as f:
        f.write("Hello\nWorld")
    print("Created: tests/fixtures/encoding/agl-only.pdf")

    # Fixture 3: fingerprint-match.pdf
    # Ground truth: "Test" (fingerprint DB lookup succeeds)
    pdf3 = create_fingerprint_match_pdf()
    with open("tests/fixtures/encoding/fingerprint-match.pdf", "wb") as f:
        f.write(pdf3)
    with open("tests/fixtures/encoding/fingerprint-match.txt", "w") as f:
        f.write("Test")
    print("Created: tests/fixtures/encoding/fingerprint-match.pdf")

    # Fixture 4: shape-match.pdf
    # Ground truth: "Shape" (shape DB lookup succeeds)
    pdf4 = create_shape_match_pdf()
    with open("tests/fixtures/encoding/shape-match.pdf", "wb") as f:
        f.write(pdf4)
    with open("tests/fixtures/encoding/shape-match.txt", "w") as f:
        f.write("Shape")
    print("Created: tests/fixtures/encoding/shape-match.pdf")

    print("\nAll encoding fixtures created successfully!")

if __name__ == "__main__":
    main()
