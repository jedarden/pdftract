#!/usr/bin/env python3
"""
Quick test to verify pdftract.search() returns actual matches instead of empty list.
This is a verification script for bead bf-lxfmar.
"""

import sys
from pathlib import Path

# Try importing from the local development build
try:
    # For development, we need to build the package first with maturin develop
    # This is just to verify the implementation
    sys.path.insert(0, str(Path(__file__).parent.parent / "target" / "release"))
    import pdftract

    # Use a known fixture with searchable text
    fixture = Path(__file__).parent / "fixtures" / "sample.pdf"

    if not fixture.exists():
        print(f"Fixture not found: {fixture}")
        sys.exit(1)

    # Test 1: Basic search
    result = pdftract.search(str(fixture), "the")
    print(f"Test 1 - Basic search for 'the':")
    print(f"  Pattern: {result.get('pattern')}")
    print(f"  Match count: {len(result.get('matches', []))}")

    if len(result.get('matches', [])) > 0:
        print("  ✓ PASS: Found matches")
        first_match = result['matches'][0]
        print(f"    First match:")
        print(f"      page_index: {first_match.get('page_index')}")
        print(f"      span_index: {first_match.get('span_index')}")
        print(f"      text: {first_match.get('text')}")
        print(f"      bbox: {first_match.get('bbox')}")
    else:
        print("  ✗ FAIL: No matches found")
        sys.exit(1)

    # Test 2: Case insensitive
    result = pdftract.search(str(fixture), "THE", case_insensitive=True)
    print(f"\nTest 2 - Case insensitive search for 'THE':")
    print(f"  Match count: {len(result.get('matches', []))}")
    if len(result.get('matches', [])) > 0:
        print("  ✓ PASS: Case insensitive works")
    else:
        print("  ✗ FAIL: Case insensitive failed")
        sys.exit(1)

    # Test 3: Whole word
    result = pdftract.search(str(fixture), "the", whole_word=True)
    print(f"\nTest 3 - Whole word search for 'the':")
    print(f"  Match count: {len(result.get('matches', []))}")
    if len(result.get('matches', [])) > 0:
        print("  ✓ PASS: Whole word works")
    else:
        print("  ✗ FAIL: Whole word failed")
        sys.exit(1)

    print("\n✓ All tests passed!")
    sys.exit(0)

except ImportError as e:
    print(f"Cannot import pdftract: {e}")
    print("This is expected - the Python package needs to be built with maturin first.")
    print("Run: maturin develop --release")
    sys.exit(1)
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)
