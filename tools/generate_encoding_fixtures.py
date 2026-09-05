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

# Character code -> glyph name table for no-mapping.pdf, taken verbatim from the
# fixture design (notes/bf-68f9i-design.md) and the glyph selection
# (notes/bf-68f9i-glyphs.md).
#
# Each entry is (char_code, glyph_name, unmapped, expected_extraction):
#   unmapped            - True when every level of the 4-level fallback chain must fail
#   expected_extraction - what a correct extractor emits for this code
#
# Codes 0-6 are unmapped: no /ToUnicode exists (Level 1), the names are absent from
# the Adobe Glyph List and match neither algorithmic convention `uniXXXX`/`uXXXXXX`
# (Level 2), the font is not embedded so there is nothing to fingerprint (Level 3),
# and no shape record exists (Level 4). They must each surface as U+FFFD.
# Codes 7-9 are standard AGL names, kept as the success-path control group.
NO_MAPPING_GLYPHS = [
    (0, "g001", True, "�"),        # PUA: arbitrary numeric name, not in AGL
    (1, "g002", True, "�"),        # PUA
    (2, "g003", True, "�"),        # PUA
    (3, "CustomA", True, "�"),     # custom encoding, meaningful-looking but non-AGL
    (4, "CustomB", True, "�"),     # custom encoding
    (5, "NotAGlyph", True, "�"),   # orphaned: named in /Differences, defined nowhere
    (6, "glyph_0041", True, "�"),  # hex digits, but `glyph_` is not an AGL
                                        # algorithmic prefix (uniXXXX / uXXXXXX only)
    (7, "A", False, "A"),               # AGL direct entry -> U+0041
    (8, "B", False, "B"),               # AGL direct entry -> U+0042
    (9, "space", False, " "),           # AGL direct entry -> U+0020
]

# Text lines as laid out in the content stream: line 1 is the PUA glyphs, line 2 the
# custom/orphaned/non-AGL names, line 3 the AGL control group.
NO_MAPPING_LINES = [
    (0, 1, 2),
    (3, 4, 5, 6),
    (7, 8, 9),
]


def no_mapping_ground_truth():
    """Expected extraction output, derived from the glyph table."""
    return "".join(expected for _, _, _, expected in NO_MAPPING_GLYPHS)


def create_no_mapping_pdf():
    """
    Create PDF with no ToUnicode CMap and custom encoding.

    This PDF uses a Type1 font whose /Differences array assigns the design-doc glyph
    names to character codes 0-9. Expected behavior: codes 0-6 fail all four recovery
    levels and surface as U+FFFD; codes 7-9 recover through Level 2 AGL lookup.

    Object offsets and the /Length of the content stream are computed rather than
    hard-coded, so editing NO_MAPPING_GLYPHS or NO_MAPPING_LINES cannot desynchronize
    the xref table from the objects it describes.
    """
    # /Differences is [starting_code /name /name ...]: the leading integer sets the
    # first code and each following name increments it, so the base code comes from
    # the table's first entry and every glyph name follows it in order.
    base_code = NO_MAPPING_GLYPHS[0][0]
    differences = " ".join(
        [str(base_code)] + [f"/{name}" for _, name, _, _ in NO_MAPPING_GLYPHS]
    )

    show_ops = "".join(
        "<{}> Tj\n".format("".join(f"{code:02X}" for code in codes))
        for codes in NO_MAPPING_LINES
    )
    content_ops = (
        "BT\n"
        "/F1 12 Tf\n"
        "50 700 Td\n"
        + show_ops +
        "ET"
    )

    # Object numbering: 1 Catalog, 2 Pages, 3 Page, 4 Font, 5 Content stream.
    objects = [
        b"<<\n/Type /Catalog\n/Pages 2 0 R\n>>",
        b"<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>",
        b"<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n"
        b"/Resources <<\n/Font <<\n/F1 4 0 R\n>>\n>>\n/Contents 5 0 R\n>>",
        (
            "<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /UnmappedTestFont\n"
            "/Encoding <<\n/Type /Encoding\n"
            f"/Differences [{differences}]\n"
            ">>\n>>"
        ).encode("ascii"),
        (
            f"<<\n/Length {len(content_ops)}\n>>\nstream\n{content_ops}\nendstream"
        ).encode("ascii"),
    ]

    pdf = bytearray(b"%PDF-1.4\n")
    offsets = []
    for number, body in enumerate(objects, start=1):
        offsets.append(len(pdf))
        pdf += f"{number} 0 obj\n".encode("ascii") + body + b"\nendobj\n"

    xref_offset = len(pdf)
    pdf += f"xref\n0 {len(objects) + 1}\n".encode("ascii")
    pdf += b"0000000000 65535 f \n"
    for offset in offsets:
        pdf += f"{offset:010d} 00000 n \n".encode("ascii")
    pdf += (
        "trailer\n"
        f"<<\n/Size {len(objects) + 1}\n/Root 1 0 R\n>>\n"
        f"startxref\n{xref_offset}\n"
        "%%EOF\n"
    ).encode("ascii")
    return bytes(pdf)

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
    # Ground truth: U+FFFD for each of the 7 unmapped codes, then the AGL control
    # group "AB " recovered via Level 2. Blocks are joined without separators,
    # matching how encoding_recovery.rs concatenates extracted text blocks.
    pdf1 = create_no_mapping_pdf()
    with open("tests/fixtures/encoding/no-mapping.pdf", "wb") as f:
        f.write(pdf1)
    with open("tests/fixtures/encoding/no-mapping.txt", "w", encoding="utf-8") as f:
        f.write(no_mapping_ground_truth())
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
