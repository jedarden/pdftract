#!/usr/bin/env python3
"""
Generate synthetic test PDFs for the classifier corpus.

Creates 200 PDFs (50 each of invoice, scientific_paper, contract, misc)
with appropriate content characteristics for each document type.
"""

import os
import random
from pathlib import Path
from reportlab.pdfgen import canvas
from reportlab.lib.pagesizes import letter, A4
from reportlab.lib.units import inch
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont

# Ensure output directory exists
OUTPUT_DIR = Path("tests/fixtures/classifier")
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# Document type configurations
DOC_TYPES = {
    "invoice": {
        "count": 50,
        "keywords": ["INVOICE", "BILL TO", "SHIP TO", "TOTAL", "DUE DATE", "PO NUMBER", "QTY", "UNIT PRICE", "AMOUNT", "BALANCE DUE", "PAYMENT TERMS"],
        "fontsizes": [16, 14, 12, 10, 9],
        "structures": ["header", "table", "totals"]
    },
    "scientific_paper": {
        "count": 50,
        "keywords": ["ABSTRACT", "INTRODUCTION", "METHODS", "RESULTS", "DISCUSSION", "CONCLUSION", "REFERENCES", "FIGURE", "TABLE", "ACKNOWLEDGMENTS", "DOI", "arXiv"],
        "fontsizes": [14, 12, 11, 10],
        "structures": ["title", "abstract", "sections", "references"]
    },
    "contract": {
        "count": 50,
        "keywords": ["AGREEMENT", "PARTIES", "TERMS", "CONDITIONS", "SHALL", "WITNESS", "CLAUSE", "LIABILITY", "INDEMNIFICATION", "TERMINATION", "GOVERNING LAW", "SIGNATURE"],
        "fontsizes": [12, 11, 10],
        "structures": ["header", "clauses", "signatures"]
    },
    "misc": {
        "count": 50,
        "subtypes": {
            "receipt": {"keywords": ["RECEIPT", "RECEIVED FROM", "AMOUNT", "DATE", "RECEIPT #"], "count": 8},
            "form": {"keywords": ["FORM", "APPLICATION", "PLEASE COMPLETE", "SECTION", "SIGNATURE"], "count": 8},
            "bank_statement": {"keywords": ["STATEMENT", "ACCOUNT", "BALANCE", "TRANSACTION", "DEPOSIT", "WITHDRAWAL"], "count": 7},
            "slide_deck": {"keywords": ["Slide", "Presentation", "Agenda", "Summary", "Key Points"], "count": 7},
            "legal_filing": {"keywords": ["COURT", "CASE NO", "PLAINTIFF", "DEFENDANT", "FILED", "CLERK"], "count": 7},
            "book_excerpt": {"keywords": ["Chapter", "The", "And", "But", "However"], "count": 6},
            "magazine": {"keywords": ["FEATURE", "ARTICLE", "ISSUE", "EDITORIAL", "SUBSCRIBE"], "count": 7}
        },
        "fontsizes": [12, 11, 10],
        "structures": ["various"]
    }
}


def draw_header(c, doc_type, doc_num):
    """Draw a header section."""
    c.setFont("Helvetica-Bold", 16)

    if doc_type == "invoice":
        c.drawString(1*inch, 10*inch, "INVOICE")
        c.setFont("Helvetica", 10)
        c.drawString(6*inch, 10*inch, f"Invoice #{doc_num:04d}")
        c.drawString(6*inch, 9.7*inch, f"Date: 2024-{random.randint(1,12):02d}-{random.randint(1,28):02d}")

    elif doc_type == "scientific_paper":
        title = random.choice([
            "A Novel Approach to Machine Learning",
            "Analysis of Distributed Systems",
            "Theoretical Frameworks in Quantum Computing",
            "Empirical Studies in Natural Language Processing",
            "Optimization Algorithms for Large-Scale Data"
        ])
        c.drawCentredString(4.25*inch, 10*inch, title)
        c.setFont("Helvetica", 10)
        c.drawCentredString(4.25*inch, 9.6*inch, f"arXiv:{random.randint(1000,9999)}.{random.randint(10000,99999)}")

    elif doc_type == "contract":
        c.drawString(1*inch, 10*inch, "SERVICE AGREEMENT")
        c.setFont("Helvetica", 10)
        c.drawString(1*inch, 9.6*inch, f"Contract ID: CT-{doc_num:04d}")

    elif doc_type == "misc":
        # Handled by subtypes
        pass


def draw_invoice_content(c, doc_num):
    """Draw invoice-specific content."""
    y = 8.5*inch
    c.setFont("Helvetica-Bold", 12)
    c.drawString(1*inch, y, "BILL TO:")
    c.setFont("Helvetica", 10)
    c.drawString(1*inch, y-0.25*inch, "Acme Corporation")
    c.drawString(1*inch, y-0.5*inch, "123 Business Street")
    c.drawString(1*inch, y-0.75*inch, "City, State 12345")

    c.setFont("Helvetica-Bold", 12)
    c.drawString(5*inch, y, "SHIP TO:")
    c.setFont("Helvetica", 10)
    c.drawString(5*inch, y-0.25*inch, "Global Tech Inc")
    c.drawString(5*inch, y-0.5*inch, "456 Enterprise Ave")
    c.drawString(5*inch, y-0.75*inch, "Metroville, CA 90210")

    # Table header
    y = 6.5*inch
    c.setFont("Helvetica-Bold", 10)
    c.drawString(1*inch, y, "DESCRIPTION")
    c.drawString(3.5*inch, y, "QTY")
    c.drawString(4.5*inch, y, "UNIT PRICE")
    c.drawString(6*inch, y, "AMOUNT")

    c.line(1*inch, y-0.1*inch, 7.5*inch, y-0.1*inch)

    # Table rows
    c.setFont("Helvetica", 9)
    items = [
        ("Professional Services", random.randint(10,100), random.randint(100,500)),
        ("Software License", random.randint(1,5), random.randint(500,5000)),
        ("Technical Support", random.randint(5,50), random.randint(75,200)),
        ("Consulting Hours", random.randint(20,80), random.randint(150,400))
    ]

    y = 5.7*inch
    total = 0
    for desc, qty, price in items:
        amount = qty * price
        total += amount
        c.drawString(1*inch, y, desc)
        c.drawString(3.5*inch, y, str(qty))
        c.drawString(4.5*inch, y, f"${price:.2f}")
        c.drawString(6*inch, y, f"${amount:.2f}")
        y -= 0.35*inch

    # Totals
    y -= 0.3*inch
    c.line(1*inch, y, 7.5*inch, y)
    y -= 0.4*inch
    c.setFont("Helvetica-Bold", 10)
    c.drawString(5.5*inch, y, "SUBTOTAL:")
    c.drawString(7*inch, y, f"${total:.2f}")
    y -= 0.3*inch
    tax = total * 0.08
    c.drawString(5.5*inch, y, "TAX (8%):")
    c.drawString(7*inch, y, f"${tax:.2f}")
    y -= 0.3*inch
    c.setFont("Helvetica-Bold", 11)
    c.drawString(5.5*inch, y, "TOTAL DUE:")
    c.drawString(7*inch, y, f"${total + tax:.2f}")


def draw_scientific_paper_content(c, doc_num):
    """Draw scientific paper content."""
    y = 9*inch

    # Abstract
    c.setFont("Helvetica-Bold", 11)
    c.drawString(1*inch, y, "ABSTRACT")
    y -= 0.3*inch
    c.setFont("Helvetica", 9)
    abstract_text = (
        "This paper presents a comprehensive analysis of novel methodologies "
        "in the field. We demonstrate significant improvements over existing "
        "approaches through extensive experimentation. Our results show that "
        "the proposed method achieves state-of-the-art performance on standard "
        "benchmarks."
    )
    draw_wrapped_text(c, abstract_text, 1*inch, y, 6.5*inch)
    y = 7*inch

    # Sections
    sections = [
        ("1. INTRODUCTION", "Introduction provides background and motivation."),
        ("2. RELATED WORK", "Related work covers prior research in this area."),
        ("3. METHODOLOGY", "Our approach combines several techniques."),
        ("4. EXPERIMENTS", "We evaluate on standard datasets."),
        ("5. RESULTS", "Results demonstrate effectiveness of our method."),
        ("6. DISCUSSION", "We analyze the implications of our findings."),
        ("7. CONCLUSION", "Future work includes extending to other domains.")
    ]

    for title, desc in sections:
        c.setFont("Helvetica-Bold", 10)
        c.drawString(1*inch, y, title)
        y -= 0.25*inch
        c.setFont("Helvetica", 9)
        y = draw_wrapped_text(c, desc, 1*inch, y, 6.5*inch)
        y -= 0.2*inch

        if y < 1.5*inch:
            c.showPage()
            y = 10*inch

    # References placeholder
    y -= 0.2*inch
    c.setFont("Helvetica-Bold", 10)
    c.drawString(1*inch, y, "REFERENCES")
    y -= 0.25*inch
    c.setFont("Helvetica", 8)
    refs = [
        "[1] Author, A. (2024). Title of the paper. Journal Name, 15(3), 123-145.",
        "[2] Smith, J. & Doe, J. (2023). Another relevant paper. Proceedings of CVPR.",
        "[3] Brown, K. et al. (2024). Recent advances. IEEE Transactions on Pattern Analysis."
    ]
    for ref in refs:
        y = draw_wrapped_text(c, ref, 1*inch, y, 6.5*inch)
        y -= 0.15*inch


def draw_contract_content(c, doc_num):
    """Draw contract content."""
    y = 8.5*inch

    # Preamble
    c.setFont("Helvetica-Bold", 10)
    c.drawString(1*inch, y, "PARTIES")
    y -= 0.25*inch
    c.setFont("Helvetica", 9)
    c.drawString(1*inch, y, "This Service Agreement (\"Agreement\") is entered into as of the date last written below,")
    y -= 0.2*inch
    y = draw_wrapped_text(c, "by and between Provider Co. (\"Provider\") and Client Inc. (\"Client\").", 1*inch, y, 6.5*inch)

    y -= 0.3*inch

    # Clauses
    clauses = [
        ("1. SERVICES", "Provider shall perform the services described in Exhibit A."),
        ("2. TERM", "This Agreement shall commence on Start Date and continue for Term months."),
        ("3. COMPENSATION", "Client shall pay Provider the fees set forth in Exhibit B."),
        ("4. CONFIDENTIALITY", "Each party shall maintain the confidentiality of proprietary information."),
        ("5. LIABILITY", "Provider's liability shall be limited to Fees paid under this Agreement."),
        ("6. TERMINATION", "Either party may terminate with 30 days written notice."),
        ("7. GOVERNING LAW", "This Agreement shall be governed by the laws of State X."),
        ("8. ENTIRE AGREEMENT", "This Agreement constitutes the entire understanding between the parties.")
    ]

    for title, text in clauses:
        if y < 2*inch:
            c.showPage()
            y = 10*inch

        c.setFont("Helvetica-Bold", 10)
        c.drawString(1*inch, y, title)
        y -= 0.25*inch
        c.setFont("Helvetica", 9)
        y = draw_wrapped_text(c, text, 1*inch, y, 6.5*inch)
        y -= 0.25*inch

    # Signatures
    y -= 0.3*inch
    c.line(1*inch, y, 3.5*inch, y)
    c.drawString(1*inch, y-0.15*inch, "Provider:")
    c.line(5*inch, y, 7.5*inch, y)
    c.drawString(5*inch, y-0.15*inch, "Client:")


def draw_misc_receipt(c, doc_num):
    """Draw receipt content."""
    c.setFont("Helvetica-Bold", 14)
    c.drawCentredString(4.25*inch, 9.5*inch, "RECEIPT")

    c.setFont("Helvetica", 10)
    c.drawCentredString(4.25*inch, 9*inch, f"Receipt #{doc_num:06d}")
    c.drawCentredString(4.25*inch, 8.7*inch, f"Date: 2024-{random.randint(1,12):02d}-{random.randint(1,28):02d}")

    y = 8*inch
    c.drawString(1*inch, y, "RECEIVED FROM:")
    c.drawString(2.5*inch, y, "John Smith")

    y -= 0.4*inch
    c.drawString(1*inch, y, "AMOUNT:")
    amount = random.randint(50, 5000)
    c.drawString(2.5*inch, y, f"${amount}.00")

    y -= 0.4*inch
    c.drawString(1*inch, y, "FOR:")
    y = draw_wrapped_text(c, "Payment for services rendered - Professional consulting - Project deliverables", 2.5*inch, y, 4.5*inch)

    y -= 0.4*inch
    c.drawString(1*inch, y, "PAYMENT METHOD:")
    c.drawString(2.5*inch, y, random.choice(["Cash", "Credit Card", "Check", "Bank Transfer"]))

    y = 2*inch
    c.line(1*inch, y, 4*inch, y)
    c.drawString(1*inch, y-0.2*inch, "AUTHORIZED SIGNATURE")


def draw_misc_form(c, doc_num):
    """Draw form content."""
    c.setFont("Helvetica-Bold", 14)
    c.drawCentredString(4.25*inch, 10*inch, "APPLICATION FORM")

    y = 9*inch
    c.setFont("Helvetica", 10)

    fields = [
        ("Full Name:", "________________________________"),
        ("Address:", "________________________________"),
        ("City/State:", "_____________________________"),
        ("Phone:", "_______________________________"),
        ("Email:", "_______________________________"),
        ("Date of Birth:", "________________________")
    ]

    for label, line in fields:
        c.drawString(1*inch, y, label)
        c.drawString(2.5*inch, y, line)
        y -= 0.35*inch

    y -= 0.2*inch
    c.drawString(1*inch, y, "Please complete all fields. Sign below:")

    y = 2*inch
    c.drawString(1*inch, y, "Signature: _______________________ Date: _________________")


def draw_misc_bank_statement(c, doc_num):
    """Draw bank statement content."""
    c.setFont("Helvetica-Bold", 12)
    c.drawString(1*inch, 10*inch, "STATEMENT OF ACCOUNT")

    c.setFont("Helvetica", 10)
    c.drawString(6*inch, 10*inch, f"Period: 01/01/2024 - 01/31/2024")
    c.drawString(1*inch, 9.6*inch, "Account: ****1234")
    c.drawString(6*inch, 9.6*inch, "Statement Date: 02/01/2024")

    # Table header
    y = 8.8*inch
    c.setFont("Helvetica-Bold", 9)
    c.drawString(1*inch, y, "DATE")
    c.drawString(2*inch, y, "DESCRIPTION")
    c.drawString(5*inch, y, "WITHDRAWAL")
    c.drawString(6.2*inch, y, "DEPOSIT")
    c.drawString(7.2*inch, y, "BALANCE")
    c.line(1*inch, y-0.1*inch, 7.8*inch, y-0.1*inch)

    # Transactions
    y = 8.3*inch
    balance = 5000
    transactions = [
        ("01/05", "Opening Balance", "", "", "5,000.00"),
        ("01/08", "Direct Deposit - Payroll", "", "3,500.00", "8,500.00"),
        ("01/10", "ACH Payment - Electric Co", "150.00", "", "8,350.00"),
        ("01/15", "POS Transaction - Grocery", "85.50", "", "8,264.50"),
        ("01/20", "ATM Withdrawal", "200.00", "", "8,064.50"),
        ("01/25", "Direct Deposit - Payroll", "", "3,500.00", "11,564.50"),
        ("01/28", "Online Payment - Credit Card", "500.00", "", "11,064.50")
    ]

    c.setFont("Helvetica", 8)
    for date, desc, wd, dep, bal in transactions:
        c.drawString(1*inch, y, date)
        c.drawString(2*inch, y, desc)
        c.drawString(5*inch, y, wd)
        c.drawString(6.2*inch, y, dep)
        c.drawString(7.2*inch, y, bal)
        y -= 0.22*inch


def draw_misc_slide_deck(c, doc_num):
    """Draw slide deck content."""
    c.setFont("Helvetica-Bold", 16)
    c.drawCentredString(4.25*inch, 9*inch, "Quarterly Business Review")

    c.setFont("Helvetica", 11)
    c.drawCentredString(4.25*inch, 8.5*inch, "Q4 2024 Performance Analysis")

    y = 7*inch
    c.setFont("Helvetica-Bold", 12)
    c.drawString(1*inch, y, "AGENDA")

    y -= 0.4*inch
    c.setFont("Helvetica", 11)
    agenda_items = [
        "1. Executive Summary",
        "2. Key Performance Indicators",
        "3. Revenue Analysis",
        "4. Market Position",
        "5. Strategic Initiatives",
        "6. Q&A"
    ]
    for item in agenda_items:
        c.drawString(1.5*inch, y, item)
        y -= 0.3*inch

    y = 4*inch
    c.setFont("Helvetica-Bold", 12)
    c.drawString(1*inch, y, "KEY POINTS")
    y -= 0.3*inch
    c.setFont("Helvetica", 10)
    points = [
        "Revenue increased 15% YoY",
        "Customer satisfaction at 94%",
        "New product launch successful",
        "Expanded to 3 new markets"
    ]
    for point in points:
        c.drawString(1*inch, y, f"• {point}")
        y -= 0.25*inch


def draw_misc_legal_filing(c, doc_num):
    """Draw legal filing content."""
    c.setFont("Helvetica-Bold", 10)
    c.drawCentredString(4.25*inch, 10*inch, "SUPERIOR COURT OF CALIFORNIA")
    c.drawCentredString(4.25*inch, 9.7*inch, "COUNTY OF LOS ANGELES")

    y = 9*inch
    c.setFont("Helvetica", 10)
    c.drawString(1*inch, y, f"CASE NO: {random.randint(100000,999999)}-{random.randint(10,99)}")
    c.drawString(5*inch, y, f"FILED: {random.randint(1,12)}/01/2024")

    y -= 0.4*inch
    c.setFont("Helvetica-Bold", 11)
    c.drawString(1*inch, y, "PLAINTIFF:")
    c.setFont("Helvetica", 10)
    c.drawString(2*inch, y, "ABC Corporation, a California corporation")

    y -= 0.3*inch
    c.setFont("Helvetica-Bold", 11)
    c.drawString(1*inch, y, "DEFENDANT:")
    c.setFont("Helvetica", 10)
    c.drawString(2*inch, y, "XYZ LLC, a California limited liability company")

    y -= 0.5*inch
    c.setFont("Helvetica-Bold", 11)
    c.drawString(1*inch, y, "COMPLAINT FOR BREACH OF CONTRACT")

    y -= 0.4*inch
    c.setFont("Helvetica", 9)
    c.drawString(1*inch, y, "COMES NOW the Plaintiff, ABC Corporation, by and through counsel, and complains of Defendant")
    y -= 0.2*inch
    y = draw_wrapped_text(c, "XYZ LLC as follows:", 1*inch, y, 6.5*inch)

    y -= 0.3*inch
    c.setFont("Helvetica-Bold", 10)
    c.drawString(1*inch, y, "COUNT I: BREACH OF CONTRACT")
    y -= 0.25*inch
    c.setFont("Helvetica", 9)
    y = draw_wrapped_text(c, "1. Plaintiff is a corporation organized under California law.", 1*inch, y, 6.5*inch)
    y -= 0.2*inch
    y = draw_wrapped_text(c, "2. Defendant is a limited liability company organized under California law.", 1*inch, y, 6.5*inch)
    y -= 0.2*inch
    y = draw_wrapped_text(c, "3. On or about June 1, 2023, the parties entered into a written contract.", 1*inch, y, 6.5*inch)

    y = 1.5*inch
    c.drawString(1*inch, y, "Dated: January 15, 2024")
    c.drawString(5*inch, y, "CLERK OF THE COURT")


def draw_misc_book_excerpt(c, doc_num):
    """Draw book excerpt content."""
    c.setFont("Helvetica-Bold", 12)
    c.drawCentredString(4.25*inch, 10*inch, "Chapter 3")

    c.setFont("Helvetica", 11)
    c.drawCentredString(4.25*inch, 9.6*inch, "The Journey Begins")

    y = 8.5*inch
    c.setFont("Helvetica", 10)

    text = (
        "The morning sun cast long shadows across the cobblestone streets as "
        "Elara made her way toward the ancient library. For three generations, "
        "her family had guarded the secrets contained within its walls, and now "
        "it was her turn to shoulder the responsibility."
    )
    y = draw_wrapped_text(c, text, 1*inch, y, 6.5*inch)
    y -= 0.3*inch

    text = (
        "She paused at the heavy oak door, her hand hovering over the iron handle. "
        "Inside, she knew, lay the answers she had been seeking since her father's "
        "disappearance. The old books held more than stories—they held the key to "
        "understanding the prophecy that had shaped her entire life."
    )
    y = draw_wrapped_text(c, text, 1*inch, y, 6.5*inch)
    y -= 0.3*inch

    text = (
        "Taking a deep breath, Elara pushed the door open. The familiar scent of "
        "parchment and dust filled her senses. In the silence of the empty hall, "
        "she could almost hear the whispers of scholars who had walked these "
        "aisles centuries before."
    )
    y = draw_wrapped_text(c, text, 1*inch, y, 6.5*inch)


def draw_misc_magazine(c, doc_num):
    """Draw magazine content."""
    c.setFont("Helvetica-Bold", 14)
    c.drawCentredString(4.25*inch, 10*inch, "TECH TODAY MAGAZINE")

    c.setFont("Helvetica", 10)
    c.drawCentredString(4.25*inch, 9.6*inch, "March 2024 Edition | Vol. 47 No. 3")

    y = 8.5*inch
    c.setFont("Helvetica-Bold", 12)
    c.drawString(1*inch, y, "FEATURE STORY")

    y -= 0.3*inch
    c.setFont("Helvetica-Bold", 11)
    c.drawString(1*inch, y, "The Future of Artificial Intelligence")

    y -= 0.3*inch
    c.setFont("Helvetica", 9)
    article = (
        "In this exclusive interview, leading researchers discuss the next frontier "
        "of machine learning. From natural language processing to computer vision, "
        "AI is transforming every industry. We explore the ethical implications and "
        "the path forward."
    )
    y = draw_wrapped_text(c, article, 1*inch, y, 6.5*inch)

    y -= 0.4*inch
    c.setFont("Helvetica-Bold", 10)
    c.drawString(1*inch, y, "IN THIS ISSUE")

    y -= 0.3*inch
    c.setFont("Helvetica", 9)
    articles = [
        "• Cloud Computing Trends for 2024",
        "• Cybersecurity Best Practices",
        "• The Rise of Edge Computing",
        "• Developer Tools Roundup",
        "• Startup Spotlight"
    ]
    for a in articles:
        c.drawString(1*inch, y, a)
        y -= 0.2*inch

    y -= 0.2*inch
    c.drawString(1*inch, y, "SUBSCRIBE at techtoday.example.com")


def draw_wrapped_text(c, text, x, y, max_width):
    """Draw text wrapped to max_width, return new y position."""
    words = text.split()
    lines = []
    current_line = []

    for word in words:
        test_line = ' '.join(current_line + [word])
        if c.stringWidth(test_line, 'Helvetica', 9) <= max_width:
            current_line.append(word)
        else:
            if current_line:
                lines.append(' '.join(current_line))
            current_line = [word]

    if current_line:
        lines.append(' '.join(current_line))

    for line in lines:
        c.drawString(x, y, line)
        y -= 0.18*inch

    return y


def generate_pdf(doc_type, doc_num, subtype=None):
    """Generate a single PDF document."""
    filename = OUTPUT_DIR / doc_type / f"{doc_num:02d}.pdf"

    c = canvas.Canvas(str(filename), pagesize=letter)

    if doc_type == "invoice":
        draw_header(c, doc_type, doc_num)
        draw_invoice_content(c, doc_num)

    elif doc_type == "scientific_paper":
        draw_header(c, doc_type, doc_num)
        draw_scientific_paper_content(c, doc_num)

    elif doc_type == "contract":
        draw_header(c, doc_type, doc_num)
        draw_contract_content(c, doc_num)

    elif doc_type == "misc":
        if subtype == "receipt":
            draw_misc_receipt(c, doc_num)
        elif subtype == "form":
            draw_misc_form(c, doc_num)
        elif subtype == "bank_statement":
            draw_misc_bank_statement(c, doc_num)
        elif subtype == "slide_deck":
            draw_misc_slide_deck(c, doc_num)
        elif subtype == "legal_filing":
            draw_misc_legal_filing(c, doc_num)
        elif subtype == "book_excerpt":
            draw_misc_book_excerpt(c, doc_num)
        elif subtype == "magazine":
            draw_misc_magazine(c, doc_num)

    c.save()
    print(f"Generated: {filename}")


def generate_manifest():
    """Generate MANIFEST.tsv file."""
    manifest_path = OUTPUT_DIR / "MANIFEST.tsv"

    sources = {
        "invoice": "Synthetic test data generated by scripts/generate_test_corpus.py",
        "scientific_paper": "Synthetic test data generated by scripts/generate_test_corpus.py",
        "contract": "Synthetic test data generated by scripts/generate_test_corpus.py",
        "misc": "Synthetic test data generated by scripts/generate_test_corpus.py"
    }

    misc_subtypes = {
        "receipt": "1-08",
        "form": "9-16",
        "bank_statement": "17-23",
        "slide_deck": "24-30",
        "legal_filing": "31-37",
        "book_excerpt": "38-43",
        "magazine": "44-50"
    }

    with open(manifest_path, 'w') as f:
        f.write("path\texpected_document_type\tsource_url\tlicense\n")

        for doc_type in ["invoice", "scientific_paper", "contract"]:
            for i in range(1, 51):
                f.write(f"{doc_type}/{i:02d}.pdf\t{doc_type}\t{sources[doc_type]}\tMIT-0\n")

        for i in range(1, 51):
            for subtype, range_str in misc_subtypes.items():
                start, end = map(int, range_str.split('-'))
                if start <= i <= end:
                    f.write(f"misc/{i:02d}.pdf\t{subtype}\t{sources['misc']}\tMIT-0\n")
                    break

    print(f"Generated: {manifest_path}")


def main():
    """Generate all test PDFs."""
    print("Generating 200-document classifier corpus...")

    # Create subdirectories
    for doc_type in ["invoice", "scientific_paper", "contract", "misc"]:
        (OUTPUT_DIR / doc_type).mkdir(exist_ok=True)

    # Generate invoices
    print("\nGenerating 50 invoices...")
    for i in range(1, 51):
        generate_pdf("invoice", i)

    # Generate scientific papers
    print("\nGenerating 50 scientific papers...")
    for i in range(1, 51):
        generate_pdf("scientific_paper", i)

    # Generate contracts
    print("\nGenerating 50 contracts...")
    for i in range(1, 51):
        generate_pdf("contract", i)

    # Generate misc documents
    print("\nGenerating 50 misc documents...")
    misc_ranges = {
        "receipt": (1, 8),
        "form": (9, 16),
        "bank_statement": (17, 23),
        "slide_deck": (24, 30),
        "legal_filing": (31, 37),
        "book_excerpt": (38, 43),
        "magazine": (44, 50)
    }

    for subtype, (start, end) in misc_ranges.items():
        for i in range(start, end + 1):
            generate_pdf("misc", i, subtype=subtype)

    # Generate manifest
    print("\nGenerating MANIFEST.tsv...")
    generate_manifest()

    print("\n✓ Corpus generation complete!")
    print(f"  - 200 PDFs in {OUTPUT_DIR}")
    print(f"  - MANIFEST.tsv with expected classifications")
    print(f"  - Total size: {sum(f.stat().st_size for f in OUTPUT_DIR.rglob('*.pdf')) / 1024 / 1024:.1f} MB")


if __name__ == "__main__":
    main()
