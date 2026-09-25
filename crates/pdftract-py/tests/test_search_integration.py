"""Focused smoke test for the compiled ``pdftract.search`` binding."""

from __future__ import annotations

from pathlib import Path

import pdftract


FIXTURE = (
    Path(__file__).with_name("fixtures") / "search_regression.pdf"
)
PATTERN = "PYTHON_SEARCH_REGRESSION"


def test_search_uses_compiled_binding():
    """The focused harness builds the extension before calling public search."""
    assert FIXTURE.is_file()
    assert pdftract._native_available, "the focused command must build the native binding"
    assert not pdftract._using_fallback, "the focused harness must not use the CLI fallback"

    # Consume the iterator so the binding is actually called. Detailed result
    # expectations belong to the follow-up regression test.
    matches = list(pdftract.search(str(FIXTURE), PATTERN))
    assert isinstance(matches, list)
