"""Regression tests for the compiled ``pdftract.search`` binding."""

from __future__ import annotations

from pathlib import Path

import pytest

try:
    import pdftract
    from pdftract import _native
except ImportError as exc:
    pytest.skip(f"compiled pdftract binding is not installed: {exc}", allow_module_level=True)


FIXTURE = (
    Path(__file__).parents[3]
    / "crates"
    / "pdftract-py"
    / "tests"
    / "fixtures"
    / "markdown_verification.pdf"
)
PATTERN = "Markdown"


def test_search_finds_known_text_and_preserves_sdk_match_shape():
    """Search must not regress to an unconditional empty result."""
    assert pdftract._native_available, "this regression must use the compiled binding"
    assert not pdftract._using_fallback, "this regression must not use the CLI fallback"
    assert Path(_native.__file__).suffix in {".so", ".pyd", ".dylib"}

    native_result = _native.search(str(FIXTURE), PATTERN)
    native_matches = native_result["matches"]
    assert native_result["pattern"] == PATTERN
    assert native_matches, (
        f"compiled _native.search() returned no matches for {PATTERN!r} in {FIXTURE}"
    )

    for match in native_matches:
        assert set(match) == {"page_index", "span_index", "text", "bbox"}
        assert isinstance(match["page_index"], int) and match["page_index"] >= 0
        assert isinstance(match["span_index"], int) and match["span_index"] >= 0
        assert isinstance(match["text"], str) and PATTERN in match["text"]
        assert isinstance(match["bbox"], list)
        assert len(match["bbox"]) == 4
        assert all(isinstance(value, (int, float)) for value in match["bbox"])

    public_matches = list(pdftract.search(str(FIXTURE), PATTERN))
    assert public_matches, (
        f"pdftract.search() returned no matches for {PATTERN!r} in {FIXTURE}"
    )
    assert len(public_matches) == len(native_matches)

    for native_match, public_match in zip(native_matches, public_matches):
        assert public_match.page == native_match["page_index"]
        assert public_match.text == native_match["text"]
        assert list(public_match.bbox) == native_match["bbox"]
