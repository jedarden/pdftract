#!/usr/bin/env python3
"""Generate tagged PDF fixtures for testing Phase 7.1.4 coverage check.

Creates three fixtures:
1. tagged-suspects-true.pdf - Suspects true, 60% coverage -> fallback to XY-cut
2. tagged-suspects-false.pdf - Suspects false, 50% coverage -> trust StructTree
3. tagged-suspects-true-high-coverage.pdf - Suspects true, 95% coverage -> trust StructTree
"""

import struct

def write_pdf(path, suspects, num_claimed, num_total):
    """Write a tagged PDF with the given parameters."""

    # Create ParentTree /Nums array with claimed and null entries
    nums_content = f"  /Nums [\n    0 ["
    for i in range(num_total):
        if i < num_claimed:
            nums_content += " 5 0 R"
        else:
            nums_content += " null"
        if i < num_total - 1:
            nums_content += ' '
    nums_content += " ]\n  ]\n"

    # Create /K array for StructElem with MCIDs
    k_array = ' '.join(str(i) for i in range(num_total))

    # Create content stream with BDC/EMC marked content sequences for each MCID
    content_ops = []
    for i in range(num_total):
        y_pos = 700 - i * 15
        content_ops.extend([
            "BT",
            "/F1 12 Tf",
            f"100 {y_pos} Td",
            f"/MCID {i} BDC",
            f"(Test{i}) Tj",
            "EMC",
            "ET",
        ])
    content_stream = '\n'.join(content_ops)
    content_length = len(content_stream)

    # Build PDF content
    pdf_lines = [
        "%PDF-1.7",
        "",
        "1 0 obj",
        "<<",
        "/Type /Catalog",
        "/Pages 2 0 R",
        "/MarkInfo <<",
        "  /Marked true",
        f"  /Suspects {'true' if suspects else 'false'}",
        ">>",
        "/StructTreeRoot 3 0 R",
        ">>",
        "endobj",
        "",
        "2 0 obj",
        "<<",
        "/Type /Pages",
        "/Kids [4 0 R]",
        "/Count 1",
        ">>",
        "endobj",
        "",
        "3 0 obj",
        "<<",
        "/Type /StructTreeRoot",
        "/K [5 0 R]",
        "/ParentTree 6 0 R",
        ">>",
        "endobj",
        "",
        "4 0 obj",
        "<<",
        "/Type /Page",
        "/Parent 2 0 R",
        "/MediaBox [0 0 612 792]",
        "/Contents 7 0 R",
        "/StructParents 0",
        ">>",
        "endobj",
        "",
        "5 0 obj",
        "<<",
        "/Type /StructElem",
        "/S /P",
        f"/K [{k_array}]",
        ">>",
        "endobj",
        "",
        "6 0 obj",
        "<<",
        nums_content,
        ">>",
        "endobj",
        "",
        "7 0 obj",
        "<<",
        f"/Length {content_length}",
        ">>",
        "stream",
        content_stream,
        "endstream",
        "endobj",
    ]

    # Join content with newlines and calculate offsets
    pdf_content = '\n'.join(pdf_lines)
    pdf_bytes = pdf_content.encode('latin-1')

    # Calculate object offsets
    obj_offsets = [0] * 8  # Objects 0-7 (0 is always null)
    current_pos = 0

    for line in pdf_lines:
        # Check if this line starts an object definition
        if line.endswith(" 0 obj"):
            obj_num = int(line.split()[0])
            obj_offsets[obj_num] = current_pos
        current_pos += len(line) + 1  # +1 for newline

    # Build xref table
    xref_lines = [
        "xref",
        "0 8",
        f"0000000000 65535 f ",
    ]
    for i in range(1, 8):
        xref_lines.append(f"{obj_offsets[i]:010d} 00000 n ")
    xref_table = '\n'.join(xref_lines)

    # Calculate startxref (offset to xref table)
    startxref = len(pdf_bytes) + 1  # +1 for the newline before xref

    # Build trailer
    trailer = f"""trailer
<<
/Size 8
/Root 1 0 R
>>
startxref
{startxref}
%%EOF"""

    # Write complete PDF
    with open(path, 'wb') as f:
        f.write(pdf_bytes)
        f.write(b'\n')
        f.write(xref_table.encode('latin-1'))
        f.write(b'\n')
        f.write(trailer.encode('latin-1'))

    coverage = (num_claimed / num_total) * 100
    print(f"Created: {path}")
    print(f"  - /MarkInfo /Suspects: {suspects}")
    print(f"  - Coverage: {coverage:.0f}% ({num_claimed}/{num_total} MCIDs claimed)")
    if suspects and coverage < 80:
        print(f"  - Expected: fallback to XY-cut, reading_order_algorithm = 'xy_cut'")
    elif not suspects or coverage >= 80:
        print(f"  - Expected: trust StructTree, reading_order_algorithm = 'struct_tree'")

def main():
    print("Generating tagged PDF fixtures for Phase 7.1.4 coverage check...")
    print()

    # Fixture 1: Suspects true, 60% coverage -> fallback to XY-cut
    write_pdf("tests/fixtures/tagged-suspects-true.pdf", True, 6, 10)
    print()

    # Fixture 2: Suspects false, 50% coverage -> trust StructTree
    write_pdf("tests/fixtures/tagged-suspects-false.pdf", False, 5, 10)
    print()

    # Fixture 3: Suspects true, 95% coverage -> trust StructTree
    write_pdf("tests/fixtures/tagged-suspects-true-high-coverage.pdf", True, 19, 20)
    print()

    print("All fixtures generated successfully!")

if __name__ == "__main__":
    main()
