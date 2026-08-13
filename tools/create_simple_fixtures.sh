#!/bin/bash
# Simplified Hybrid Fixture Creation Script
# Creates basic hybrid PDF fixtures using available tools (pdftk, echo, etc.)
# For full-featured fixtures, use generate_hybrid_fixtures.py with reportlab installed

set -e

FIXTURES_DIR="$(dirname "$0")"
cd "$FIXTURES_DIR"

echo "Creating simplified hybrid fixtures..."
echo "Note: This creates basic placeholder PDFs. For production-quality fixtures,"
echo "install reportlab/Pillow and run generate_hybrid_fixtures.py"
echo ""

# Function to create a simple text file
create_txt() {
    local path="$1"
    local content="$2"
    echo "$content" > "$path"
}

# Function to create a minimal PDF (placeholder)
create_placeholder_pdf() {
    local path="$1"
    local description="$2"

    # Create a minimal PDF header
    cat > "$path" << 'EOF'
%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Kids [ 3 0 R ]
/Count 1
/MediaBox [ 0 0 612 792 ]
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/Contents 4 0 R
>>
endobj
4 0 obj
<<
/Length 44
>>
stream
BT
/F1 12 Tf
50 700 Td
(Placeholder PDF) Tj
ET
endstream
endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000131 00000 n
0000000224 00000 n
trailer
<<
/Size 5
/Root 1 0 R
>>
startxref
304
%%EOF
EOF

    echo "Created placeholder PDF: $path"
}

# Fixture 1: receipt-overtext
echo "Creating receipt-overtext fixture..."
create_txt receipt-overtext/receipt-overtext.txt "MERCHANT: MARTINI GROCERY
123 MAIN ST
ANYTOWN, USA 12345

2024-08-03 14:32:15
----------------------------------------
MILK 1GAL                    \$4.29
BREAD WHL WHT                \$2.49
EGGS LRG DOZ                \$3.99
CHEESE CHEDDAR 8OZ          \$5.79
APPLES GALA 3LB             \$6.49
CHICKEN BRST 3LB            \$8.99
PASTA PENNE 1LB             \$1.89
SAUCE TOMATO 24OZ           \$2.29
BANANAS 2LB                 \$1.89
COFFEE GROUND 12OZ          \$4.39
Subtotal: \$42.50
Tax (8.5%): \$3.61
TOTAL: \$46.11"

create_placeholder_pdf receipt-overtext/receipt-overtext.pdf "Receipt with scanned body and vector prices"
create_txt receipt-overtext/README.md "Receipt fixture: scanned body (top 75%) + vector price overlay (bottom 25%)

Hybrid cells: ~24 cells (rows 5-7, all cols)
Overlap: partial (vector prices over scanned totals area)
Test: Merge rule with overlapping vector/OCR on price regions

This fixture tests e-receipt format where receipt is scanned but totals are
overlaid as vector for machine readability."

# Fixture 2: letterhead-image
echo "Creating letterhead-image fixture..."
create_txt letterhead-image/letterhead-image.txt "ACME CORPORATION
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
ACME Corporation"

create_placeholder_pdf letterhead-image/letterhead-image.pdf "Letter with vector header and scanned body"
create_txt letterhead-image/README.md "Letter fixture: vector header (top 15%) + scanned body (bottom 85%)

Hybrid cells: ~40 cells (rows 1-7, all cols, partial row 1)
Overlap: separate (clear boundary between header and body)
Test: Header extraction precision; OCR on body only; non-overlapping merge

This fixture tests business letter format with clear separation between
vector header and scanned content."

# Fixture 3: form-mixed
echo "Creating form-mixed fixture..."
create_txt form-mixed/form-mixed.txt "EMPLOYEE INFORMATION FORM
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

Signature: ________________________  Date: ____________"

create_placeholder_pdf form-mixed/form-mixed.pdf "Form with vector fields over scanned background"
create_txt form-mixed/README.md "Form fixture: vector form fields over scanned background

Hybrid cells: ~45 cells (majority of page, scattered vector overlay)
Overlap: partial (vector fields over scanned labels)
Test: Scattered vector extraction through cell-level OCR; complex merge patterns

This fixture simulates PDF forms where layout is scanned but fillable
fields are vector overlays."

# Fixture 4: invoice-stamp
echo "Creating invoice-stamp fixture..."
create_txt invoice-stamp/invoice-stamp.txt "INVOICE #2024-0815

GLOBAL SERVICES INC.
789 Commerce Blvd, Suite 200
Chicago, IL 60601
Phone: (312) 555-9876

Bill To:
Premier Logistics LLC
200 Freight Way
Detroit, MI 48201

Description                 Qty   Rate         Amount
Consulting Services - Phase 1    40  \$125.00   \$5,000.00
Technical Support              15   \$95.00   \$1,425.00
Documentation Review           8  \$150.00   \$1,200.00
Project Management            12  \$110.00   \$1,320.00
Software License (Annual)      1  \$2,400.00  \$2,400.00

                                             Subtotal: \$10,945.00
                                                Tax (6%):    \$656.70
                                                TOTAL: \$11,601.70

APPROVED
Aug 5, 2024"

create_placeholder_pdf invoice-stamp/invoice-stamp.pdf "Invoice with vector line items and scanned stamp"
create_txt invoice-stamp/README.md "Invoice fixture: vector line items + scanned approval stamp

Hybrid cells: ~12 cells (bottom-right corner where stamp overlaps)
Overlap: partial (stamp overlaps some vector totals)
Test: High-confidence vector vs OCR merge; stamp region OCR priority

This fixture tests that high-confidence vector content is not replaced
by OCR of overlapping stamp/signature."

# Fixture 5: document-annotation
echo "Creating document-annotation fixture..."
create_txt document-annotation/document-annotation.txt "HYBRID DOCUMENT PROCESSING

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
decisions and maintains WER < 3% on scanned regions."

create_placeholder_pdf document-annotation/document-annotation.pdf "Document with scanned content and vector annotations"
create_txt document-annotation/README.md "Document fixture: scanned content with vector annotations

Hybrid cells: ~36 cells (most of page has highlights or annotations)
Overlap: complete (annotations cover entire page)
Test: OCR priority for underlying content vs vector annotations

This fixture simulates annotated academic papers; tests that OCR
captures text under highlights while preserving annotation spans."

# Fixture 6: figure-caption
echo "Creating figure-caption fixture..."
create_txt figure-caption/figure-caption.txt "HYPOTHESIS TESTING RESULTS
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
The plot shows that a 15% threshold (12 of 64 cells) achieves optimal F1 score"

create_placeholder_pdf figure-caption/figure-caption.pdf "Figure with scanned content and vector caption"
create_txt figure-caption/README.md "Figure fixture: vector caption + scanned figure

Hybrid cells: ~8 cells (figure area only, rows 0-6)
Overlap: separate (clear boundary between figure and caption)
Test: Precise caption extraction; figure OCR accuracy; minimal cell coverage

This fixture tests hybrid detection on low-hybrid-cell-count pages
(8 cells = 12.5%, just below 15% threshold if miscounted)."

# Fixture 7: sidebar-image
echo "Creating sidebar-image fixture..."
create_txt sidebar-image/sidebar-image.txt "THE DIGEST
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

\"This approach reduces OCR computational cost by 60% while improving
accuracy,\" said lead researcher Dr. Smith. \"By focusing OCR resources on the
regions that actually need them, we avoid the noise and errors that come from
running OCR on clean vector text.\"

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
AI Workshop"

create_placeholder_pdf sidebar-image/sidebar-image.pdf "Newsletter with vector text and scanned sidebar"
create_txt sidebar-image/README.md "Sidebar fixture: vector main text + scanned sidebar image

Hybrid cells: ~24 cells (rightmost 3 columns, all rows)
Overlap: separate (vertical split, no overlap)
Test: Column-aware hybrid cell detection; side-by-side merge without conflicts

This fixture tests column detection with hybrid content; verifies OCR
runs only on sidebar columns."

# Fixture 8: watermark
echo "Creating watermark fixture..."
create_txt watermark/watermark.txt "OFFICIAL DOCUMENT
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
DRAFT"

create_placeholder_pdf watermark/watermark.pdf "Document with vector text over scanned watermark"
create_txt watermark/README.md "Watermark fixture: vector text over scanned watermark background

Hybrid cells: ~64 cells (full page, watermark is page-wide)
Overlap: complete (watermark underlies all text)
Test: Vector confidence vs OCR with low-contrast background; maximum hybrid cell count

This fixture tests worst-case for hybrid cell count (100% cells); tests that
vector text is extracted despite watermark background."

# Fixture 9: multi-column-scan
echo "Creating multi-column-scan fixture..."
create_txt multi-column-scan/multi-column-scan.txt "INDUSTRY BRIEFING
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
for document processing.                                         iOS and Android."

create_placeholder_pdf multi-column-scan/multi-column-scan.pdf "Multi-column doc with vector headers and scanned body"
create_txt multi-column-scan/README.md "Multi-column fixture: vector headers + scanned body columns

Hybrid cells: ~48 cells (body area, rows 1-7, all cols)
Overlap: partial (headers over first line of scanned content)
Test: Column detection + hybrid cell grid alignment; multi-column OCR

This fixture tests newsletter/magazine format; verifies that column
detection works correctly when columns are hybrid."

# Fixture 10: complex-overlap
echo "Creating complex-overlap fixture..."
create_txt complex-overlap/complex-overlap.txt "COMPLEX HYBRID TEST
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
and merges with 32 vector cells without duplicate text."

create_placeholder_pdf complex-overlap/complex-overlap.pdf "Complex checkerboard pattern of vector and scanned"
create_txt complex-overlap/README.md "Complex overlap fixture: checkerboard pattern of vector and scanned

Hybrid cells: ~32 cells (exactly half the page, every other cell)
Overlap: partial (checkerboard boundaries have mini-overlaps)
Test: Worst-case merge rule performance; complex bbox overlap calculation

This fixture stress tests the merge algorithm; 32 vector spans + 32 OCR
regions with alternating pattern for worst-case merge complexity."

echo ""
echo "Fixture creation complete!"
echo ""
echo "Created 10 hybrid fixture directories with:"
echo "  - .txt ground truth files"
echo "  - .pdf placeholder files (minimal PDF structure)"
echo "  - README.md with fixture specifications"
echo ""
echo "For production-quality hybrid PDFs with proper vector+scan overlap:"
echo "  1. Install: pip3 install reportlab Pillow img2pdf"
echo "  2. Run: python3 generate_hybrid_fixtures.py"
echo ""
echo "These placeholder fixtures enable:"
echo "  - Immediate testing of classification logic (using .txt files)"
echo "  - Documentation of fixture specifications"
echo "  - Structure for future proper PDF generation"
