#!/usr/bin/env python3
"""
Test script to demonstrate extract_markdown() bug.

This script tests extract_text() and extract_markdown() on a PDF fixture
and shows whether they produce different outputs (they shouldn't be identical).
"""

import sys
sys.path.insert(0, '/home/coding/pdftract/crates/pdftract-py/.venv/lib/python3.12/site-packages')

import pdftract
import os

def test_extract_functions(fixture_path):
    """Test extract_text() and extract_markdown() on a fixture."""

    print(f"Testing fixture: {fixture_path}")
    print(f"File exists: {os.path.exists(fixture_path)}")
    print()

    # Extract text
    try:
        text_output = pdftract.extract_text(fixture_path)
        print(f"extract_text() returned {len(text_output)} characters")
        print(f"First 200 chars:\n{text_output[:200]}")
        print()
    except Exception as e:
        print(f"extract_text() failed: {e}")
        text_output = None

    # Extract markdown
    try:
        md_output = pdftract.extract_markdown(fixture_path)
        print(f"extract_markdown() returned {len(md_output)} characters")
        print(f"First 200 chars:\n{md_output[:200]}")
        print()
    except Exception as e:
        print(f"extract_markdown() failed: {e}")
        md_output = None

    # Compare outputs
    if text_output is not None and md_output is not None:
        if text_output == md_output:
            print("⚠️  BUG CONFIRMED: extract_text() and extract_markdown() produce IDENTICAL output!")
            print(f"Both outputs are {len(text_output)} characters and exactly the same.")
            return True
        else:
            print("✓ Outputs are different (expected behavior)")
            print(f"text_output length: {len(text_output)}")
            print(f"md_output length: {len(md_output)}")
            return False

    return None

if __name__ == "__main__":
    # Test on sample.pdf
    fixture = "/home/coding/pdftract/tests/fixtures/sample.pdf"
    result = test_extract_functions(fixture)

    if result:
        print("\n" + "="*60)
        print("BUG STATE CONFIRMED")
        print("="*60)
        sys.exit(1)
    elif result is False:
        print("\n" + "="*60)
        print("NO BUG - outputs are different")
        print("="*60)
        sys.exit(0)
    else:
        print("\n" + "="*60)
        print("TEST INCONCLUSIVE")
        print("="*60)
        sys.exit(2)
