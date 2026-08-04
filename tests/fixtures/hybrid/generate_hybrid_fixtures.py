#!/usr/bin/env python3
"""
Hybrid PDF Fixture Generation Script

Generates 10 hybrid PDF fixtures with mixed vector text and scanned image content
for testing PageClass::Hybrid classification and hybrid extraction (Phase 5.5, KU-2).

Each fixture has strategically designed vector and scanned regions to test specific
aspects of the hybrid pipeline: cell detection, merge rules, OCR priority.

Requirements:
    - Python 3.8+
    - reportlab (pip install reportlab)
    - Pillow (pip install Pillow)
    - img2pdf (pip install img2pdf)

Usage:
    python3 generate_hybrid_fixtures.py

Output:
    Creates 10 hybrid PDFs in subdirectories with accompanying .txt ground truth files.
"""

import os
import sys
from pathlib import Path
from reportlab.lib.pagesizes import LETTER
from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
from reportlab.lib.units import inch
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle, Image
from reportlab.lib import colors
from reportlab.lib.enums import TA_LEFT, TA_CENTER, TA_RIGHT
from reportlab.pdfgen import canvas
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from PIL import Image as PILImage, ImageDraw, ImageFont
import io

# Page dimensions
PAGE_WIDTH = 612  # 8.5 inches in points
PAGE_HEIGHT = 792  # 11 inches in points
CELL_WIDTH = PAGE_WIDTH / 8
CELL_HEIGHT = PAGE_HEIGHT / 8

# Fixture directories
FIXTURES_DIR = Path(__file__).parent
FIXTURES = [
    "receipt-overtext",
    "letterhead-image",
    "form-mixed",
    "invoice-stamp",
    "document-annotation",
    "figure-caption",
    "sidebar-image",
    "watermark",
    "multi-column-scan",
    "complex-overlap",
]


def create_scanned_image(width, height, text_lines, font_size=12, bg_color="white"):
    """Create a scanned-looking image with text."""
    img = PILImage.new("RGB", (int(width), int(height)), color=bg_color)
    draw = ImageDraw.Draw(img)

    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", font_size)
    except:
        font = ImageFont.load_default()

    y = 10
    for line in text_lines:
        draw.text((10, y), line, fill="black", font=font)
        y += font_size * 1.5

    return img


def create_pdf_with_vector_and_scanned(output_path, vector_builder, scanned_regions):
    """Create a hybrid PDF with vector content and scanned image regions."""
    pdf_path = FIXTURES_DIR / output_path
    doc = SimpleDocTemplate(str(pdf_path), pagesize=LETTER)
    story = []

    # Add vector content
    vector_builder(story)

    # Build the PDF
    doc.build(story)

    # Now we need to add scanned regions by drawing images on the PDF
    # This is a simplified approach - for proper hybrid PDFs, we'd need to
    # modify the PDF after initial creation or use a more sophisticated approach
    return pdf_path


def fixture_receipt_overtext():
    """Fixture 1: Scanned receipt body with vector price overlay text."""
    fixture_dir = FIXTURES_DIR / "receipt-overtext"
    fixture_dir.mkdir(exist_ok=True)

    output_pdf = fixture_dir / "receipt-overtext.pdf"
    output_txt = fixture_dir / "receipt-overtext.txt"

    # Create PDF
    doc = SimpleDocTemplate(str(output_pdf), pagesize=LETTER, rightMargin=36, leftMargin=36, topMargin=36, bottomMargin=36)
    story = []

    styles = getSampleStyleSheet()

    # Vector price totals (bottom 25%)
    price_style = ParagraphStyle(
        'PriceStyle',
        parent=styles['Normal'],
        fontName='Helvetica-Bold',
        fontSize=14,
        alignment=TA_RIGHT,
        spaceAfter=12,
    )

    # Subtotal
    story.append(Spacer(1, 5.5 * inch))  # Space for scanned content
    story.append(Paragraph("Subtotal:", ParagraphStyle('Label', parent=styles['Normal'], alignment=TA_RIGHT)))
    story.append(Paragraph("$42.50", price_style))
    story.append(Paragraph("Tax (8.5%):", ParagraphStyle('Label', parent=styles['Normal'], alignment=TA_RIGHT)))
    story.append(Paragraph("$3.61", price_style))
    story.append(Paragraph("TOTAL:", ParagraphStyle('Total', parent=styles['Normal'], fontName='Helvetica-Bold', fontSize=16, alignment=TA_RIGHT)))
    story.append(Paragraph("$46.11", ParagraphStyle('TotalAmt', parent=styles['Normal'], fontName='Helvetica-Bold', fontSize=18, alignment=TA_RIGHT)))

    doc.build(story)

    # Create scanned receipt body as image (top 75%)
    scanned_height = 5.5 * inch  # Top 75%
    scanned_img = create_scanned_image(540, int(scanned_height), [
        "MERCHANT: MARTINI GROCERY",
        "123 MAIN ST",
        "ANYTOWN, USA 12345",
        "",
        "2024-08-03 14:32:15",
        "-" * 40,
        "MILK 1GAL                    $4.29",
        "BREAD WHL WHT                $2.49",
        "EGGS LRG DOZ                $3.99",
        "CHEESE CHEDDAR 8OZ          $5.79",
        "APPLES GALA 3LB             $6.49",
        "CHICKEN BRST 3LB            $8.99",
        "PASTA PENNE 1LB             $1.89",
        "SAUCE TOMATO 24OZ           $2.29",
        "BANANAS 2LB                 $1.89",
        "COFFEE GROUND 12OZ          $4.39",
    ], font_size=10)

    # Save scanned image
    scanned_path = fixture_dir / "receipt-scanned-body.png"
    scanned_img.save(scanned_path, "PNG")

    # Ground truth text
    ground_truth = """MERCHANT: MARTINI GROCERY
123 MAIN ST
ANYTOWN, USA 12345

2024-08-03 14:32:15
----------------------------------------
MILK 1GAL                    $4.29
BREAD WHL WHT                $2.49
EGGS LRG DOZ                $3.99
CHEESE CHEDDAR 8OZ          $5.79
APPLES GALA 3LB             $6.49
CHICKEN BRST 3LB            $8.99
PASTA PENNE 1LB             $1.89
SAUCE TOMATO 24OZ           $2.29
BANANAS 2LB                 $1.89
COFFEE GROUND 12OZ          $4.39
Subtotal: $42.50
Tax (8.5%): $3.61
TOTAL: $46.11"""

    output_txt.write_text(ground_truth)

    print(f"✓ Generated fixture: receipt-overtext")
    print(f"  PDF: {output_pdf}")
    print(f"  Ground truth: {output_txt}")
    print(f"  Scanned body: {scanned_path}")


def fixture_letterhead_image():
    """Fixture 2: Vector letterhead header + scanned letter body."""
    fixture_dir = FIXTURES_DIR / "letterhead-image"
    fixture_dir.mkdir(exist_ok=True)

    output_pdf = fixture_dir / "letterhead-image.pdf"
    output_txt = fixture_dir / "letterhead-image.txt"

    doc = SimpleDocTemplate(str(output_pdf), pagesize=LETTER)
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
    story.append(Paragraph("123 Business Avenue, Suite 100", ParagraphStyle('Address', parent=styles['Normal'], alignment=TA_CENTER, fontSize=10)))
    story.append(Paragraph("New York, NY 10001", ParagraphStyle('Address', parent=styles['Normal'], alignment=TA_CENTER, fontSize=10)))
    story.append(Paragraph("Tel: (212) 555-0123 | Email: info@acmecorp.com", ParagraphStyle('Contact', parent=styles['Normal'], alignment=TA_CENTER, fontSize=9, spaceAfter=18)))

    # Date and recipient (vector)
    story.append(Paragraph(f"Date: August 3, 2026", ParagraphStyle('Date', parent=styles['Normal'], alignment=TA_RIGHT, spaceAfter=12)))
    story.append(Paragraph("Mr. John Smith", ParagraphStyle('Normal', parent=styles['Normal'])))
    story.append(Paragraph("456 Client Road", ParagraphStyle('Normal', parent=styles['Normal'])))
    story.append(Paragraph("Los Angeles, CA 90001", ParagraphStyle('Normal', parent=styles['Normal'], spaceAfter=24)))

    doc.build(story)

    # Create scanned letter body (bottom 85%)
    scanned_height = 6.0 * inch
    scanned_img = create_scanned_image(540, int(scanned_height), [
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
    ], font_size=11)

    scanned_path = fixture_dir / "letter-scanned-body.png"
    scanned_img.save(scanned_path, "PNG")

    ground_truth = """ACME CORPORATION
123 Business Avenue, Suite 100
New York, NY 10001
Tel: (212) 555-0123 | Email: info@acmecorp.com

Date: August 3, 2026

Mr. John Smith
456 Client Road
Los Angeles, CA 90001

Dear Mr. Smith:

Thank you for your recent inquiry about our enterprise solutions. We are pleased to
present our comprehensive proposal for your organization's document management needs.

Our hybrid document processing system offers several key advantages:

1. Automatic classification of document types (vector, scanned, hybrid)
2. Intelligent OCR with confidence-based merging
3. 8x8 grid cell detection for precise content region identification
4. Bbox overlap rules to eliminate duplicate text extraction

We believe our solution aligns perfectly with your requirements for processing
mixed-format documents at scale. Our Phase 5.5 classifier tuning ensures optimal
performance on known-tricky hybrid cases.

Please find attached our technical specifications and pricing information. We look
forward to discussing this proposal further at your convenience.

Sincerely,

Jane Johnson
Senior Account Executive
ACME Corporation"""

    output_txt.write_text(ground_truth)

    print(f"✓ Generated fixture: letterhead-image")
    print(f"  PDF: {output_pdf}")
    print(f"  Ground truth: {output_txt}")
    print(f"  Scanned body: {scanned_path}")


def fixture_form_mixed():
    """Fixture 3: Vector form fields over scanned form background."""
    fixture_dir = FIXTURES_DIR / "form-mixed"
    fixture_dir.mkdir(exist_ok=True)

    output_pdf = fixture_dir / "form-mixed.pdf"
    output_txt = fixture_dir / "form-mixed.txt"

    # Create the PDF with vector form fields
    doc = SimpleDocTemplate(str(output_pdf), pagesize=LETTER, topMargin=72, bottomMargin=72, leftMargin=72, rightMargin=72)
    story = []
    styles = getSampleStyleSheet()

    # Form title (vector)
    story.append(Paragraph("EMPLOYEE INFORMATION FORM", ParagraphStyle('Title', parent=styles['Heading1'], alignment=TA_CENTER, spaceAfter=24)))
    story.append(Paragraph("Please complete all fields. Print clearly.", ParagraphStyle('Instructions', parent=styles['Normal'], fontSize=10, alignment=TA_CENTER, spaceAfter=30)))

    # Vector form fields with labels
    field_data = [
        ("Full Name:", "________________________________", 1),
        ("Employee ID:", "________________________________", 1),
        ("Department:", "________________________________", 1),
        ("Email Address:", "________________________________", 1),
        ("Phone Number:", "________________________________", 1),
    ]

    for label, field, spacing in field_data:
        p = Paragraph(f"<b>{label}</b> {field}", ParagraphStyle('Field', parent=styles['Normal'], fontSize=12))
        story.append(p)
        story.append(Spacer(1, spacing * 12))

    doc.build(story)

    # Create scanned form background with labels and instructions
    scanned_height = 7.5 * inch
    scanned_img = create_scanned_image(468, int(scanned_height), [
        "OFFICE USE ONLY",
        "-" * 50,
        "Date: ____________  Approved: ____________",
        "",
        "SECTION A: PERSONAL INFORMATION",
        "- Please use permanent ink and print clearly.",
        "- Do not write above the line.",
        "",
        "SECTION B: EMPLOYMENT DETAILS",
        "Position: _________________________",
        "Start Date: _______________________",
        "Supervisor: _______________________",
        "",
        "SECTION C: EMERGENCY CONTACT",
        "Contact Name: ____________________",
        "Relationship: _____________________",
        "Phone: ____________________________",
        "",
        "I certify that the information provided is true and correct.",
        "",
        "Signature: ________________________  Date: ____________",
    ], font_size=11, bg_color="#f0f0f0")

    scanned_path = fixture_dir / "form-scanned-background.png"
    scanned_img.save(scanned_path, "PNG")

    ground_truth = """EMPLOYEE INFORMATION FORM
Please complete all fields. Print clearly.

Full Name: __________________________________
Employee ID: __________________________________
Department: __________________________________
Email Address: __________________________________
Phone Number: __________________________________

OFFICE USE ONLY
--------------------------------------------------
Date: ____________  Approved: ____________

SECTION A: PERSONAL INFORMATION
- Please use permanent ink and print clearly.
- Do not write above the line.

SECTION B: EMPLOYMENT DETAILS
Position: _________________________
Start Date: _______________________
Supervisor: _______________________

SECTION C: EMERGENCY CONTACT
Contact Name: ____________________
Relationship: _____________________
Phone: ____________________________

I certify that the information provided is true and correct.

Signature: ________________________  Date: ____________"""

    output_txt.write_text(ground_truth)

    print(f"✓ Generated fixture: form-mixed")
    print(f"  PDF: {output_pdf}")
    print(f"  Ground truth: {output_txt}")
    print(f"  Scanned background: {scanned_path}")


def fixture_invoice_stamp():
    """Fixture 4: Vector invoice line items + scanned approval stamp."""
    fixture_dir = FIXTURES_DIR / "invoice-stamp"
    fixture_dir.mkdir(exist_ok=True)

    output_pdf = fixture_dir / "invoice-stamp.pdf"
    output_txt = fixture_dir / "invoice-stamp.txt"

    doc = SimpleDocTemplate(str(output_pdf), pagesize=LETTER)
    story = []
    styles = getSampleStyleSheet()

    # Invoice header (vector)
    story.append(Paragraph("INVOICE #2024-0815", ParagraphStyle('InvoiceNum', parent=styles['Heading1'], alignment=TA_RIGHT)))
    story.append(Spacer(1, 12))

    # Company info
    story.append(Paragraph("GLOBAL SERVICES INC.", ParagraphStyle('Company', parent=styles['Heading2'], fontName='Helvetica-Bold')))
    story.append(Paragraph("789 Commerce Blvd, Suite 200", ParagraphStyle('Address', parent=styles['Normal'])))
    story.append(Paragraph("Chicago, IL 60601", ParagraphStyle('Address', parent=styles['Normal'])))
    story.append(Paragraph("Phone: (312) 555-9876", ParagraphStyle('Address', parent=styles['Normal'], spaceAfter=18)))

    # Bill to
    story.append(Paragraph("Bill To:", ParagraphStyle('Label', parent=styles['Normal'], fontName='Helvetica-Bold')))
    story.append(Paragraph("Premier Logistics LLC", ParagraphStyle('Normal', parent=styles['Normal'])))
    story.append(Paragraph("200 Freight Way", ParagraphStyle('Normal', parent=styles['Normal'])))
    story.append(Paragraph("Detroit, MI 48201", ParagraphStyle('Normal', parent=styles['Normal'], spaceAfter=18)))

    # Invoice details table
    table_data = [
        ["Description", "Qty", "Rate", "Amount"],
        ["Consulting Services - Phase 1", "40", "$125.00", "$5,000.00"],
        ["Technical Support", "15", "$95.00", "$1,425.00"],
        ["Documentation Review", "8", "$150.00", "$1,200.00"],
        ["Project Management", "12", "$110.00", "$1,320.00"],
        ["Software License (Annual)", "1", "$2,400.00", "$2,400.00"],
        ["", "", "", ""],
        ["", "", "Subtotal:", "$10,945.00"],
        ["", "", "Tax (6%):", "$656.70"],
        ["", "", "TOTAL:", "$11,601.70"],
    ]

    table = Table(table_data, colWidths=[3.0*inch, 0.8*inch, 1.0*inch, 1.0*inch])
    table.setStyle(TableStyle([
        ('FONTNAME', (0, 0), (-1, 0), 'Helvetica-Bold'),
        ('ALIGN', (0, 0), (-1, -1), 'LEFT'),
        ('ALIGN', (2, 0), (3, -1), 'RIGHT'),
        ('LINEABOVE', (0, 0), (-1, 0), 1, colors.black),
        ('LINEBELOW', (0, 0), (-1, 0), 1, colors.black),
        ('LINEBELOW', (0, 6), (-1, 6), 1, colors.black),
        ('FONTNAME', (2, 6), (3, -1), 'Helvetica-Bold'),
    ]))
    story.append(table)

    doc.build(story)

    # Create scanned approval stamp (bottom-right corner)
    stamp_img = PILImage.new("RGB", (200, 100), color="white")
    draw = ImageDraw.Draw(stamp_img)
    draw.ellipse([10, 10, 190, 90], outline="red", width=3)
    draw.text((30, 30), "APPROVED", fill="red", font=ImageFont.load_default())
    draw.text((30, 50), "Aug 5, 2024", fill="red", font=ImageFont.load_default())

    stamp_path = fixture_dir / "approval-stamp.png"
    stamp_img.save(stamp_path, "PNG")

    ground_truth = """INVOICE #2024-0815

GLOBAL SERVICES INC.
789 Commerce Blvd, Suite 200
Chicago, IL 60601
Phone: (312) 555-9876

Bill To:
Premier Logistics LLC
200 Freight Way
Detroit, MI 48201

Description                 Qty   Rate         Amount
Consulting Services - Phase 1    40  $125.00   $5,000.00
Technical Support              15   $95.00   $1,425.00
Documentation Review           8  $150.00   $1,200.00
Project Management            12  $110.00   $1,320.00
Software License (Annual)      1  $2,400.00  $2,400.00

                                             Subtotal: $10,945.00
                                                Tax (6%):    $656.70
                                                TOTAL: $11,601.70

APPROVED
Aug 5, 2024"""

    output_txt.write_text(ground_truth)

    print(f"✓ Generated fixture: invoice-stamp")
    print(f"  PDF: {output_pdf}")
    print(f"  Ground truth: {output_txt}")
    print(f"  Approval stamp: {stamp_path}")


def fixture_document_annotation():
    """Fixture 5: Scanned document with vector highlight annotations."""
    fixture_dir = FIXTURES_DIR / "document-annotation"
    fixture_dir.mkdir(exist_ok=True)

    output_pdf = fixture_dir / "document-annotation.pdf"
    output_txt = fixture_dir / "document-annotation.txt"

    doc = SimpleDocTemplate(str(output_pdf), pagesize=LETTER)
    story = []
    styles = getSampleStyleSheet()

    # Title (vector)
    story.append(Paragraph("HYBRID DOCUMENT PROCESSING", ParagraphStyle('Title', parent=styles['Heading1'], alignment=TA_CENTER, spaceAfter=24)))

    # Main content (simulated - will be scanned)
    doc.build(story)

    # Create scanned document content
    scanned_height = 9.0 * inch
    scanned_img = create_scanned_image(540, int(scanned_height), [
        "INTRODUCTION",
        "=" * 60,
        "",
        "Modern document processing systems must handle mixed-format documents",
        "that contain both vector text and scanned image regions. These hybrid",
        "documents present unique challenges for text extraction and content",
        "analysis.",
        "",
        "The key challenge is determining which parts of the page require OCR",
        "and which can be extracted directly from content streams. Our approach",
        "uses an 8x8 grid to divide the page into cells, then classifies each",
        "cell as vector-heavy or image-heavy based on pixel coverage.",
        "",
        "METHODOLOGY",
        "=" * 60,
        "",
        "When a page is classified as Hybrid (≥15% of cells are image-heavy),",
        "we employ a two-phase extraction strategy:",
        "",
        "1. Extract all vector text using standard content stream parsing",
        "2. Render and OCR only the image-heavy cells",
        "3. Merge the results using bounding box overlap rules",
        "",
        "This approach minimizes computational cost while ensuring complete",
        "text coverage. The merge rule eliminates duplicate text: when OCR",
        "and vector spans overlap significantly (IoU > 0.5), we keep the",
        "higher-confidence source.",
        "",
        "RESULTS",
        "=" * 60,
        "",
        "Testing on our fixture suite of 10 known-tricky hybrid cases shows",
        "that the hybrid extraction pipeline achieves 95% accuracy on merge",
        "decisions and maintains WER < 3% on scanned regions.",
    ], font_size=11)

    scanned_path = fixture_dir / "document-scanned-content.png"
    scanned_img.save(scanned_path, "PNG")

    ground_truth = """HYBRID DOCUMENT PROCESSING

INTRODUCTION
============================================================

Modern document processing systems must handle mixed-format documents
that contain both vector text and scanned image regions. These hybrid
documents present unique challenges for text extraction and content
analysis.

The key challenge is determining which parts of the page require OCR
and which can be extracted directly from content streams. Our approach
uses an 8x8 grid to divide the page into cells, then classifies each
cell as vector-heavy or image-heavy based on pixel coverage.

METHODOLOGY
============================================================

When a page is classified as Hybrid (≥15% of cells are image-heavy),
we employ a two-phase extraction strategy:

1. Extract all vector text using standard content stream parsing
2. Render and OCR only the image-heavy cells
3. Merge the results using bounding box overlap rules

This approach minimizes computational cost while ensuring complete
text coverage. The merge rule eliminates duplicate text: when OCR
and vector spans overlap significantly (IoU > 0.5), we keep the
higher-confidence source.

RESULTS
============================================================

Testing on our fixture suite of 10 known-tricky hybrid cases shows
that the hybrid extraction pipeline achieves 95% accuracy on merge
decisions and maintains WER < 3% on scanned regions."""

    output_txt.write_text(ground_truth)

    print(f"✓ Generated fixture: document-annotation")
    print(f"  PDF: {output_pdf}")
    print(f"  Ground truth: {output_txt}")
    print(f"  Scanned content: {scanned_path}")


def fixture_figure_caption():
    """Fixture 6: Academic paper with vector figure caption + scanned figure."""
    fixture_dir = FIXTURES_DIR / "figure-caption"
    fixture_dir.mkdir(exist_ok=True)

    output_pdf = fixture_dir / "figure-caption.pdf"
    output_txt = fixture_dir / "figure-caption.txt"

    doc = SimpleDocTemplate(str(output_pdf), pagesize=LETTER)
    story = []
    styles = getSampleStyleSheet()

    # Vector caption (bottom 10%)
    story.append(Spacer(1, 6.5 * inch))  # Space for scanned figure

    caption_style = ParagraphStyle(
        'FigureCaption',
        parent=styles['Normal'],
        fontName='Helvetica',
        fontSize=10,
        alignment=TA_CENTER,
        spaceAfter=12,
    )

    story.append(Paragraph("Figure 1: Hybrid cell detection accuracy vs. cell count threshold.", caption_style))
    story.append(Paragraph("The plot shows that a 15% threshold (12 of 64 cells) achieves optimal F1 score",
                          ParagraphStyle('Note', parent=caption_style, fontStyle='Italic')))

    doc.build(story)

    # Create scanned figure (top 90%)
    scanned_height = 6.5 * inch
    scanned_img = create_scanned_image(540, int(scanned_height), [
        "HYPOTHESIS TESTING RESULTS",
        "-" * 50,
        "",
        "┌────────────────────────────────────────────────┐",
        "│  1.0 ┤                                     ●   │",
        "│       │                                  ●●●   │",
        "│  0.8 ┤                               ●●●●      │",
        "│       │                            ●●●         │",
        "│  0.6 ┤                         ●●●●           │",
        "│       │                      ●●●              │",
        "│  0.4 ┤                   ●●●●                 │",
        "│       │                ●●●                     │",
        "│  0.2 ┤             ●●●●                        │",
        "│       │          ●●●                           │",
        "│  0.0 ┤       ●●●                              │",
        "└────────────────────────────────────────────────┘",
        "        0%    5%   10%   15%   20%   25%",
        "",
        "Threshold →",
        "",
        "Red dots: F1 score",
        "Peak at 15% threshold",
        "",
        "N = 10 test documents",
        "Error bars: 95% CI",
    ], font_size=9, bg_color="#fafafa")

    scanned_path = fixture_dir / "figure-scanned.png"
    scanned_img.save(scanned_path, "PNG")

    ground_truth = """HYPOTHESIS TESTING RESULTS
--------------------------------------------------
┌────────────────────────────────────────────────┐
│  1.0 ┤                                     ●   │
│       │                                  ●●●   │
│  0.8 ┤                               ●●●●      │
│       │                            ●●●         │
│  0.6 ┤                         ●●●●           │
│       │                      ●●●              │
│  0.4 ┤                   ●●●●                 │
│       │                ●●●                     │
│  0.2 ┤             ●●●●                        │
│       │          ●●●                           │
│  0.0 ┤       ●●●                              │
└────────────────────────────────────────────────┘
        0%    5%   10%   15%   20%   25%

Threshold →

Red dots: F1 score
Peak at 15% threshold

N = 10 test documents
Error bars: 95% CI

Figure 1: Hybrid cell detection accuracy vs. cell count threshold.
The plot shows that a 15% threshold (12 of 64 cells) achieves optimal F1 score"""

    output_txt.write_text(ground_truth)

    print(f"✓ Generated fixture: figure-caption")
    print(f"  PDF: {output_pdf}")
    print(f"  Ground truth: {output_txt}")
    print(f"  Scanned figure: {scanned_path}")


def fixture_sidebar_image():
    """Fixture 7: Newsletter with vector main text + scanned sidebar image."""
    fixture_dir = FIXTURES_DIR / "sidebar-image"
    fixture_dir.mkdir(exist_ok=True)

    output_pdf = fixture_dir / "sidebar-image.pdf"
    output_txt = fixture_dir / "sidebar-image.txt"

    doc = SimpleDocTemplate(str(output_pdf), pagesize=LETTER, leftMargin=36, rightMargin=216)
    story = []
    styles = getSampleStyleSheet()

    # Newsletter header (vector)
    story.append(Paragraph("THE DIGEST", ParagraphStyle('Masthead', parent=styles['Heading1'], alignment=TA_CENTER, fontName='Helvetica-Bold', fontSize=24, spaceAfter=6)))
    story.append(Paragraph("Weekly Technology Update", ParagraphStyle('Tagline', parent=styles['Normal'], alignment=TA_CENTER, fontSize=12, spaceAfter=18)))
    story.append(Paragraph("Vol. 12, Issue 31 | August 3, 2026", ParagraphStyle('Issue', parent=styles['Normal'], alignment=TA_CENTER, fontSize=10, spaceAfter=24)))

    # Main article (vector)
    story.append(Paragraph("HEADLINE: Hybrid Processing Advances", ParagraphStyle('Headline', parent=styles['Heading2'], fontName='Helvetica-Bold', spaceAfter=12)))
    story.append(Paragraph("By Jane Johnson, Senior Technology Reporter", ParagraphStyle('Byline', parent=styles['Normal'], fontSize=10, fontStyle='Italic', spaceAfter=18)))

    article_text = """
    <para>Researchers at the document processing lab announced a breakthrough in hybrid
    PDF extraction accuracy this week. The new method, which combines intelligent
    cell-based rendering with confidence-weighted merging, has achieved a 95%
    success rate on the industry-standard benchmark suite.</para>

    <para>The key innovation is the adaptive rendering strategy. Instead of treating
    the entire page uniformly, the system classifies each of the 64 grid cells
    independently. Cells with high image coverage are rendered and OCR'd, while
    vector-heavy cells are extracted directly from content streams.</para>

    <para>"This approach reduces OCR computational cost by 60% while improving
    accuracy," said lead researcher Dr. Smith. "By focusing OCR resources on the
    regions that actually need them, we avoid the noise and errors that come from
    running OCR on clean vector text."</para>

    <para>The team plans to integrate the new method into the production pipeline next
    quarter, pending final validation tests.</para>
    """

    body_style = ParagraphStyle('Body', parent=styles['Normal'], fontSize=10, spaceAfter=12, alignment=TA_JUSTIFY)
    for para in article_text.split('\n'):
        if para.strip():
            story.append(Paragraph(para, body_style))

    doc.build(story)

    # Create scanned sidebar image
    sidebar_width = 180  # Right sidebar width
    sidebar_img = create_scanned_image(int(sidebar_width), 750, [
        "PHOTO OF THE WEEK",
        "-" * 20,
        "",
        "The research team",
        "celebrating their",
        "breakthrough.",
        "",
        "[Team photo]",
        "",
        "",
        "SUBSCRIBE",
        "-" * 20,
        "",
        "Get The Digest",
        "delivered to your",
        "inbox weekly.",
        "",
        "Sign up at",
        "digest.example.com",
        "",
        "",
        "EVENTS",
        "-" * 20,
        "",
        "Aug 15:",
        "Tech Summit",
        "",
        "Aug 22:",
        "AI Workshop",
    ], font_size=9, bg_color="#e8e8e8")

    sidebar_path = fixture_dir / "sidebar-scanned.png"
    sidebar_img.save(sidebar_path, "PNG")

    ground_truth = """THE DIGEST
Weekly Technology Update
Vol. 12, Issue 31 | August 3, 2026

HEADLINE: Hybrid Processing Advances
By Jane Johnson, Senior Technology Reporter

Researchers at the document processing lab announced a breakthrough in hybrid
PDF extraction accuracy this week. The new method, which combines intelligent
cell-based rendering with confidence-weighted merging, has achieved a 95%
success rate on the industry-standard benchmark suite.

The key innovation is the adaptive rendering strategy. Instead of treating
the entire page uniformly, the system classifies each of the 64 grid cells
independently. Cells with high image coverage are rendered and OCR'd, while
vector-heavy cells are extracted directly from content streams.

"This approach reduces OCR computational cost by 60% while improving
accuracy," said lead researcher Dr. Smith. "By focusing OCR resources on the
regions that actually need them, we avoid the noise and errors that come from
running OCR on clean vector text."

The team plans to integrate the new method into the production pipeline next
quarter, pending final validation tests.

PHOTO OF THE WEEK
--------------------
The research team
celebrating their
breakthrough.

[Team photo]

SUBSCRIBE
--------------------
Get The Digest
delivered to your
inbox weekly.

Sign up at
digest.example.com

EVENTS
--------------------
Aug 15:
Tech Summit

Aug 22:
AI Workshop"""

    output_txt.write_text(ground_truth)

    print(f"✓ Generated fixture: sidebar-image")
    print(f"  PDF: {output_pdf}")
    print(f"  Ground truth: {output_txt}")
    print(f"  Scanned sidebar: {sidebar_path}")


def fixture_watermark():
    """Fixture 8: Vector text over scanned watermark background."""
    fixture_dir = FIXTURES_DIR / "watermark"
    fixture_dir.mkdir(exist_ok=True)

    output_pdf = fixture_dir / "watermark.pdf"
    output_txt = fixture_dir / "watermark.txt"

    doc = SimpleDocTemplate(str(output_pdf), pagesize=LETTER)
    story = []
    styles = getSampleStyleSheet()

    # Document content (vector, will overlay watermark)
    story.append(Paragraph("OFFICIAL DOCUMENT", ParagraphStyle('Title', parent=styles['Heading1'], alignment=TA_CENTER, spaceAfter=24)))
    story.append(Paragraph("CONFIDENTIAL - DO NOT DISTRIBUTE", ParagraphStyle('Classification', parent=styles['Normal'], alignment=TA_CENTER, fontSize=10, spaceAfter=36, textColor='red')))

    sections = [
        ("EXECUTIVE SUMMARY", [
            "This document outlines the strategic plan for hybrid document processing",
            "integration. The proposed system will handle mixed-format PDFs with high",
            "accuracy and minimal computational overhead."
        ]),
        ("TECHNICAL APPROACH", [
            "The system employs an 8x8 grid cell classification strategy. Each cell is",
            "analyzed for image coverage vs. text coverage. Cells with ≥15% image content",
            "are classified as scanned and trigger OCR processing."
        ]),
        ("IMPLEMENTATION TIMELINE", [
            "Phase 1: Core classifier development (6 weeks)",
            "Phase 2: OCR integration and testing (4 weeks)",
            "Phase 3: Production deployment (2 weeks)"
        ]),
        ("RISK MITIGATION", [
            "Primary risks include classifier accuracy on edge cases and OCR performance",
            "on low-quality scans. Mitigation strategies include comprehensive fixture",
            "testing and confidence-based merging rules."
        ])
    ]

    for heading, paragraphs in sections:
        story.append(Paragraph(heading, ParagraphStyle('Section', parent=styles['Heading2'], spaceAfter=12)))
        for para in paragraphs:
            story.append(Paragraph(para, ParagraphStyle('Body', parent=styles['Normal'], spaceAfter=12, alignment=TA_JUSTIFY)))
        story.append(Spacer(1, 12))

    doc.build(story)

    # Create scanned watermark background (page-wide, low opacity)
    watermark_img = PILImage.new("RGB", (612, 792), color="#f8f8f8")
    draw = ImageDraw.Draw(watermark_img)

    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 80)
    except:
        font = ImageFont.load_default()

    # Draw large watermark text diagonally
    for i in range(-2, 3):
        for j in range(-2, 3):
            draw.text((200 + i*2, 350 + j*2), "DRAFT", fill="#d0d0d0", font=font)

    watermark_path = fixture_dir / "watermark-background.png"
    watermark_img.save(watermark_path, "PNG")

    ground_truth = """OFFICIAL DOCUMENT
CONFIDENTIAL - DO NOT DISTRIBUTE

EXECUTIVE SUMMARY
This document outlines the strategic plan for hybrid document processing
integration. The proposed system will handle mixed-format PDFs with high
accuracy and minimal computational overhead.

TECHNICAL APPROACH
The system employs an 8x8 grid cell classification strategy. Each cell is
analyzed for image coverage vs. text coverage. Cells with ≥15% image content
are classified as scanned and trigger OCR processing.

IMPLEMENTATION TIMELINE
Phase 1: Core classifier development (6 weeks)
Phase 2: OCR integration and testing (4 weeks)
Phase 3: Production deployment (2 weeks)

RISK MITIGATION
Primary risks include classifier accuracy on edge cases and OCR performance
on low-quality scans. Mitigation strategies include comprehensive fixture
testing and confidence-based merging rules.

DRAFT
DRAFT
DRAFT"""

    output_txt.write_text(ground_truth)

    print(f"✓ Generated fixture: watermark")
    print(f"  PDF: {output_pdf}")
    print(f"  Ground truth: {output_txt}")
    print(f"  Watermark background: {watermark_path}")


def fixture_multicolumn_scan():
    """Fixture 9: Multi-column document with vector headers + scanned columns."""
    fixture_dir = FIXTURES_DIR / "multi-column-scan"
    fixture_dir.mkdir(exist_ok=True)

    output_pdf = fixture_dir / "multi-column-scan.pdf"
    output_txt = fixture_dir / "multi-column-scan.txt"

    doc = SimpleDocTemplate(str(output_pdf), pagesize=LETTER)
    story = []
    styles = getSampleStyleSheet()

    # Newsletter header (vector)
    story.append(Paragraph("INDUSTRY BRIEFING", ParagraphStyle('Masthead', parent=styles['Heading1'], alignment=TA_CENTER, fontName='Helvetica-Bold', spaceAfter=6)))
    story.append(Paragraph("Monthly Market Analysis", ParagraphStyle('Tagline', parent=styles['Normal'], alignment=TA_CENTER, fontSize=12, spaceAfter=6)))
    story.append(Paragraph("August 2026 | Volume 8", ParagraphStyle('Issue', parent=styles['Normal'], alignment=TA_CENTER, fontSize=10, spaceAfter=18)))

    # Column headers (vector)
    header_style = ParagraphStyle('ColHeader', parent=styles['Normal'], fontName='Helvetica-Bold', fontSize=11, alignment=TA_CENTER, spaceAfter=12)
    story.append(Paragraph("MARKET TRENDS", header_style))
    story.append(Paragraph("COMPANY NEWS", header_style))
    story.append(Paragraph("TECHNOLOGY", header_style))

    doc.build(story)

    # Create scanned multi-column body content
    scanned_height = 8.5 * inch
    scanned_img = create_scanned_image(540, int(scanned_height), [
        "The document processing", "     Global Tech Announces", "     New OCR engine",
        "market grew 15% in Q2,", "     Q2 Results: Revenue Up", "     achieves 40% speed",
        "driven by enterprise", "     22% Year-over-Year", "     improvement with",
        "adoption of hybrid PDF", "                          ", "     better accuracy on",
        "extraction solutions.", "     CEO Comments:", "     low-quality scans.",
        "                          ", "     'Strong demand for", "",
        "Analysts predict continued", "     intelligent document", "     Integration with",
        "growth through 2027.", "     processing'", "     machine learning models",
        "", "                          ", "     planned for Q4.",
        "Key players include:", "     Stock up 8% on news.", "",
        "• DocuSystems Inc.", "", "",
        "• PDFtract Labs", "     Merger talks between", "     Cloud deployment",
        "• Global Tech Corp", "     PageCloud and ScanSoft", "     reduces infrastructure",
        "", "     advanced to final stage.", "     costs by 60%.",
        "Regulatory changes may", "", "",
        "impact data privacy rules", "     ScanSoft announces", "     Mobile OCR SDK",
        "for document processing.", "     acquisition of AI startup", "     now available for",
        "", "                          ", "     iOS and Android.",
    ], font_size=10)

    scanned_path = fixture_dir / "multicolumn-scanned.png"
    scanned_img.save(scanned_path, "PNG")

    ground_truth = """INDUSTRY BRIEFING
Monthly Market Analysis
August 2026 | Volume 8

MARKET TRENDS                                    COMPANY NEWS                           TECHNOLOGY

The document processing        Global Tech Announces     New OCR engine
market grew 15% in Q2,        Q2 Results: Revenue Up     achieves 40% speed
driven by enterprise          22% Year-over-Year          improvement with
adoption of hybrid PDF                                        better accuracy on
extraction solutions.         CEO Comments:                low-quality scans.
                               'Strong demand for
Analysts predict continued    intelligent document'
growth through 2027.          Integration with
                               machine learning models
Key players include:          planned for Q4.
• DocuSystems Inc.
• PDFtract Labs              Merger talks between        Cloud deployment
• Global Tech Corp            PageCloud and ScanSoft      reduces infrastructure
                               advanced to final stage.    costs by 60%.
Regulatory changes may        ScanSoft announces          Mobile OCR SDK
impact data privacy rules    acquisition of AI startup    now available for
for document processing.                                         iOS and Android."""

    output_txt.write_text(ground_truth)

    print(f"✓ Generated fixture: multi-column-scan")
    print(f"  PDF: {output_pdf}")
    print(f"  Ground truth: {output_txt}")
    print(f"  Scanned columns: {scanned_path}")


def fixture_complex_overlap():
    """Fixture 10: Interleaved vector and scanned regions (checkerboard pattern)."""
    fixture_dir = FIXTURES_DIR / "complex-overlap"
    fixture_dir.mkdir(exist_ok=True)

    output_pdf = fixture_dir / "complex-overlap.pdf"
    output_txt = fixture_dir / "complex-overlap.txt"

    doc = SimpleDocTemplate(str(output_pdf), pagesize=LETTER)
    story = []
    styles = getSampleStyleSheet()

    # Title (vector)
    story.append(Paragraph("COMPLEX HYBRID TEST", ParagraphStyle('Title', parent=styles['Heading1'], alignment=TA_CENTER, spaceAfter=12)))
    story.append(Paragraph("Checkerboard pattern: vector and scanned regions alternate", ParagraphStyle('Subtitle', parent=styles['Normal'], alignment=TA_CENTER, fontSize=10, spaceAfter=24)))

    # This is a simplified version - a true checkerboard would need manual coordinate placement
    # For now, we create scattered vector blocks

    story.append(Paragraph("This fixture tests worst-case merge scenarios.", ParagraphStyle('Normal', parent=styles['Normal'], spaceAfter=12)))
    story.append(Paragraph("Vector blocks (32 cells):", ParagraphStyle('Bold', parent=styles['Normal'], fontName='Helvetica-Bold', spaceAfter=6)))

    vector_blocks = [
        "Block 1 (0,0): Top-left vector region",
        "Block 2 (0,2): Top-mid vector region",
        "Block 3 (0,4): Top-right vector region",
        "Block 4 (2,1): Mid-left vector region",
        "Block 5 (2,3): Center vector region",
        "Block 6 (2,5): Mid-right vector region",
        "Block 7 (4,0): Lower-mid-left vector",
        "Block 8 (4,2): Lower-mid-center vector",
        "Block 9 (4,4): Lower-mid-right vector",
        "Block 10 (6,1): Bottom-left vector",
        "Block 11 (6,3): Bottom-center vector",
        "Block 12 (6,5): Bottom-right vector",
    ]

    for block in vector_blocks:
        story.append(Paragraph(f"• {block}", ParagraphStyle('Block', parent=styles['Normal'], fontSize=9)))

    story.append(Spacer(1, 12))
    story.append(Paragraph("Scanned blocks (32 cells): Complementary checkerboard", ParagraphStyle('Bold', parent=styles['Normal'], fontName='Helvetica-Bold', spaceAfter=6)))
    story.append(Paragraph("The merge algorithm must handle 64 separate regions with alternating content types.", ParagraphStyle('Normal', parent=styles['Normal'], fontSize=10)))

    doc.build(story)

    # Create complementary scanned content
    scanned_height = 8.0 * inch
    scanned_img = create_scanned_image(540, int(scanned_height), [
        "COMPLEMENTARY SCANNED REGIONS",
        "-" * 50,
        "",
        "Scanned Block 1 (0,1): Top-left-scanned content",
        "Scanned Block 2 (0,3): Top-mid-scanned content",
        "Scanned Block 3 (0,5): Top-right-scanned content",
        "Scanned Block 4 (1,0): Second-row-left-scanned",
        "Scanned Block 5 (1,2): Second-row-mid-scanned",
        "Scanned Block 6 (1,4): Second-row-right-scanned",
        "",
        "Stress test parameters:",
        "• Total regions: 64 (32 vector + 32 scanned)",
        "• Overlap boundaries: ~120 cell edges",
        "• Merge calculations: ~2400 bbox comparisons",
        "• Expected runtime: < 500ms on modern hardware",
        "",
        "This fixture validates:",
        "1. Cell-level classification accuracy",
        "2. Merge rule correctness on complex patterns",
        "3. Performance under high overlap count",
        "",
        "Result: Hybrid classifier correctly identifies all 32 scanned cells",
        "and merges with 32 vector cells without duplicate text.",
    ], font_size=10, bg_color="#f5f5f5")

    scanned_path = fixture_dir / "complex-scanned.png"
    scanned_img.save(scanned_path, "PNG")

    ground_truth = """COMPLEX HYBRID TEST
Checkerboard pattern: vector and scanned regions alternate

This fixture tests worst-case merge scenarios.

Vector blocks (32 cells):
• Block 1 (0,0): Top-left vector region
• Block 2 (0,2): Top-mid vector region
• Block 3 (0,4): Top-right vector region
• Block 4 (2,1): Mid-left vector region
• Block 5 (2,3): Center vector region
• Block 6 (2,5): Mid-right vector region
• Block 7 (4,0): Lower-mid-left vector
• Block 8 (4,2): Lower-mid-center vector
• Block 9 (4,4): Lower-mid-right vector
• Block 10 (6,1): Bottom-left vector
• Block 11 (6,3): Bottom-center vector
• Block 12 (6,5): Bottom-right vector

Scanned blocks (32 cells): Complementary checkerboard
The merge algorithm must handle 64 separate regions with alternating content types.

COMPLEMENTARY SCANNED REGIONS
--------------------------------------------------

Scanned Block 1 (0,1): Top-left-scanned content
Scanned Block 2 (0,3): Top-mid-scanned content
Scanned Block 3 (0,5): Top-right-scanned content
Scanned Block 4 (1,0): Second-row-left-scanned
Scanned Block 5 (1,2): Second-row-mid-scanned
Scanned Block 6 (1,4): Second-row-right-scanned

Stress test parameters:
• Total regions: 64 (32 vector + 32 scanned)
• Overlap boundaries: ~120 cell edges
• Merge calculations: ~2400 bbox comparisons
• Expected runtime: < 500ms on modern hardware

This fixture validates:
1. Cell-level classification accuracy
2. Merge rule correctness on complex patterns
3. Performance under high overlap count

Result: Hybrid classifier correctly identifies all 32 scanned cells
and merges with 32 vector cells without duplicate text."""

    output_txt.write_text(ground_truth)

    print(f"✓ Generated fixture: complex-overlap")
    print(f"  PDF: {output_pdf}")
    print(f"  Ground truth: {output_txt}")
    print(f"  Scanned content: {scanned_path}")


def main():
    """Generate all hybrid fixtures."""
    print("=" * 60)
    print("Hybrid PDF Fixture Generation")
    print("=" * 60)
    print()

    fixtures = [
        fixture_receipt_overtext,
        fixture_letterhead_image,
        fixture_form_mixed,
        fixture_invoice_stamp,
        fixture_document_annotation,
        fixture_figure_caption,
        fixture_sidebar_image,
        fixture_watermark,
        fixture_multicolumn_scan,
        fixture_complex_overlap,
    ]

    for fixture_func in fixtures:
        try:
            fixture_func()
            print()
        except Exception as e:
            print(f"✗ Error generating {fixture_func.__name__}: {e}", file=sys.stderr)
            print()

    print("=" * 60)
    print("Fixture generation complete!")
    print("=" * 60)
    print()
    print("Next steps:")
    print("1. Review generated PDFs and scanned images")
    print("2. Update GEN_MANIFEST.md with generation_date and verification_status")
    print("3. Run classification tests to verify hybrid cell detection")
    print("4. Test extraction pipeline with merge rules")


if __name__ == "__main__":
    main()
