#!/usr/bin/env python3
"""
Generate a test PDF with Markdown structures for testing extract_markdown vs extract_text.

This creates a PDF with headings, links, lists, and other structures that should
render differently when extracted as Markdown vs plain text.

Usage:
    python3 markdown_test_fixture.py
"""

import os
from pathlib import Path
from reportlab.lib.pagesizes import LETTER
from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
from reportlab.lib.units import inch
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle
from reportlab.lib import colors
from reportlab.lib.enums import TA_LEFT, TA_CENTER, TA_RIGHT
from reportlab.pdfgen import canvas
from reportlab.lib import colors

# Fixture directory
FIXTURES_DIR = Path(__file__).parent / "markdown"
FIXTURES_DIR.mkdir(exist_ok=True)

def create_markdown_test_fixture():
    """Create a test PDF with Markdown structures."""
    output_pdf = FIXTURES_DIR / "markdown-structures.pdf"
    output_txt = FIXTURES_DIR / "markdown-structures.txt"

    doc = SimpleDocTemplate(str(output_pdf), pagesize=LETTER, rightMargin=72, leftMargin=72, topMargin=72, bottomMargin=72)
    story = []
    styles = getSampleStyleSheet()

    # Title (Heading 1)
    story.append(Paragraph("Markdown Test Document", ParagraphStyle('Heading1', parent=styles['Heading1'], fontName='Helvetica-Bold', fontSize=18, spaceAfter=12)))

    # Subtitle (Heading 2)
    story.append(Paragraph("Testing extract_markdown() vs extract_text()", ParagraphStyle('Heading2', parent=styles['Heading2'], fontName='Helvetica-Bold', fontSize=14, spaceAfter=12)))

    # Author (Heading 3)
    story.append(Paragraph("By Test Author", ParagraphStyle('Heading3', parent=styles['Heading3'], fontName='Helvetica-Bold', fontSize=12, spaceAfter=18)))

    # Introduction paragraph
    intro_text = """This document tests the difference between extract_text() and extract_markdown().
    The extract_text() function should return plain text without Markdown formatting.
    The extract_markdown() function should return properly formatted Markdown with
    headings marked with #, links with [text](url) syntax, and lists with - or 1. syntax."""
    story.append(Paragraph(intro_text, ParagraphStyle('Body', parent=styles['Normal'], fontSize=11, spaceAfter=12)))

    # Section with heading
    story.append(Paragraph("Links Section", ParagraphStyle('Heading4', parent=styles['Heading2'], fontName='Helvetica-Bold', fontSize=14, spaceAfter=12)))

    # Paragraph with a link (this won't render as a link in PDF without annotation)
    link_text = """Visit our website at https://example.com for more information.
    You can also email us at support@example.com or check our documentation at
    https://docs.example.com/guide."""
    story.append(Paragraph(link_text, ParagraphStyle('Body', parent=styles['Normal'], fontSize=11, spaceAfter=12)))

    # Bullet list section
    story.append(Paragraph("Key Features", ParagraphStyle('Heading5', parent=styles['Heading2'], fontName='Helvetica-Bold', fontSize=14, spaceAfter=12)))

    # Bullet list items
    bullet_items = [
        "• Feature one: Text extraction from PDFs",
        "• Feature two: Markdown formatting support",
        "• Feature three: Link preservation",
        "• Feature four: List detection",
        "• Feature five: Table extraction",
    ]
    for item in bullet_items:
        story.append(Paragraph(item, ParagraphStyle('Bullet', parent=styles['Normal'], fontSize=11, spaceAfter=6)))

    story.append(Spacer(1, 12))

    # Numbered list section
    story.append(Paragraph("Implementation Steps", ParagraphStyle('Heading6', parent=styles['Heading2'], fontName='Helvetica-Bold', fontSize=14, spaceAfter=12)))

    # Numbered list items
    numbered_items = [
        "1. Parse the PDF content stream",
        "2. Extract text spans with metadata",
        "3. Detect block types (heading, paragraph, list)",
        "4. Format blocks as Markdown",
        "5. Return the complete Markdown output",
    ]
    for item in numbered_items:
        story.append(Paragraph(item, ParagraphStyle('Numbered', parent=styles['Normal'], fontSize=11, spaceAfter=6)))

    story.append(Spacer(1, 12))

    # Table section
    story.append(Paragraph("Sample Table", ParagraphStyle('Heading7', parent=styles['Heading2'], fontName='Helvetica-Bold', fontSize=14, spaceAfter=12)))

    table_data = [
        ["Name", "Type", "Description"],
        ["extract_text", "Function", "Returns plain text from PDF"],
        ["extract_markdown", "Function", "Returns Markdown formatted text"],
        ["Markdown", "Format", "Lightweight markup language"],
        ["PDF", "Format", "Portable Document Format"],
    ]

    table = Table(table_data, colWidths=[2.0*inch, 1.5*inch, 2.5*inch])
    table.setStyle(TableStyle([
        ('FONTNAME', (0, 0), (-1, 0), 'Helvetica-Bold'),
        ('ALIGN', (0, 0), (-1, -1), 'LEFT'),
        ('LINEABOVE', (0, 0), (-1, 0), 1, colors.black),
        ('LINEBELOW', (0, 0), (-1, 0), 1, colors.black),
        ('LINEBELOW', (0, -1), (-1, -1), 1, colors.black),
    ]))
    story.append(table)

    # Build the PDF
    doc.build(story)

    # Ground truth - what extract_text() should return (plain text)
    ground_truth_text = """Markdown Test Document
Testing extract_markdown() vs extract_text()
By Test Author
This document tests the difference between extract_text() and extract_markdown().
The extract_text() function should return plain text without Markdown formatting.
The extract_markdown() function should return properly formatted Markdown with
headings marked with #, links with [text](url) syntax, and lists with - or 1. syntax.
Links Section
Visit our website at https://example.com for more information.
You can also email us at support@example.com or check our documentation at
https://docs.example.com/guide.
Key Features
• Feature one: Text extraction from PDFs
• Feature two: Markdown formatting support
• Feature three: Link preservation
• Feature four: List detection
• Feature five: Table extraction
Implementation Steps
1. Parse the PDF content stream
2. Extract text spans with metadata
3. Detect block types (heading, paragraph, list)
4. Format blocks as Markdown
5. Return the complete Markdown output
Sample Table
Name               Type                Description
extract_text       Function            Returns plain text from PDF
extract_markdown   Function            Returns Markdown formatted text
Markdown           Format              Lightweight markup language
PDF                Format              Portable Document Format"""

    # Ground truth - what extract_markdown() should return (formatted Markdown)
    ground_truth_markdown = """# Markdown Test Document

## Testing extract_markdown() vs extract_text()

### By Test Author

This document tests the difference between extract_text() and extract_markdown().
The extract_text() function should return plain text without Markdown formatting.
The extract_markdown() function should return properly formatted Markdown with
headings marked with #, links with [text](url) syntax, and lists with - or 1. syntax.

## Links Section

Visit our website at https://example.com for more information.
You can also email us at support@example.com or check our documentation at
https://docs.example.com/guide.

## Key Features

- Feature one: Text extraction from PDFs
- Feature two: Markdown formatting support
- Feature three: Link preservation
- Feature four: List detection
- Feature five: Table extraction

## Implementation Steps

1. Parse the PDF content stream
2. Extract text spans with metadata
3. Detect block types (heading, paragraph, list)
4. Format blocks as Markdown
5. Return the complete Markdown output

## Sample Table

| Name | Type | Description |
| --- | --- | --- |
| extract_text | Function | Returns plain text from PDF |
| extract_markdown | Function | Returns Markdown formatted text |
| Markdown | Format | Lightweight markup language |
| PDF | Format | Portable Document Format |"""

    # Save ground truth files
    (FIXTURES_DIR / "markdown-structures-expect-text.txt").write_text(ground_truth_text)
    (FIXTURES_DIR / "markdown-structures-expect-markdown.txt").write_text(ground_truth_markdown)

    print(f"✓ Generated test fixture: markdown-structures.pdf")
    print(f"  PDF: {output_pdf}")
    print(f"  Expected text output: {FIXTURES_DIR / 'markdown-structures-expect-text.txt'}")
    print(f"  Expected Markdown output: {FIXTURES_DIR / 'markdown-structures-expect-markdown.txt'}")
    print(f"  Fixture directory: {FIXTURES_DIR}")

if __name__ == "__main__":
    create_markdown_test_fixture()
