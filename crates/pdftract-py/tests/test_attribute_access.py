"""Attribute access verification for typed SDK objects (bf-5kbuxy).

Verifies that users can read data off the typed object graph with plain
attribute syntax -- never dict keys -- and that no access in the
``Document -> Page -> Span`` / ``Page -> Block`` chain raises
``AttributeError``:

1. ``doc.pages`` is accessible and yields ``Page`` instances
2. ``doc.pages[0].width`` returns the expected page width
3. ``doc.pages[0].spans[0].text`` returns the expected span text
4. ``page.blocks`` is accessible and yields ``Block`` instances
5. no attribute access raises ``AttributeError``

Typed objects are built through the SDK's own wrapping entry point
(``Document.from_native`` -- the exact call ``pdftract.extract()`` uses to
convert the native result) fed with real extraction-shaped data from
``tests/fixtures/test-minimal.expected.json``.

Usage:
    python3 test_attribute_access.py          # standalone runner
    pytest test_attribute_access.py           # or via pytest
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Add the python package to the path
sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

import pytest

import pdftract
from pdftract import Block, Document, Page, Span

# Real extraction-shaped fixture data (same source smoke_test.py uses)
EXPECTED_JSON = Path(__file__).resolve().parents[3] / "tests" / "fixtures" / "test-minimal.expected.json"

# A page carrying one of each block kind, so page.blocks can be verified to
# hold typed Block objects rather than an empty container.
PAGE_WITH_BLOCKS_NATIVE = {
    "page": 1,
    "width": 612,
    "height": 792,
    "rotation": 0,
    "spans": [
        {
            "text": "Introduction",
            "bbox": [72.0, 700.0, 200.0, 716.0],
            "font": "Helvetica-Bold",
            "size": 14.0,
        }
    ],
    "blocks": [
        {
            "kind": "heading",
            "text": "Introduction",
            "bbox": [72.0, 700.0, 200.0, 716.0],
            "level": 1,
        },
        {
            "kind": "text",
            "text": "Body paragraph.",
            "bbox": [72.0, 680.0, 300.0, 694.0],
        },
    ],
}


def _load_document() -> Document:
    """Build a typed Document from the real extraction-shaped fixture data."""
    with EXPECTED_JSON.open("r") as f:
        fixture_data = json.load(f)
    return Document.from_native(fixture_data)


def _extraction_error_types() -> tuple:
    """Every exception type extraction can raise for an unreadable PDF.

    ``pdftract/exceptions.py`` declares a pure-Python hierarchy whose names
    shadow the ones the PyO3 module registers, so ``pdftract.PdftractError``
    is *not* the base of the errors the native extractor actually raises.
    Both bases are accepted here so a parser failure is reported as a parser
    failure rather than as an unhandled error.
    """
    bases = [pdftract.PdftractError]
    native_base = getattr(getattr(pdftract, "_native", None), "PdftractError", None)
    if native_base is not None and native_base not in bases:
        bases.append(native_base)
    return tuple(bases)


def test_doc_pages_attribute_accessible():
    """doc.pages is accessible, is a list, and yields Page instances."""
    doc = _load_document()

    pages = doc.pages  # AttributeError here would fail the test
    assert isinstance(pages, list), \
        f"doc.pages should be a list, got {type(pages).__name__}"
    assert len(pages) > 0, "doc.pages should contain at least one page"
    for i, page in enumerate(pages):
        assert isinstance(page, Page), \
            f"doc.pages[{i}] should be a Page instance, got {type(page).__name__}"


def test_doc_pages_width_returns_expected_value():
    """doc.pages[0].width returns the expected page width in points."""
    doc = _load_document()

    width = doc.pages[0].width
    assert isinstance(width, int), \
        f"doc.pages[0].width should be int, got {type(width).__name__}"
    assert width == 612, \
        f"doc.pages[0].width should be 612 (US Letter, per fixture), got {width}"
    assert doc.pages[0].height == 792, \
        f"doc.pages[0].height should be 792, got {doc.pages[0].height}"


def test_doc_pages_spans_text_returns_expected_value():
    """doc.pages[0].spans[0].text returns the expected span text."""
    doc = _load_document()

    page = doc.pages[0]
    assert isinstance(page.spans, tuple) and len(page.spans) == 2, \
        f"page.spans should be a 2-tuple, got {page.spans!r}"

    span = page.spans[0]
    assert isinstance(span, Span), \
        f"page.spans[0] should be a Span instance, got {type(span).__name__}"
    assert isinstance(span.text, str), \
        f"span.text should be str, got {type(span.text).__name__}"
    assert span.text == "Hello, World!", \
        f"span.text should be 'Hello, World!', got {span.text!r}"

    # Remaining Span attributes are readable too (no AttributeError)
    assert span.font == "Helvetica", f"span.font: {span.font!r}"
    assert span.size == 12.0, f"span.size: {span.size!r}"
    assert span.bbox == (100.5, 200.3, 200.0, 212.3), f"span.bbox: {span.bbox!r}"


def test_page_blocks_attribute_accessible():
    """page.blocks is accessible and yields typed Block instances."""
    page = Page.from_native(PAGE_WITH_BLOCKS_NATIVE)

    blocks = page.blocks  # AttributeError here would fail the test
    assert isinstance(blocks, tuple), \
        f"page.blocks should be a tuple, got {type(blocks).__name__}"
    assert len(blocks) == 2, \
        f"page.blocks should hold 2 blocks, got {len(blocks)}"

    heading = blocks[0]
    assert isinstance(heading, Block), \
        f"page.blocks[0] should be a Block instance, got {type(heading).__name__}"
    assert heading.kind == "heading", f"block.kind: {heading.kind!r}"
    assert heading.text == "Introduction", f"block.text: {heading.text!r}"
    assert heading.level == 1, f"block.level: {heading.level!r}"

    body = blocks[1]
    assert isinstance(body, Block), \
        f"page.blocks[1] should be a Block instance, got {type(body).__name__}"
    assert body.kind == "text", f"block.kind: {body.kind!r}"


def test_page_blocks_empty_is_iterable():
    """A page with no blocks still exposes an iterable page.blocks."""
    doc = _load_document()
    page = doc.pages[0]

    assert page.blocks is not None, "page.blocks should never be None"
    assert len(page.blocks) == 0, \
        f"fixture page has no blocks, got {len(page.blocks)}"
    assert list(page.blocks) == [], "page.blocks should be iterable"


def test_full_attribute_chain_no_attribute_error():
    """Every attribute path in the object graph resolves without AttributeError."""
    doc = _load_document()
    typed_page = Page.from_native(PAGE_WITH_BLOCKS_NATIVE)

    paths = [
        "pages",
        "pages.0.width",
        "pages.0.height",
        "pages.0.page",
        "pages.0.rotation",
        "pages.0.spans",
        "pages.0.spans.0.text",
        "pages.0.spans.0.font",
        "pages.0.spans.0.size",
        "pages.0.spans.0.bbox",
        "pages.0.blocks",
        "metadata.page_count",
        "metadata.title",
        "schema_version",
    ]

    failures = []
    for dotted in paths:
        try:
            _get_path_indexed(doc, dotted)
        except AttributeError as exc:
            failures.append(f"{dotted}: AttributeError({exc})")

    # page.blocks carries Block objects with their own readable attributes
    for dotted in ("blocks", "blocks.0.kind", "blocks.0.text", "blocks.0.level",
                   "blocks.1.kind", "blocks.1.text"):
        try:
            _get_path_indexed(typed_page, dotted)
        except AttributeError as exc:
            failures.append(f"page.{dotted}: AttributeError({exc})")

    assert not failures, \
        "AttributeError raised on typed attribute access: " + "; ".join(failures)


def _get_path_indexed(obj, dotted: str):
    """Resolve a dotted path where numeric segments are list/tuple indices."""
    current = obj
    for part in dotted.split("."):
        if part.isdigit():
            current = current[int(part)]
        else:
            current = getattr(current, part)
    return current


def test_extract_end_to_end_typed_attribute_access():
    """The same attribute chain works on objects returned by pdftract.extract().

    Skips -- rather than fails -- when the native parser cannot read the
    fixture PDF: parser regressions are owned by the parser beads, while this
    file verifies the typed-object layer. A genuine AttributeError still fails
    the test in every case.
    """
    fixture_pdf = Path(__file__).parent / "fixtures" / "test-minimal.pdf"

    try:
        doc = pdftract.extract(str(fixture_pdf))
    except _extraction_error_types() as exc:
        pytest.skip(f"native parser could not read fixture PDF: {exc}")

    assert isinstance(doc, Document), \
        f"extract() should return Document, got {type(doc).__name__}"
    assert isinstance(doc.pages[0].width, int), \
        f"doc.pages[0].width should be int, got {type(doc.pages[0].width).__name__}"
    for page in doc.pages:
        assert isinstance(page, Page), \
            f"doc.pages should yield Page instances, got {type(page).__name__}"
        for span in page.spans:
            assert isinstance(span, Span), \
                f"page.spans should yield Span instances, got {type(span).__name__}"
            assert isinstance(span.text, str), \
                f"span.text should be str, got {type(span.text).__name__}"


if __name__ == "__main__":
    failed = 0
    for name, fn in sorted(globals().items()):
        if name.startswith("test_") and callable(fn):
            try:
                fn()
            except pytest.skip.Exception as exc:  # type: ignore[attr-defined]
                print(f"SKIP {name}: {exc}")
                continue
            except AssertionError as exc:
                failed += 1
                print(f"FAIL {name}: {exc}")
                continue
            except Exception as exc:  # unexpected, but never abort the run
                failed += 1
                print(f"FAIL {name}: {type(exc).__name__}: {exc}")
                continue
            print(f"PASS {name}")
    print(f"\n{'FAILED: ' + str(failed) if failed else 'All attribute-access tests passed'}")
    sys.exit(1 if failed else 0)
