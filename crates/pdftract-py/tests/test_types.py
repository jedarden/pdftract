"""Smoke test for pdftract SDK type system.

This test verifies that the SDK returns properly typed objects
rather than raw dicts, ensuring users get IDE autocomplete support
and type safety throughout the API surface.

The test exercises the core extraction flow and validates that:
1. extract() returns a Document instance (not a dict)
2. Document.pages contains Page instances
3. Page.spans contains Span instances
4. Attribute access works on typed objects

This is a foundational smoke test for the type system contract.
"""

from __future__ import annotations

import pytest
import pdftract
from pdftract import Document, Page, Span


def test_extract_returns_typed_document():
    """Verify extract() returns a typed Document with proper object hierarchy.

    This smoke test validates the core type contract:
    - extract() returns Document instance (not dict)
    - Document.pages[0] is Page instance
    - Page.spans[0] is Span instance
    - Attribute access works (width, text, etc.)

    Uses a minimal PDF fixture to keep the test fast and reliable.
    """
    # Use a simple, minimal PDF fixture that should always parse successfully
    fixture_path = "tests/fixtures/test-minimal.pdf"

    # Extract the document
    doc = pdftract.extract(fixture_path)

    # Assert top-level return type is Document, not dict
    assert isinstance(doc, Document), \
        f"extract() should return Document instance, got {type(doc).__name__}"

    # Assert document has pages
    assert doc.pages, "Document should contain pages"

    # Assert ALL pages are Page instances with expected attributes
    for i, page in enumerate(doc.pages):
        assert isinstance(page, pdftract.Page), \
            f"Page {i} should be a Page instance, got {type(page).__name__}"
        assert hasattr(page, "spans"), \
            f"Page {i} should have spans attribute"
        assert hasattr(page, "width"), \
            f"Page {i} should have width attribute"
        assert hasattr(page, "height"), \
            f"Page {i} should have height attribute"

    # Assert first page is Page instance
    first_page = doc.pages[0]
    assert isinstance(first_page, Page), \
        f"doc.pages[0] should be Page instance, got {type(first_page).__name__}"

    # Assert page has spans (most PDFs will have some text content)
    if len(first_page.spans) > 0:
        # Assert first span is Span instance
        first_span = first_page.spans[0]
        assert isinstance(first_span, Span), \
            f"page.spans[0] should be Span instance, got {type(first_span).__name__}"

        # Assert attribute access works (not just dict-style access)
        assert hasattr(first_span, 'text'), \
            "Span should have 'text' attribute for IDE autocomplete"
        assert isinstance(first_span.text, str), \
            "Span.text should return a string"


def test_extract_returns_typed_document_with_valid_minimal():
    """Alternative smoke test using valid-minimal.pdf fixture.

    Provides redundancy in case one fixture has parsing issues.
    Tests the same type contract with a different minimal PDF.
    """
    fixture_path = "tests/fixtures/valid-minimal.pdf"

    doc = pdftract.extract(fixture_path)

    # Verify Document type
    assert isinstance(doc, Document), \
        f"extract() should return Document instance, got {type(doc).__name__}"

    # Verify Page type in hierarchy
    assert len(doc.pages) > 0, "Document should have at least one page"
    assert isinstance(doc.pages[0], Page), \
        f"doc.pages[0] should be Page instance, got {type(doc.pages[0]).__name__}"

    # Verify Span type in hierarchy (if spans exist)
    if len(doc.pages[0].spans) > 0:
        assert isinstance(doc.pages[0].spans[0], Span), \
            f"page.spans[0] should be Span instance, got {type(doc.pages[0].spans[0]).__name__}"
