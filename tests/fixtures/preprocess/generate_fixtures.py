#!/usr/bin/env python3
"""
Generate preprocessing test fixtures.

This script creates synthetic test images for the preprocessing pipeline:
- skewed_2deg: 2-degree skewed text lines (tests deskew)
- uneven_lighting: gradient background with text (tests Sauvola binarization)
- clean_digital: crisp digital text (tests Otsu binarization)
- jbig2_scan: binary text (tests JBIG2 skip logic)
"""

import math
from PIL import Image, ImageDraw, ImageFont


def create_skewed_2deg():
    """Create a 2-degree skewed image for deskew testing."""
    width, height = 400, 300

    # Create an image with horizontal text lines
    img = Image.new('L', (width, height), color=255)
    draw = ImageDraw.Draw(img)

    # Draw horizontal text lines
    for y in range(50, 250, 20):
        draw.text((50, y), "Lorem ipsum dolor sit amet", fill=0)

    # Rotate by 2 degrees
    img_skewed = img.rotate(2, resample=Image.BICUBIC, expand=False, fillcolor=255)

    img_skewed.save('tests/fixtures/preprocess/skewed_2deg/source.png')
    print("Created skewed_2deg/source.png")


def create_uneven_lighting():
    """Create an image with uneven lighting for Sauvola testing."""
    width, height = 400, 300

    # Create a gradient background (uneven lighting)
    img = Image.new('L', (width, height))
    pixels = img.load()

    for x in range(width):
        for y in range(height):
            # Gradient from darker (left) to lighter (right)
            val = int(150 + (x / width) * 80)
            pixels[x, y] = val

    draw = ImageDraw.Draw(img)

    # Draw text on the uneven background
    for y in range(50, 250, 25):
        draw.text((50, y), "Sample text for testing", fill=0)

    img.save('tests/fixtures/preprocess/uneven_lighting/source.png')
    print("Created uneven_lighting/source.png")


def create_clean_digital():
    """Create a clean digital-origin image for Otsu testing."""
    width, height = 400, 300

    # Create a clean white background
    img = Image.new('L', (width, height), color=255)
    draw = ImageDraw.Draw(img)

    # Draw crisp text (as if from a digital PDF)
    for y in range(50, 250, 25):
        draw.text((50, y), "Digital document text", fill=0)

    img.save('tests/fixtures/preprocess/clean_digital/source.png')
    print("Created clean_digital/source.png")


def create_jbig2_scan():
    """Create a binary image (simulating JBIG2)."""
    width, height = 400, 300

    # Create a pure binary image
    img = Image.new('L', (width, height), color=255)
    draw = ImageDraw.Draw(img)

    # Draw binary text
    for y in range(50, 250, 25):
        draw.text((50, y), "Binary JBIG2 text", fill=0)

    # Ensure it's truly binary (only 0 and 255)
    pixels = img.load()
    for x in range(width):
        for y in range(height):
            val = pixels[x, y]
            if val < 128:
                pixels[x, y] = 0
            else:
                pixels[x, y] = 255

    img.save('tests/fixtures/preprocess/jbig2_scan/source.png')
    print("Created jbig2_scan/source.png")


if __name__ == '__main__':
    print("Generating preprocessing test fixtures...")
    create_skewed_2deg()
    create_uneven_lighting()
    create_clean_digital()
    create_jbig2_scan()
    print("Done!")
