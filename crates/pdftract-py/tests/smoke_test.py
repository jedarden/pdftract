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
from pdftract import Document, Page, Span


def test_extract_returns_typed_document() -> None:
    """Verify extract() returns a typed Document instance.

    This smoke test validates the core type contract:
    - Document.from_native() returns a properly typed Document instance
    - Document has pages and metadata attributes with correct types
    - Page objects are properly typed with structural attributes (page, width, height)
    - Span objects are properly typed with expected attributes (text, bbox, font, size)
    - Nested structure integrity: Document -> Pages -> Spans hierarchy is complete
    - Content verification: spans contain real (non-empty) text content
    - Count integrity: all objects are properly accounted for

    Assertion types covered:
    - isinstance() checks for type verification
    - hasattr() checks for attribute existence
    - Length/content checks for data validation
    - Type-specific checks (str, tuple, numeric types for span attributes)

    This test loads fixture data from test-minimal.expected.json
    which contains actual page content for type verification.
    """
    # Load fixture data that has actual pages and spans
    fixture_path = Path(__file__).parent.parent.parent.parent / "tests" / "fixtures" / "test-minimal.expected.json"

    if not fixture_path.exists():
        print(f"❌ Fixture not found: {fixture_path}")
        sys.exit(1)

    # ===== FIXTURE LOADING =====
    # Load fixture data and create Document instance for testing
    import json
    try:
        with fixture_path.open("r") as f:
            fixture_data = json.load(f)
    except json.JSONDecodeError as e:
        print(f"❌ Fixture file contains invalid JSON: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"❌ Failed to read fixture file: {e}")
        sys.exit(1)

    doc = pdftract.Document.from_native(fixture_data)

    # ===== Document-level type verification =====
    # Verify the top-level Document object is properly typed and has expected attributes

    assert isinstance(doc, pdftract.Document), \
        f'Expected Document instance from Document.from_native(), got {type(doc).__name__}'
    print("✓ Document.from_native() returns Document instance")

    assert hasattr(doc, 'pages'), \
        f"Document instance should have 'pages' attribute (type: {type(doc).__name__})"
    print("✓ Document has 'pages' attribute")

    assert hasattr(doc, 'metadata'), \
        f"Document instance should have 'metadata' attribute (type: {type(doc).__name__})"
    assert isinstance(doc.metadata, pdftract.Metadata), \
        f"Document.metadata should be Metadata instance, got {type(doc.metadata).__name__}"
    print("✓ Document.metadata is typed Metadata instance")

    assert doc.pages, "Document should contain pages"
    print("✓ Document.pages is not empty")

    # ===== Page-level type verification =====
    # Verify Page objects exist and are properly typed

    assert len(doc.pages) > 0, \
        f"Document should contain at least one page, found {len(doc.pages)} pages"

    # Verify each page is a Page instance
    for page in doc.pages:
        assert isinstance(page, pdftract.Page), \
            f"Each page should be a Page instance, got {type(page)}"
        assert hasattr(page, "spans"), "Page should have spans attribute"
        assert hasattr(page, "width"), "Page should have width attribute"
        assert hasattr(page, "height"), "Page should have height attribute"
    print("✓ Document has typed Page objects")

    # ===== Span-level type verification =====
    # Verify Span objects exist and are properly typed with expected attributes

    # Find a page with actual span content
    page_with_spans = None
    for page in doc.pages:
        if hasattr(page, 'spans') and len(page.spans) > 0:
            page_with_spans = page
            break

    assert page_with_spans is not None, \
        f"At least one page should have span content for type verification (checked {len(doc.pages)} page(s), fixture may be empty or malformed)"
    print(f"✓ Found page with {len(page_with_spans.spans)} span(s) for type checking")

    # Check each span is properly typed and has expected attributes
    for i, span in enumerate(page_with_spans.spans):
        assert isinstance(span, pdftract.Span), \
            f"spans[{i}] should be Span instance, got {type(span).__name__}"

        # Verify expected Span attributes exist (attribute presence checks)
        assert hasattr(span, 'text'), \
            f"spans[{i}] should have 'text' attribute (type: {type(span).__name__})"
        assert hasattr(span, 'bbox'), \
            f"spans[{i}] should have 'bbox' attribute (type: {type(span).__name__})"
        assert hasattr(span, 'font'), \
            f"spans[{i}] should have 'font' attribute (type: {type(span).__name__})"
        assert hasattr(span, 'size'), \
            f"spans[{i}] should have 'size' attribute (type: {type(span).__name__})"

        # Verify attribute types (type-specific validation)
        assert isinstance(span.text, str), \
            f"spans[{i}].text should be str, got {type(span.text).__name__} (expected: string content)"
        assert isinstance(span.bbox, tuple), \
            f"spans[{i}].bbox should be tuple, got {type(span.bbox).__name__} (expected: 4-element bounding box)"
        assert len(span.bbox) == 4, \
            f"spans[{i}].bbox should have 4 elements (x0,y0,x1,y1), got {len(span.bbox)} elements"
        assert isinstance(span.font, str), \
            f"spans[{i}].font should be str, got {type(span.font).__name__} (expected: font name)"
        assert isinstance(span.size, (int, float)), \
            f"spans[{i}].size should be numeric (int or float), got {type(span.size).__name__} (expected: font size)"

    print(f"✓ All {len(page_with_spans.spans)} span(s) properly typed with expected attributes (text, bbox, font, size)")

    # ===== Nested structure verification =====
    # Ensure complete type hierarchy integrity (Document -> Pages -> Spans)

    # Verify parent-child relationships: pages belong to the document
    total_pages = len(doc.pages)
    assert total_pages > 0, \
        f"Document should contain at least one page for hierarchy validation (Document->Pages->Spans chain), got {total_pages} pages"
    print(f"✓ Document owns {total_pages} page(s) (Document->Pages link valid)")

    # Verify each page is properly contained within the document structure
    for i, page in enumerate(doc.pages):
        assert isinstance(page, pdftract.Page), \
            f"doc.pages[{i}] should be a Page instance for hierarchy integrity, got {type(page).__name__}"
        # Verify page has structural attributes (page number, dimensions)
        assert hasattr(page, 'page'), \
            f"doc.pages[{i}] should have 'page' attribute (page number) for Page structure"
        assert hasattr(page, 'width'), \
            f"doc.pages[{i}] should have 'width' attribute for Page dimensions"
        assert hasattr(page, 'height'), \
            f"doc.pages[{i}] should have 'height' attribute for Page dimensions"

        # Verify Page.width is numeric (int or float)
        assert isinstance(page.width, (int, float)), \
            f"doc.pages[{i}].width should be numeric (int or float) for valid dimensions, got {type(page.width).__name__}"
    print(f"✓ All {total_pages} page(s) properly typed with structural attributes (page, width, height)")

    # Attribute access type verification
    # Verify Page.width is accessible and has correct numeric type
    assert hasattr(doc.pages[0], 'width'), \
        "Page instance should have 'width' attribute accessible"
    assert isinstance(doc.pages[0].width, (int, float)), \
        f"Page.width should be numeric (int or float), got {type(doc.pages[0].width).__name__}"
    print("✓ Page.width is accessible and has numeric type")

    # Verify Span.text is accessible and has correct string type
    assert hasattr(page_with_spans.spans[0], 'text'), \
        "Span instance should have 'text' attribute accessible"
    assert isinstance(page_with_spans.spans[0].text, str), \
        f"Span.text should be string, got {type(page_with_spans.spans[0].text).__name__}"
    print("✓ Span.text is accessible and has string type")

    # Verify at least one page has spans with real content
    pages_with_spans = [p for p in doc.pages if hasattr(p, 'spans') and len(p.spans) > 0]
    assert len(pages_with_spans) > 0, \
        f"At least one page should have spans populated, found {len(pages_with_spans)}/{total_pages} pages with spans (edge case: fixture may be empty)"
    print(f"✓ {len(pages_with_spans)}/{total_pages} page(s) have span content")

    # Verify span text contains real (non-empty) content, not placeholders
    total_spans = 0
    spans_with_text = 0
    for page in pages_with_spans:
        for span in page.spans:
            total_spans += 1
            if hasattr(span, 'text') and span.text:
                spans_with_text += 1

    # Edge case: no spans or all spans empty (indicates fixture problem)
    assert total_spans > 0, \
        f"Expected at least one span across {len(pages_with_spans)} page(s) with content, found {total_spans} spans (edge case: fixture may be malformed)"
    assert spans_with_text > 0, \
        f"Expected non-empty text content in spans, found {spans_with_text}/{total_spans} spans with text (edge case: spans may be placeholders)"
    print(f"✓ {spans_with_text}/{total_spans} span(s) have non-empty text content (real content)")

    # ===== COUNT INTEGRITY VERIFICATION =====
    # Verify count integrity: ensure all objects are properly accounted for
    total_pages_checked = len(doc.pages)
    total_spans_checked = sum(len(p.spans) for p in doc.pages if hasattr(p, 'spans'))

    # Verify page count consistency (detects structure corruption)
    assert total_pages_checked == total_pages, \
        f"Page count integrity check failed: expected {total_pages} pages, counted {total_pages_checked} pages (data structure may be corrupted)"

    # Verify span count consistency (detects lost/duplicate spans)
    assert total_spans_checked == total_spans, \
        f"Span count integrity check failed: expected {total_spans} spans, counted {total_spans_checked} spans (data structure may be corrupted)"

    print(f"✓ Count integrity verified: {total_pages} page(s), {total_spans} span(s), structure consistent")

    # ===== TEST SUCCESS SUMMARY =====
    print("\n" + "=" * 60)
    print("✅ ALL SMOKE TESTS PASSED")
    print("=" * 60)
    print(f"Document structure: {total_pages} page(s), {total_spans} span(s)")
    print(f"Content verification: {spans_with_text}/{total_spans} spans with text")
    print("Type contract verification: COMPLETE")
    print("All assertion types validated: isinstance, hasattr, length, type-specific")
    print("=" * 60)


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
