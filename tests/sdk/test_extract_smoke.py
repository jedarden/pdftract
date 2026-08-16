#!/usr/bin/env python3
"""Basic smoke test for SDK extract() return type.

This test verifies that pdftract.extract() returns a Document instance
(not a dict or plain JSON structure) as specified in the acceptance criteria.
"""

import sys
from pathlib import Path

# Add the python package to the path
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "crates" / "pdftract-py" / "python"))

import pdftract
from pdftract import Document


def test_extract_returns_document_instance():
    """Verify extract() returns a Document instance, not a dict.

    This smoke test validates the basic type contract of the SDK:
    - extract() returns a Document instance
    - The object is not a dict or plain JSON structure
    - The return type is correctly verified
    """
    # Use a test fixture that's known to work with the SDK
    # markdown_structure.pdf is used in the existing SDK tests
    fixture_path = Path(__file__).parent.parent / "fixtures" / "markdown_structure.pdf"

    if not fixture_path.exists():
        raise FileNotFoundError(f"Required fixture not found: {fixture_path}")

    # Run extract() as specified in implementation guidance
    doc = pdftract.extract(str(fixture_path))

    # Verify the object is a Document instance (not a dict)
    assert isinstance(doc, Document), \
        f"extract() should return Document instance, got {type(doc).__name__}"

    # Verify the object is NOT a dict
    assert not isinstance(doc, dict), \
        "extract() should not return a plain dict"

    # Verify the object is NOT a plain JSON structure
    # (Document should be a proper class instance, not dict-like)
    assert hasattr(doc, '__class__'), \
        "extract() should return a class instance, not a plain dict"
    assert doc.__class__.__name__ == 'Document', \
        f"extract() should return Document class, got {doc.__class__.__name__}"

    print(f"✅ Smoke test passed: extract() returns Document instance")
    print(f"   - Type verified: {type(doc).__name__}")
    print(f"   - Not a dict: ✓")
    print(f"   - Not plain JSON: ✓")


if __name__ == "__main__":
    print("=" * 60)
    print("SDK extract() Return Type Smoke Test")
    print("=" * 60)
    test_extract_returns_document_instance()
    print("\n" + "=" * 60)
    print("✅ SMOKE TEST PASSED")
    print("=" * 60)
