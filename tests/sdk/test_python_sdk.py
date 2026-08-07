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

    TODO: Add assertions to verify:
    - extract() returns Document instance
    - Document.pages contains Page instances
    - Page.spans contains Span instances
    - No raw dicts are returned to the caller
    """
    # Load fixture PDF and extract with the SDK
    fixture_path = Path(__file__).parent.parent.parent / "tests" / "fixtures" / "markdown_structure.pdf"
    doc = pdftract.extract(str(fixture_path))

    # First type assertion: verify extract() returns Document type
    assert isinstance(doc, Document), \
        f"Expected Document type, got {type(doc)}"
