#!/usr/bin/env python3
"""Generate unmapped glyph PDF fixtures with custom glyph names and encodings.

This generator creates PDF fixtures for testing unmapped glyph handling in pdftract.
It supports custom glyph names, custom encodings, and simple text layouts.

Usage:
    python3 generate_unmapped_glyphs.py [OPTIONS]

Options:
    --output PATH       Output PDF path (default: unmapped-glyphs.pdf)
    --ground-truth PATH  Ground truth .txt path (default: unmapped-glyphs.txt)
    --title NAME         Font base name (default: UnmappedTestFont)
    --glyphs JSON       Glyph mappings as JSON string (default: built-in test set)

Examples:
    # Generate with default test glyphs
    python3 generate_unmapped_glyphs.py

    # Generate with custom glyphs
    python3 generate_unmapped_glyphs.py --glyphs '{"0": "/CustomGlyph1", "1": "/CustomGlyph2"}'

    # Generate to specific output file
    python3 generate_unmapped_glyphs.py --output my-test.pdf --ground-truth my-test.txt

Default glyph set (10 character codes):
    - Codes 0-2: /g001, /g002, /g003 (PUA unmapped)
    - Codes 3-6: /CustomA, /CustomB, /NotAGlyph, /glyph_0041 (unmapped)
    - Codes 7-9: /A, /B, /space (AGL mapped)

This fixture tests the 4-level Unicode fallback chain failure path:
    - Level 1 (ToUnicode CMap): Not present
    - Level 2 (AGL lookup): 7 glyphs not in AGL
    - Level 3 (Font fingerprint): Font not embedded
    - Level 4 (Shape recognition): Disabled by default

Fixture design: notes/bf-68f9i-design.md
Glyph selection: notes/bf-68f9i-glyphs.md
"""

import argparse
import json
import os
import sys


# Default glyph mappings matching the design specification
DEFAULT_GLYPHS = {
    "0": "/g001",
    "1": "/g002",
    "2": "/g003",
    "3": "/CustomA",
    "4": "/CustomB",
    "5": "/NotAGlyph",
    "6": "/glyph_0041",
    "7": "/A",
    "8": "/B",
    "9": "/space"
}


def create_unmapped_glyph_pdf(output_path, glyphs, font_title="UnmappedTestFont"):
    """Create a PDF with custom glyph encoding.

    Args:
        output_path: Path to write the PDF file
        glyphs: Dict mapping character codes (as strings) to glyph names
        font_title: BaseFont name for the font dictionary
    """

    # Sort glyph codes numerically for the Differences array
    sorted_codes = sorted(glyphs.keys(), key=int)

    # Build the Differences array
    differences = " ".join([f"{code} {glyphs[code]}" for code in sorted_codes])
    first_code = sorted_codes[0]

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
    # Generate hex byte sequence for all character codes in order
    byte_sequence = "".join([f"{int(code):02x}" for code in sorted_codes])

    content = f"""BT
/F1 12 Tf
50 700 Td
<{byte_sequence}> Tj
ET
"""
    content_length = len(content)
    obj4 = f"4 0 obj\n<<\n/Length {content_length}\n>>\nstream\n{content}endstream\nendobj\n"

    # Object 5: Font dictionary with custom encoding
    obj5 = f"""5 0 obj
<<
/Type /Font
/Subtype /Type1
/BaseFont /{font_title}
/Encoding <<
/Type /Encoding
/Differences [{first_code} {differences}]
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


def create_ground_truth(output_path, glyphs):
    """Create ground truth text file.

    For this fixture, we can't automatically determine which glyphs map to Unicode
    without the AGL database, so we create a placeholder that documents the structure.

    Args:
        output_path: Path to write the ground truth file
        glyphs: Dict mapping character codes to glyph names
    """
    with open(output_path, 'w', encoding='utf-8') as f:
        # Document the glyph structure
        f.write(f"# Ground truth for unmapped glyph fixture\n")
        f.write(f"# {len(glyphs)} character codes mapped\n\n")

        sorted_codes = sorted(glyphs.keys(), key=int)
        for code in sorted_codes:
            glyph_name = glyphs[code]
            f.write(f"# Code {code} → {glyph_name}\n")

        # For the default test set, we know the expected output
        if glyphs == DEFAULT_GLYPHS:
            f.write("\n# Expected extraction output:\n")
            f.write("���\n")      # Line 1: g001, g002, g003
            f.write("����\n")      # Line 2: CustomA, CustomB, NotAGlyph, glyph_0041
            f.write("AB \n")       # Line 3: A, B, space


def parse_glyphs_arg(glyphs_str):
    """Parse glyph mappings from JSON string.

    Args:
        glyphs_str: JSON string like '{"0": "/g001", "1": "/g002"}'

    Returns:
        Dict mapping character codes (as strings) to glyph names
    """
    try:
        glyphs = json.loads(glyphs_str)
        # Ensure all keys are strings
        return {str(k): v for k, v in glyphs.items()}
    except json.JSONDecodeError as e:
        print(f"Error parsing --glyphs JSON: {e}", file=sys.stderr)
        sys.exit(1)


def main():
    parser = argparse.ArgumentParser(
        description="Generate unmapped glyph PDF fixtures for testing",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )

    parser.add_argument(
        '--output',
        default='unmapped-glyphs.pdf',
        help='Output PDF path (default: unmapped-glyphs.pdf)'
    )
    parser.add_argument(
        '--ground-truth',
        default='unmapped-glyphs.txt',
        help='Ground truth .txt path (default: unmapped-glyphs.txt)'
    )
    parser.add_argument(
        '--title',
        default='UnmappedTestFont',
        help='Font base name (default: UnmappedTestFont)'
    )
    parser.add_argument(
        '--glyphs',
        type=str,
        help='Glyph mappings as JSON string (default: built-in test set)'
    )
    parser.add_argument(
        '--no-ground-truth',
        action='store_true',
        help='Skip generating ground truth file'
    )

    args = parser.parse_args()

    # Use default glyphs or parse from argument
    if args.glyphs:
        glyphs = parse_glyphs_arg(args.glyphs)
    else:
        glyphs = DEFAULT_GLYPHS.copy()

    # Generate PDF
    create_unmapped_glyph_pdf(args.output, glyphs, args.title)

    # Generate ground truth
    if not args.no_ground_truth:
        create_ground_truth(args.ground_truth, glyphs)

    # Print summary
    print(f"Generated unmapped glyph fixture:")
    print(f"  PDF: {args.output} ({os.path.getsize(args.output)} bytes)")
    if not args.no_ground_truth:
        print(f"  Ground truth: {args.ground_truth} ({os.path.getsize(args.ground_truth)} bytes)")

    print(f"\nFixture contains {len(glyphs)} character codes:")
    sorted_codes = sorted(glyphs.keys(), key=int)
    for code in sorted_codes:
        print(f"  Code {code} → {glyphs[code]}")

    if glyphs == DEFAULT_GLYPHS:
        print("\nExpected extraction output:")
        print("  Line 1: ��� (3 U+FFFD for /g001, /g002, /g003)")
        print("  Line 2: ��� (4 U+FFFD for /CustomA, /CustomB, /NotAGlyph, /glyph_0041)")
        print("  Line 3: AB  (U+0041, U+0042, U+0020 for /A, /B, /space)")
        print("\nExpected diagnostics: 7 GLYPH_UNMAPPED warnings")


if __name__ == '__main__':
    main()
