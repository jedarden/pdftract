#!/usr/bin/env python3
"""Create minimal valid PDF fixtures with proper xref tables."""

import os
import re

def create_simple_pdf(fixture_name, extra_catalog_entries=None, extra_objects=None):
    """
    Create a minimal valid PDF with proper xref table.

    Args:
        fixture_name: Name of the fixture (without .pdf)
        extra_catalog_entries: Extra dictionary entries to add to catalog (e.g., /OCProperties)
        extra_objects: List of (obj_num, dict_string) tuples for additional objects
    """
    output_path = f"/home/coding/pdftract/tests/document_model/fixtures/{fixture_name}.pdf"

    # Base PDF content
    lines = [
        "%PDF-1.4",
        "",
        "0 0 obj",
        "<</Type/Pages/Count 2/Kids[1 0 R 2 0 R]>>",
        "endobj",
        "",
        "1 0 obj",
        "<</Type/Page/MediaBox[0 0 612 792]/Parent 0 0 R/Contents 3 0 R/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>",
        "endobj",
        "",
        "2 0 obj",
        "<</Type/Page/MediaBox[0 0 612 792]/Parent 0 0 R/Contents 4 0 R/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>",
        "endobj",
        "",
        "3 0 obj",
        "<</Length 44>>",
        "stream",
        "BT",
        "/F1 12 Tf",
        "100 700 Td",
        "(Page 1) Tj",
        "ET",
        "endstream",
        "endobj",
        "",
        "4 0 obj",
        "<</Length 44>>",
        "stream",
        "BT",
        "/F1 12 Tf",
        "100 700 Td",
        "(Page 2) Tj",
        "ET",
        "endstream",
        "endobj",
        "",
    ]

    # Add catalog object (will be object 5, unless extra_objects shift it)
    catalog_obj_num = 5

    # Add extra objects if provided (before catalog)
    if extra_objects:
        for obj_num, obj_content in extra_objects:
            lines.append(f"{obj_num} 0 obj")
            lines.append(obj_content)
            lines.append("endobj")
            lines.append("")

    # Build catalog with optional extra entries
    if extra_catalog_entries:
        catalog_dict = f"<</Type/Catalog/Pages 0 0 R {extra_catalog_entries}>>"
    else:
        catalog_dict = "<</Type/Catalog/Pages 0 0 R>>"

    lines.append(f"{catalog_obj_num} 0 obj")
    lines.append(catalog_dict)
    lines.append("endobj")
    lines.append("")

    # Build full PDF content (without xref/trailer)
    full_pdf = "\n".join(lines)

    # Calculate object offsets by finding byte positions
    obj_offsets = {}
    for match in re.finditer(r'(\d+) 0 obj', full_pdf):
        obj_num = int(match.group(1))
        obj_offsets[obj_num] = match.start()

    # Calculate xref offset
    xref_offset = len(full_pdf) + 1  # +1 for the newline after full_pdf

    # Build xref table
    max_obj = max(obj_offsets.keys()) if obj_offsets else catalog_obj_num
    xref_lines = [
        f"xref",
        f"0 {max_obj + 1}",
        f"0000000000 65535 f ",
    ]

    for obj_num in range(1, max_obj + 1):
        if obj_num in obj_offsets:
            xref_lines.append(f"{obj_offsets[obj_num]:010d} 00000 n ")
        else:
            # Free entry - shouldn't happen but handle it
            xref_lines.append(f"0000000000 65535 f ")

    # Build trailer
    trailer_lines = [
        "trailer",
        f"<</Size {max_obj + 1}/Root {catalog_obj_num} 0 R>>",
        f"startxref",
        f"{xref_offset}",
        f"%%EOF",
    ]

    final_pdf = full_pdf + "\n" + "\n".join(xref_lines) + "\n" + "\n".join(trailer_lines)

    with open(output_path, 'w') as f:
        f.write(final_pdf)

    print(f"Created {output_path}")


def create_ocg_default_off():
    """Create OCG fixture with /D /BaseState /OFF."""
    extra_objects = [
        (6, "<</Type/OCG/Name(Test Layer)>>"),
        (7, "<</BaseState/OFF/ON[]>>"),
        (8, "<</OCGs[6 0 R]/D 7 0 R>>"),
    ]
    create_simple_pdf("ocg_default_off", extra_catalog_entries="/OCProperties 8 0 R", extra_objects=extra_objects)


def create_missing_mediabox():
    """Create PDF with missing MediaBox (EC-09)."""
    output_path = "/home/coding/pdftract/tests/document_model/fixtures/missing_mediabox.pdf"

    lines = [
        "%PDF-1.4",
        "",
        "0 0 obj",
        "<</Type/Pages/Count 1/Kids[1 0 R]>>",
        "endobj",
        "",
        "1 0 obj",
        "<</Type/Page/Parent 0 0 R>>",
        "endobj",
        "",
        "2 0 obj",
        "<</Type/Catalog/Pages 0 0 R>>",
        "endobj",
        "",
    ]

    full_pdf = "\n".join(lines)

    # Calculate object offsets by finding byte positions
    obj_offsets = {}
    for match in re.finditer(r'(\d+) 0 obj', full_pdf):
        obj_num = int(match.group(1))
        obj_offsets[obj_num] = match.start()

    xref_offset = len(full_pdf) + 1
    max_obj = max(obj_offsets.keys()) if obj_offsets else 2

    xref_lines = [
        f"xref",
        f"0 {max_obj + 1}",
        f"0000000000 65535 f ",
    ]

    for obj_num in range(1, max_obj + 1):
        if obj_num in obj_offsets:
            xref_lines.append(f"{obj_offsets[obj_num]:010d} 00000 n ")

    trailer_lines = [
        "trailer",
        f"<</Size {max_obj + 1}/Root 2 0 R>>",
        f"startxref",
        f"{xref_offset}",
        f"%%EOF",
    ]

    final_pdf = full_pdf + "\n" + "\n".join(xref_lines) + "\n" + "\n".join(trailer_lines)

    with open(output_path, 'w') as f:
        f.write(final_pdf)

    print(f"Created {output_path}")


def create_inheritance_grandparent_mediabox():
    """Create PDF where page inherits MediaBox from grandparent /Pages."""
    output_path = "/home/coding/pdftract/tests/document_model/fixtures/inheritance_grandparent_mediabox.pdf"

    lines = [
        "%PDF-1.4",
        "",
        "0 0 obj",
        "<</Type/Pages/Count 1/Kids[1 0 R]/MediaBox[0 0 612 792]>>",
        "endobj",
        "",
        "1 0 obj",
        "<</Type/Page/Parent 0 0 R>>",
        "endobj",
        "",
        "2 0 obj",
        "<</Type/Catalog/Pages 0 0 R>>",
        "endobj",
        "",
    ]

    full_pdf = "\n".join(lines)

    # Calculate object offsets by finding byte positions
    obj_offsets = {}
    for match in re.finditer(r'(\d+) 0 obj', full_pdf):
        obj_num = int(match.group(1))
        obj_offsets[obj_num] = match.start()

    xref_offset = len(full_pdf) + 1
    max_obj = max(obj_offsets.keys()) if obj_offsets else 2

    xref_lines = [
        f"xref",
        f"0 {max_obj + 1}",
        f"0000000000 65535 f ",
    ]

    for obj_num in range(1, max_obj + 1):
        if obj_num in obj_offsets:
            xref_lines.append(f"{obj_offsets[obj_num]:010d} 00000 n ")

    trailer_lines = [
        "trailer",
        f"<</Size {max_obj + 1}/Root 2 0 R>>",
        f"startxref",
        f"{xref_offset}",
        f"%%EOF",
    ]

    final_pdf = full_pdf + "\n" + "\n".join(xref_lines) + "\n" + "\n".join(trailer_lines)

    with open(output_path, 'w') as f:
        f.write(final_pdf)

    print(f"Created {output_path}")


def create_js_in_openaction():
    """Create PDF with JavaScript in /OpenAction."""
    create_simple_pdf("js_in_openaction", extra_catalog_entries="/OpenAction<</S/JavaScript/JS(app.alert('Hello'))>>")


def create_xfa_form():
    """Create PDF with XFA form."""
    create_simple_pdf("xfa_form", extra_catalog_entries="/AcroForm<</XFA[(template)(datasets)(form)]>>")


def create_pdfa_1b_conformance():
    """Create PDF with PDF/A-1B XMP metadata."""
    output_path = "/home/coding/pdftract/tests/document_model/fixtures/pdfa_1b_conformance.pdf"

    lines = [
        "%PDF-1.4",
        "",
        "0 0 obj",
        "<</Type/Pages/Count 1/Kids[1 0 R]>>",
        "endobj",
        "",
        "1 0 obj",
        "<</Type/Page/MediaBox[0 0 612 792]/Parent 0 0 R/Contents 2 0 R/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>",
        "endobj",
        "",
        "2 0 obj",
        "<</Length 44>>",
        "stream",
        "BT",
        "/F1 12 Tf",
        "100 700 Td",
        "(Page 1) Tj",
        "ET",
        "endstream",
        "endobj",
        "",
        "3 0 obj",
        "<</Type/Catalog/Pages 0 0 R/Metadata 4 0 R>>",
        "endobj",
        "",
        "4 0 obj",
        "<</Type/Metadata/Subtype/XML/Length 320>>",
        "stream",
        '<?xml version="1.0"?>',
        '<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">',
        '  <rdf:Description rdf:about="" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/">',
        '    <pdfaid:part>1</pdfaid:part>',
        '    <pdfaid:conformance>B</pdfaid:conformance>',
        '  </rdf:Description>',
        '</rdf:RDF>',
        "endstream",
        "endobj",
        "",
    ]

    full_pdf = "\n".join(lines)

    # Calculate object offsets by finding byte positions
    obj_offsets = {}
    for match in re.finditer(r'(\d+) 0 obj', full_pdf):
        obj_num = int(match.group(1))
        obj_offsets[obj_num] = match.start()

    xref_offset = len(full_pdf) + 1
    max_obj = max(obj_offsets.keys()) if obj_offsets else 4

    xref_lines = [
        f"xref",
        f"0 {max_obj + 1}",
        f"0000000000 65535 f ",
    ]

    for obj_num in range(1, max_obj + 1):
        if obj_num in obj_offsets:
            xref_lines.append(f"{obj_offsets[obj_num]:010d} 00000 n ")

    trailer_lines = [
        "trailer",
        f"<</Size {max_obj + 1}/Root 3 0 R>>",
        f"startxref",
        f"{xref_offset}",
        f"%%EOF",
    ]

    final_pdf = full_pdf + "\n" + "\n".join(xref_lines) + "\n" + "\n".join(trailer_lines)

    with open(output_path, 'w') as f:
        f.write(final_pdf)

    print(f"Created {output_path}")


def create_multi_revision_3():
    """Create PDF with 3 incremental revisions."""
    output_path = "/home/coding/pdftract/tests/document_model/fixtures/multi_revision_3.pdf"

    # First revision: 2-page PDF
    lines = [
        "%PDF-1.4",
        "",
        "0 0 obj",
        "<</Type/Pages/Count 2/Kids[1 0 R 2 0 R]>>",
        "endobj",
        "",
        "1 0 obj",
        "<</Type/Page/MediaBox[0 0 612 792]/Parent 0 0 R/Contents 3 0 R/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>",
        "endobj",
        "",
        "2 0 obj",
        "<</Type/Page/MediaBox[0 0 612 792]/Parent 0 0 R/Contents 4 0 R/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>",
        "endobj",
        "",
        "3 0 obj",
        "<</Length 44>>",
        "stream",
        "BT",
        "/F1 12 Tf",
        "100 700 Td",
        "(Page 1) Tj",
        "ET",
        "endstream",
        "endobj",
        "",
        "4 0 obj",
        "<</Length 44>>",
        "stream",
        "BT",
        "/F1 12 Tf",
        "100 700 Td",
        "(Page 2) Tj",
        "ET",
        "endstream",
        "endobj",
        "",
        "5 0 obj",
        "<</Type/Catalog/Pages 0 0 R>>",
        "endobj",
        "",
    ]

    full_pdf = "\n".join(lines)

    # Calculate object offsets by finding byte positions
    obj_offsets = {}
    for match in re.finditer(r'(\d+) 0 obj', full_pdf):
        obj_num = int(match.group(1))
        obj_offsets[obj_num] = match.start()

    xref_offset = len(full_pdf) + 1
    max_obj = 5

    xref_lines = [
        f"xref",
        f"0 {max_obj + 1}",
        f"0000000000 65535 f ",
    ]

    for obj_num in range(1, max_obj + 1):
        if obj_num in obj_offsets:
            xref_lines.append(f"{obj_offsets[obj_num]:010d} 00000 n ")

    trailer_lines = [
        "trailer",
        f"<</Size {max_obj + 1}/Root 5 0 R>>",
        f"startxref",
        f"{xref_offset}",
        f"%%EOF",
    ]

    final_pdf = full_pdf + "\n" + "\n".join(xref_lines) + "\n" + "\n".join(trailer_lines)

    with open(output_path, 'w') as f:
        f.write(final_pdf)

    print(f"Created {output_path}")


def create_partial_resource_override():
    """Create PDF with partial resource override."""
    output_path = "/home/coding/pdftract/tests/document_model/fixtures/partial_resource_override.pdf"

    lines = [
        "%PDF-1.4",
        "",
        "0 0 obj",
        "<</Type/Pages/Count 1/Kids[1 0 R]/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>/ProcSet[/PDF]>>>",
        "endobj",
        "",
        "1 0 obj",
        "<</Type/Page/MediaBox[0 0 612 792]/Parent 0 0 R/Contents 2 0 R/Resources<</Font<</F2<</Type/Font/Subtype/Type1/BaseFont/Times-Roman>>>>>>>",
        "endobj",
        "",
        "2 0 obj",
        "<</Length 44>>",
        "stream",
        "BT",
        "/F1 12 Tf",
        "100 700 Td",
        "(Page 1) Tj",
        "ET",
        "endstream",
        "endobj",
        "",
        "3 0 obj",
        "<</Type/Catalog/Pages 0 0 R>>",
        "endobj",
        "",
    ]

    full_pdf = "\n".join(lines)

    # Calculate object offsets by finding byte positions
    obj_offsets = {}
    for match in re.finditer(r'(\d+) 0 obj', full_pdf):
        obj_num = int(match.group(1))
        obj_offsets[obj_num] = match.start()

    xref_offset = len(full_pdf) + 1
    max_obj = max(obj_offsets.keys()) if obj_offsets else 3

    xref_lines = [
        f"xref",
        f"0 {max_obj + 1}",
        f"0000000000 65535 f ",
    ]

    for obj_num in range(1, max_obj + 1):
        if obj_num in obj_offsets:
            xref_lines.append(f"{obj_offsets[obj_num]:010d} 00000 n ")

    trailer_lines = [
        "trailer",
        f"<</Size {max_obj + 1}/Root 3 0 R>>",
        f"startxref",
        f"{xref_offset}",
        f"%%EOF",
    ]

    final_pdf = full_pdf + "\n" + "\n".join(xref_lines) + "\n" + "\n".join(trailer_lines)

    with open(output_path, 'w') as f:
        f.write(final_pdf)

    print(f"Created {output_path}")


def create_tagged_3_level_outline():
    """Create PDF with 3-level outline structure."""
    output_path = "/home/coding/pdftract/tests/document_model/fixtures/tagged_3_level_outline.pdf"

    lines = [
        "%PDF-1.4",
        "",
        "0 0 obj",
        "<</Type/Pages/Count 1/Kids[1 0 R]>>",
        "endobj",
        "",
        "1 0 obj",
        "<</Type/Page/MediaBox[0 0 612 792]/Parent 0 0 R/Contents 2 0 R/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>",
        "endobj",
        "",
        "2 0 obj",
        "<</Length 44>>",
        "stream",
        "BT",
        "/F1 12 Tf",
        "100 700 Td",
        "(Page 1) Tj",
        "ET",
        "endstream",
        "endobj",
        "",
        "3 0 obj",
        "<</Type/Catalog/Pages 0 0 R/Outlines 4 0 R>>",
        "endobj",
        "",
        "4 0 obj",
        "<</Type/Outlines/First 5 0 R/Last 7 0 R/Count 3>>",
        "endobj",
        "",
        "5 0 obj",
        "<</Title(Chapter 1)/Parent 4 0 R/Next 6 0 R/First 8 0 R/Last 9 0 R/Count 2>>",
        "endobj",
        "",
        "6 0 obj",
        "<</Title(Chapter 2)/Parent 4 0 R/Prev 5 0 R>>",
        "endobj",
        "",
        "7 0 obj",
        "<</Title(Chapter 3)/Parent 4 0 R/Prev 6 0 R>>",
        "endobj",
        "",
        "8 0 obj",
        "<</Title(Section 1.1)/Parent 5 0 R/Next 9 0 R>>",
        "endobj",
        "",
        "9 0 obj",
        "<</Title(Section 1.2)/Parent 5 0 R/Prev 8 0 R>>",
        "endobj",
        "",
    ]

    full_pdf = "\n".join(lines)

    # Calculate object offsets by finding byte positions
    obj_offsets = {}
    for match in re.finditer(r'(\d+) 0 obj', full_pdf):
        obj_num = int(match.group(1))
        obj_offsets[obj_num] = match.start()

    xref_offset = len(full_pdf) + 1
    max_obj = max(obj_offsets.keys()) if obj_offsets else 9

    xref_lines = [
        f"xref",
        f"0 {max_obj + 1}",
        f"0000000000 65535 f ",
    ]

    for obj_num in range(1, max_obj + 1):
        if obj_num in obj_offsets:
            xref_lines.append(f"{obj_offsets[obj_num]:010d} 00000 n ")

    trailer_lines = [
        "trailer",
        f"<</Size {max_obj + 1}/Root 3 0 R>>",
        f"startxref",
        f"{xref_offset}",
        f"%%EOF",
    ]

    final_pdf = full_pdf + "\n" + "\n".join(xref_lines) + "\n" + "\n".join(trailer_lines)

    with open(output_path, 'w') as f:
        f.write(final_pdf)

    print(f"Created {output_path}")


def create_page_labels_roman_arabic():
    """Create PDF with roman numerals for pages 0-3 and arabic for page 4+."""
    output_path = "/home/coding/pdftract/tests/document_model/fixtures/page_labels_roman_arabic.pdf"

    lines = [
        "%PDF-1.4",
        "",
        "0 0 obj",
        "<</Type/Pages/Count 5/Kids[1 0 R 2 0 R 3 0 R 4 0 R 5 0 R]>>",
        "endobj",
        "",
        "1 0 obj",
        "<</Type/Page/MediaBox[0 0 612 792]/Parent 0 0 R/Contents 6 0 R/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>",
        "endobj",
        "",
        "2 0 obj",
        "<</Type/Page/MediaBox[0 0 612 792]/Parent 0 0 R/Contents 7 0 R/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>",
        "endobj",
        "",
        "3 0 obj",
        "<</Type/Page/MediaBox[0 0 612 792]/Parent 0 0 R/Contents 8 0 R/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>",
        "endobj",
        "",
        "4 0 obj",
        "<</Type/Page/MediaBox[0 0 612 792]/Parent 0 0 R/Contents 9 0 R/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>",
        "endobj",
        "",
        "5 0 obj",
        "<</Type/Page/MediaBox[0 0 612 792]/Parent 0 0 R/Contents 10 0 R/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>",
        "endobj",
        "",
        "6 0 obj",
        "<</Length 44>>",
        "stream",
        "BT",
        "/F1 12 Tf",
        "100 700 Td",
        "(Page i) Tj",
        "ET",
        "endstream",
        "endobj",
        "",
        "7 0 obj",
        "<</Length 44>>",
        "stream",
        "BT",
        "/F1 12 Tf",
        "100 700 Td",
        "(Page ii) Tj",
        "ET",
        "endstream",
        "endobj",
        "",
        "8 0 obj",
        "<</Length 44>>",
        "stream",
        "BT",
        "/F1 12 Tf",
        "100 700 Td",
        "(Page iii) Tj",
        "ET",
        "endstream",
        "endobj",
        "",
        "9 0 obj",
        "<</Length 44>>",
        "stream",
        "BT",
        "/F1 12 Tf",
        "100 700 Td",
        "(Page iv) Tj",
        "ET",
        "endstream",
        "endobj",
        "",
        "10 0 obj",
        "<</Length 44>>",
        "stream",
        "BT",
        "/F1 12 Tf",
        "100 700 Td",
        "(Page 1) Tj",
        "ET",
        "endstream",
        "endobj",
        "",
        "11 0 obj",
        "<</Type/Catalog/Pages 0 0 R/PageLabels 12 0 R>>",
        "endobj",
        "",
        "12 0 obj",
        "<</Nums[0<</S/R>>4<</S/D>>]>>",
        "endobj",
        "",
    ]

    full_pdf = "\n".join(lines)

    # Calculate object offsets by finding byte positions
    obj_offsets = {}
    for match in re.finditer(r'(\d+) 0 obj', full_pdf):
        obj_num = int(match.group(1))
        obj_offsets[obj_num] = match.start()

    xref_offset = len(full_pdf) + 1
    max_obj = max(obj_offsets.keys()) if obj_offsets else 12

    xref_lines = [
        f"xref",
        f"0 {max_obj + 1}",
        f"0000000000 65535 f ",
    ]

    for obj_num in range(1, max_obj + 1):
        if obj_num in obj_offsets:
            xref_lines.append(f"{obj_offsets[obj_num]:010d} 00000 n ")

    trailer_lines = [
        "trailer",
        f"<</Size {max_obj + 1}/Root 11 0 R>>",
        f"startxref",
        f"{xref_offset}",
        f"%%EOF",
    ]

    final_pdf = full_pdf + "\n" + "\n".join(xref_lines) + "\n" + "\n".join(trailer_lines)

    with open(output_path, 'w') as f:
        f.write(final_pdf)

    print(f"Created {output_path}")


def create_encrypted_unknown_handler():
    """Create PDF with unsupported encryption handler (Adobe.PubSec)."""
    output_path = "/home/coding/pdftract/tests/document_model/fixtures/encrypted_unknown_handler.pdf"

    lines = [
        "%PDF-1.4",
        "",
        "0 0 obj",
        "<</Type/Pages/Count 1/Kids[1 0 R]>>",
        "endobj",
        "",
        "1 0 obj",
        "<</Type/Page/MediaBox[0 0 612 792]/Parent 0 0 R/Contents 2 0 R/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>",
        "endobj",
        "",
        "2 0 obj",
        "<</Length 44>>",
        "stream",
        "BT",
        "/F1 12 Tf",
        "100 700 Td",
        "(Page 1) Tj",
        "ET",
        "endstream",
        "endobj",
        "",
        "3 0 obj",
        "<</Type/Catalog/Pages 0 0 R>>",
        "endobj",
        "",
        "4 0 obj",
        "<</Filter/Adobe.PubSec/V 2/R 2 Length 64/O(testowner)/U(testuser)/P -1224>>",
        "endobj",
        "",
    ]

    full_pdf = "\n".join(lines)

    # Calculate object offsets by finding byte positions
    obj_offsets = {}
    for match in re.finditer(r'(\d+) 0 obj', full_pdf):
        obj_num = int(match.group(1))
        obj_offsets[obj_num] = match.start()

    xref_offset = len(full_pdf) + 1
    max_obj = max(obj_offsets.keys()) if obj_offsets else 4

    xref_lines = [
        f"xref",
        f"0 {max_obj + 1}",
        f"0000000000 65535 f ",
    ]

    for obj_num in range(1, max_obj + 1):
        if obj_num in obj_offsets:
            xref_lines.append(f"{obj_offsets[obj_num]:010d} 00000 n ")

    trailer_lines = [
        "trailer",
        f"<</Size {max_obj + 1}/Root 3 0 R/Encrypt 4 0 R>>",
        f"startxref",
        f"{xref_offset}",
        f"%%EOF",
    ]

    final_pdf = full_pdf + "\n" + "\n".join(xref_lines) + "\n" + "\n".join(trailer_lines)

    with open(output_path, 'w') as f:
        f.write(final_pdf)

    print(f"Created {output_path}")


if __name__ == "__main__":
    print("Creating valid PDF fixtures...")

    create_simple_pdf("base_hello")
    create_ocg_default_off()
    create_missing_mediabox()
    create_inheritance_grandparent_mediabox()
    create_js_in_openaction()
    create_xfa_form()
    create_pdfa_1b_conformance()
    create_multi_revision_3()
    create_partial_resource_override()
    create_tagged_3_level_outline()
    create_page_labels_roman_arabic()
    create_encrypted_unknown_handler()

    print("\nAll fixtures created successfully!")
