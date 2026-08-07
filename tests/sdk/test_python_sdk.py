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
    # TODO: Implement type assertions using fixture data
    # Next step: load fixture PDF and verify types
    pass
