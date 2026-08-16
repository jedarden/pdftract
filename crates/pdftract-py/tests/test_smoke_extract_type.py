"""Basic smoke test for SDK extract() return type.

This test verifies that pdftract.extract() returns a Document instance (not a dict),
ensuring type safety and proper object hierarchy in the Python SDK.
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import pdftract
from pdftract import Document


def test_extract_returns_document_instance():
    """Verify extract() returns a Document instance (not a dict)."""
    # Use available fixture (test-minimal.pdf exists in tests/fixtures/)
    fixture_path = "tests/fixtures/test-minimal.pdf"

    # Extract the document
    doc = pdftract.extract(fixture_path)

    # Verify extract() returns a Document instance, not a dict
    assert isinstance(doc, Document), \
        f"extract() should return Document instance, got {type(doc).__name__}"

    # Verify the object is not a dict or plain JSON structure
    assert not isinstance(doc, dict), \
        "extract() should not return a plain dict"

    # Verify it's a proper Document object with expected attributes
    assert hasattr(doc, 'pages'), "Document should have 'pages' attribute"
    assert hasattr(doc, 'metadata'), "Document should have 'metadata' attribute"

    print("✓ Smoke test passed: extract() returns Document instance")
    return True


if __name__ == "__main__":
    try:
        test_extract_returns_document_instance()
        print("\n✓ All smoke tests passed!")
        sys.exit(0)
    except Exception as e:
        print(f"\n✗ Smoke test failed: {e}")
        sys.exit(1)
