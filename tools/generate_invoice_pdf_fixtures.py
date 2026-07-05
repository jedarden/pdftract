#!/usr/bin/env python3
"""
Generate invoice OCR test fixtures with proper DPI metadata.

This creates a scanned PDF from an image with correct 300 DPI settings.
The PDF contains a single invoice page with ground truth text for OCR testing.
"""

import sys
import os

def create_invoice_pdf():
    """Create invoice PDF with proper 300 DPI metadata."""
    try:
        from PIL import Image

        # Load the extracted image
        img_path = "/tmp/invoice_img-000.png"
        if not os.path.exists(img_path):
            print(f"Error: {img_path} not found")
            return False

        # Open image and verify dimensions
        img = Image.open(img_path)
        width, height = img.size

        # Expected dimensions for 300 DPI letter-size (8.5" x 11")
        # 8.5 * 300 = 2550, 11 * 300 = 3300
        expected_width = 2550
        expected_height = 3300

        if width != expected_width or height != expected_height:
            print(f"Warning: Image dimensions {width}x{height} differ from expected {expected_width}x{expected_height}")

        # Convert to RGB if needed
        if img.mode != 'RGB':
            img = img.convert('RGB')

        # Save as JPEG for PDF embedding
        jpeg_path = "/tmp/invoice_300dpi.jpg"
        img.save(jpeg_path, format='JPEG', quality=95, dpi=(300, 300))
        print(f"Created JPEG with 300 DPI: {jpeg_path}")

        # Now use img2pdf approach via subprocess if available
        import subprocess
        output_path = "tests/fixtures/scanned/invoice/invoice-300dpi.pdf"
        os.makedirs(os.path.dirname(output_path), exist_ok=True)

        # Try using img2pdf from Python
        try:
            import img2pdf

            # Convert image to PDF with proper DPI
            with open(jpeg_path, 'rb') as img_file:
                pdf_bytes = img2pdf.convert(
                    img_file.read(),
                    dpi=(300, 300),
                    with_img2pdf=False
                )

            with open(output_path, 'wb') as pdf_file:
                pdf_file.write(pdf_bytes)

            print(f"Created: {output_path}")
            print(f"  Image size: {width} x {height} pixels")
            print(f"  DPI: 300 x 300")
            print(f"  Physical size: {width/300:.2f} x {height/300:.2f} inches")

            return True

        except ImportError:
            print("img2pdf not available, using alternative approach...")
            # Use PIL's PDF approach
            # Create a simple one-page PDF using reportlab if available
            try:
                from reportlab.lib.pagesizes import letter
                from reportlab.platypus import Image as RLImage, SimpleDocTemplate
                from reportlab.lib.units import inch

                # Calculate size at 300 DPI
                img_width_inch = width / 300
                img_height_inch = height / 300

                doc = SimpleDocTemplate(
                    output_path,
                    pagesize=(img_width_inch * inch, img_height_inch * inch)
                )

                story = []
                rl_img = RLImage(jpeg_path, width=img_width_inch * inch, height=img_height_inch * inch)
                story.append(rl_img)

                doc.build(story)

                print(f"Created: {output_path}")
                print(f"  Image size: {width} x {height} pixels")
                print(f"  DPI: 300 x 300")
                print(f"  Physical size: {img_width_inch:.2f} x {img_height_inch:.2f} inches")

                return True

            except ImportError:
                print("Neither img2pdf nor reportlab available")
                return False

    except Exception as e:
        print(f"Error creating PDF: {e}")
        import traceback
        traceback.print_exc()
        return False

def verify_pdf(pdf_path):
    """Verify the PDF was created correctly."""
    try:
        import subprocess

        # Check if pdfimages is available
        result = subprocess.run(
            ["pdfimages", "-list", pdf_path],
            capture_output=True,
            text=True
        )

        if result.returncode == 0:
            print("\nPDF verification (pdfimages -list):")
            for line in result.stdout.split('\n'):
                if line.strip():
                    print(f"  {line}")

        # Check pdfinfo if available
        result = subprocess.run(
            ["pdfinfo", pdf_path],
            capture_output=True,
            text=True
        )

        if result.returncode == 0:
            print("\nPDF info (pdfinfo):")
            for line in result.stdout.split('\n'):
                if any(keyword in line.lower() for keyword in ['page size', 'dpi', 'file size', 'pages']):
                    print(f"  {line.strip()}")

        return True

    except Exception as e:
        print(f"Warning: Could not verify PDF - {e}")
        return False

if __name__ == "__main__":
    print("Generating invoice OCR fixture...")

    if create_invoice_pdf():
        output_path = "tests/fixtures/scanned/invoice/invoice-300dpi.pdf"
        verify_pdf(output_path)
        print("\n✓ Invoice fixture created successfully!")
    else:
        print("\n✗ Failed to create invoice fixture")
        sys.exit(1)
