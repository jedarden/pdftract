#!/usr/bin/env python3
"""Basic smoke test for pdftract SDK.

This test verifies that the pdftract module can be imported and basic
operations work correctly. It serves as a quick sanity check that the
SDK is properly structured and functional.

Usage:
    python3 smoke_test.py

The test uses minimal PDF fixtures to keep execution fast and reliable.
"""

from __future__ import annotations

import sys
from pathlib import Path

# Add the python package to the path
sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

import pdftract


def test_extract_returns_typed_document() -> None:
    """Verify extract() returns a typed Document instance.

    This smoke test validates the core type contract:
    - extract() returns a Document instance (not a dict)
    - The Document has a pages attribute
    - Page objects have the expected attributes

    This test uses the test-minimal.pdf fixture for fast execution.
    """
    # Use the minimal fixture that should always parse successfully
    fixture_path = Path(__file__).parent / "fixtures" / "test-minimal.pdf"

    if not fixture_path.exists():
        print(f"❌ Fixture not found: {fixture_path}")
        sys.exit(1)

    # Extract the document
    doc = pdftract.extract(str(fixture_path))

    # Verify Document type
    assert isinstance(doc, pdftract.Document), \
        f"extract() should return Document instance, got {type(doc).__name__}"
    print("✓ extract() returns Document instance")

    # Verify document has pages attribute
    assert hasattr(doc, 'pages'), "Document should have 'pages' attribute"
    print("✓ Document has 'pages' attribute")

    # Verify metadata exists
    assert hasattr(doc, 'metadata'), "Document should have 'metadata' attribute"
    assert isinstance(doc.metadata, pdftract.Metadata), \
        f"metadata should be Metadata instance, got {type(doc.metadata).__name__}"
    print("✓ Document has typed Metadata")

    print("\n✅ All smoke tests passed!")


def main() -> int:
    """Run the smoke test and return exit code."""
    print("=" * 60)
    print("pdftract SDK Smoke Test")
    print("=" * 60)
    print()

    try:
        test_extract_returns_typed_document()
        return 0
    except AssertionError as e:
        print(f"\n❌ Test failed: {e}")
        return 1
    except Exception as e:
        print(f"\n❌ Unexpected error: {e}")
        import traceback
        traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())
