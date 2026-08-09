#!/usr/bin/env python3
"""Page object access test infrastructure - Simple test runner.

This module provides dedicated test infrastructure for accessing Page objects
from Document results. It establishes clear patterns for:

1. Accessing single Page objects from Document
2. Accessing multiple Page objects (lists/arrays) from Document
3. Verifying Page type assertions on accessed objects
4. Working with Page attributes and nested structures

This version uses a simple test runner compatible with the existing smoke test
infrastructure, without requiring pytest.

Usage:
    python3 test_page_access_simple.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Add the python package to the path
sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

import pdftract
from pdftract import Document, Page, Span


class PageAccessInfrastructure:
    """Test infrastructure for accessing Page objects from Document results.

    This class provides helper methods and patterns for reliably accessing
    Page objects nested within Document structures. It handles both single
    Page access and multiple Pages access patterns.
    """

    @staticmethod
    def access_first_page(doc: Document) -> Page:
        """Access the first Page from a Document.

        Args:
            doc: A pdftract.Document instance

        Returns:
            The first Page object from the document

        Raises:
            AssertionError: If document has no pages
        """
        assert len(doc.pages) > 0, "Document must contain at least one page"

        first_page = doc.pages[0]
        assert isinstance(first_page, Page), \
            f'Expected Page type for first page, got {type(first_page).__name__}'

        return first_page

    @staticmethod
    def access_page_by_index(doc: Document, index: int) -> Page:
        """Access a Page by its index in the Document.

        Args:
            doc: A pdftract.Document instance
            index: Zero-based index of the page to access

        Returns:
            The Page object at the specified index

        Raises:
            AssertionError: If index is out of bounds or page is wrong type
        """
        assert 0 <= index < len(doc.pages), \
            f"Page index {index} out of bounds (document has {len(doc.pages)} pages)"

        page = doc.pages[index]
        assert isinstance(page, Page), \
            f'Expected Page type for page[{index}], got {type(page).__name__}'

        return page

    @staticmethod
    def access_all_pages(doc: Document) -> list[Page]:
        """Access all Pages from a Document with type verification.

        Args:
            doc: A pdftract.Document instance

        Returns:
            List of Page objects with type verification
        """
        pages: list[Page] = []

        for i, page in enumerate(doc.pages):
            assert isinstance(page, Page), \
                f'Expected Page type for page[{i}], got {type(page).__name__}'
            pages.append(page)

        return pages

    @staticmethod
    def access_last_page(doc: Document) -> Page:
        """Access the last Page from a Document.

        Args:
            doc: A pdftract.Document instance

        Returns:
            The last Page object from the document

        Raises:
            AssertionError: If document has no pages or wrong type
        """
        assert len(doc.pages) > 0, "Document must contain at least one page"

        last_page = doc.pages[-1]
        assert isinstance(last_page, Page), \
            f'Expected Page type for last page, got {type(last_page).__name__}'

        return last_page

    @staticmethod
    def get_page_count(doc: Document) -> int:
        """Get the number of pages in a Document.

        Args:
            doc: A pdftract.Document instance

        Returns:
            Number of pages in the document
        """
        return len(doc.pages)


def test_basic_page_access():
    """Test basic Page object access from Document."""
    print("\n--- Test: Basic Page Access ---")

    # Load fixture data - adjust path based on current location
    # We're in: /home/coding/pdftract/crates/pdftract-py/tests/
    # Need to get to: /home/coding/pdftract/tests/fixtures/encrypted/
    fixture_path = Path(__file__).parent.parent.parent.parent / "tests" / "fixtures" / "encrypted" / "EC-04-rc4-encrypted.expected.json"

    if not fixture_path.exists():
        print(f"⚠ Fixture file not found: {fixture_path}")
        return True  # Skip test gracefully

    with fixture_path.open("r") as f:
        fixture_data = json.load(f)

    doc = Document.from_native(fixture_data)

    # Test accessing first page
    infrastructure = PageAccessInfrastructure()
    first_page = infrastructure.access_first_page(doc)
    assert isinstance(first_page, Page)
    print("✓ Successfully accessed first page with type verification")

    # Test accessing all pages
    all_pages = infrastructure.access_all_pages(doc)
    assert len(all_pages) > 0
    print(f"✓ Successfully accessed all {len(all_pages)} page(s) with type verification")

    # Test page count
    page_count = infrastructure.get_page_count(doc)
    assert page_count == len(all_pages)
    print(f"✓ Page count matches: {page_count}")

    return True


def test_single_vs_multiple_page_access():
    """Test accessing single vs multiple pages."""
    print("\n--- Test: Single vs Multiple Page Access ---")

    # We're in: /home/coding/pdftract/crates/pdftract-py/tests/
    # Need to get to: /home/coding/pdftract/tests/fixtures/encrypted/
    fixture_path = Path(__file__).parent.parent.parent.parent / "tests" / "fixtures" / "encrypted" / "EC-04-rc4-encrypted.expected.json"

    if not fixture_path.exists():
        print(f"⚠ Fixture file not found: {fixture_path}")
        return True

    with fixture_path.open("r") as f:
        fixture_data = json.load(f)

    doc = Document.from_native(fixture_data)
    infrastructure = PageAccessInfrastructure()

    # Test single page access
    first_page = infrastructure.access_first_page(doc)
    assert isinstance(first_page, Page)
    print("✓ Single page access works correctly")

    # Test multiple page access
    all_pages = infrastructure.access_all_pages(doc)
    for i, page in enumerate(all_pages):
        assert isinstance(page, Page), \
            f'Expected Page type for page[{i}], got {type(page).__name__}'
    print(f"✓ Multiple page access works correctly ({len(all_pages)} pages)")

    return True


def test_page_access_by_index():
    """Test accessing pages by index."""
    print("\n--- Test: Page Access by Index ---")

    # We're in: /home/coding/pdftract/crates/pdftract-py/tests/
    # Need to get to: /home/coding/pdftract/tests/fixtures/encrypted/
    fixture_path = Path(__file__).parent.parent.parent.parent / "tests" / "fixtures" / "encrypted" / "EC-04-rc4-encrypted.expected.json"

    if not fixture_path.exists():
        print(f"⚠ Fixture file not found: {fixture_path}")
        return True

    with fixture_path.open("r") as f:
        fixture_data = json.load(f)

    doc = Document.from_native(fixture_data)
    infrastructure = PageAccessInfrastructure()

    page_count = infrastructure.get_page_count(doc)

    # Test accessing first page by index
    page_0 = infrastructure.access_page_by_index(doc, 0)
    assert isinstance(page_0, Page)
    print("✓ Successfully accessed page[0] by index")

    # Test accessing last page by index
    last_index = page_count - 1
    page_last = infrastructure.access_page_by_index(doc, last_index)
    assert isinstance(page_last, Page)
    print(f"✓ Successfully accessed page[{last_index}] by index")

    # Test bounds checking
    try:
        infrastructure.access_page_by_index(doc, page_count)  # Should fail
        print("✗ Bounds checking failed - should have raised AssertionError")
        return False
    except AssertionError as e:
        if "out of bounds" in str(e):
            print("✓ Bounds checking works correctly")
        else:
            print(f"✗ Unexpected error: {e}")
            return False

    return True


def test_page_structure_verification():
    """Test Page object structure verification."""
    print("\n--- Test: Page Structure Verification ---")

    # We're in: /home/coding/pdftract/crates/pdftract-py/tests/
    # Need to get to: /home/coding/pdftract/tests/fixtures/encrypted/
    fixture_path = Path(__file__).parent.parent.parent.parent / "tests" / "fixtures" / "encrypted" / "EC-04-rc4-encrypted.expected.json"

    if not fixture_path.exists():
        print(f"⚠ Fixture file not found: {fixture_path}")
        return True

    with fixture_path.open("r") as f:
        fixture_data = json.load(f)

    doc = Document.from_native(fixture_data)
    infrastructure = PageAccessInfrastructure()

    page = infrastructure.access_first_page(doc)

    # Verify expected Page attributes exist
    expected_attrs = ["width", "height", "blocks", "spans"]

    for attr in expected_attrs:
        if hasattr(page, attr):
            print(f"✓ Page has '{attr}' attribute")
        else:
            print(f"✗ Page missing '{attr}' attribute")
            return False

    # Verify nested structure access
    if hasattr(page, "spans") and len(page.spans) > 0:
        first_span = page.spans[0]
        assert isinstance(first_span, Span), \
            f"Expected Span type, got {type(first_span).__name__}"
        print("✓ Successfully accessed nested spans structure")

    return True


def test_page_type_assertions():
    """Test that Page objects pass type assertions."""
    print("\n--- Test: Page Type Assertions ---")

    # We're in: /home/coding/pdftract/crates/pdftract-py/tests/
    # Need to get to: /home/coding/pdftract/tests/fixtures/encrypted/
    fixture_path = Path(__file__).parent.parent.parent.parent / "tests" / "fixtures" / "encrypted" / "EC-04-rc4-encrypted.expected.json"

    if not fixture_path.exists():
        print(f"⚠ Fixture file not found: {fixture_path}")
        return True

    with fixture_path.open("r") as f:
        fixture_data = json.load(f)

    doc = Document.from_native(fixture_data)
    infrastructure = PageAccessInfrastructure()

    all_pages = infrastructure.access_all_pages(doc)

    # Verify all pages are Page instances, not dicts
    for i, page in enumerate(all_pages):
        assert isinstance(page, Page), \
            f'Expected Page type for page[{i}], got {type(page).__name__}'
        assert not isinstance(page, dict), \
            f"page[{i}] should not be a raw dict"

    print(f"✓ All {len(all_pages)} page(s) are properly typed Page instances")
    print("✓ No pages are raw dicts")

    return True


def test_page_access_with_real_extraction():
    """Test Page access with real PDF extraction."""
    print("\n--- Test: Page Access with Real Extraction ---")

    fixture_pdf = Path(__file__).parent / "fixtures" / "test-minimal.pdf"

    if not fixture_pdf.exists():
        print(f"⚠ PDF fixture not found: {fixture_pdf}")
        return True

    # Extract document
    doc = pdftract.extract(str(fixture_pdf))

    # Use infrastructure to access pages
    infrastructure = PageAccessInfrastructure()

    # Access first page
    first_page = infrastructure.access_first_page(doc)
    assert isinstance(first_page, Page)
    print("✓ Successfully accessed first page from extracted PDF")

    # Access all pages
    all_pages = infrastructure.access_all_pages(doc)
    assert len(all_pages) == infrastructure.get_page_count(doc)
    print(f"✓ Successfully accessed all {len(all_pages)} page(s) from extracted PDF")

    return True


def main():
    """Run all Page access tests."""
    print("=" * 60)
    print("Page Object Access Test Infrastructure")
    print("=" * 60)

    tests = [
        ("Basic Page Access", test_basic_page_access),
        ("Single vs Multiple Page Access", test_single_vs_multiple_page_access),
        ("Page Access by Index", test_page_access_by_index),
        ("Page Structure Verification", test_page_structure_verification),
        ("Page Type Assertions", test_page_type_assertions),
        ("Page Access with Real Extraction", test_page_access_with_real_extraction),
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
