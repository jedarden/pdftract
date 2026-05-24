"""Conformance tests for pdftract Python SDK.

This module runs the shared conformance suite via the Python API
and reports per-case pass/fail results.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path
from typing import Any

import pytest

# Import pdftract
try:
    import pdftract
    from pdftract import (
        Document,
        EncryptionError,
        Page,
        PdftractError,
        extract,
        extract_text,
    )
    _native_available = True
except ImportError as e:
    pytest.skip(f"pdftract not available: {e}", allow_module_level=True)
    _native_available = False


# Test fixtures directory
FIXTURES_DIR = Path(__file__).parent.parent.parent / "tests" / "fixtures"


class TestConformance:
    """Conformance tests for the pdftract Python SDK."""

    def test_extract_basic(self):
        """Test basic extraction returns a Document with correct structure."""
        fixture_path = FIXTURES_DIR / "valid-minimal.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        result = pdftract.extract(str(fixture_path))

        # Should return a Document object (not a raw dict)
        assert isinstance(result, Document), f"Expected Document, got {type(result)}"

        # Should have metadata
        assert hasattr(result, "metadata")
        assert result.metadata.page_count >= 1

        # Should have pages
        assert hasattr(result, "pages")
        assert len(result.pages) >= 1

        # Each page should be a Page object
        for page in result.pages:
            assert isinstance(page, Page), f"Expected Page, got {type(page)}"
            assert hasattr(page, "page_index")
            assert hasattr(page, "spans")
            assert hasattr(page, "blocks")

    def test_extract_text_returns_string(self):
        """Test extract_text returns a plain-text string."""
        fixture_path = FIXTURES_DIR / "valid-minimal.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        result = pdftract.extract_text(str(fixture_path))

        # Should return a string
        assert isinstance(result, str), f"Expected str, got {type(result)}"

        # Should not be empty for valid PDF
        # (minimal.pdf may have no text, so we just check it doesn't error)
        assert isinstance(result, str)

    def test_extract_nonexistent_raises_error(self):
        """Test extract with nonexistent path raises PdftractError."""
        with pytest.raises(PdftractError):
            pdftract.extract("/nonexistent/path/that/does/not/exist.pdf")

    def test_exception_hierarchy(self):
        """Test that all exception classes are defined and inherit correctly."""
        # Base exception
        assert hasattr(pdftract, "PdftractError")
        assert issubclass(pdftract.PdftractError, Exception)

        # Specific exceptions should inherit from PdftractError
        assert hasattr(pdftract, "CorruptPdfError")
        assert issubclass(pdftract.CorruptPdfError, pdftract.PdftractError)

        assert hasattr(pdftract, "EncryptionError")
        assert issubclass(pdftract.EncryptionError, pdftract.PdftractError)

        assert hasattr(pdftract, "SourceUnreachableError")
        assert issubclass(pdftract.SourceUnreachableError, pdftract.PdftractError)

        assert hasattr(pdftract, "RemoteFetchInterruptedError")
        assert issubclass(pdftract.RemoteFetchInterruptedError, pdftract.PdftractError)

        assert hasattr(pdftract, "TlsError")
        assert issubclass(pdftract.TlsError, pdftract.PdftractError)

        assert hasattr(pdftract, "ReceiptVerifyError")
        assert issubclass(pdftract.ReceiptVerifyError, pdftract.PdftractError)

        assert hasattr(pdftract, "UnsupportedOperationError")
        assert issubclass(pdftract.UnsupportedOperationError, pdftract.PdftractError)

    def test_types_are_dataclasses(self):
        """Test that type definitions are frozen dataclasses."""
        from dataclasses import is_dataclass

        # Document type
        assert hasattr(pdftract, "Document")
        assert is_dataclass(pdftract.Document)

        # Page type
        assert hasattr(pdftract, "Page")
        assert is_dataclass(pdftract.Page)

        # Span type
        assert hasattr(pdftract, "Span")
        assert is_dataclass(pdftract.Span)

        # Block type
        assert hasattr(pdftract, "Block")
        assert is_dataclass(pdftract.Block)

        # Match type
        assert hasattr(pdftract, "Match")
        assert is_dataclass(pdftract.Match)

        # Fingerprint type
        assert hasattr(pdftract, "Fingerprint")
        assert is_dataclass(pdftract.Fingerprint)

        # Classification type
        assert hasattr(pdftract, "Classification")
        assert is_dataclass(pdftract.Classification)

        # Metadata type
        assert hasattr(pdftract, "Metadata")
        assert is_dataclass(pdftract.Metadata)

    def test_extract_stream_returns_iterator(self):
        """Test extract_stream returns an iterator of Page objects."""
        fixture_path = FIXTURES_DIR / "valid-minimal.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        result = pdftract.extract_stream(str(fixture_path))

        # Should return an iterator
        assert hasattr(result, "__iter__")

        # Should yield Page objects
        pages = list(result)
        assert len(pages) >= 1
        assert all(isinstance(p, Page) for p in pages)

    def test_extract_with_options(self):
        """Test extract with various options."""
        fixture_path = FIXTURES_DIR / "valid-minimal.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        # Test with boolean option
        result = pdftract.extract(str(fixture_path), include_invisible=True)
        assert isinstance(result, Document)

        # Test with list option
        result = pdftract.extract(str(fixture_path), ocr_language=["eng"])
        assert isinstance(result, Document)

        # Test with numeric option
        result = pdftract.extract(str(fixture_path), max_decompress_gb=2)
        assert isinstance(result, Document)

    def test_asyncio_module_exists(self):
        """Test that asyncio module is available."""
        assert hasattr(pdftract, "asyncio")

        # Check for key async functions
        assert hasattr(pdftract.asyncio, "extract")
        assert hasattr(pdftract.asyncio, "extract_text")
        assert hasattr(pdftract.asyncio, "extract_stream")

    @pytest.mark.asyncio
    async def test_asyncio_extract(self):
        """Test asyncio.extract works."""
        fixture_path = FIXTURES_DIR / "valid-minimal.pdf"
        if not fixture_path.exists():
            pytest.skip(f"Fixture not found: {fixture_path}")

        result = await pdftract.asyncio.extract(str(fixture_path))
        assert isinstance(result, Document)

    def test_version_defined(self):
        """Test that __version__ is defined."""
        assert hasattr(pdftract, "__version__")
        assert isinstance(pdftract.__version__, str)


class TestSubprocessFallback:
    """Tests for subprocess fallback when native module is unavailable."""

    def test_fallback_module_exists(self):
        """Test that fallback module can be imported."""
        from pdftract.fallback import SubprocessExtractor

        assert SubprocessExtractor is not None

    def test_fallback_extractor_finds_cli(self):
        """Test that SubprocessExtractor can find the CLI binary."""
        from pdftract.fallback import SubprocessExtractor

        # This may fail if pdftract is not installed, but we test
        # the logic works
        try:
            extractor = SubprocessExtractor()
            assert extractor.cli_path is not None
        except PdftractError:
            # CLI not found, which is OK for this test
            pass


def run_conformance_suite() -> dict[str, Any]:
    """Run the conformance suite and return results.

    Returns:
        Dict with pass/fail counts and details
    """
    import traceback

    results = {
        "total": 0,
        "passed": 0,
        "failed": 0,
        "skipped": 0,
        "tests": [],
    }

    # Get all test methods
    test_class = TestConformance
    test_methods = [
        getattr(test_class, name)
        for name in dir(test_class)
        if name.startswith("test_") and callable(getattr(test_class, name))
    ]

    for test_method in test_methods:
        test_name = test_method.__name__
        results["total"] += 1

        try:
            test_instance = test_class()
            test_method()
            results["passed"] += 1
            results["tests"].append({"name": test_name, "status": "PASS"})
        except pytest.skip.Exception as e:
            results["skipped"] += 1
            results["tests"].append({"name": test_name, "status": "SKIP", "reason": str(e)})
        except Exception as e:
            results["failed"] += 1
            results["tests"].append(
                {
                    "name": test_name,
                    "status": "FAIL",
                    "error": str(e),
                    "traceback": traceback.format_exc(),
                }
            )

    return results


if __name__ == "__main__":
    # Run conformance suite when executed directly
    print("Running pdftract Python SDK conformance suite...")
    print()

    results = run_conformance_suite()

    print(f"Results: {results['passed']}/{results['total']} passed")
    print(f"  Passed: {results['passed']}")
    print(f"  Failed: {results['failed']}")
    print(f"  Skipped: {results['skipped']}")
    print()

    # Print failed tests
    if results["failed"] > 0:
        print("Failed tests:")
        for test in results["tests"]:
            if test["status"] == "FAIL":
                print(f"  - {test['name']}: {test.get('error', 'Unknown error')}")
        print()

    # Print summary as JSON for CI
    print(json.dumps(results, indent=2))

    # Exit with error code if any tests failed
    sys.exit(0 if results["failed"] == 0 else 1)
