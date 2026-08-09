#!/usr/bin/env python3
"""
Test script to demonstrate the current extract_markdown() behavior.
This creates a simple text file to simulate PDF content and tests both functions.
"""

import sys
sys.path.insert(0, '/home/coding/pdftract/crates/pdftract-py/.venv/lib/python3.12/site-packages')

import pdftract
import os

def test_with_valid_pdf():
    """Test with a valid PDF that should have extractable content."""

    # Try different PDF files until we find one that works
    test_files = [
        'tests/fixtures/remote_100page.pdf',
        'tests/fixtures/tagged-suspects-true.pdf',
        'tests/fixtures/tagged-suspects-true-high-coverage.pdf',
    ]

    for fixture in test_files:
        if not os.path.exists(fixture):
            print(f"File does not exist: {fixture}")
            continue

        print(f"\n{'='*60}")
        print(f"Testing fixture: {fixture}")
        print(f"File exists: True")
        print(f"File size: {os.path.getsize(fixture)} bytes")
        print('='*60)
        print()

        # Extract text
        try:
            text_output = pdftract.extract_text(fixture)
            print(f"✓ extract_text() succeeded")
            print(f"  Length: {len(text_output)} characters")
            if text_output:
                print(f"  First 200 chars:\n{text_output[:200]}")
            else:
                print(f"  (empty output)")
            print()
        except Exception as e:
            print(f"✗ extract_text() failed: {e}")
            print()
            text_output = None

        # Extract markdown
        try:
            md_output = pdftract.extract_markdown(fixture)
            print(f"✓ extract_markdown() succeeded")
            print(f"  Length: {len(md_output)} characters")
            if md_output:
                print(f"  First 200 chars:\n{md_output[:200]}")
            else:
                print(f"  (empty output)")
            print()
        except Exception as e:
            print(f"✗ extract_markdown() failed: {e}")
            print()
            md_output = None

        # Compare outputs
        if text_output is not None and md_output is not None:
            print("-" * 60)
            if text_output == md_output:
                print("⚠️  BUG CONFIRMED")
                print("   extract_text() and extract_markdown() produce IDENTICAL output")
                print(f"   Both outputs: {len(text_output)} characters")
                if text_output:
                    print(f"   Byte-for-byte identical: YES")
                else:
                    print(f"   Both are empty (cannot differentiate behavior)")
            else:
                print("✓ Outputs are different (expected behavior)")
                print(f"   extract_text(): {len(text_output)} chars")
                print(f"   extract_markdown(): {len(md_output)} chars")

                # Show differences
                if len(text_output) > 0 and len(md_output) > 0:
                    print("\n   Sample differences:")
                    print(f"   Text starts: {text_output[:100]!r}")
                    print(f"   Markdown starts: {md_output[:100]!r}")
            print("-" * 60)

            # If we found a working PDF, stop here
            if text_output or md_output:
                return fixture, text_output, md_output

    return None, None, None

if __name__ == "__main__":
    fixture, text_output, md_output = test_with_valid_pdf()

    if fixture is None:
        print("\n" + "="*60)
        print("NO WORKING PDF FOUND")
        print("All test PDFs failed to extract content")
        print("="*60)
        sys.exit(2)
    else:
        print("\n" + "="*60)
        print("TEST COMPLETED")
        print(f"Working fixture: {fixture}")
        print("="*60)

        if text_output == md_output and text_output:
            print("\nBUG STATE: CONFIRMED")
            print("Both functions produce identical output")
            sys.exit(1)
        elif text_output == md_output and not text_output:
            print("\nBUG STATE: INCONCLUSIVE")
            print("Both functions return empty output")
            sys.exit(2)
        else:
            print("\nBUG STATE: NOT CONFIRMED")
            print("Functions produce different output (expected)")
            sys.exit(0)
