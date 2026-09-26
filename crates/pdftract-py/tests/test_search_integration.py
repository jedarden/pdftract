"""Focused smoke test for the compiled ``pdftract.search`` binding."""

from __future__ import annotations

import json
from pathlib import Path

import pdftract
import pytest


FIXTURE = (
    Path(__file__).with_name("fixtures") / "search_regression.pdf"
)
PATTERN = "PYTHON_SEARCH_REGRESSION"
EXPECTED = FIXTURE.with_suffix(".expected.json")


def assert_search_matches(pattern: str, expected_texts: list[str], **options) -> None:
    """Assert the raw native result and typed public iterator agree."""
    native_result = pdftract._native.search(str(FIXTURE), pattern, **options)
    assert native_result["pattern"] == pattern
    assert [match["text"] for match in native_result["matches"]] == expected_texts

    public_matches = list(pdftract.search(str(FIXTURE), pattern, **options))
    assert [match.text for match in public_matches] == expected_texts
    assert all(isinstance(match, pdftract.Match) for match in public_matches)


def test_search_result_shape_matches_sdk():
    """Public search returns the compiled SDK result, including its location."""
    assert FIXTURE.is_file()
    assert EXPECTED.is_file()
    assert pdftract._native_available, "the focused command must build the native binding"
    assert not pdftract._using_fallback, "the focused harness must not use the CLI fallback"

    expected = json.loads(EXPECTED.read_text())
    assert expected["pattern"] == PATTERN
    sdk_matches = expected["matches"]
    assert sdk_matches, "the SDK contract fixture must contain a known match"

    native_result = pdftract._native.search(str(FIXTURE), PATTERN)
    assert native_result["pattern"] == PATTERN
    native_matches = native_result["matches"]
    assert isinstance(native_matches, list)
    assert native_matches, "a known string in the fixture must produce matches"

    for index, match in enumerate(native_matches):
        assert {
            "page_index",
            "span_index",
            "text",
            "bbox",
        } <= match.keys(), f"match {index} is missing search result fields: {match}"
        assert isinstance(match["page_index"], int) and match["page_index"] >= 0
        assert isinstance(match["span_index"], int) and match["span_index"] >= 0
        assert isinstance(match["text"], str) and match["text"]
        assert (
            isinstance(match["bbox"], list)
            and len(match["bbox"]) == 4
            and all(isinstance(value, (int, float)) for value in match["bbox"])
        ), f"match {index} has invalid bbox data: {match}"

    # The checked-in result is the sdk::search output for this deterministic
    # fixture. Exact comparison catches field loss, reordered/extra results,
    # and fabricated coordinates, not just an empty-list regression.
    assert native_matches == sdk_matches

    public_matches = list(pdftract.search(str(FIXTURE), PATTERN))
    assert public_matches, "public pdftract.search() must surface native matches"
    assert len(public_matches) == len(sdk_matches)
    for public_match, sdk_match in zip(public_matches, sdk_matches):
        assert isinstance(public_match, pdftract.Match)
        assert public_match.page == sdk_match["page_index"]
        assert public_match.text == sdk_match["text"]
        assert list(public_match.bbox) == sdk_match["bbox"]


def test_search_non_matching_pattern_is_empty():
    """A pattern absent from the fixture must not produce a false match."""
    non_matching_pattern = "PATTERN_NOT_PRESENT_IN_FIXTURE"

    native_result = pdftract._native.search(str(FIXTURE), non_matching_pattern)
    assert native_result["pattern"] == non_matching_pattern
    assert native_result["matches"] == []

    assert list(pdftract.search(str(FIXTURE), non_matching_pattern)) == []


def test_search_options_cover_plain_case_regex_and_whole_word_matching():
    """Every search option must preserve matching and near-matching semantics."""
    # Plain text matching is case-sensitive by default.
    assert_search_matches(PATTERN, [PATTERN])
    assert_search_matches(PATTERN.lower(), [])

    # Case-insensitive matching finds the same known span.
    assert_search_matches(PATTERN.lower(), [PATTERN], case_insensitive=True)

    # The regex must match the complete known token; the trailing-X variant
    # is a near match and must not be accepted by the end anchor.
    assert_search_matches(r"PYTHON_SEARCH_[A-Z]+$", [PATTERN], regex=True)
    assert_search_matches(r"PYTHON_SEARCH_[A-Z]+X$", [], regex=True)

    # Underscores are word characters: SEARCH is only a substring of the
    # fixture token, while the complete token is a whole-word match.
    assert_search_matches(PATTERN, [PATTERN], whole_word=True)
    assert_search_matches("SEARCH", [], whole_word=True)


def test_search_invalid_regex_raises_python_error():
    """Invalid regex input is surfaced consistently on both Python APIs."""
    with pytest.raises(pdftract.CorruptPdfError, match="Invalid regex pattern"):
        pdftract._native.search(str(FIXTURE), "[", regex=True)

    with pytest.raises(pdftract.CorruptPdfError, match="Invalid regex pattern"):
        list(pdftract.search(str(FIXTURE), "[", regex=True))
