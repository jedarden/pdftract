#!/usr/bin/env python3
"""
Create a degraded 200 DPI PDF from the Abraham Lincoln public domain source document.

This script:
1. Creates a clean PDF from the text at 200 DPI
2. Applies degradation effects (noise, blur, compression)
3. Saves the result as degraded-200dpi.pdf

Requirements:
    pip3 install reportlab Pillow img2pdf
"""

import os
import sys
import random
from pathlib import Path

try:
    from reportlab.pdfgen import canvas
    from reportlab.lib.pagesizes import letter
    from reportlab.lib.units import inch
except ImportError:
    print("Error: reportlab is not installed.")
    print("Install with: pip3 install reportlab")
    sys.exit(1)

try:
    from PIL import Image, ImageFilter, ImageEnhance
except ImportError:
    print("Error: Pillow is not installed.")
    print("Install with: pip3 install Pillow")
    sys.exit(1)

def add_noise(image, amount=15):
    """Add random noise to simulate scan artifacts."""
    pixels = image.load()
    width, height = image.size

    for i in range(width):
        for j in range(height):
            # Get pixel values
            pixel = pixels[i, j]
            if len(pixel) == 3:  # RGB
                r, g, b = pixel
                # Add random noise
                noise = random.randint(-amount, amount)
                r = max(0, min(255, r + noise))
                g = max(0, min(255, g + noise))
                b = max(0, min(255, b + noise))
                pixels[i, j] = (r, g, b)
            elif len(pixel) == 4:  # RGBA
                r, g, b, a = pixel
                noise = random.randint(-amount, amount)
                r = max(0, min(255, r + noise))
                g = max(0, min(255, g + noise))
                b = max(0, min(255, b + noise))
                pixels[i, j] = (r, g, b, a)

    return image

def create_degraded_pdf():
    """Create a degraded 200 DPI PDF from the source text."""
    script_dir = Path(__file__).parent

    # Paths
    source_txt = script_dir / "source-document-abraham-lincoln-public-domain.txt"
    output_pdf = script_dir / "degraded-200dpi.pdf"

    print(f"Creating degraded 200 DPI PDF...")
    print(f"Source: {source_txt}")
    print(f"Output: {output_pdf}")

    if not source_txt.exists():
        print(f"Error: Source file not found: {source_txt}")
        sys.exit(1)

    # Read the source text
    with open(source_txt, 'r', encoding='utf-8') as f:
        text = f.read()

    # Take first ~2000 characters for a single-page fixture
    # (Full text would be too long for a single degraded fixture)
    text = text[:2500]

    # Step 1: Create a clean PDF from text at 200 DPI
    print("\nStep 1: Creating clean PDF from text...")

    # Page configuration (letter size, 200 DPI equivalent)
    page_width, page_height = letter

    # Create temporary clean PDF
    temp_pdf = script_dir / "temp_clean.pdf"
    c = canvas.Canvas(str(temp_pdf), pagesize=letter)

    # Font settings for 200 DPI (smaller, slightly degraded look)
    c.setFont("Times-Roman", 10)

    # Margins
    left_margin = 0.75 * inch
    top_margin = 0.75 * inch
    right_margin = 0.75 * inch
    bottom_margin = 0.75 * inch

    y_position = page_height - top_margin
    line_spacing = 12

    # Draw text line by line
    lines = text.split('\n')
    for line in lines:
        if y_position < bottom_margin + line_spacing:
            c.showPage()
            c.setFont("Times-Roman", 10)
            y_position = page_height - top_margin

        c.drawString(left_margin, y_position, line)
        y_position -= line_spacing

    c.save()
    print(f"  Created temporary clean PDF: {temp_pdf}")

    # Step 2: Convert PDF to images at 200 DPI
    print("\nStep 2: Converting PDF to images at 200 DPI...")

    import tempfile
    import subprocess

    with tempfile.TemporaryDirectory() as tmpdir:
        # Convert PDF to PPM images at 200 DPI
        result = subprocess.run(
            ["pdftoppm", "-r", "200", str(temp_pdf),
             os.path.join(tmpdir, "page")],
            capture_output=True,
            text=True
        )

        if result.returncode != 0:
            print(f"Error: pdftoppm failed: {result.stderr}")
            temp_pdf.unlink()
            sys.exit(1)

        # Get the generated images
        images = sorted(Path(tmpdir).glob("page-*.ppm"))

        if not images:
            print("Error: No images generated")
            temp_pdf.unlink()
            sys.exit(1)

        print(f"  Generated {len(images)} image(s)")

        # Step 3: Apply degradation effects
        print("\nStep 3: Applying degradation effects...")

        degraded_images = []
        for i, img_path in enumerate(images):
            print(f"  Processing page {i+1}/{len(images)}...")

            # Load image
            img = Image.open(str(img_path)).convert('RGB')

            # Apply degradation effects:

            # 1. Add mild Gaussian blur (simulating poor focus)
            img = img.filter(ImageFilter.GaussianBlur(radius=0.3))

            # 2. Add random noise (simulating scan noise)
            img = add_noise(img, amount=12)

            # 3. Slightly reduce contrast (simulating poor scan quality)
            enhancer = ImageEnhance.Contrast(img)
            img = enhancer.enhance(0.9)

            # 4. Slightly reduce sharpness
            enhancer = ImageEnhance.Sharpness(img)
            img = enhancer.enhance(0.85)

            degraded_images.append(img)

        # Step 4: Convert degraded images back to PDF
        print("\nStep 4: Creating degraded PDF...")

        # Use PIL directly (more reliable than img2pdf for our use case)
        degraded_images[0].save(
            str(output_pdf),
            save_all=True,
            append_images=degraded_images[1:],
            resolution=200,
            quality=85  # Add compression artifacts
        )

        print(f"  Created: {output_pdf}")

    # Clean up temporary file
    temp_pdf.unlink()

    # Step 5: Verify the output
    print("\nStep 5: Verifying output...")
    file_size = output_pdf.stat().st_size
    print(f"  File size: {file_size} bytes ({file_size / 1024:.1f} KB)")

    if output_pdf.exists():
        print(f"\n✓ Success! Created degraded 200 DPI PDF: {output_pdf}")
        return 0
    else:
        print(f"\n✗ Failed to create output PDF")
        return 1

if __name__ == "__main__":
    sys.exit(create_degraded_pdf())