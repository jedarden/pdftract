#!/usr/bin/env python3
"""Smoke test for the Python SDK subprocess fallback.

The native extension is blocked through the import machinery, so this test
does not rename or modify an installed extension in the shared checkout.
"""

import importlib.abc
import sys
from pathlib import Path


class _BlockNative(importlib.abc.MetaPathFinder):
    """Force the same ImportError path used by an unavailable native wheel."""

    def find_spec(self, fullname, path=None, target=None):
        if fullname == "pdftract._native":
            raise ImportError("forced fallback smoke-test ImportError")
        return None


def test_fallback_smoke():
    """Import through the fallback and exercise its typed public API."""

    original_modules = {
        name: module
        for name, module in sys.modules.items()
        if name == "pdftract" or name.startswith("pdftract.")
    }
    original_path = list(sys.path)
    original_meta_path = list(sys.meta_path)

    try:
        for module in list(sys.modules):
            if module == "pdftract" or module.startswith("pdftract."):
                del sys.modules[module]
        sys.meta_path.insert(0, _BlockNative())
        sys.path.insert(0, str(Path(__file__).parent / "python"))

        import pdftract
        from pdftract.types import Document, Match, Metadata, Page

        assert pdftract._using_fallback is True
        assert pdftract._native_available is False

        test_pdf = (
            Path(__file__).parents[2]
            / "crates"
            / "pdftract-core"
            / "tests"
            / "fixtures"
            / "test-minimal.pdf"
        )
        assert test_pdf.exists(), f"Test fixture not found: {test_pdf}"

        document = pdftract.extract(str(test_pdf))
        assert isinstance(document, Document)
        assert document.pages and isinstance(document.pages[0], Page)

        text = pdftract.extract_text(str(test_pdf))
        assert "Dummy PDF file" in text

        pages = list(pdftract.extract_stream(str(test_pdf)))
        assert pages and all(isinstance(page, Page) for page in pages)

        matches = list(pdftract.search(str(test_pdf), "Dummy"))
        assert matches and all(isinstance(match, Match) for match in matches)

        metadata = pdftract.get_metadata(str(test_pdf))
        assert isinstance(metadata, Metadata) and metadata.page_count == 1

        fingerprint = pdftract.hash(str(test_pdf))
        assert fingerprint.hash.startswith("pdftract-v1:")

        print("Smoke test PASSED: fallback import and API round trips work")
    finally:
        sys.path[:] = original_path
        sys.meta_path[:] = original_meta_path
        for module in list(sys.modules):
            if module == "pdftract" or module.startswith("pdftract."):
                del sys.modules[module]
        sys.modules.update(original_modules)

if __name__ == "__main__":
    test_fallback_smoke()
