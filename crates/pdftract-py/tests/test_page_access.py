"""Page object access test infrastructure.

This module provides dedicated test infrastructure for accessing Page objects
from Document results. It establishes clear patterns for:

1. Accessing single Page objects from Document
2. Accessing multiple Page objects (lists/arrays) from Document
3. Verifying Page type assertions on accessed objects
4. Working with Page attributes and nested structures

The infrastructure is designed to work with both extracted PDF data and
fixture data loaded from expected.json files.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

# Add the python package to the path
import sys
sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

import pdftract


class PageAccessInfrastructure:
    """Test infrastructure for accessing Page objects from Document results.

    This class provides helper methods and patterns for reliably accessing
    Page objects nested within Document structures. It handles both single
    Page access and multiple Pages access patterns.
    """

    @staticmethod
    def access_first_page(doc: pdftract.Document) -> pdftract.Page:
        """Access the first Page from a Document.

        Args:
            doc: A pdftract.Document instance

        Returns:
            The first Page object from the document

        Raises:
            AssertionError: If document has no pages
            TypeError: If first page is not a Page instance
        """
        assert len(doc.pages) > 0, "Document must contain at least one page"

        first_page = doc.pages[0]
        assert isinstance(first_page, pdftract.Page), \
            f'Expected Page type for first page, got {type(first_page).__name__}'

        return first_page

    @staticmethod
    def access_page_by_index(doc: pdftract.Document, index: int) -> pdftract.Page:
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
        assert isinstance(page, pdftract.Page), \
            f'Expected Page type for page[{index}], got {type(page).__name__}'

        return page

    @staticmethod
    def access_all_pages(doc: pdftract.Document) -> list[pdftract.Page]:
        """Access all Pages from a Document with type verification.

        Args:
            doc: A pdftract.Document instance

        Returns:
            List of Page objects with type verification

        Raises:
            AssertionError: If any page is not a Page instance
        """
        pages: list[pdftract.Page] = []

        for i, page in enumerate(doc.pages):
            assert isinstance(page, pdftract.Page), \
                f'Expected Page type for page[{i}], got {type(page).__name__}'
            pages.append(page)

        return pages

    @staticmethod
    def access_last_page(doc: pdftract.Document) -> pdftract.Page:
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
        assert isinstance(last_page, pdftract.Page), \
            f'Expected Page type for last page, got {type(last_page).__name__}'

        return last_page

    @staticmethod
    def iterate_pages_with_indices(doc: pdftract.Document) -> list[tuple[int, pdftract.Page]]:
        """Iterate over all pages with their indices.

        Args:
            doc: A pdftract.Document instance

        Returns:
            List of (index, Page) tuples
        """
        page_tuples: list[tuple[int, pdftract.Page]] = []

        for i, page in enumerate(doc.pages):
            assert isinstance(page, pdftract.Page), \
                f'Expected Page type for page[{i}], got {type(page).__name__}'
            page_tuples.append((i, page))

        return page_tuples

    @staticmethod
    def get_page_count(doc: pdftract.Document) -> int:
        """Get the number of pages in a Document.

        Args:
            doc: A pdftract.Document instance

        Returns:
            Number of pages in the document
        """
        return len(doc.pages)

    @staticmethod
    def verify_page_structure(page: pdftract.Page) -> None:
        """Verify that a Page object has the expected structure.

        Args:
            page: A pdftract.Page instance to verify

        Raises:
            AssertionError: If page doesn't have expected attributes
        """
        # Check for expected Page attributes
        expected_attrs = ["width", "height", "blocks", "spans"]

        for attr in expected_attrs:
            assert hasattr(page, attr), f"Page should have '{attr}' attribute"


@pytest.fixture
def sample_document() -> pdftract.Document:
    """Load a sample Document for Page access testing.

    This fixture creates a Document instance from fixture data for
    testing Page object access patterns.
    """
    fixture_path = Path(__file__).parent.parent.parent.parent.parent / \
                   "tests" / "fixtures" / "encrypted" / "EC-04-rc4-encrypted.expected.json"

    if not fixture_path.exists():
        pytest.skip(f"Fixture file not found: {fixture_path}")

    with fixture_path.open("r") as f:
        fixture_data = json.load(f)

    return pdftract.Document.from_native(fixture_data)


@pytest.fixture
def infrastructure() -> PageAccessInfrastructure:
    """Provide the PageAccessInfrastructure helper class."""
    return PageAccessInfrastructure()


class TestSinglePageAccess:
    """Tests for accessing single Page objects from Document."""

    def test_access_first_page(self, sample_document: pdftract.Document,
                               infrastructure: PageAccessInfrastructure) -> None:
        """Test accessing the first page from a Document."""
        first_page = infrastructure.access_first_page(sample_document)

        # Verify it's a Page instance
        assert isinstance(first_page, pdftract.Page)

        # Verify it has expected structure
        infrastructure.verify_page_structure(first_page)

    def test_access_last_page(self, sample_document: pdftract.Document,
                             infrastructure: PageAccessInfrastructure) -> None:
        """Test accessing the last page from a Document."""
        last_page = infrastructure.access_last_page(sample_document)

        # Verify it's a Page instance
        assert isinstance(last_page, pdftract.Page)

        # Verify it has expected structure
        infrastructure.verify_page_structure(last_page)

    def test_access_page_by_index(self, sample_document: pdftract.Document,
                                  infrastructure: PageAccessInfrastructure) -> None:
        """Test accessing a specific page by index."""
        page_count = infrastructure.get_page_count(sample_document)

        if page_count > 1:
            # Access middle page
            middle_index = page_count // 2
            page = infrastructure.access_page_by_index(sample_document, middle_index)

            # Verify it's a Page instance
            assert isinstance(page, pdftract.Page)
            infrastructure.verify_page_structure(page)

    def test_access_page_by_index_bounds_checking(self, sample_document: pdftract.Document,
                                                   infrastructure: PageAccessInfrastructure) -> None:
        """Test that index bounds are properly checked."""
        page_count = infrastructure.get_page_count(sample_document)

        # Test out of bounds access
        with pytest.raises(AssertionError, match="Page index.*out of bounds"):
            infrastructure.access_page_by_index(sample_document, page_count)

    def test_single_page_type_assertion(self, sample_document: pdftract.Document,
                                       infrastructure: PageAccessInfrastructure) -> None:
        """Test that single page access performs type assertion."""
        page = infrastructure.access_first_page(sample_document)

        # This should be a Page, not a dict
        assert isinstance(page, pdftract.Page), \
            f'Expected Page type, got {type(page).__name__}'
        assert not isinstance(page, dict), "Page should not be a raw dict"


class TestMultiplePageAccess:
    """Tests for accessing multiple Page objects from Document."""

    def test_access_all_pages(self, sample_document: pdftract.Document,
                             infrastructure: PageAccessInfrastructure) -> None:
        """Test accessing all pages from a Document."""
        pages = infrastructure.access_all_pages(sample_document)

        # Verify all are Page instances
        for page in pages:
            assert isinstance(page, pdftract.Page), \
                f'Expected Page type, got {type(page).__name__}'

    def test_iterate_pages_with_indices(self, sample_document: pdftract.Document,
                                       infrastructure: PageAccessInfrastructure) -> None:
        """Test iterating over pages with their indices."""
        page_tuples = infrastructure.iterate_pages_with_indices(sample_document)

        # Verify structure
        for index, page in page_tuples:
            assert isinstance(index, int)
            assert isinstance(page, pdftract.Page), \
                f'Expected Page type at index {index}, got {type(page).__name__}'

    def test_page_count_matches_actual(self, sample_document: pdftract.Document,
                                      infrastructure: PageAccessInfrastructure) -> None:
        """Test that page count matches the actual number of pages."""
        count = infrastructure.get_page_count(sample_document)
        pages = infrastructure.access_all_pages(sample_document)

        assert count == len(pages), \
            f"Page count {count} doesn't match actual pages {len(pages)}"

    def test_multiple_pages_type_assertion(self, sample_document: pdftract.Document,
                                          infrastructure: PageAccessInfrastructure) -> None:
        """Test that all pages in a Document are properly typed."""
        pages = infrastructure.access_all_pages(sample_document)

        # Every page should be a Page instance, not a dict
        for i, page in enumerate(pages):
            assert isinstance(page, pdftract.Page), \
                f'Expected Page type for page[{i}], got {type(page).__name__}'
            assert not isinstance(page, dict), \
                f"page[{i}] should not be a raw dict"


class TestPageAccessPatterns:
    """Tests demonstrating common Page access patterns."""

    def test_pattern_access_first_and_last(self, sample_document: pdftract.Document,
                                          infrastructure: PageAccessInfrastructure) -> None:
        """Test common pattern: access first and last pages."""
        first_page = infrastructure.access_first_page(sample_document)
        last_page = infrastructure.access_last_page(sample_document)

        # Both should be Page instances
        assert isinstance(first_page, pdftract.Page)
        assert isinstance(last_page, pdftract.Page)

        # If document has only one page, first and last should be same object
        if infrastructure.get_page_count(sample_document) == 1:
            assert first_page is last_page

    def test_pattern_iterate_all_pages(self, sample_document: pdftract.Document,
                                      infrastructure: PageAccessInfrastructure) -> None:
        """Test common pattern: iterate over all pages and verify types."""
        for i, page in enumerate(sample_document.pages):
            assert isinstance(page, pdftract.Page), \
                f'Expected Page type for page[{i}], got {type(page).__name__}'

    def test_pattern_access_nested_structure(self, sample_document: pdftract.Document,
                                            infrastructure: PageAccessInfrastructure) -> None:
        """Test accessing nested structures from Page objects."""
        page = infrastructure.access_first_page(sample_document)

        # Verify we can access nested structures
        assert hasattr(page, "spans"), "Page should have spans attribute"

        # If spans exist, verify they're accessible
        if len(page.spans) > 0:
            first_span = page.spans[0]
            assert isinstance(first_span, pdftract.Span), \
                f"Expected Span type, got {type(first_span).__name__}"

    def test_span_type_assertions_from_page_result(self, sample_document: pdftract.Document,
                                                   infrastructure: PageAccessInfrastructure) -> None:
        """Test Span type assertions when accessing Span objects from Page results.

        Parent bead: bf-6d70ph
        This test ensures that Span objects accessed from Page results maintain
        correct type contracts in the type hierarchy. It validates isinstance()
        assertions for Span type when accessing spans from Page results.
        """
        # Access Page from Document result
        page = infrastructure.access_first_page(sample_document)

        # Verify Page has spans attribute
        assert hasattr(page, "spans"), "Page should have spans attribute"
        assert isinstance(page.spans, (list, tuple)), \
            f"page.spans should be a list or tuple, got {type(page.spans).__name__}"

        # Skip test if page has no spans
        if len(page.spans) == 0:
            pytest.skip("No spans found in first page - cannot test span type assertions")

        # Test: Access Span objects from Page result
        # Add isinstance() assertion for Span type (not generic type)
        for span_idx, span in enumerate(page.spans):
            assert isinstance(span, pdftract.Span), \
                f"page.spans[{span_idx}] should be Span type, got {type(span).__name__}"
            assert not isinstance(span, dict), \
                f"page.spans[{span_idx}] should be typed Span instance, not raw dict"

        # Verify first span specifically has Span type
        first_span = page.spans[0]
        assert isinstance(first_span, pdftract.Span), \
            f"Expected Span type for first span, got {type(first_span).__name__}"

        # Verify all spans maintain Span type (not generic types)
        for span in page.spans:
            assert isinstance(span, pdftract.Span), \
                f"Expected Span type, got {type(span).__name__}"
            # Assertion would fail if Span type is incorrect

    def test_pattern_filter_pages_by_criteria(self, sample_document: pdftract.Document,
                                              infrastructure: PageAccessInfrastructure) -> None:
        """Test pattern: filter pages based on criteria."""
        pages = infrastructure.access_all_pages(sample_document)

        # Filter pages that have spans
        pages_with_spans = [p for p in pages if len(p.spans) > 0]

        # Verify all filtered results are still Page instances
        for page in pages_with_spans:
            assert isinstance(page, pdftract.Page)


class TestPageAccessWithRealExtraction:
    """Tests Page access patterns with real PDF extraction."""

    def test_extract_and_access_pages(self) -> None:
        """Test Page access with a real extracted PDF."""
        fixture_pdf = Path(__file__).parent / "fixtures" / "test-minimal.pdf"

        if not fixture_pdf.exists():
            pytest.skip(f"PDF fixture not found: {fixture_pdf}")

        # Extract document
        doc = pdftract.extract(str(fixture_pdf))

        # Use infrastructure to access pages
        infrastructure = PageAccessInfrastructure()

        # Access first page
        first_page = infrastructure.access_first_page(doc)
        assert isinstance(first_page, pdftract.Page)

        # Access all pages
        all_pages = infrastructure.access_all_pages(doc)
        assert len(all_pages) == infrastructure.get_page_count(doc)

    def test_extract_single_vs_multiple_pages(self) -> None:
        """Test Page access handles both single and multi-page documents."""
        fixture_pdf = Path(__file__).parent / "fixtures" / "test-minimal.pdf"

        if not fixture_pdf.exists():
            pytest.skip(f"PDF fixture not found: {fixture_pdf}")

        doc = pdftract.extract(str(fixture_pdf))
        infrastructure = PageAccessInfrastructure()

        page_count = infrastructure.get_page_count(doc)

        # Should work for both single and multiple page documents
        if page_count == 1:
            # Single page document
            first_page = infrastructure.access_first_page(doc)
            last_page = infrastructure.access_last_page(doc)
            assert first_page is last_page
        else:
            # Multi-page document
            all_pages = infrastructure.access_all_pages(doc)
            assert len(all_pages) > 1


class TestPageAccessErrorHandling:
    """Tests for error handling in Page access."""

    def test_empty_document_handling(self) -> None:
        """Test accessing pages from an empty document."""
        # Create a minimal document with no pages
        empty_data = {
            "schema_version": "1.0",
            "pages": [],
            "metadata": {"page_count": 0}
        }

        doc = pdftract.Document.from_native(empty_data)
        infrastructure = PageAccessInfrastructure()

        # Should handle gracefully
        with pytest.raises(AssertionError, match="Document must contain at least one page"):
            infrastructure.access_first_page(doc)

    def test_invalid_index_handling(self, sample_document: pdftract.Document,
                                   infrastructure: PageAccessInfrastructure) -> None:
        """Test handling of invalid page indices."""
        # Negative index
        with pytest.raises(AssertionError, match="Page index.*out of bounds"):
            infrastructure.access_page_by_index(sample_document, -1)

        # Index too large
        page_count = infrastructure.get_page_count(sample_document)
        with pytest.raises(AssertionError, match="Page index.*out of bounds"):
            infrastructure.access_page_by_index(sample_document, page_count)


def test_page_access_infrastructure_integration():
    """Integration test for Page access infrastructure."""
    fixture_path = Path(__file__).parent.parent.parent.parent.parent / \
                   "tests" / "fixtures" / "encrypted" / "EC-04-rc4-encrypted.expected.json"

    if not fixture_path.exists():
        pytest.skip(f"Fixture file not found: {fixture_path}")

    with fixture_path.open("r") as f:
        fixture_data = json.load(f)

    doc = pdftract.Document.from_native(fixture_data)
    infrastructure = PageAccessInfrastructure()

    # Test complete access pattern
    page_count = infrastructure.get_page_count(doc)
    assert page_count > 0, "Fixture should have pages"

    # Access first page
    first_page = infrastructure.access_first_page(doc)
    assert isinstance(first_page, pdftract.Page)

    # Access all pages
    all_pages = infrastructure.access_all_pages(doc)
    assert len(all_pages) == page_count

    # Verify all pages are correctly typed
    for i, page in enumerate(all_pages):
        assert isinstance(page, pdftract.Page), \
            f'Expected Page type for page[{i}], got {type(page).__name__}'
        infrastructure.verify_page_structure(page)
