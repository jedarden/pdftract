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
from pdftract import Document, Page, Span


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

    # Verify nested type assertions for Page and Span objects
    print(f"\n🔍 Verifying nested type assertions...")

    # Check if doc.pages exists and has at least one page
    assert hasattr(doc, 'pages'), \
        "Document should have 'pages' attribute"
    assert len(doc.pages) > 0, \
        "Document should contain at least one page for testing nested types"

    # Verify the first page is a Page instance
    first_page = doc.pages[0]
    assert isinstance(first_page, Page), \
        f"doc.pages[0] should be a Page instance, got {type(first_page).__name__}"
    print(f"   - doc.pages[0] is Page instance: ✓")

    # Check if the page has spans and at least one span
    assert hasattr(first_page, 'spans'), \
        "Page should have 'spans' attribute"
    assert len(first_page.spans) > 0, \
        "Page should contain at least one span for testing nested types"

    # Verify the first span is a Span instance
    first_span = first_page.spans[0]
    assert isinstance(first_span, Span), \
        f"doc.pages[0].spans[0] should be a Span instance, got {type(first_span).__name__}"
    print(f"   - doc.pages[0].spans[0] is Span instance: ✓")

    # Verify the chain of nested objects is correctly preserved
    assert isinstance(first_span, Span), \
        f"Type chain verification failed: doc.pages[0].spans[0] should be Span instance, got {type(first_span).__name__}"
    assert isinstance(first_page, Page), \
        f"Type chain verification failed: doc.pages[0] should be Page instance, got {type(first_page).__name__}"
    assert isinstance(doc, Document), \
        f"Type chain verification failed: extract() should return Document instance, got {type(doc).__name__}"
    print(f"   - Type chain correctly preserved: ✓")

    print(f"\n✅ All nested type assertions passed!")
    print(f"   - Document → Page → Span type chain verified")
    print(f"   - {len(doc.pages)} page(s), {len(first_page.spans)} span(s) in first page")


if __name__ == "__main__":
    print("=" * 60)
    print("SDK extract() Return Type Smoke Test")
    print("=" * 60)
    test_extract_returns_document_instance()
    print("\n" + "=" * 60)
    print("✅ SMOKE TEST PASSED")
    print("=" * 60)
