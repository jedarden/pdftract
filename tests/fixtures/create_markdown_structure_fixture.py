#!/usr/bin/env python3
"""
Create a test PDF with visible Markdown structure markers.

This generates a PDF where the Markdown syntax (# headings, [links], etc.)
is visible in the text content, not just styling. This allows testing
extract_markdown() vs extract_text() behavior.
"""

from pathlib import Path
from reportlab.lib.pagesizes import LETTER
from reportlab.lib.styles import getSampleStyleSheet
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer
from reportlab.lib.units import inch

def create_fixture():
    """Create a PDF with visible Markdown syntax markers."""
    output_path = Path(__file__).parent / "markdown_structure.pdf"

    doc = SimpleDocTemplate(
        str(output_path),
        pagesize=LETTER,
        rightMargin=72,
        leftMargin=72,
        topMargin=72,
        bottomMargin=72
    )

    story = []
    styles = getSampleStyleSheet()

    # Use normal font for all content so # markers are visible
    normal_style = styles['Normal']
    normal_style.fontSize = 12

    # Heading 1 with visible # marker
    story.append(Paragraph("# Main Document Title", normal_style))
    story.append(Spacer(1, 12))

    # Heading 2 with visible ## marker
    story.append(Paragraph("## Section Subtitle", normal_style))
    story.append(Spacer(1, 12))

    # Paragraph with link in [text] format
    story.append(Paragraph(
        "This is a paragraph with a <b>[link to example.com]</b> and some regular text following it.",
        normal_style
    ))
    story.append(Spacer(1, 12))

    # More paragraph with links
    story.append(Paragraph(
        "Visit <b>[GitHub]</b> for more information or check <b>[the documentation]</b>.",
        normal_style
    ))
    story.append(Spacer(1, 12))

    # Heading 3
    story.append(Paragraph("### Subsection Header", normal_style))
    story.append(Spacer(1, 12))

    # Bullet list items (using bullet characters)
    story.append(Paragraph("• First bullet point item", normal_style))
    story.append(Paragraph("• Second bullet with more detail", normal_style))
    story.append(Paragraph("• Third bullet point that spans a bit more text", normal_style))
    story.append(Spacer(1, 12))

    # Numbered list
    story.append(Paragraph("1. First numbered item", normal_style))
    story.append(Paragraph("2. Second numbered item", normal_style))
    story.append(Paragraph("3. Third numbered item", normal_style))
    story.append(Spacer(1, 12))

    # Paragraph with inline code
    story.append(Paragraph(
        "Here is some inline code: <b>var x = 42;</b> within a paragraph of text.",
        normal_style
    ))
    story.append(Spacer(1, 12))

    # Code block (using preformatted style)
    code_style = styles['Code']
    code_style.fontSize = 10
    code_style.fontName = 'Courier'

    story.append(Paragraph("```", code_style))
    story.append(Paragraph("def example_function():", code_style))
    story.append(Paragraph("    return 'hello world'", code_style))
    story.append(Paragraph("```", code_style))
    story.append(Spacer(1, 12))

    # Build PDF
    doc.build(story)

    print(f"✓ Created fixture: {output_path}")
    print(f"  Size: {output_path.stat().st_size} bytes")

    # Print expected text output
    print("\nExpected extract_text() output:")
    print("-" * 60)
    print("""# Main Document Title
## Section Subtitle
This is a paragraph with a [link to example.com] and some regular text following it.
Visit [GitHub] for more information or check [the documentation].
### Subsection Header
• First bullet point item
• Second bullet with more detail
• Third bullet point that spans a bit more text
1. First numbered item
2. Second numbered item
3. Third numbered item
Here is some inline code: var x = 42; within a paragraph of text.
```
def example_function():
    return 'hello world'
```""")

    return output_path

if __name__ == "__main__":
    create_fixture()
