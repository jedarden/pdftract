"""Regression tests for the Python Markdown extraction entry point."""

from __future__ import annotations

import re
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

try:
    import pdftract
except ImportError as exc:
    pdftract = None
    _NATIVE_IMPORT_ERROR = f"{type(exc).__name__}: {exc}"
else:
    _NATIVE_IMPORT_ERROR = None


_requires_native = pytest.mark.skipif(
    pdftract is None or not getattr(pdftract, "_native_available", False),
    reason=f"pdftract native bindings unavailable ({_NATIVE_IMPORT_ERROR})",
)


@_requires_native
def test_extract_markdown_is_not_plain_text() -> None:
    """Pin extract_markdown() to Markdown output rather than plain text."""

    document = Path(__file__).parent / "fixtures" / "markdown_verification.pdf"
    if not document.exists():
        pytest.skip(f"PDF fixture not found: {document}")

    markdown = pdftract.extract_markdown(str(document))
    text = pdftract.extract_text(str(document))

    assert markdown != text, (
        "extract_markdown() must not silently regress to extract_text() output"
    )
    assert (
        re.search(r"(?m)^#{1,6}\s+\S", markdown)
        or re.search(r"\[[^\]]+\]\(https?://[^)]+\)", markdown)
    ), "Markdown output should contain a heading or a link"
