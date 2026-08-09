#!/usr/bin/env python3
"""
Test script to demonstrate the current behavior of extract_text() vs extract_markdown().
This shows that both functions currently return identical output (the bug).
"""

import sys
from pathlib import Path

# Add the Python package to the path
sys.path.insert(0, str(Path(__file__).parent.parent / "crates" / "pdftract-py" / "python" / "pdftract"))

try:
    import pdftract
    print("✓ pdftract Python module loaded successfully")
except ImportError as e:
    print(f"✗ Failed to import pdftract: {e}")
    sys.exit(1)

# Path to the test fixture
fixture_pdf = Path(__file__).parent.parent / "tests" / "markdown" / "markdown-structures.pdf"

if not fixture_pdf.exists():
    print(f"✗ Test fixture not found: {fixture_pdf}")
    print("  Run: python3 tests/fixtures/markdown_test_fixture.py")
    sys.exit(1)

print(f"\nTesting fixture: {fixture_pdf}")
print("=" * 70)

# Test extract_text()
print("\n1. Testing pdftract.extract_text()...")
try:
    text_output = pdftract.extract_text(str(fixture_pdf))
    print(f"✓ extract_text() returned {len(text_output)} characters")
    # Save to file for comparison
    text_output_path = Path(__file__).parent / "bf-2jwxel-extract-text-output.txt"
    text_output_path.write_text(text_output)
    print(f"  Saved to: {text_output_path}")
except Exception as e:
    print(f"✗ extract_text() failed: {e}")
    text_output = None

# Test extract_markdown()
print("\n2. Testing pdftract.extract_markdown()...")
try:
    markdown_output = pdftract.extract_markdown(str(fixture_pdf))
    print(f"✓ extract_markdown() returned {len(markdown_output)} characters")
    # Save to file for comparison
    markdown_output_path = Path(__file__).parent / "bf-2jwxel-extract-markdown-output.txt"
    markdown_output_path.write_text(markdown_output)
    print(f"  Saved to: {markdown_output_path}")
except Exception as e:
    print(f"✗ extract_markdown() failed: {e}")
    markdown_output = None

# Compare the outputs
if text_output is not None and markdown_output is not None:
    print("\n3. Comparing outputs...")
    if text_output == markdown_output:
        print("⚠️  BUG CONFIRMED: extract_text() and extract_markdown() returned IDENTICAL output!")
        print(f"   Both are {len(text_output)} bytes, byte-for-byte identical.")
    else:
        print("✓ Outputs differ (bug is fixed)")
        # Show first difference
        for i, (c1, c2) in enumerate(zip(text_output, markdown_output)):
            if c1 != c2:
                print(f"   First difference at byte {i}:")
                print(f"   extract_text(): {repr(text_output[max(0,i-20):i+20])}")
                print(f"   extract_markdown(): {repr(markdown_output[max(0,i-20):i+20])}")
                break

# Load expected outputs for comparison
expected_text = (Path(__file__).parent.parent / "tests" / "markdown" / "markdown-structures-expect-text.txt").read_text()
expected_markdown = (Path(__file__).parent.parent / "tests" / "markdown" / "markdown-structures-expect-markdown.txt").read_text()

print("\n4. Comparing with expected outputs...")
if text_output is not None:
    if text_output == expected_text:
        print("✓ extract_text() matches expected plain text output")
    else:
        print("⚠️  extract_text() does NOT match expected plain text output")
        print(f"   Expected {len(expected_text)} chars, got {len(text_output)} chars")

if markdown_output is not None:
    if markdown_output == expected_markdown:
        print("✓ extract_markdown() matches expected Markdown output")
    else:
        print("⚠️  extract_markdown() does NOT match expected Markdown output")
        print(f"   Expected {len(expected_markdown)} chars, got {len(markdown_output)} chars")

print("\n" + "=" * 70)
print("Test complete. Output files saved in tools/ directory")
