#!/usr/bin/env python3
"""
Generate scanned PDF fixtures from ground truth text files.

This script creates proper 300 DPI PDFs from ground truth text files for OCR testing.
Usage: python3 generate_scanned_fixtures.py

Requirements:
    pip3 install reportlab Pillow img2pdf
"""

import os
import sys
from pathlib import Path

# Check for required dependencies
try:
    from reportlab.pdfgen import canvas
    from reportlab.lib.pagesizes import letter, A4
    from reportlab.lib.units import inch
    from reportlab.pdfbase import pdfmetrics
    from reportlab.pdfbase.ttfonts import TTFont
except ImportError:
    print("Error: reportlab is not installed.")
    print("Install with: pip3 install reportlab")
    sys.exit(1)

try:
    from PIL import Image
except ImportError:
    print("Warning: Pillow not installed, rasterization step will be skipped.")
    print("Install with: pip3 install Pillow")

# Fixture configuration
FIXTURES = [
    {
        "name": "receipt-300dpi",
        "dir": "receipt",
        "font": "Helvetica",
        "font_size": 10,
        "page_size": letter,
        "margins": {"left": 0.5 * inch, "top": 0.5 * inch, "right": 0.5 * inch, "bottom": 0.5 * inch},
        "line_spacing": 14,
    },
    {
        "name": "invoice-300dpi",
        "dir": "documents",
        "font": "Helvetica",
        "font_size": 11,
        "page_size": letter,
        "margins": {"left": 0.75 * inch, "top": 0.75 * inch, "right": 0.75 * inch, "bottom": 0.75 * inch},
        "line_spacing": 16,
    },
    {
        "name": "form-300dpi",
        "dir": "documents",
        "font": "Helvetica",
        "font_size": 11,
        "page_size": letter,
        "margins": {"left": 0.75 * inch, "top": 0.75 * inch, "right": 0.75 * inch, "bottom": 0.75 * inch},
        "line_spacing": 18,
    },
    {
        "name": "doc-10page-300dpi",
        "dir": "multi-page",
        "font": "Times-Roman",
        "font_size": 12,
        "page_size": letter,
        "margins": {"left": 1.0 * inch, "top": 0.75 * inch, "right": 1.0 * inch, "bottom": 0.75 * inch},
        "line_spacing": 18,
        "multi_page": True,
        "page_marker": "Page 1:",
    }
]


def create_pdf_from_text(source_text_path, output_pdf_path, config):
    """Create a PDF from text using reportlab."""
    # Read the ground truth text
    with open(source_text_path, 'r', encoding='utf-8') as f:
        text = f.read()

    # Create PDF canvas (convert Path to string for reportlab)
    page_width, page_height = config["page_size"]
    c = canvas.Canvas(str(output_pdf_path), pagesize=config["page_size"])

    # Set font
    c.setFont(config["font"], config["font_size"])

    # Calculate drawing area
    left_margin = config["margins"]["left"]
    top_margin = config["margins"]["top"]
    right_margin = config["margins"]["right"]
    bottom_margin = config["margins"]["bottom"]

    max_width = page_width - left_margin - right_margin
    y_position = page_height - top_margin

    # Process text line by line
    lines = text.split('\n')

    if config.get("multi_page") and config.get("page_marker"):
        # Multi-page document with explicit page markers
        current_page = 1
        for line in lines:
            # Check for page marker
            if line.startswith(config["page_marker"].replace("1", str(current_page))):
                if current_page > 1:
                    c.showPage()
                    c.setFont(config["font"], config["font_size"])
                    y_position = page_height - top_margin
                current_page += 1
                # Draw the page header
                c.drawString(left_margin, y_position, line)
                y_position -= config["line_spacing"]
                continue

            # Check if we need a new page
            if y_position < bottom_margin + config["line_spacing"]:
                c.showPage()
                c.setFont(config["font"], config["font_size"])
                y_position = page_height - top_margin

            # Draw the line
            c.drawString(left_margin, y_position, line)
            y_position -= config["line_spacing"]
    else:
        # Single page or simple multi-page
        for line in lines:
            # Check if we need a new page
            if y_position < bottom_margin + config["line_spacing"]:
                c.showPage()
                c.setFont(config["font"], config["font_size"])
                y_position = page_height - top_margin

            # Draw the line
            c.drawString(left_margin, y_position, line)
            y_position -= config["line_spacing"]

    c.save()
    print(f"  Created: {output_pdf_path}")


def rasterize_pdf_to_scanned(pdf_path, scanned_pdf_path, dpi=300):
    """Rasterize a PDF back to PDF at specified DPI (simulating a scan)."""
    try:
        from PIL import Image
        import tempfile
        import subprocess

        # Use pdftoppm to convert PDF to images at specified DPI
        with tempfile.TemporaryDirectory() as tmpdir:
            # Convert PDF to PPM images
            result = subprocess.run(
                ["pdftoppm", "-r", str(dpi), str(pdf_path), os.path.join(tmpdir, "page")],
                capture_output=True,
                text=True
            )

            if result.returncode != 0:
                print(f"  Warning: pdftoppm failed, copying original PDF")
                import shutil
                shutil.copy(str(pdf_path), str(scanned_pdf_path))
                return

            # Convert images back to PDF
            images = sorted(Path(tmpdir).glob("page-*.ppm"))

            if not images:
                print(f"  Warning: No images generated, copying original PDF")
                import shutil
                shutil.copy(str(pdf_path), str(scanned_pdf_path))
                return

            # Convert images to PDF using img2pdf or PIL
            try:
                import img2pdf
                with open(str(scanned_pdf_path), "wb") as f:
                    f.write(img2pdf.convert([str(img) for img in images]))
                print(f"  Created scanned: {scanned_pdf_path}")
            except ImportError:
                # Fallback to PIL
                pdf_images = []
                for img_path in images:
                    img = Image.open(str(img_path))
                    pdf_images.append(img.convert('RGB'))

                if pdf_images:
                    pdf_images[0].save(
                        str(scanned_pdf_path),
                        save_all=True,
                        append_images=pdf_images[1:],
                        resolution=dpi
                    )
                    print(f"  Created scanned: {scanned_pdf_path}")

    except Exception as e:
        print(f"  Warning: Rasterization failed ({e}), using original PDF")
        import shutil
        shutil.copy(str(pdf_path), str(scanned_pdf_path))


def generate_all_fixtures():
    """Generate all fixture PDFs."""
    script_dir = Path(__file__).parent

    for fixture in FIXTURES:
        name = fixture["name"]
        fixture_dir = script_dir / fixture["dir"]
        txt_path = fixture_dir / f"{name}.txt"
        pdf_path = fixture_dir / f"{name}.pdf"

        print(f"Generating {name}...")

        if not txt_path.exists():
            print(f"  Error: {txt_path} not found")
            continue

        try:
            # Create the PDF from text
            create_pdf_from_text(txt_path, pdf_path, fixture)

            # Optionally rasterize to simulate a scan
            # This step requires pdftoppm (poppler-utils)
            scanned_path = fixture_dir / f"{name}-scanned.pdf"
            rasterize_pdf_to_scanned(pdf_path, scanned_path, dpi=300)

            print(f"  Success: {name}")
        except Exception as e:
            print(f"  Error generating {name}: {e}")
            import traceback
            traceback.print_exc()


def main():
    """Main entry point."""
    print("Generating scanned fixture PDFs...")
    print("=" * 60)

    if len(sys.argv) > 1:
        # Generate specific fixture
        fixture_name = sys.argv[1]
        for fixture in FIXTURES:
            if fixture["name"] == fixture_name:
                script_dir = Path(__file__).parent
                fixture_dir = script_dir / fixture["dir"]
                txt_path = fixture_dir / f"{fixture_name}.txt"
                pdf_path = fixture_dir / f"{fixture_name}.pdf"

                if txt_path.exists():
                    print(f"Generating {fixture_name}...")
                    create_pdf_from_text(txt_path, pdf_path, fixture)
                    print(f"  Created: {pdf_path}")
                else:
                    print(f"  Error: {txt_path} not found")
                break
        else:
            print(f"Unknown fixture: {fixture_name}")
            print(f"Available fixtures: {', '.join(f['name'] for f in FIXTURES)}")
    else:
        # Generate all fixtures
        generate_all_fixtures()

    print("=" * 60)
    print("Done!")


if __name__ == "__main__":
    main()
