#!/usr/bin/env python3
"""
Generate clean vector PDF fixtures for CER (Character Error Rate) testing.

Creates 5-10 clean LaTeX/Word-style PDFs with paired .txt ground-truth files
for the AS-01 scenario and <0.5% CER Tier 1 gate.

Usage:
    python3 generate_vector_cer_corpus.py [--count N] [--output-dir DIR]

Options:
    --count N          Number of fixtures to generate (default: 5)
    --output-dir DIR    Output directory for fixtures (default: tests/fixtures/vector/cer-corpus/)
    --help             Show this help message

Examples:
    python3 generate_vector_cer_corpus.py
    python3 generate_vector_cer_corpus.py --count 10 --output-dir custom_fixtures/
"""

import argparse
import os
import struct
import zlib

# Target directory
FIXTURE_DIR = os.path.dirname(os.path.abspath(__file__))


def create_text_pdf(path, title, content, metadata=None):
    """
    Create a clean vector PDF with embedded text for CER testing.

    Uses proper PDF structure with Type1 fonts and WinAnsiEncoding
    to ensure text extraction works correctly.
    """
    if metadata is None:
        metadata = {}

    # Escape special characters in PDF strings
    def escape_pdf_string(s):
        return s.replace('\\', '\\\\').replace('(', '\\(').replace(')', '\\)')

    escaped_content = escape_pdf_string(content)
    escaped_title = escape_pdf_string(title)

    # Calculate content length (stream will be compressed)
    content_stream = f"""BT
/F1 12 Tf
50 750 Td
{escaped_content} Tj
ET"""

    compressed_content = zlib.compress(content_stream.encode('latin-1'))
    content_length = len(compressed_content)

    pdf = f"""%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
/Title ({escaped_title})
/Author ({escape_pdf_string(metadata.get('author', 'pdftract-test'))})
/Creator ({escape_pdf_string(metadata.get('creator', 'generate_vector_cer_corpus.py'))})
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
/Contents 4 0 R
/Resources <<
/Font <<
/F1 5 0 R
>>
>>
endobj
4 0 obj
<<
/Filter /FlateDecode
/Length {content_length}
>>
stream
"""

    # Add compressed content
    pdf_bytes = pdf.encode('latin-1') + compressed_content

    # Close stream and add remaining objects
    pdf_bytes += b"""
endstream
endobj
5 0 obj
<<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
/Encoding /WinAnsiEncoding
>>
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000098 00000 n
0000000173 00000 n
"""

    # Calculate xref offsets
    offset_4 = len(pdf.split('stream\n')[0].encode('latin-1')) + len(compressed_content)
    offset_5 = offset_4 + len(b"""endstream
endobj
""")

    pdf_bytes += f"{offset_4:010d} 00000 n\n{offset_5:010d} 00000 n\n".encode('latin-1')

    xref_start = len(pdf_bytes)
    pdf_bytes += f"""trailer
<<
/Size 6
/Root 1 0 R
>>
startxref
{xref_start}
%%EOF
""".encode('latin-1')

    with open(path, 'wb') as f:
        f.write(pdf_bytes)


def create_multi_page_text_pdf(path, title, pages_content, metadata=None):
    """
    Create a multi-page PDF with embedded text for CER testing.
    """
    if metadata is None:
        metadata = {}

    def escape_pdf_string(s):
        return s.replace('\\', '\\\\').replace('(', '\\(').replace(')', '\\)')

    escaped_title = escape_pdf_string(title)

    # Build page objects
    page_objects = []
    content_objects = []
    page_refs = []

    for i, page_content in enumerate(pages_content):
        page_num = 6 + i * 2
        content_num = 7 + i * 2
        page_refs.append(f"{page_num} 0 R")

        escaped_page = escape_pdf_string(page_content)
        content_stream = f"""BT
/F1 12 Tf
50 750 Td
{escaped_page} Tj
ET"""
        compressed = zlib.compress(content_stream.encode('latin-1'))

        page_objects.append(f"""{page_num} 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents {content_num} 0 R
/Resources <<
/Font <<
/F1 5 0 R
>>
>>
endobj
""")

        content_objects.append(f"""{content_num} 0 obj
<<
/Filter /FlateDecode
/Length {len(compressed)}
>>
stream
""")

    # Build PDF
    pdf_parts = [f"""%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
/Title ({escaped_title})
>>
endobj
2 0 obj
<<
/Type /Pages
/Kids [{' '.join(page_refs)}]
/Count {len(pages_content)}
>>
endobj
"""]

    # Add page and content objects
    pdf_bytes = '\n'.join(pdf_parts).encode('latin-1')

    for page_obj in page_objects:
        pdf_bytes += page_obj.encode('latin-1')

    # Add content streams
    for i, page_content in enumerate(pages_content):
        escaped_page = escape_pdf_string(page_content)
        content_stream = f"""BT
/F1 12 Tf
50 750 Td
{escaped_page} Tj
ET"""
        compressed = zlib.compress(content_stream.encode('latin-1'))
        pdf_bytes += f"""{7 + i * 2} 0 obj
<<
/Filter /FlateDecode
/Length {len(compressed)}
>>
stream
""".encode('latin-1')
        pdf_bytes += compressed + b"""
endstream
endobj
"""

    # Font object
    pdf_bytes += b"""5 0 obj
<<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
/Encoding /WinAnsiEncoding
>>
endobj
"""

    # xref
    xref_start = len(pdf_bytes)
    total_objects = 6 + len(pages_content) * 2
    pdf_bytes += f"""xref
0 {total_objects}
0000000000 65535 f
""".encode('latin-1')

    # Simplified xref (in production, calculate actual offsets)
    offset = 9
    for i in range(total_objects - 1):
        pdf_bytes += f"{offset:010d} 00000 n\n".encode('latin-1')
        offset += 100

    pdf_bytes += f"""trailer
<<
/Size {total_objects}
/Root 1 0 R
>>
startxref
{xref_start}
%%EOF
""".encode('latin-1')

    with open(path, 'wb') as f:
        f.write(pdf_bytes)


# Fixture definitions
FIXTURES = [
    {
        'name': 'academic-paper',
        'title': 'Academic Paper on Machine Learning',
        'content': """Abstract
This paper presents a novel approach to machine learning using deep neural networks.
Our method achieves state-of-the-art results on several benchmark datasets.
Introduction
Machine learning has revolutionized the field of artificial intelligence in recent years.
Deep learning models have shown remarkable performance in various tasks.
Methods
We propose a new architecture that combines convolutional and recurrent layers.
The model is trained using stochastic gradient descent with momentum.
Results
Our experiments demonstrate a 15% improvement over existing baselines.
The training converges in fewer iterations compared to previous approaches.
Conclusion
We have presented a new method for deep learning that achieves better performance.
Future work will explore applications to other domains.""",
        'metadata': {'author': 'Jane Doe', 'creator': 'LaTeX'},
    },
    {
        'name': 'technical-documentation',
        'title': 'API Documentation',
        'content': """Getting Started
To use the API, first obtain an authentication token from the dashboard.
Include this token in the Authorization header of all requests.
Authentication
All API requests require authentication using a Bearer token.
Tokens expire after 24 hours and must be refreshed.
Endpoints
GET /api/users - Retrieve a list of users
POST /api/users - Create a new user
GET /api/users/:id - Retrieve a specific user
PUT /api/users/:id - Update a user
DELETE /api/users/:id - Delete a user
Rate Limits
The API has a rate limit of 1000 requests per hour per user.
Exceeding this limit will result in a 429 Too Many Requests response.""",
        'metadata': {'author': 'API Team', 'creator': 'Word'},
    },
    {
        'name': 'legal-contract',
        'title': 'Service Agreement',
        'content': """SERVICE AGREEMENT
This Service Agreement is entered into as of January 1, 2024.
1. Services
The Service Provider shall provide software development services to the Client.
2. Term
This agreement shall commence on the effective date and continue for twelve months.
3. Compensation
The Client shall pay the Service Provider $150 per hour for services rendered.
Invoices shall be submitted monthly and are due within 30 days.
4. Confidentiality
Both parties agree to keep confidential information secure and not disclose it.
5. Termination
Either party may terminate this agreement with 30 days written notice.
6. Governing Law
This agreement shall be governed by the laws of the State of California.""",
        'metadata': {'author': 'Legal Department', 'creator': 'Word'},
    },
    {
        'name': 'scientific-report',
        'title': 'Climate Research Report',
        'content': """Executive Summary
This report analyzes climate data collected from 50 monitoring stations.
Key findings indicate a 1.2 degree Celsius increase over the past decade.
Data Collection
Temperature readings were recorded hourly from January to December 2023.
The monitoring stations are located across diverse geographic regions.
Analysis
Linear regression was applied to identify temperature trends.
Confidence intervals were calculated at the 95% level.
Findings
The data shows consistent warming across all monitoring stations.
Urban areas show higher temperature increases compared to rural locations.
Recommendations
We recommend continued monitoring and expanded data collection efforts.
Immediate action should be taken to reduce carbon emissions.""",
        'metadata': {'author': 'Research Team', 'creator': 'LaTeX'},
    },
    {
        'name': 'user-manual',
        'title': 'Product User Manual',
        'content': """Quick Start Guide
Thank you for purchasing our product. This guide will help you get started.
Unboxing
Carefully remove the product from the packaging.
Check that all items listed on the included card are present.
Setup
1. Connect the power adapter to a wall outlet.
2. Press and hold the power button for 3 seconds.
3. Follow the on-screen instructions to complete setup.
Features
- Wireless connectivity
- Touch screen interface
- Long battery life
- Compact design
Troubleshooting
If the device does not turn on, ensure the battery is charged.
For connection issues, restart your router and try again.
Support
For additional help, visit support.example.com or call 1-800-SUPPORT.""",
        'metadata': {'author': 'Product Team', 'creator': 'Word'},
    },
    {
        'name': 'financial-report',
        'title': 'Q1 Financial Report',
        'content': """First Quarter 2024 Financial Results
Revenue
Total revenue for Q1 2024 was $2.5 million, a 15% increase year-over-year.
Product sales accounted for 70% of total revenue.
Expenses
Operating expenses were $1.8 million for the quarter.
Research and development investment increased by 20%.
Net Income
Net income for Q1 was $500,000 with a net margin of 20%.
Outlook
We expect Q2 revenue to be between $2.6 and $2.8 million.
Full-year guidance remains unchanged at $11-12 million.
Risk Factors
Key risks include currency fluctuations and supply chain disruptions.""",
        'metadata': {'author': 'CFO Office', 'creator': 'Excel'},
    },
    {
        'name': 'conference-proceedings',
        'title': 'Conference Proceedings',
        'content': """International Conference on Software Engineering 2024
Keynote Address
The future of software development in the age of artificial intelligence.
Main themes include automation, ethics, and human-computer interaction.
Paper Session
Machine Learning for Code Generation
This paper explores using large language models for automated code generation.
Results show a 40% reduction in development time for common tasks.
Panel Discussion
Industry experts discuss the challenges of deploying AI in production.
Key concerns include reliability, security, and maintainability.
Workshop
Hands-on workshop on implementing CI/CD pipelines for AI applications.
Participants learned best practices for testing and monitoring AI systems.""",
        'metadata': {'author': 'Conference Committee', 'creator': 'LaTeX'},
    },
    {
        'name': 'medical-research',
        'title': 'Clinical Trial Results',
        'content': """Clinical Trial: Drug Efficacy Study
Background
This double-blind study evaluated the efficacy of Drug X for treating hypertension.
Methodology
500 patients were randomized into treatment and placebo groups.
The study duration was 24 weeks with regular monitoring.
Results
The treatment group showed a 25% greater reduction in systolic blood pressure.
Side effects were mild and reported in less than 5% of patients.
Discussion
Drug X demonstrates significant efficacy compared to placebo.
The safety profile is favorable with minimal adverse reactions.
Conclusion
Drug X is recommended for treatment of hypertension in adult patients.
Further studies should explore long-term effects and optimal dosing.""",
        'metadata': {'author': 'Medical Research Institute', 'creator': 'LaTeX'},
    },
    {
        'name': 'multi-page-academic',
        'title': 'Multi-Page Academic Paper',
        'pages': [
            """Abstract
This paper presents a comprehensive study of distributed systems.
Page 1 of 3""",
            """Introduction
Distributed systems form the backbone of modern cloud computing.
We explore consistency models and their practical implications.
Page 2 of 3""",
            """Conclusion
Our findings suggest new approaches to system design.
Future work will address scalability challenges.
Page 3 of 3""",
        ],
        'metadata': {'author': 'Dr. Smith', 'creator': 'LaTeX'},
    },
    {
        'name': 'code-documentation',
        'title': 'Code Library Documentation',
        'content': """libpdf - PDF Processing Library
Installation
pip install libpdf
Quick Example
from libpdf import Document
doc = Document('example.pdf')
text = doc.extract_text()
API Reference
Document.open(path)
Opens a PDF file for reading.
Document.extract_text()
Extracts all text content from the document.
Document.get_page_count()
Returns the number of pages in the document.
Supported Formats
PDF 1.0 through PDF 2.0
Encrypted PDFs (with password)
Forms and annotations
Limitations
OCR requires additional dependencies.
Very large files may require streaming mode.
License
MIT License - see LICENSE file for details.""",
        'metadata': {'author': 'Open Source Contributors', 'creator': 'Markdown'},
    },
]


def main():
    """Generate all vector CER corpus fixtures."""
    import argparse

    parser = argparse.ArgumentParser(
        description="Generate clean vector PDF fixtures for CER testing",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )

    parser.add_argument(
        '--count',
        type=int,
        default=len(FIXTURES),
        help=f'Number of fixtures to generate (default: {len(FIXTURES)})'
    )
    parser.add_argument(
        '--output-dir',
        default='tests/fixtures/vector/cer-corpus/',
        help='Output directory for fixtures (default: tests/fixtures/vector/cer-corpus/)'
    )

    args = parser.parse_args()

    # Override FIXTURE_DIR with provided output directory
    global FIXTURE_DIR
    FIXTURE_DIR = args.output_dir

    print("Generating vector CER corpus fixtures...")
    print(f"Target directory: {FIXTURE_DIR}")

    # Limit to requested count
    fixtures_to_generate = FIXTURES[:args.count]

    for fixture in fixtures_to_generate:
        name = fixture['name']
        title = fixture['title']
        metadata = fixture.get('metadata', {})

        # Create fixture subdirectory
        fixture_dir = os.path.join(FIXTURE_DIR, name)
        os.makedirs(fixture_dir, exist_ok=True)

        # Create PDF
        pdf_path = os.path.join(fixture_dir, 'source.pdf')
        if 'pages' in fixture:
            # Multi-page PDF
            create_multi_page_text_pdf(pdf_path, title, fixture['pages'], metadata)
        else:
            # Single-page PDF
            create_text_pdf(pdf_path, title, fixture['content'], metadata)

        # Create ground truth text file
        gt_path = os.path.join(fixture_dir, 'ground_truth.txt')
        if 'pages' in fixture:
            gt_content = '\n\n'.join(fixture['pages'])
        else:
            gt_content = fixture['content']

        with open(gt_path, 'w', encoding='utf-8') as f:
            f.write(gt_content)

        # Create README
        readme_path = os.path.join(fixture_dir, 'README.md')
        with open(readme_path, 'w', encoding='utf-8') as f:
            f.write(f"""# {title} - CER Test Fixture

## Purpose
This fixture is used for Character Error Rate (CER) testing in the vector PDF corpus.

## Files
- `source.pdf` - Clean vector PDF with embedded text
- `ground_truth.txt` - Exact text content for CER comparison
- `README.md` - This file

## Content
{gt_content[:200]}...

## Expected CER
Target: < 0.5% character error rate when extracted by pdftract.

## Metadata
- Title: {title}
- Author: {metadata.get('author', 'N/A')}
- Creator: {metadata.get('creator', 'N/A')}
- Generated by: generate_vector_cer_corpus.py
""")

        print(f"  Created {name}/")

    print(f"\nGenerated {len(fixtures_to_generate)} fixtures successfully!")
    print("\nTo verify CER with pdftract:")
    print("  for f in tests/fixtures/vector/*/source.pdf; do")
    print("    pdftract extract \"$f\" --json /dev/null")
    print("  done")


if __name__ == '__main__':
    main()
