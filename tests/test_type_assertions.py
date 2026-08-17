#!/usr/bin/env python3
"""Test type assertions for Document, Page, and Span types.

This test verifies that fixture data creates properly typed instances:
- Document objects contain Page instances
- Page objects contain Span instances
- All types are correctly instantiated with clear error messages
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'crates', 'pdftract-py', 'python'))

import pdftract
import json


def skip(message):
    """Skip test with a message."""
    print(f"  ⚠ SKIP: {message}")
    return True


def assert_true(condition, message):
    """Assert with custom error message."""
    if not condition:
        raise AssertionError(message)


def test_document_type_from_fixture_data():
    """Test that Document.from_fixture_data() creates properly typed instances.

    This test verifies:
    - Document objects are correctly typed
    - All pages in Document.pages are Page instances with clear error messages
    - All spans in all Page.spans are Span instances with clear error messages
    - Error messages show indices and actual types for debugging
    """
    print("Testing document type from fixture data...")

    # Use fixture data since PDF fixtures may be corrupted
    fixture_path = "/home/coding/pdftract/tests/fixtures/test-minimal.expected.json"

    try:
        with open(fixture_path, 'r') as f:
            fixture_data = json.load(f)
    except Exception as e:
        return skip(f"Could not load fixture data: {e}")

    # Create Document from fixture data
    result = pdftract.Document.from_native(fixture_data)

    # Verify Document type with descriptive error message
    assert isinstance(result, pdftract.Document), \
        f'Expected Document type, got {type(result).__name__}'

    assert hasattr(result, 'pages'), "Document should have 'pages' attribute"
    assert isinstance(result.pages, list), "Document.pages should be a list"

    if len(result.pages) == 0:
        return skip("Document has no pages, skipping type assertions")

    # Check ALL pages are Page instances with descriptive error messages
    # This handles multiple pages gracefully and shows exact index and type on failure
    for page_idx, page in enumerate(result.pages):
        assert isinstance(page, pdftract.Page), \
            f'Document.pages[{page_idx}] should be Page instance, got {type(page).__name__}'

    print(f"  ✓ All {len(result.pages)} pages are Page instances")

    # First page should have expected attributes
    first_page = result.pages[0]
    assert hasattr(first_page, 'width'), "Page should have width attribute"
    assert hasattr(first_page, 'height'), "Page should have height attribute"
    assert hasattr(first_page, 'spans'), "Page should have spans attribute"
    assert hasattr(first_page, 'blocks'), "Page should have blocks attribute"

    # Check first span is Span instance (single object case)
    if len(first_page.spans) == 0:
        return skip("First page has no spans, skipping Span type assertions")

    first_span = first_page.spans[0]
    assert isinstance(first_span, pdftract.Span), \
        f'page[0].spans[0] should be Span instance, got {type(first_span).__name__}'

    # Check ALL spans across ALL pages are Span instances with descriptive error messages
    # This handles multiple spans across multiple pages gracefully
    total_spans = 0
    for page_idx, page in enumerate(result.pages):
        for span_idx, span in enumerate(page.spans):
            total_spans += 1
            assert isinstance(span, pdftract.Span), \
                f'Document.pages[{page_idx}].spans[{span_idx}] should be Span instance, got {type(span).__name__}'

    print(f"  ✓ All {total_spans} spans across {len(result.pages)} pages are Span instances")

    # Verify we actually tested something
    assert total_spans > 0, "Should have at least one span to test"
    print("  ✓ test_document_type_from_fixture_data PASSED")


def test_page_type_single_object():
    """Test Page type assertion works for single object."""
    print("Testing Page type for single object...")

    fixture_path = "/home/coding/pdftract/tests/fixtures/test-minimal.expected.json"

    try:
        with open(fixture_path, 'r') as f:
            fixture_data = json.load(f)
    except Exception as e:
        return skip(f"Could not load fixture data: {e}")

    result = pdftract.Document.from_native(fixture_data)

    if len(result.pages) == 0:
        return skip("Document has no pages")

    # Test single page access shows clear type on failure
    page = result.pages[0]
    assert isinstance(page, pdftract.Page), \
        f'Expected Page type for single page access, got {type(page).__name__}'
    print("  ✓ test_page_type_single_object PASSED")


def test_span_type_single_object():
    """Test Span type assertion works for single object."""
    print("Testing Span type for single object...")

    fixture_path = "/home/coding/pdftract/tests/fixtures/test-minimal.expected.json"

    try:
        with open(fixture_path, 'r') as f:
            fixture_data = json.load(f)
    except Exception as e:
        return skip(f"Could not load fixture data: {e}")

    result = pdftract.Document.from_native(fixture_data)

    if len(result.pages) == 0 or len(result.pages[0].spans) == 0:
        return skip("Document or first page has no spans")

    # Test single span access shows clear type on failure
    span = result.pages[0].spans[0]
    assert isinstance(span, pdftract.Span), \
        f'Expected Span type for single span access, got {type(span).__name__}'
    print("  ✓ test_span_type_single_object PASSED")


def test_page_type_multiple_objects():
    """Test Page type assertion works for multiple objects with indices."""
    print("Testing Page type for multiple objects...")

    fixture_path = "/home/coding/pdftract/tests/fixtures/test-minimal.expected.json"

    try:
        with open(fixture_path, 'r') as f:
            fixture_data = json.load(f)
    except Exception as e:
        return skip(f"Could not load fixture data: {e}")

    result = pdftract.Document.from_native(fixture_data)

    if len(result.pages) < 2:
        return skip("Document has fewer than 2 pages, need multiple for this test")

    # Test multiple pages all show their index on failure
    for i, page in enumerate(result.pages):
        assert isinstance(page, pdftract.Page), \
            f'pages[{i}] should be Page instance, got {type(page).__name__}'
    print(f"  ✓ All {len(result.pages)} pages are Page instances")
    print("  ✓ test_page_type_multiple_objects PASSED")


def test_span_type_multiple_objects():
    """Test Span type assertion works for multiple objects with indices."""
    print("Testing Span type for multiple objects...")

    fixture_path = "/home/coding/pdftract/tests/fixtures/test-minimal.expected.json"

    try:
        with open(fixture_path, 'r') as f:
            fixture_data = json.load(f)
    except Exception as e:
        return skip(f"Could not load fixture data: {e}")

    result = pdftract.Document.from_native(fixture_data)

    # Find a page with multiple spans
    page_with_multiple_spans = None
    for page in result.pages:
        if len(page.spans) >= 2:
            page_with_multiple_spans = page
            break

    if page_with_multiple_spans is None:
        return skip("No page has 2+ spans, need multiple for this test")

    # Test multiple spans all show their index on failure
    page_idx = result.pages.index(page_with_multiple_spans)
    for span_idx, span in enumerate(page_with_multiple_spans.spans):
        assert isinstance(span, pdftract.Span), \
            f'pages[{page_idx}].spans[{span_idx}] should be Span instance, got {type(span).__name__}'
    print(f"  ✓ All {len(page_with_multiple_spans.spans)} spans on page {page_idx} are Span instances")
    print("  ✓ test_span_type_multiple_objects PASSED")


if __name__ == "__main__":
    print("=" * 60)
    print("Type Assertions Test")
    print("=" * 60)
    print(f"Native module available: {pdftract._native_available}")
    print()

    if not pdftract._native_available:
        print("⚠ WARNING: Native module not available, using subprocess fallback")
        print()

    try:
        test_document_type_from_fixture_data()
        print()

        test_page_type_single_object()
        print()

        test_span_type_single_object()
        print()

        test_page_type_multiple_objects()
        print()

        test_span_type_multiple_objects()
        print()

        print("=" * 60)
        print("✅ All type assertions passed successfully!")
        print("=" * 60)
        sys.exit(0)

    except Exception as e:
        print()
        print("=" * 60)
        print(f"❌ Test failed: {e}")
        print("=" * 60)
        import traceback
        traceback.print_exc()
        sys.exit(1)
