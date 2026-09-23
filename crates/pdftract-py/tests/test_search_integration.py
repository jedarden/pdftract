"""Integration tests for pdftract.search() function.

These tests exercise the end-to-end search contract:

* the native PyO3 ``search()`` returns ``{"pattern": ..., "matches": [...]}``
  where each match carries ``page_index``, ``span_index``, ``text`` and
  ``bbox`` (mirroring ``pdftract_core::sdk::search`` output), and
* the typed ``pdftract.search()`` wrapper yields non-empty ``Match`` objects
  mapped from that result.
"""

from __future__ import annotations

import pytest
from pathlib import Path

try:
    import pdftract
    from pdftract import _native
    _native_available = True
except ImportError as e:
    pytest.skip(f"pdftract not available: {e}", allow_module_level=True)
    _native_available = False


# Test fixtures directory
FIXTURES_DIR = Path(__file__).parent.parent.parent.parent / "tests" / "fixtures"


class TestSearchIntegration:
    """End-to-end tests for the search() contract."""

    def test_native_search_finds_pattern_present_in_pdf(self):
        """Native search() returns non-empty matches for a present pattern."""
        fixture_path = FIXTURES_DIR / "grep-corpus" / "corpus" / "synthetic_10.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        # The synthetic grep-corpus pages contain lorem-ipsum placeholder
        # text; "ipsum" is present in every document, "text" is not.
        result = _native.search(str(fixture_path), "ipsum")

        assert isinstance(result, dict), "native search() should return a dict"
        assert result["pattern"] == "ipsum", "result should echo the pattern"
        matches = result["matches"]
        assert len(matches) > 0, (
            "native search() should return non-empty matches when the "
            f"pattern exists; got {len(matches)} matches"
        )

    def test_native_match_structure_carries_page_span_bbox(self):
        """Each native match carries page_index, span_index, text and bbox."""
        fixture_path = FIXTURES_DIR / "grep-corpus" / "corpus" / "synthetic_10.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        matches = _native.search(str(fixture_path), "ipsum")["matches"]
        assert matches, "expected non-empty matches"

        for match in matches:
            assert set(match) == {"page_index", "span_index", "text", "bbox"}, (
                "native search matches should expose exactly the sdk::SearchMatch fields"
            )
            assert isinstance(match["page_index"], int), "page_index should be int"
            assert isinstance(match["span_index"], int), "span_index should be int"
            assert isinstance(match["text"], str) and match["text"], (
                "text should be a non-empty str"
            )
            assert isinstance(match["bbox"], list), "bbox should be a list"
            assert len(match["bbox"]) == 4, "bbox should have 4 elements [x0, y0, x1, y1]"
            assert all(isinstance(x, (int, float)) for x in match["bbox"]), (
                "bbox elements should be numeric"
            )

    def test_typed_search_yields_match_objects(self):
        """Typed pdftract.search() maps native matches to Match objects."""
        fixture_path = FIXTURES_DIR / "grep-corpus" / "corpus" / "synthetic_10.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        matches = list(pdftract.search(str(fixture_path), "ipsum"))
        assert len(matches) > 0, "typed search() should yield matches"

        for match in matches:
            assert isinstance(match, pdftract.Match), "should yield typed Match objects"
            assert isinstance(match.page, int) and match.page >= 0, (
                "page should be a non-negative int"
            )
            assert isinstance(match.text, str) and match.text, "text should be non-empty"
            assert len(match.bbox) == 4, "bbox should have 4 elements [x0, y0, x1, y1]"
            assert all(isinstance(x, (int, float)) for x in match.bbox), (
                "bbox elements should be numeric"
            )

    def test_typed_matches_consistent_with_native_result(self):
        """Typed matches mirror the native sdk::search result one-to-one."""
        fixture_path = FIXTURES_DIR / "grep-corpus" / "corpus" / "synthetic_10.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        native_matches = _native.search(str(fixture_path), "ipsum")["matches"]
        typed_matches = list(pdftract.search(str(fixture_path), "ipsum"))

        assert len(typed_matches) == len(native_matches), (
            "typed search() should yield exactly as many matches as native search()"
        )
        for native_match, typed_match in zip(native_matches, typed_matches):
            assert typed_match.text == native_match["text"]
            assert typed_match.page == native_match["page_index"]
            assert list(typed_match.bbox) == list(native_match["bbox"])

    def test_search_with_case_insensitive(self):
        """case_insensitive=True matches regardless of letter case."""
        fixture_path = FIXTURES_DIR / "grep-corpus" / "corpus" / "synthetic_100.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        # Fixture pages say "Lorem ipsum ...", so upper-case LOREM only
        # matches when case_insensitive=True actually reaches the matcher.
        matches = list(pdftract.search(str(fixture_path), "LOREM", case_insensitive=True))
        assert len(matches) > 0, (
            f"case_insensitive search should find 'LOREM' matching 'Lorem'; "
            f"got {len(matches)} matches"
        )

    def test_search_whole_word_and_regex_kwargs_pass_through(self):
        """whole_word and regex kwargs pass through to the sdk matcher."""
        fixture_path = FIXTURES_DIR / "grep-corpus" / "corpus" / "synthetic_10.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        whole = _native.search(str(fixture_path), "ipsum", whole_word=True)
        assert len(whole["matches"]) > 0, (
            "whole_word=True should still match the standalone word 'ipsum'"
        )
        regexed = _native.search(str(fixture_path), "Lo.em", regex=True)
        assert len(regexed["matches"]) > 0, (
            "regex='Lo.em' should match 'Lorem' via the regex engine"
        )

    def test_search_pattern_field_set_correctly(self):
        """The result echoes the requested pattern."""
        fixture_path = FIXTURES_DIR / "grep-corpus" / "corpus" / "synthetic_10.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        test_pattern = "ipsum"
        result = _native.search(str(fixture_path), test_pattern)
        assert result["pattern"] == test_pattern


if __name__ == "__main__":
    # Run tests with verbose output
    pytest.main([__file__, "-v", "-s"])
