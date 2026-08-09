#!/usr/bin/env python3
"""Span object access test infrastructure - Simple test runner.

This module provides dedicated test infrastructure for accessing Span objects
from Page results. It demonstrates that:

1. Span objects can be accessed from Page objects
2. Single Span access works correctly
3. Multiple Span access (list/array) works correctly
4. Type assertions work properly

This version uses a simple test runner compatible with the existing smoke test
infrastructure, without requiring pytest.

Usage:
    python3 test_span_access_simple.py
"""

from __future__ import annotations

import sys
from pathlib import Path

# Add the python package to the path
sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

import pdftract
from pdftract import Document, Page, Span


class SpanAccessInfrastructure:
    """Test infrastructure for accessing Span objects from Page results.

    This class provides helper methods and patterns for reliably accessing
    Span objects nested within Page structures. It handles both single
    Span access and multiple Spans access patterns.
    """

    @staticmethod
    def access_first_span(page: Page) -> Span:
        """Access the first Span from a Page.

        Args:
            page: A pdftract.Page instance

        Returns:
            The first Span object from the page

        Raises:
            AssertionError: If page has no spans
        """
        assert len(page.spans) > 0, "Page must contain at least one span"

        first_span = page.spans[0]
        assert isinstance(first_span, Span), \
            f'Expected Span type for first span, got {type(first_span).__name__}'

        return first_span

    @staticmethod
    def access_span_by_index(page: Page, index: int) -> Span:
        """Access a Span by its index in the Page.

        Args:
            page: A pdftract.Page instance
            index: Zero-based index of the span to access

        Returns:
            The Span object at the specified index

        Raises:
            AssertionError: If index is out of bounds or span is wrong type
        """
        assert 0 <= index < len(page.spans), \
            f"Span index {index} out of bounds (page has {len(page.spans)} spans)"

        span = page.spans[index]
        assert isinstance(span, Span), \
            f'Expected Span type for span[{index}], got {type(span).__name__}'

        return span

    @staticmethod
    def access_all_spans(page: Page) -> list[Span]:
        """Access all Spans from a Page with type verification.

        Args:
            page: A pdftract.Page instance

        Returns:
            List of Span objects with type verification
        """
        spans: list[Span] = []

        for i, span in enumerate(page.spans):
            assert isinstance(span, Span), \
                f'Expected Span type for span[{i}], got {type(span).__name__}'
            spans.append(span)

        return spans

    @staticmethod
    def access_last_span(page: Page) -> Span:
        """Access the last Span from a Page.

        Args:
            page: A pdftract.Page instance

        Returns:
            The last Span object from the page

        Raises:
            AssertionError: If page has no spans or wrong type
        """
        assert len(page.spans) > 0, "Page must contain at least one span"

        last_span = page.spans[-1]
        assert isinstance(last_span, Span), \
            f'Expected Span type for last span, got {type(last_span).__name__}'

        return last_span

    @staticmethod
    def get_span_count(page: Page) -> int:
        """Get the number of spans in a Page.

        Args:
            page: A pdftract.Page instance

        Returns:
            Number of spans in the page
        """
        return len(page.spans)

    @staticmethod
    def verify_span_structure(span: Span) -> None:
        """Verify that a Span object has the expected structure.

        Args:
            span: A pdftract.Span instance to verify

        Raises:
            AssertionError: If span doesn't have expected attributes
        """
        expected_attrs = ["text", "bbox", "font", "size"]

        for attr in expected_attrs:
            assert hasattr(span, attr), f"Span should have '{attr}' attribute"


def test_create_sample_span():
    """Test creating and accessing a sample Span."""
    print("\n--- Test: Create Sample Span ---")

    # Create a sample span
    span_data = {
        "text": "Hello World",
        "bbox": [100.0, 200.0, 300.0, 400.0],
        "font": "Helvetica",
        "size": 12.0,
        "confidence": 0.95
    }

    span = Span.from_native(span_data)

    # Verify it's a Span instance
    assert isinstance(span, Span)
    print("✓ Created Span instance successfully")

    # Verify attributes
    assert span.text == "Hello World"
    assert span.bbox == (100.0, 200.0, 300.0, 400.0)
    assert span.font == "Helvetica"
    assert span.size == 12.0
    assert span.confidence == 0.95
    print("✓ Span attributes are correct")

    return True


def test_create_page_with_single_span():
    """Test creating a Page with a single Span."""
    print("\n--- Test: Page with Single Span ---")

    # Create a page with one span
    page_data = {
        "page": 1,
        "width": 612,
        "height": 792,
        "rotation": 0,
        "spans": [
            {
                "text": "Single span text",
                "bbox": [100.0, 200.0, 300.0, 400.0],
                "font": "Arial",
                "size": 14.0,
                "confidence": 0.98
            }
        ],
        "blocks": []
    }

    page = Page.from_native(page_data)

    # Verify it's a Page instance
    assert isinstance(page, Page)
    print("✓ Created Page instance successfully")

    # Use infrastructure to access the single span
    infra = SpanAccessInfrastructure()

    # Test accessing first span
    first_span = infra.access_first_span(page)
    assert isinstance(first_span, Span)
    assert first_span.text == "Single span text"
    print("✓ Successfully accessed single Span from Page")

    # Test accessing last span (should be same as first)
    last_span = infra.access_last_span(page)
    assert last_span is first_span
    print("✓ First and last Span are same object (single span case)")

    # Test span count
    count = infra.get_span_count(page)
    assert count == 1
    print("✓ Span count is correct: 1")

    return True


def test_create_page_with_multiple_spans():
    """Test creating a Page with multiple Spans."""
    print("\n--- Test: Page with Multiple Spans ---")

    # Create a page with multiple spans
    page_data = {
        "page": 1,
        "width": 612,
        "height": 792,
        "rotation": 0,
        "spans": [
            {
                "text": "First span",
                "bbox": [100.0, 200.0, 150.0, 210.0],
                "font": "Arial",
                "size": 12.0,
                "confidence": 0.95
            },
            {
                "text": "Second span",
                "bbox": [150.0, 200.0, 200.0, 210.0],
                "font": "Times-Roman",
                "size": 14.0,
                "confidence": 0.96
            },
            {
                "text": "Third span",
                "bbox": [200.0, 200.0, 250.0, 210.0],
                "font": "Courier",
                "size": 10.0,
                "confidence": 0.97
            }
        ],
        "blocks": []
    }

    page = Page.from_native(page_data)

    # Verify it's a Page instance
    assert isinstance(page, Page)
    print("✓ Created Page instance with multiple spans")

    # Use infrastructure to access spans
    infra = SpanAccessInfrastructure()

    # Test accessing first span
    first_span = infra.access_first_span(page)
    assert isinstance(first_span, Span)
    assert first_span.text == "First span"
    print("✓ Successfully accessed first Span")

    # Test accessing last span
    last_span = infra.access_last_span(page)
    assert isinstance(last_span, Span)
    assert last_span.text == "Third span"
    print("✓ Successfully accessed last Span")

    # Test accessing span by index
    middle_span = infra.access_span_by_index(page, 1)
    assert isinstance(middle_span, Span)
    assert middle_span.text == "Second span"
    print("✓ Successfully accessed middle Span by index")

    # Test accessing all spans
    all_spans = infra.access_all_spans(page)
    assert len(all_spans) == 3
    assert all(isinstance(s, Span) for s in all_spans)
    print("✓ Successfully accessed all Spans with type verification")

    # Test span count
    count = infra.get_span_count(page)
    assert count == 3
    print("✓ Span count is correct: 3")

    return True


def test_span_type_assertions():
    """Test that Span objects pass type assertions."""
    print("\n--- Test: Span Type Assertions ---")

    # Create a page with spans
    page_data = {
        "page": 1,
        "width": 612,
        "height": 792,
        "rotation": 0,
        "spans": [
            {"text": "Span 1", "bbox": [100.0, 200.0, 150.0, 210.0], "font": "Arial", "size": 12.0},
            {"text": "Span 2", "bbox": [150.0, 200.0, 200.0, 210.0], "font": "Times", "size": 14.0}
        ],
        "blocks": []
    }

    page = Page.from_native(page_data)
    infra = SpanAccessInfrastructure()

    # Test all spans are properly typed
    all_spans = infra.access_all_spans(page)

    for i, span in enumerate(all_spans):
        assert isinstance(span, Span), \
            f'Expected Span type for span[{i}], got {type(span).__name__}'
        assert not isinstance(span, dict), \
            f"span[{i}] should not be a raw dict"

    print("✓ All Spans are properly typed Span instances")
    print("✓ No Spans are raw dicts")

    return True


def test_span_structure_verification():
    """Test Span object structure verification."""
    print("\n--- Test: Span Structure Verification ---")

    # Create a sample span
    span_data = {
        "text": "Test text",
        "bbox": [50.0, 100.0, 200.0, 150.0],
        "font": "Helvetica-Bold",
        "size": 16.0,
        "confidence": 0.99
    }

    span = Span.from_native(span_data)
    infra = SpanAccessInfrastructure()

    # Verify structure
    infra.verify_span_structure(span)
    print("✓ Span has all expected attributes: text, bbox, font, size")

    return True


def test_empty_page_handling():
    """Test accessing spans from a page with no spans."""
    print("\n--- Test: Empty Page Handling ---")

    # Create a page with no spans
    empty_page_data = {
        "page": 1,
        "width": 612,
        "height": 792,
        "rotation": 0,
        "spans": [],
        "blocks": []
    }

    page = Page.from_native(empty_page_data)
    infra = SpanAccessInfrastructure()

    # Should handle gracefully
    try:
        infra.access_first_span(page)
        print("✗ Should have raised AssertionError for empty page")
        return False
    except AssertionError as e:
        if "at least one span" in str(e):
            print("✓ Empty page handling works correctly")
            return True
        else:
            print(f"✗ Unexpected error: {e}")
            return False


def test_bounds_checking():
    """Test span index bounds checking."""
    print("\n--- Test: Bounds Checking ---")

    # Create a page with spans
    page_data = {
        "page": 1,
        "width": 612,
        "height": 792,
        "rotation": 0,
        "spans": [
            {"text": "Span 1", "bbox": [100.0, 200.0, 150.0, 210.0], "font": "Arial", "size": 12.0}
        ],
        "blocks": []
    }

    page = Page.from_native(page_data)
    infra = SpanAccessInfrastructure()

    # Test negative index
    try:
        infra.access_span_by_index(page, -1)
        print("✗ Should have raised AssertionError for negative index")
        return False
    except AssertionError as e:
        if "out of bounds" in str(e):
            print("✓ Negative index bounds checking works")
        else:
            print(f"✗ Unexpected error for negative index: {e}")
            return False

    # Test index too large
    try:
        infra.access_span_by_index(page, 1)  # Only 1 span at index 0
        print("✗ Should have raised AssertionError for too-large index")
        return False
    except AssertionError as e:
        if "out of bounds" in str(e):
            print("✓ Too-large index bounds checking works")
            return True
        else:
            print(f"✗ Unexpected error for too-large index: {e}")
            return False


def test_integration_with_page_infrastructure():
    """Test Span infrastructure integrated with Page infrastructure."""
    print("\n--- Test: Integration with Page Infrastructure ---")

    # Import Page infrastructure
    from test_page_access_simple import PageAccessInfrastructure

    # Create a document with pages containing spans
    doc_data = {
        "schema_version": "1.0",
        "pages": [
            {
                "page": 1,
                "width": 612,
                "height": 792,
                "rotation": 0,
                "spans": [
                    {"text": "Page 1 Span 1", "bbox": [100.0, 200.0, 150.0, 210.0], "font": "Arial", "size": 12.0},
                    {"text": "Page 1 Span 2", "bbox": [150.0, 200.0, 200.0, 210.0], "font": "Times", "size": 14.0}
                ],
                "blocks": []
            },
            {
                "page": 2,
                "width": 612,
                "height": 792,
                "rotation": 0,
                "spans": [
                    {"text": "Page 2 Span 1", "bbox": [100.0, 300.0, 150.0, 310.0], "font": "Courier", "size": 10.0}
                ],
                "blocks": []
            }
        ],
        "metadata": {"page_count": 2}
    }

    doc = Document.from_native(doc_data)

    # Use Page infrastructure to access pages
    page_infra = PageAccessInfrastructure()
    all_pages = page_infra.access_all_pages(doc)

    # Use Span infrastructure to access spans from each page
    span_infra = SpanAccessInfrastructure()

    total_spans = 0
    for page_idx, page in enumerate(all_pages):
        assert isinstance(page, Page), \
            f'Expected Page type for page[{page_idx}], got {type(page).__name__}'

        span_count = span_infra.get_span_count(page)
        print(f"  Page {page_idx + 1}: {span_count} span(s)")

        if span_count > 0:
            all_spans = span_infra.access_all_spans(page)
            for span_idx, span in enumerate(all_spans):
                assert isinstance(span, Span), \
                    f'Expected Span type for page[{page_idx}].spans[{span_idx}], got {type(span).__name__}'
                total_spans += 1

    assert total_spans == 3  # 2 spans on page 1, 1 span on page 2
    print(f"✓ Successfully accessed {total_spans} total Spans from {len(all_pages)} pages")

    return True


def main():
    """Run all Span access tests."""
    print("=" * 60)
    print("Span Object Access Test Infrastructure - Simple Tests")
    print("=" * 60)

    tests = [
        ("Create Sample Span", test_create_sample_span),
        ("Page with Single Span", test_create_page_with_single_span),
        ("Page with Multiple Spans", test_create_page_with_multiple_spans),
        ("Span Type Assertions", test_span_type_assertions),
        ("Span Structure Verification", test_span_structure_verification),
        ("Empty Page Handling", test_empty_page_handling),
        ("Bounds Checking", test_bounds_checking),
        ("Integration with Page Infrastructure", test_integration_with_page_infrastructure),
    ]

    passed = 0
    failed = 0
    skipped = 0

    for test_name, test_func in tests:
        try:
            result = test_func()
            if result is True:
                passed += 1
            elif result is False:
                failed += 1
            else:
                skipped += 1
        except Exception as e:
            print(f"✗ Test failed with exception: {e}")
            import traceback
            traceback.print_exc()
            failed += 1

    print("\n" + "=" * 60)
    print(f"Test Results: {passed} passed, {failed} failed, {skipped} skipped")
    print("=" * 60)

    if failed > 0:
        print("❌ SOME TESTS FAILED")
        return 1
    else:
        print("✅ ALL TESTS PASSED")
        return 0


if __name__ == "__main__":
    sys.exit(main())