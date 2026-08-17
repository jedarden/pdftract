"""Pytest-style tests for type assertions using fixture data.

This test module verifies that the pdftract SDK returns properly typed
objects by loading real fixture data and validating type contracts.
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


@pytest.fixture
def fixture_data() -> dict[str, Any]:
    """Load sample fixture data for type assertion tests.

    This fixture loads an expected.json file containing a real parsed
    PDF result. Tests use this data to verify that the SDK returns
    properly typed objects matching the expected structure.

    Returns:
        Dictionary containing the fixture data with pages, metadata,
        and other parsed content.
    """
    fixture_path = Path(__file__).parent.parent.parent.parent / "tests" / "fixtures" / "encrypted" / "EC-04-rc4-encrypted.expected.json"

    if not fixture_path.exists():
        pytest.skip(f"Fixture file not found: {fixture_path}")

    with fixture_path.open("r") as f:
        return json.load(f)


def test_extract_returns_document_type() -> None:
    """Verify extract() returns a Document instance.

    This test validates the core type contract that extract() returns
    a properly typed Document object, not a raw dict.
    """
    # Use a simple PDF fixture for testing
    fixture_pdf = Path(__file__).parent / "fixtures" / "test-minimal.pdf"

    if not fixture_pdf.exists():
        pytest.skip(f"PDF fixture not found: {fixture_pdf}")

    doc = pdftract.extract(str(fixture_pdf))

    # Verify Document type
    assert isinstance(doc, pdftract.Document), \
        f'Expected Document, got {type(doc).__name__}'

    # Verify it's not a raw dict
    assert not isinstance(doc, dict), "extract() should not return a raw dict"

    # Verify Document contains pages
    assert doc.pages, "Document should contain pages"


def test_document_has_required_attributes() -> None:
    """Verify Document has all required attributes.

    Tests that Document objects expose the expected attributes for
    IDE autocomplete and type checking.
    """
    fixture_pdf = Path(__file__).parent / "fixtures" / "test-minimal.pdf"

    if not fixture_pdf.exists():
        pytest.skip(f"PDF fixture not found: {fixture_pdf}")

    doc = pdftract.extract(str(fixture_pdf))

    # Check required Document attributes
    required_attrs = ["pages", "metadata", "schema_version"]

    for attr in required_attrs:
        assert hasattr(doc, attr), f"Document should have '{attr}' attribute"


def test_metadata_is_typed() -> None:
    """Verify metadata is a typed Metadata instance.

    Tests that doc.metadata returns a proper Metadata object with
    type-checked attributes.
    """
    fixture_pdf = Path(__file__).parent / "fixtures" / "test-minimal.pdf"

    if not fixture_pdf.exists():
        pytest.skip(f"PDF fixture not found: {fixture_pdf}")

    doc = pdftract.extract(str(fixture_pdf))

    # Verify Metadata type
    assert isinstance(doc.metadata, pdftract.Metadata), \
        f"doc.metadata should be Metadata instance, got {type(doc.metadata).__name__}"

    # Verify metadata has expected attributes
    metadata_attrs = ["page_count", "title", "author", "subject"]

    for attr in metadata_attrs:
        assert hasattr(doc.metadata, attr), f"Metadata should have '{attr}' attribute"


def test_pages_is_list() -> None:
    """Verify doc.pages is a list of Page instances.

    Tests that the pages attribute returns a proper list and that
    page objects are typed correctly.
    """
    fixture_pdf = Path(__file__).parent / "fixtures" / "test-minimal.pdf"

    if not fixture_pdf.exists():
        pytest.skip(f"PDF fixture not found: {fixture_pdf}")

    doc = pdftract.extract(str(fixture_pdf))

    # Verify pages is a list
    assert isinstance(doc.pages, list), "doc.pages should be a list"

    # If we have pages, verify they are Page instances
    if len(doc.pages) > 0:
        page = doc.pages[0]
        assert isinstance(page, pdftract.Page), \
            f"Expected Page type, got {type(page).__name__}"
        assert not isinstance(page, dict), "Page should not be a raw dict"


def test_fixture_data_structure(fixture_data: dict[str, Any]) -> None:
    """Verify fixture data has expected structure.

    This test validates that fixture files contain the required
    top-level keys for testing purposes.

    Args:
        fixture_data: Loaded fixture data from the fixture.
    """
    # Verify fixture has required top-level keys
    expected_keys = ["pages", "metadata", "schema_version"]

    for key in expected_keys:
        assert key in fixture_data, f"Fixture data should contain '{key}' key"

    # Verify metadata has required fields
    if "metadata" in fixture_data:
        metadata_keys = ["page_count"]
        for key in metadata_keys:
            assert key in fixture_data["metadata"], \
                f"Fixture metadata should contain '{key}' key"


def test_document_type_from_pdf_extraction() -> None:
    """Test type assertions from a real PDF extraction.

    This test loads a real PDF fixture, extracts it using the pdftract SDK,
    and validates that all returned objects match their expected types.
    """
    fixture_pdf = Path(__file__).parent / "fixtures" / "test-minimal.pdf"

    if not fixture_pdf.exists():
        pytest.skip(f"PDF fixture not found: {fixture_pdf}")

    # Extract document from PDF
    doc = pdftract.extract(str(fixture_pdf))

    # Verify Document type
    assert isinstance(doc, pdftract.Document), \
        f'Expected Document, got {type(doc).__name__}'

    # Verify metadata is typed
    assert isinstance(doc.metadata, pdftract.Metadata), \
        f"doc.metadata should be Metadata instance, got {type(doc.metadata).__name__}"

    # Verify pages is a list
    assert isinstance(doc.pages, list), \
        f"doc.pages should be a list, got {type(doc.pages).__name__}"

    # Verify each page is a Page instance (if pages exist)
    for i, page in enumerate(doc.pages):
        assert isinstance(page, pdftract.Page), \
            f"Expected Page type, got {type(page).__name__}"


def test_metadata_field_types() -> None:
    """Test that metadata fields have correct types.

    Validates that the Metadata object returns values with the expected
    Python types for each field.
    """
    fixture_pdf = Path(__file__).parent / "fixtures" / "test-minimal.pdf"

    if not fixture_pdf.exists():
        pytest.skip(f"PDF fixture not found: {fixture_pdf}")

    doc = pdftract.extract(str(fixture_pdf))
    metadata = doc.metadata

    # Verify metadata fields are properly typed
    assert isinstance(metadata.page_count, int), \
        f"metadata.page_count should be int, got {type(metadata.page_count).__name__}"

    # String fields may be None or str
    if metadata.title is not None:
        assert isinstance(metadata.title, str), \
            f"metadata.title should be str or None, got {type(metadata.title).__name__}"

    if metadata.author is not None:
        assert isinstance(metadata.author, str), \
            f"metadata.author should be str or None, got {type(metadata.author).__name__}"


def test_document_type_from_fixture_data(fixture_data: dict[str, Any]) -> None:
    """Verify Document.from_native() returns a Document instance.

    This test validates the core type assertion that when calling
    Document.from_native() with loaded fixture data, it returns
    a properly typed Document object, not a raw dict.

    Args:
        fixture_data: Loaded fixture data from the fixture.
    """
    # Call Document.from_native with fixture data
    result = pdftract.Document.from_native(fixture_data)

    # Verify Document type
    assert isinstance(result, pdftract.Document), \
        f'Expected Document type, got {type(result).__name__}'

    # Verify Document is not a raw dict
    assert not isinstance(result, dict), \
        "Document should be a typed Document instance, not a raw dict"

    # Verify ALL pages are Page instances (handle multiple objects)
    assert len(result.pages) > 0, "Document should have at least one page"

    # Verify pages is a list
    assert isinstance(result.pages, list), \
        f"doc.pages should be a list, got {type(result.pages).__name__}"

    # Verify first page is a Page instance with descriptive error
    assert isinstance(result.pages[0], pdftract.Page), \
        f'Expected Page type for doc.pages[0], got {type(result.pages[0]).__name__}'

    # Verify first page is not a raw dict
    assert not isinstance(result.pages[0], dict), \
        "doc.pages[0] should be a typed Page instance, not a raw dict"

    # Add isinstance() assertion for Page type with descriptive error message
    # Handle all pages in the result list with a for loop
    for page_idx, page in enumerate(result.pages):
        # Verify each page is a Page instance with detailed error
        assert isinstance(page, pdftract.Page), \
            f"Expected Page type for doc.pages[{page_idx}], got {type(page).__name__}"

        # Verify each page is not a raw dict
        assert not isinstance(page, dict), \
            f"doc.pages[{page_idx}] should be a typed Page instance, not a raw dict"

        # Verify ALL spans in this page are Span instances (handle multiple objects)
        for span_idx, span in enumerate(page.spans):
            assert isinstance(span, pdftract.Span), \
                f"doc.pages[{page_idx}].spans[{span_idx}] should be Span instance, got {type(span).__name__}"
            assert not isinstance(span, dict), \
                f"doc.pages[{page_idx}].spans[{span_idx}] should be typed Span instance, not a raw dict"


def test_type_assertions_from_fixture_data(fixture_data: dict[str, Any]) -> None:
    """Test type assertions using loaded fixture data.

    This test validates that we can verify type contracts directly from
    fixture data without needing to extract a PDF. It exercises the
    fixture loading mechanism and validates type assertions against
    real parsed data.

    Args:
        fixture_data: Loaded fixture data containing parsed PDF results.
    """
    # Verify fixture contains expected top-level structure
    assert "schema_version" in fixture_data, "Fixture should contain schema_version"
    assert "pages" in fixture_data, "Fixture should contain pages"
    assert "metadata" in fixture_data, "Fixture should contain metadata"

    # Verify metadata type assertions
    metadata = fixture_data["metadata"]
    assert isinstance(metadata, dict), "metadata should be a dict"
    assert "page_count" in metadata, "metadata should contain page_count"
    assert isinstance(metadata["page_count"], int), \
        f"page_count should be int, got {type(metadata['page_count']).__name__}"

    # Verify pages is a list
    pages = fixture_data["pages"]
    assert isinstance(pages, list), "pages should be a list"

    # Verify each page has expected structure (if pages exist)
    for i, page in enumerate(pages):
        assert isinstance(page, dict), f"page {i} should be a dict"

        # Check for expected page-level keys
        expected_page_keys = ["width", "height", "blocks"]
        for key in expected_page_keys:
            if key in page:
                # Verify blocks is a list if present
                if key == "blocks":
                    assert isinstance(page[key], list), \
                        f"page {i} blocks should be a list"

    # Verify other list fields are properly typed
    for list_field in ["attachments", "form_fields", "links", "signatures"]:
        if list_field in fixture_data:
            assert isinstance(fixture_data[list_field], list), \
                f"{list_field} should be a list"


def test_page_type_assertion_from_document() -> None:
    """Test that Page objects accessed from Document are properly typed.

    This test verifies that Page objects returned from Document are the correct type.
    It is the first level of nested type verification, ensuring that when you access
    doc.pages, each Page object is a properly typed Page instance, not a raw dict.

    The test uses descriptive error messages that clearly indicate what type was
    expected vs received, making debugging easier if type assertions fail.
    """
    fixture_path = Path(__file__).parent.parent.parent.parent / "tests" / "fixtures" / "encrypted" / "EC-04-rc4-encrypted.expected.json"

    if not fixture_path.exists():
        pytest.skip(f"Fixture file not found: {fixture_path}")

    with fixture_path.open("r") as f:
        fixture_data = json.load(f)

    # Create Document from fixture data
    doc = pdftract.Document.from_native(fixture_data)

    # Verify Document is properly typed first
    assert isinstance(doc, pdftract.Document), \
        f"Expected Document type, got {type(doc).__name__}"

    # Access Page objects from Document result
    pages = doc.pages

    # Verify pages is a list
    assert isinstance(pages, list), \
        f"doc.pages should be a list, got {type(pages).__name__}"

    # Verify document has at least one page
    assert len(pages) > 0, \
        f"Document should contain at least one page, got {len(pages)} pages"

    # Add isinstance() assertion for Page type with descriptive error message
    for page_idx, page in enumerate(pages):
        assert isinstance(page, pdftract.Page), \
            f"Expected Page type for doc.pages[{page_idx}], got {type(page).__name__}"
        assert not isinstance(page, dict), \
            f"doc.pages[{page_idx}] should be a typed Page instance, not a raw dict"


def test_span_type_assertion(fixture_data: dict[str, Any]) -> None:
    """Test that span objects within Pages are properly typed.

    This test verifies that Span objects accessed from a Page are
    properly typed Span instances, not raw dicts. It handles empty
    spans gracefully and uses a clear error message format.

    Args:
        fixture_data: Loaded fixture data containing parsed PDF results.
    """
    # Create Document from fixture data
    doc = pdftract.Document.from_native(fixture_data)

    # ASSERTION 1: Verify Document has pages (basic structure validation)
    assert len(doc.pages) > 0, \
        f"Document should have at least one page, got {len(doc.pages)} pages"

    # ASSERTION 2: Verify Document is properly typed (not raw dict)
    assert isinstance(doc, pdftract.Document), \
        f"Expected Document type, got {type(doc).__name__}"

    # ASSERTION 3: Verify Document is not a raw dict
    assert not isinstance(doc, dict), \
        "Document should be a typed Document instance, not a raw dict"

    # Access the first page
    page = doc.pages[0]

    # ASSERTION 4: Verify the page is a Page instance with detailed error
    assert isinstance(page, pdftract.Page), \
        f"Expected Page type for doc.pages[0], got {type(page).__name__}"

    # ASSERTION 5: Verify page is not a raw dict
    assert not isinstance(page, dict), \
        "Page should be a typed Page instance, not a raw dict"

    # Access spans from the page
    spans = page.spans

    # ASSERTION 6: Verify spans is a sequence (list or tuple)
    assert isinstance(spans, (list, tuple)), \
        f"page.spans should be a list or tuple, got {type(spans).__name__}"

    # Handle empty spans case gracefully - this is acceptable
    if len(spans) == 0:
        pytest.skip("No spans found in first page - cannot test span types")

    # ASSERTION 7: Verify spans list is not empty for thorough testing
    assert len(spans) > 0, \
        f"Expected at least one span for thorough testing, got {len(spans)} spans"

    # Check each span is properly typed with detailed error messages
    for span_idx, span in enumerate(spans):
        # ASSERTION 8+: Verify each span is a Span instance (not raw dict)
        assert isinstance(span, pdftract.Span), \
            f"page.spans[{span_idx}] should be Span instance, got {type(span).__name__}"
        assert not isinstance(span, dict), \
            f"page.spans[{span_idx}] should be typed Span instance, not raw dict"
