"""Integration tests for pdftract.search() function.

This module contains failing tests that demonstrate the current bug where
search() returns empty matches even when the pattern exists in the PDF.

These tests are the TDD "red" phase - they fail now, and will pass after
the search() function is fixed.
"""

from __future__ import annotations

import pytest
from pathlib import Path

try:
    import pdftract
    _native_available = True
except ImportError as e:
    pytest.skip(f"pdftract not available: {e}", allow_module_level=True)
    _native_available = False


# Test fixtures directory
FIXTURES_DIR = Path(__file__).parent.parent.parent.parent / "tests" / "fixtures"


class TestSearchIntegration:
    """Integration tests for search() function bug."""

    def test_search_empty_result_when_pattern_present(self):
        """Test that search() returns non-empty matches when pattern exists.

        This test FAILS because search() currently returns an empty iterator
        even when the pattern is clearly present in the PDF.

        Expected: search() should yield at least one match
        Actual: search() yields no matches (empty iterator)
        """
        # Use a synthetic PDF that has known text content
        fixture_path = FIXTURES_DIR / "grep-corpus" / "corpus" / "synthetic_10.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        # Search for "text" which should appear in the synthetic PDF
        matches = list(pdftract.search(str(fixture_path), "text"))

        # THE BUG: matches list is empty even though "text" exists in the PDF
        # This assertion FAILS - demonstrating the bug
        assert len(matches) > 0, (
            f"search() should return non-empty matches when pattern exists. "
            f"Pattern: 'text', "
            f"Matches found: {len(matches)}, "
            f"Expected: at least 1 match"
        )

    def test_search_returns_match_structure(self):
        """Test that each match has the correct structure.

        This test FAILS because the matches list is empty, so we can't verify
        the structure of individual match objects.

        Expected: each match should have page_index, span_index, text, bbox
        Actual: matches list is empty, can't verify structure
        """
        fixture_path = FIXTURES_DIR / "valid-minimal.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        # Search for a common pattern
        matches = list(pdftract.search(str(fixture_path), "test"))

        # If we have matches, verify their structure
        # THE BUG: this loop doesn't execute because matches is empty
        for match in matches:
            assert hasattr(match, "page_index"), "Match should have 'page_index'"
            assert hasattr(match, "span_index"), "Match should have 'span_index'"
            assert hasattr(match, "text"), "Match should have 'text'"
            assert hasattr(match, "bbox"), "Match should have 'bbox'"

            # Verify types
            assert isinstance(match.page_index, int), "page_index should be int"
            assert isinstance(match.span_index, int), "span_index should be int"
            assert isinstance(match.text, str), "text should be str"
            assert isinstance(match.bbox, list), "bbox should be a list"
            assert len(match.bbox) == 4, "bbox should have 4 elements [x0, y0, x1, y1]"
            assert all(isinstance(x, (int, float)) for x in match.bbox), (
                "bbox elements should be numeric"
            )

    def test_search_with_case_insensitive(self):
        """Test search with case_insensitive option.

        This test FAILS because search() returns empty matches regardless of
        the case_insensitive option.
        """
        fixture_path = FIXTURES_DIR / "grep-corpus" / "corpus" / "synthetic_100.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        # Search with different case
        matches = list(pdftract.search(str(fixture_path), "TEXT", case_insensitive=True))

        # Should find matches (case-insensitive)
        assert len(matches) > 0, (
            f"case_insensitive search should find 'TEXT' matching 'text'. "
            f"Matches found: {len(matches)}"
        )

    def test_search_pattern_field_set_correctly(self):
        """Test that search() returns matches with the pattern field set correctly.

        This test FAILS because search() returns an empty iterator even though
        the pattern is clearly present in the PDF.

        Expected: search() should yield matches for the pattern
        Actual: search() yields no matches (empty iterator)
        """
        fixture_path = FIXTURES_DIR / "valid-minimal.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        test_pattern = "sample"
        matches = list(pdftract.search(str(fixture_path), test_pattern))

        # THE BUG: matches should be populated for a pattern that exists
        assert len(matches) > 0, (
            f"search() should return matches for pattern '{test_pattern}'. "
            f"Matches found: {len(matches)}"
        )


if __name__ == "__main__":
    # Run tests with verbose output
    pytest.main([__file__, "-v", "-s"])
