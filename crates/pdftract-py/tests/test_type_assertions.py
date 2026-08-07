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
    fixture_path = Path(__file__).parent.parent.parent / "tests" / "fixtures" / "encrypted" / "EC-04-rc4-encrypted.expected.json"

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
        f"extract() should return Document instance, got {type(doc).__name__}"

    # Verify it's not a raw dict
    assert not isinstance(doc, dict), "extract() should not return a raw dict"


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
            f"doc.pages[0] should be Page instance, got {type(page).__name__}"
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
