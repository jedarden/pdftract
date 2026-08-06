#!/usr/bin/env python3
"""
Create hybrid-001-vector-header-over-scan.pdf fixture

Generates a hybrid PDF with vector letterhead header overlaid on a scanned letter body.
Tests PageClass::Hybrid classification with clear regional separation.
"""

import sys
from pathlib import Path
from reportlab.lib.pagesizes import LETTER
from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
from reportlab.lib.units import inch
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer
from reportlab.lib.enums import TA_CENTER, TA_RIGHT
from reportlab.pdfgen import canvas
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from PIL import Image as PILImage, ImageDraw, ImageFont
import io

# Page dimensions
PAGE_WIDTH = 612  # 8.5 inches in points
PAGE_HEIGHT = 792  # 11 inches in points

# Output paths
OUTPUT_DIR = Path(__file__).parent
OUTPUT_PDF = OUTPUT_DIR / "hybrid-001-vector-header-over-scan.pdf"


def create_scanned_image(width, height, text_lines, font_size=11):
    """Create a scanned-looking image with text."""
    img = PILImage.new("RGB", (int(width), int(height)), color="white")
    draw = ImageDraw.Draw(img)

    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", font_size)
    except:
        try:
            font = ImageFont.truetype("/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf", font_size)
        except:
            font = ImageFont.load_default()

    y = 10
    for line in text_lines:
        draw.text((10, y), line, fill="black", font=font)
        y += int(font_size * 1.6)

    return img


def create_hybrid_pdf():
    """Create hybrid PDF with vector header and scanned body."""

    # Create PDF with vector letterhead
    doc = SimpleDocTemplate(str(OUTPUT_PDF), pagesize=LETTER,
                            rightMargin=36, leftMargin=36,
                            topMargin=36, bottomMargin=36)
    story = []
    styles = getSampleStyleSheet()

    # Vector letterhead (top 15%)
    header_style = ParagraphStyle(
        'Header',
        parent=styles['Heading1'],
        fontName='Helvetica-Bold',
        fontSize=18,
        alignment=TA_CENTER,
        spaceAfter=6,
    )

    story.append(Paragraph("ACME CORPORATION", header_style))
    story.append(Paragraph("123 Business Avenue, Suite 100",
                          ParagraphStyle('Address', parent=styles['Normal'],
                                        alignment=TA_CENTER, fontSize=10)))
    story.append(Paragraph("New York, NY 10001",
                          ParagraphStyle('Address', parent=styles['Normal'],
                                        alignment=TA_CENTER, fontSize=10)))
    story.append(Paragraph("Tel: (212) 555-0123 | Email: info@acmecorp.com",
                          ParagraphStyle('Contact', parent=styles['Normal'],
                                       alignment=TA_CENTER, fontSize=9, spaceAfter=18)))

    # Date and recipient (vector)
    story.append(Paragraph("Date: August 6, 2026",
                          ParagraphStyle('Date', parent=styles['Normal'],
                                       alignment=TA_RIGHT, spaceAfter=12)))
    story.append(Paragraph("Mr. John Smith", ParagraphStyle('Normal', parent=styles['Normal'])))
    story.append(Paragraph("456 Client Road", ParagraphStyle('Normal', parent=styles['Normal'])))
    story.append(Paragraph("Los Angeles, CA 90001",
                          ParagraphStyle('Normal', parent=styles['Normal'], spaceAfter=24)))

    # Build the PDF with vector content first
    doc.build(story)

    # Now add the scanned body image to the PDF
    # We need to modify the PDF to add the image overlay
    from PyPDF2 import PdfReader, PdfWriter

    # Create scanned body image
    scanned_height = 6.0 * inch
    scanned_img = create_scanned_image(
        540, int(scanned_height),
        [
            "Dear Mr. Smith:",
            "",
            "Thank you for your recent inquiry about our enterprise solutions. We are pleased to",
            "present our comprehensive proposal for your organization's document management needs.",
            "",
            "Our hybrid document processing system offers several key advantages:",
            "",
            "1. Automatic classification of document types (vector, scanned, hybrid)",
            "2. Intelligent OCR with confidence-based merging",
            "3. 8x8 grid cell detection for precise content region identification",
            "4. Bbox overlap rules to eliminate duplicate text extraction",
            "",
            "We believe our solution aligns perfectly with your requirements for processing",
            "mixed-format documents at scale. Our Phase 5.5 classifier tuning ensures optimal",
            "performance on known-tricky hybrid cases.",
            "",
            "Please find attached our technical specifications and pricing information. We look",
            "forward to discussing this proposal further at your convenience.",
            "",
            "Sincerely,",
            "",
            "Jane Johnson",
            "Senior Account Executive",
            "ACME Corporation",
        ],
        font_size=11
    )

    # Save scanned image temporarily
    scanned_path = OUTPUT_DIR / "hybrid-001-scanned-body.png"
    scanned_img.save(scanned_path, "PNG")

    # Create a new PDF with the scanned image
    from reportlab.lib.utils import ImageReader
    from reportlab.pdfgen import canvas

    # Create a canvas to draw over the existing PDF
    packet = io.BytesIO()
    can = canvas.Canvas(packet, pagesize=LETTER)

    # Draw the scanned image at the appropriate position (after the header)
    # Header takes approximately 1.5 inches, so image starts at y = PAGE_HEIGHT - 1.5*inch
    img_y = PAGE_HEIGHT - 3.0 * inch  # Position after letterhead and recipient info
    can.drawImage(scanned_path, 36, img_y - int(scanned_height),
                 width=540, height=int(scanned_height), preserveAspectRatio=True)

    can.save()

    # Merge the image with the original PDF
    new_pdf = PdfReader(packet)
    existing_pdf = PdfReader(OUTPUT_PDF)
    output = PdfWriter()

    # Get the first page from existing PDF and overlay the image
    page = existing_pdf.pages[0]
    page.merge_page(new_pdf.pages[0])
    output.add_page(page)

    # Save the final PDF
    with open(OUTPUT_PDF, 'wb') as f:
        output.write(f)

    # Clean up temporary image
    scanned_path.unlink()

    print(f"✓ Generated hybrid-001-vector-header-over-scan.pdf")
    print(f"  Location: {OUTPUT_PDF}")
    print(f"  File size: {OUTPUT_PDF.stat().st_size} bytes")

    return OUTPUT_PDF


if __name__ == "__main__":
    create_hybrid_pdf()
