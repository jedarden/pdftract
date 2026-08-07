#!/usr/bin/env python3
"""
Generate markdown_structure.pdf fixture for testing extract_text() vs extract_markdown().

This PDF contains structural elements that resemble markdown syntax:
- Headings with # markers (# Main Title, ## Subtitle)
- Links with [text](url) syntax
- Lists (bullet points and numbered)
- Code blocks and inline code elements
"""

import argparse
import os
import sys

from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
from reportlab.lib.units import inch
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer
from reportlab.lib.enums import TA_LEFT
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont


def create_markdown_structure_pdf(output_path):
    """Create PDF with markdown-style structural elements."""

    doc = SimpleDocTemplate(
        output_path,
        pagesize=letter,
        rightMargin=72,
        leftMargin=72,
        topMargin=72,
        bottomMargin=18
    )

    story = []
    styles = getSampleStyleSheet()

    # Custom styles that mimic markdown structure
    heading1_style = ParagraphStyle(
        'CustomHeading1',
        parent=styles['Heading1'],
        fontSize=18,
        textColor='#000000',
        spaceAfter=12,
        leading=22
    )

    heading2_style = ParagraphStyle(
        'CustomHeading2',
        parent=styles['Heading2'],
        fontSize=14,
        textColor='#000000',
        spaceAfter=10,
        leading=18
    )

    heading3_style = ParagraphStyle(
        'CustomHeading3',
        parent=styles['Heading3'],
        fontSize=12,
        textColor='#000000',
        spaceAfter=8,
        leading=16
    )

    body_style = ParagraphStyle(
        'CustomBody',
        parent=styles['BodyText'],
        fontSize=11,
        textColor='#000000',
        spaceAfter=12,
        leading=14
    )

    code_style = ParagraphStyle(
        'CustomCode',
        parent=styles['Code'],
        fontSize=10,
        textColor='#444444',
        spaceAfter=12,
        leading=12,
        fontName='Courier'
    )

    # Title
    story.append(Paragraph("# Main Document Title", heading1_style))
    story.append(Spacer(1, 0.2 * inch))

    # Subtitle
    story.append(Paragraph("## Section Subtitle", heading2_style))
    story.append(Spacer(1, 0.2 * inch))

    # Regular paragraph with markdown-style link
    story.append(Paragraph(
        "This is a paragraph with a <a href='https://example.com' color='blue'>[link to example.com]</a> "
        "and some regular text following it.",
        body_style
    ))
    story.append(Spacer(1, 0.2 * inch))

    # Another link example
    story.append(Paragraph(
        "Visit <a href='https://github.com' color='blue'>[GitHub]</a> for more information "
        "or check <a href='https://docs.example.com' color='blue'>[the documentation]</a>.",
        body_style
    ))
    story.append(Spacer(1, 0.3 * inch))

    # Third-level heading
    story.append(Paragraph("### Subsection Header", heading3_style))
    story.append(Spacer(1, 0.2 * inch))

    # Bullet list items (as paragraphs)
    story.append(Paragraph("• First bullet point item", body_style))
    story.append(Paragraph("• Second bullet point with more detail", body_style))
    story.append(Paragraph("• Third bullet point that spans a bit more text", body_style))
    story.append(Spacer(1, 0.2 * inch))

    # Numbered list items
    story.append(Paragraph("1. First numbered item", body_style))
    story.append(Paragraph("2. Second numbered item", body_style))
    story.append(Paragraph("3. Third numbered item", body_style))
    story.append(Spacer(1, 0.3 * inch))

    # Inline code example
    story.append(Paragraph(
        "Here is some inline code: <font name='Courier'><code>var x = 42;</code></font> "
        "within a paragraph of text.",
        body_style
    ))
    story.append(Spacer(1, 0.2 * inch))

    # Code block simulation (preformatted text)
    story.append(Paragraph("```", code_style))
    story.append(Paragraph("def example_function():", code_style))
    story.append(Paragraph("    return 'hello world'", code_style))
    story.append(Paragraph("```", code_style))
    story.append(Spacer(1, 0.2 * inch))

    # Another section with more links
    story.append(Paragraph("## Resources Section", heading2_style))
    story.append(Spacer(1, 0.2 * inch))

    story.append(Paragraph(
        "See also: <a href='https://example.org' color='blue'>[Example.org]</a>, "
        "<a href='https://test.example' color='blue'>[Test Site]</a>",
        body_style
    ))
    story.append(Spacer(1, 0.2 * inch))

    # Mixed content paragraph
    story.append(Paragraph(
        "This paragraph has <a href='https://mixed.content' color='blue'>[a link]</a>, "
        "followed by <font name='Courier'><code>inline code</code></font>, "
        "and then more text to test extraction.",
        body_style
    ))
    story.append(Spacer(1, 0.2 * inch))

    # Final heading
    story.append(Paragraph("### Conclusion", heading3_style))
    story.append(Spacer(1, 0.2 * inch))

    story.append(Paragraph(
        "End of markdown structure test document. "
        "Visit <a href='https://final.link' color='blue'>[final link]</a> for more.",
        body_style
    ))

    # Build the PDF
    doc.build(story)
    print(f"Created: {output_path}")


def main():
    """Generate the markdown structure fixture."""
    parser = argparse.ArgumentParser(
        description='Generate a PDF fixture with Markdown-style structural elements for testing extract_text() vs extract_markdown().',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s                                    # Output to tests/fixtures/markdown_structure.pdf
  %(prog)s -o /tmp/output.pdf                 # Output to custom path
  %(prog)s --output custom/path/file.pdf      # Output to custom path (long form)

The generated PDF contains:
  - Headings with # markers (#, ##, ###)
  - Links with [text](url) syntax
  - Bullet lists (• items)
  - Numbered lists (1. 2. 3.)
  - Code blocks (```)
  - Inline code (<code>)
        """
    )
    parser.add_argument(
        '-o', '--output',
        dest='output_path',
        metavar='PATH',
        help='Output PDF file path (default: tests/fixtures/markdown_structure.pdf)'
    )

    args = parser.parse_args()

    # Determine output path
    if args.output_path:
        output_path = args.output_path
        output_dir = os.path.dirname(output_path)
    else:
        output_dir = "tests/fixtures"
        output_path = os.path.join(output_dir, "markdown_structure.pdf")

    # Create output directory if needed
    if output_dir:
        try:
            os.makedirs(output_dir, exist_ok=True)
        except OSError as e:
            print(f"Error: Failed to create output directory '{output_dir}': {e}", file=sys.stderr)
            sys.exit(1)

    # Validate output path is writable
    try:
        with open(output_path, 'wb') as test_file:
            pass
        os.remove(output_path)  # Remove the test file
    except OSError as e:
        print(f"Error: Cannot write to output path '{output_path}': {e}", file=sys.stderr)
        sys.exit(1)

    create_markdown_structure_pdf(output_path)

    print(f"\nmarkdown_structure.pdf fixture created successfully!")
    print(f"Location: {output_path}")
    print("\nContains structural elements:")
    print("  - Headings with # markers (#, ##, ###)")
    print("  - Links with [text](url) syntax")
    print("  - Bullet lists (• items)")
    print("  - Numbered lists (1. 2. 3.)")
    print("  - Code blocks (```)")
    print("  - Inline code (<code>)")


if __name__ == "__main__":
    main()
