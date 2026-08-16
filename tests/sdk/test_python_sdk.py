"""Test Python SDK type assertions.

This test module verifies that the pdftract Python SDK returns properly
typed objects (Document, Page, Span) by exercising the SDK against
real fixture data and validating type contracts.
"""

from __future__ import annotations

from pathlib import Path

import pytest

# Add the python package to the path
import sys
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "crates" / "pdftract-py" / "python"))

import pdftract
from pdftract import Document, Page, Span


def test_python_sdk_types() -> None:
    """Verify Python SDK returns properly typed objects.

    This test validates that the pdftract SDK's extract() function
    returns typed Document, Page, and Span objects rather than raw
    dictionaries, ensuring IDE autocomplete and type checking work
    correctly.
    """
    # Load fixture PDF and extract with the SDK
    fixture_path = Path(__file__).parent.parent.parent / "tests" / "fixtures" / "markdown_structure.pdf"
    doc = pdftract.extract(str(fixture_path))

    # First type assertion: verify extract() returns Document type
    assert isinstance(doc, Document), \
        f"Expected Document type, got {type(doc)}"

    # Verify document has pages
    assert hasattr(doc, "pages"), "Document should have pages attribute"

    # Verify doc.pages is not empty
    assert doc.pages, "Document should contain pages"

    # Type assertion: verify first page is Page instance
    assert isinstance(doc.pages[0], Page), \
        f"Expected Page type for doc.pages[0], got {type(doc.pages[0]).__name__}"
    # Iterate through pages and verify Page attributes
    for page in doc.pages:
        # Verify page has spans attribute
        assert hasattr(page, "spans"), "Page should have spans attribute"

        # Verify page has width attribute
        assert hasattr(page, "width"), "Page should have width attribute"

        # Verify page has height attribute
        assert hasattr(page, "height"), "Page should have height attribute"
