"""Span object access test infrastructure.

This module provides dedicated test infrastructure for accessing Span objects
from Page results. It establishes clear patterns for:

1. Accessing single Span objects from Page
2. Accessing multiple Span objects (lists/arrays) from Page
3. Verifying Span type assertions on accessed objects
4. Working with Span attributes and nested structures

The infrastructure is designed to work with both extracted PDF data and
fixture data loaded from expected.json files, building on the Page access
infrastructure.
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

# Import the Page access infrastructure
from test_page_access import PageAccessInfrastructure


class SpanAccessInfrastructure:
    """Test infrastructure for accessing Span objects from Page results.

    This class provides helper methods and patterns for reliably accessing
    Span objects nested within Page structures. It handles both single
    Span access and multiple Spans access patterns.
    """

    @staticmethod
    def access_first_span(page: pdftract.Page) -> pdftract.Span:
        """Access the first Span from a Page.

        Args:
            page: A pdftract.Page instance

        Returns:
            The first Span object from the page

        Raises:
            AssertionError: If page has no spans
            TypeError: If first span is not a Span instance
        """
        assert len(page.spans) > 0, "Page must contain at least one span"

        first_span = page.spans[0]
        assert isinstance(first_span, pdftract.Span), \
            f'Expected Span type for first span, got {type(first_span).__name__}'

        return first_span

    @staticmethod
    def access_span_by_index(page: pdftract.Page, index: int) -> pdftract.Span:
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
        assert isinstance(span, pdftract.Span), \
            f'Expected Span type for span[{index}], got {type(span).__name__}'

        return span

    @staticmethod
    def access_all_spans(page: pdftract.Page) -> list[pdftract.Span]:
        """Access all Spans from a Page with type verification.

        Args:
            page: A pdftract.Page instance

        Returns:
            List of Span objects with type verification

        Raises:
            AssertionError: If any span is not a Span instance
        """
        spans: list[pdftract.Span] = []

        for i, span in enumerate(page.spans):
            assert isinstance(span, pdftract.Span), \
                f'Expected Span type for span[{i}], got {type(span).__name__}'
            spans.append(span)

        return spans

    @staticmethod
    def access_last_span(page: pdftract.Page) -> pdftract.Span:
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
        assert isinstance(last_span, pdftract.Span), \
            f'Expected Span type for last span, got {type(last_span).__name__}'

        return last_span

    @staticmethod
    def iterate_spans_with_indices(page: pdftract.Page) -> list[tuple[int, pdftract.Span]]:
        """Iterate over all spans with their indices.

        Args:
            page: A pdftract.Page instance

        Returns:
            List of (index, Span) tuples
        """
        span_tuples: list[tuple[int, pdftract.Span]] = []

        for i, span in enumerate(page.spans):
            assert isinstance(span, pdftract.Span), \
                f'Expected Span type for span[{i}], got {type(span).__name__}'
            span_tuples.append((i, span))

        return span_tuples

    @staticmethod
    def get_span_count(page: pdftract.Page) -> int:
        """Get the number of spans in a Page.

        Args:
            page: A pdftract.Page instance

        Returns:
            Number of spans in the page
        """
        return len(page.spans)

    @staticmethod
    def verify_span_structure(span: pdftract.Span) -> None:
        """Verify that a Span object has the expected structure.

        Args:
            span: A pdftract.Span instance to verify

        Raises:
            AssertionError: If span doesn't have expected attributes
        """
        # Check for expected Span attributes
        expected_attrs = ["text", "bbox", "font", "size", "confidence"]

        for attr in expected_attrs:
            assert hasattr(span, attr), f"Span should have '{attr}' attribute"

    @staticmethod
    def filter_spans_by_criteria(page: pdftract.Page,
                                 criteria: callable[[pdftract.Span], bool]) -> list[pdftract.Span]:
        """Filter spans based on custom criteria.

        Args:
            page: A pdftract.Page instance
            criteria: A callable that takes a Span and returns bool

        Returns:
            List of Span objects matching the criteria
        """
        filtered_spans: list[pdftract.Span] = []

        for span in page.spans:
            if isinstance(span, pdftract.Span) and criteria(span):
                filtered_spans.append(span)

        return filtered_spans


@pytest.fixture
def sample_page() -> pdftract.Page:
    """Load a sample Page for Span access testing.

    This fixture creates a Page instance from fixture data for
    testing Span object access patterns.
    """
    # Get the correct path to fixtures
    # We're in: /home/coding/pdftract/crates/pdftract-py/tests/
    # Need to get to: /home/coding/pdftract/tests/fixtures/
    current_dir = Path(__file__).parent
    fixture_path = current_dir.parent.parent.parent.parent / \
                   "tests" / "fixtures" / "test-minimal.expected.json"

    if not fixture_path.exists():
        pytest.skip(f"Fixture file not found: {fixture_path}")

    with fixture_path.open("r") as f:
        fixture_data = json.load(f)

    doc = pdftract.Document.from_native(fixture_data)

    # Use PageAccessInfrastructure to get the first page
    page_infra = PageAccessInfrastructure()
    return page_infra.access_first_page(doc)


@pytest.fixture
def span_infrastructure() -> SpanAccessInfrastructure:
    """Provide the SpanAccessInfrastructure helper class."""
    return SpanAccessInfrastructure()


class TestSingleSpanAccess:
    """Tests for accessing single Span objects from Page."""

    def test_access_first_span(self, sample_page: pdftract.Page,
                               span_infrastructure: SpanAccessInfrastructure) -> None:
        """Test accessing the first span from a Page."""
        # Only test if page has spans
        if span_infrastructure.get_span_count(sample_page) == 0:
            pytest.skip("Page has no spans to test")

        first_span = span_infrastructure.access_first_span(sample_page)

        # Verify it's a Span instance
        assert isinstance(first_span, pdftract.Span)

        # Verify it has expected structure
        span_infrastructure.verify_span_structure(first_span)

    def test_access_last_span(self, sample_page: pdftract.Page,
                             span_infrastructure: SpanAccessInfrastructure) -> None:
        """Test accessing the last span from a Page."""
        # Only test if page has spans
        if span_infrastructure.get_span_count(sample_page) == 0:
            pytest.skip("Page has no spans to test")

        last_span = span_infrastructure.access_last_span(sample_page)

        # Verify it's a Span instance
        assert isinstance(last_span, pdftract.Span)

        # Verify it has expected structure
        span_infrastructure.verify_span_structure(last_span)

    def test_access_span_by_index(self, sample_page: pdftract.Page,
                                  span_infrastructure: SpanAccessInfrastructure) -> None:
        """Test accessing a specific span by index."""
        span_count = span_infrastructure.get_span_count(sample_page)

        if span_count == 0:
            pytest.skip("Page has no spans to test")

        if span_count > 1:
            # Access middle span
            middle_index = span_count // 2
            span = span_infrastructure.access_span_by_index(sample_page, middle_index)

            # Verify it's a Span instance
            assert isinstance(span, pdftract.Span)
            span_infrastructure.verify_span_structure(span)

    def test_access_span_by_index_bounds_checking(self, sample_page: pdftract.Page,
                                                   span_infrastructure: SpanAccessInfrastructure) -> None:
        """Test that index bounds are properly checked."""
        span_count = span_infrastructure.get_span_count(sample_page)

        if span_count == 0:
            pytest.skip("Page has no spans to test")

        # Test out of bounds access
        with pytest.raises(AssertionError, match="Span index.*out of bounds"):
            span_infrastructure.access_span_by_index(sample_page, span_count)

    def test_single_span_type_assertion(self, sample_page: pdftract.Page,
                                       span_infrastructure: SpanAccessInfrastructure) -> None:
        """Test that single span access performs type assertion."""
        # Only test if page has spans
        if span_infrastructure.get_span_count(sample_page) == 0:
            pytest.skip("Page has no spans to test")

        span = span_infrastructure.access_first_span(sample_page)

        # This should be a Span, not a dict
        assert isinstance(span, pdftract.Span), \
            f'Expected Span type, got {type(span).__name__}'
        assert not isinstance(span, dict), "Span should not be a raw dict"


class TestMultipleSpanAccess:
    """Tests for accessing multiple Span objects from Page."""

    def test_access_all_spans(self, sample_page: pdftract.Page,
                             span_infrastructure: SpanAccessInfrastructure) -> None:
        """Test accessing all spans from a Page."""
        # Only test if page has spans
        if span_infrastructure.get_span_count(sample_page) == 0:
            pytest.skip("Page has no spans to test")

        spans = span_infrastructure.access_all_spans(sample_page)

        # Verify all are Span instances
        for span in spans:
            assert isinstance(span, pdftract.Span), \
                f'Expected Span type, got {type(span).__name__}'

    def test_iterate_spans_with_indices(self, sample_page: pdftract.Page,
                                       span_infrastructure: SpanAccessInfrastructure) -> None:
        """Test iterating over spans with their indices."""
        # Only test if page has spans
        if span_infrastructure.get_span_count(sample_page) == 0:
            pytest.skip("Page has no spans to test")

        span_tuples = span_infrastructure.iterate_spans_with_indices(sample_page)

        # Verify structure
        for index, span in span_tuples:
            assert isinstance(index, int)
            assert isinstance(span, pdftract.Span), \
                f'Expected Span type at index {index}, got {type(span).__name__}'

    def test_span_count_matches_actual(self, sample_page: pdftract.Page,
                                      span_infrastructure: SpanAccessInfrastructure) -> None:
        """Test that span count matches the actual number of spans."""
        count = span_infrastructure.get_span_count(sample_page)

        # Only test if page has spans
        if count == 0:
            pytest.skip("Page has no spans to test")

        spans = span_infrastructure.access_all_spans(sample_page)

        assert count == len(spans), \
            f"Span count {count} doesn't match actual spans {len(spans)}"

    def test_multiple_spans_type_assertion(self, sample_page: pdftract.Page,
                                          span_infrastructure: SpanAccessInfrastructure) -> None:
        """Test that all spans in a Page are properly typed."""
        # Only test if page has spans
        if span_infrastructure.get_span_count(sample_page) == 0:
            pytest.skip("Page has no spans to test")

        spans = span_infrastructure.access_all_spans(sample_page)

        # Every span should be a Span instance, not a dict
        for i, span in enumerate(spans):
            assert isinstance(span, pdftract.Span), \
                f'Expected Span type for span[{i}], got {type(span).__name__}'
            assert not isinstance(span, dict), \
                f"span[{i}] should not be a raw dict"


class TestSpanAccessPatterns:
    """Tests demonstrating common Span access patterns."""

    def test_pattern_access_first_and_last(self, sample_page: pdftract.Page,
                                          span_infrastructure: SpanAccessInfrastructure) -> None:
        """Test common pattern: access first and last spans."""
        # Only test if page has spans
        if span_infrastructure.get_span_count(sample_page) == 0:
            pytest.skip("Page has no spans to test")

        first_span = span_infrastructure.access_first_span(sample_page)
        last_span = span_infrastructure.access_last_span(sample_page)

        # Both should be Span instances
        assert isinstance(first_span, pdftract.Span)
        assert isinstance(last_span, pdftract.Span)

        # If page has only one span, first and last should be same object
        if span_infrastructure.get_span_count(sample_page) == 1:
            assert first_span is last_span

    def test_pattern_iterate_all_spans(self, sample_page: pdftract.Page,
                                      span_infrastructure: SpanAccessInfrastructure) -> None:
        """Test common pattern: iterate over all spans and verify types."""
        # Only test if page has spans
        if span_infrastructure.get_span_count(sample_page) == 0:
            pytest.skip("Page has no spans to test")

        for i, span in enumerate(sample_page.spans):
            assert isinstance(span, pdftract.Span), \
                f'Expected Span type for span[{i}], got {type(span).__name__}'

    def test_pattern_filter_spans_by_font(self, sample_page: pdftract.Page,
                                         span_infrastructure: SpanAccessInfrastructure) -> None:
        """Test pattern: filter spans by font criteria."""
        # Only test if page has spans
        if span_infrastructure.get_span_count(sample_page) == 0:
            pytest.skip("Page has no spans to test")

        # Get all spans first to find a font to filter by
        all_spans = span_infrastructure.access_all_spans(sample_page)

        # Filter spans that have a font attribute
        spans_with_font = [s for s in all_spans if hasattr(s, 'font') and s.font]

        if not spans_with_font:
            pytest.skip("No spans with font attribute to test filtering")

        # Use the first font as filter criteria
        target_font = spans_with_font[0].font

        filtered = span_infrastructure.filter_spans_by_criteria(
            sample_page,
            lambda s: hasattr(s, 'font') and s.font == target_font
        )

        # Verify all filtered results are still Span instances
        for span in filtered:
            assert isinstance(span, pdftract.Span)
            assert span.font == target_font


class TestSpanAccessWithRealExtraction:
    """Tests Span access patterns with real PDF extraction."""

    def test_extract_and_access_spans(self) -> None:
        """Test Span access with a real extracted PDF."""
        fixture_pdf = Path(__file__).parent / "fixtures" / "test-minimal.pdf"

        if not fixture_pdf.exists():
            pytest.skip(f"PDF fixture not found: {fixture_pdf}")

        # Extract document
        doc = pdftract.extract(str(fixture_pdf))

        # Use Page infrastructure to get first page
        page_infra = PageAccessInfrastructure()
        first_page = page_infra.access_first_page(doc)

        # Use Span infrastructure to access spans
        span_infra = SpanAccessInfrastructure()

        # If page has spans, test access
        if span_infra.get_span_count(first_page) > 0:
            # Access first span
            first_span = span_infra.access_first_span(first_page)
            assert isinstance(first_span, pdftract.Span)

            # Access all spans
            all_spans = span_infra.access_all_spans(first_page)
            assert len(all_spans) == span_infra.get_span_count(first_page)
        else:
            pytest.skip("Extracted page has no spans to test")

    def test_extract_single_vs_multiple_spans(self) -> None:
        """Test Span access handles both single and multi-span pages."""
        fixture_pdf = Path(__file__).parent / "fixtures" / "test-minimal.pdf"

        if not fixture_pdf.exists():
            pytest.skip(f"PDF fixture not found: {fixture_pdf}")

        doc = pdftract.extract(str(fixture_pdf))

        # Use Page infrastructure to get first page
        page_infra = PageAccessInfrastructure()
        first_page = page_infra.access_first_page(doc)

        # Use Span infrastructure
        span_infra = SpanAccessInfrastructure()
        span_count = span_infra.get_span_count(first_page)

        # Should work for both single and multiple span pages
        if span_count == 0:
            pytest.skip("Extracted page has no spans to test")
        elif span_count == 1:
            # Single span page
            first_span = span_infra.access_first_span(first_page)
            last_span = span_infra.access_last_span(first_page)
            assert first_span is last_span
        else:
            # Multi-span page
            all_spans = span_infra.access_all_spans(first_page)
            assert len(all_spans) > 1


class TestSpanAccessErrorHandling:
    """Tests for error handling in Span access."""

    def test_empty_page_handling(self, sample_page: pdftract.Page,
                                 span_infrastructure: SpanAccessInfrastructure) -> None:
        """Test accessing spans from a page with no spans."""
        # Create a minimal page with no spans
        empty_page_data = {
            "page": 1,
            "width": 612,
            "height": 792,
            "rotation": 0,
            "spans": [],
            "blocks": []
        }

        empty_page = pdftract.Page.from_native(empty_page_data)
        infrastructure = SpanAccessInfrastructure()

        # Should handle gracefully
        with pytest.raises(AssertionError, match="Page must contain at least one span"):
            infrastructure.access_first_span(empty_page)

    def test_invalid_index_handling(self, sample_page: pdftract.Page,
                                   span_infrastructure: SpanAccessInfrastructure) -> None:
        """Test handling of invalid span indices."""
        span_count = span_infrastructure.get_span_count(sample_page)

        if span_count == 0:
            pytest.skip("Page has no spans to test invalid index handling")

        # Negative index
        with pytest.raises(AssertionError, match="Span index.*out of bounds"):
            span_infrastructure.access_span_by_index(sample_page, -1)

        # Index too large
        with pytest.raises(AssertionError, match="Span index.*out of bounds"):
            span_infrastructure.access_span_by_index(sample_page, span_count)


def test_span_access_infrastructure_integration():
    """Integration test for Span access infrastructure."""
    current_dir = Path(__file__).parent
    fixture_path = current_dir.parent.parent.parent.parent / \
                   "tests" / "fixtures" / "test-minimal.expected.json"

    if not fixture_path.exists():
        pytest.skip(f"Fixture file not found: {fixture_path}")

    with fixture_path.open("r") as f:
        fixture_data = json.load(f)

    doc = pdftract.Document.from_native(fixture_data)

    # Use Page infrastructure to get first page
    page_infra = PageAccessInfrastructure()
    first_page = page_infra.access_first_page(doc)

    # Use Span infrastructure
    span_infra = SpanAccessInfrastructure()

    # Test complete access pattern
    span_count = span_infra.get_span_count(first_page)

    if span_count == 0:
        pytest.skip("Fixture page has no spans to test")

    # Access first span
    first_span = span_infra.access_first_span(first_page)
    assert isinstance(first_span, pdftract.Span)

    # Access all spans
    all_spans = span_infra.access_all_spans(first_page)
    assert len(all_spans) == span_count

    # Verify all spans are correctly typed
    for i, span in enumerate(all_spans):
        assert isinstance(span, pdftract.Span), \
            f'Expected Span type for span[{i}], got {type(span).__name__}'
        span_infra.verify_span_structure(span)


def test_span_access_with_page_infrastructure_integration():
    """Test Span infrastructure integrated with Page infrastructure."""
    fixture_path = Path(__file__).parent.parent.parent.parent.parent / \
                   "tests" / "fixtures" / "encrypted" / "EC-04-rc4-encrypted.expected.json"

    if not fixture_path.exists():
        pytest.skip(f"Fixture file not found: {fixture_path}")

    with fixture_path.open("r") as f:
        fixture_data = json.load(f)

    doc = pdftract.Document.from_native(fixture_data)

    # Use both infrastructures together
    page_infra = PageAccessInfrastructure()
    span_infra = SpanAccessInfrastructure()

    # Access all pages
    all_pages = page_infra.access_all_pages(doc)

    # For each page, access all spans
    for page_idx, page in enumerate(all_pages):
        assert isinstance(page, pdftract.Page), \
            f'Expected Page type for page[{page_idx}], got {type(page).__name__}'

        span_count = span_infra.get_span_count(page)

        if span_count > 0:
            # Access spans from this page
            all_spans = span_infra.access_all_spans(page)

            # Verify all spans are correctly typed
            for span_idx, span in enumerate(all_spans):
                assert isinstance(span, pdftract.Span), \
                    f'Expected Span type for page[{page_idx}].spans[{span_idx}], got {type(span).__name__}'
