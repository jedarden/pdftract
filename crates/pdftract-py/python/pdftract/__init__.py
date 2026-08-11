"""pdftract — PDF text extraction library.

This module provides Python bindings for the pdftract-core library,
with idiomatic Python ergonomics including exception hierarchy,
dataclass types, and optional asyncio wrappers.

Available types:
    Document: Complete PDF document extraction result
    Page: A single page with spans and blocks
    Span: A text span with font and position information
    Block: A semantic block (text, heading, list, table, figure)
    Match: A regex match result from search
    Fingerprint: PDF structural fingerprint for identity verification
    Classification: Page classification result
    Metadata: Document metadata (title, author, page count, etc.)

Example usage:
    import pdftract

    # Basic extraction
    doc = pdftract.extract("document.pdf")
    print(f"Extracted {len(doc.pages)} pages")

    # Text-only extraction
    text = pdftract.extract_text("document.pdf")

    # Streaming extraction for large PDFs
    for page in pdftract.extract_stream("large.pdf"):
        print(f"Page {page.page_index}: {len(page.spans)} spans")
"""

# Import native module (PyO3 bindings)
import shutil

_using_fallback = False
_native_available = False

try:
    from pdftract._native import *
    _native_available = True
except ImportError as e:
    _native_available = False
    _import_error = str(e)

    # Detect CLI binary for subprocess fallback
    _cli_path = shutil.which("pdftract")
    if _cli_path:
        _using_fallback = True
    else:
        # Both native module and CLI binary are unavailable
        raise ImportError(
            f"pdftract native module failed to import: {_import_error}. "
            "Subprocess fallback also unavailable: pdftract CLI binary not found in PATH. "
            "Install pdftract from https://github.com/jedarden/pdftract or ensure the CLI binary is in PATH."
        ) from e

# Import exception hierarchy
from pdftract.exceptions import (
    PdftractError,
    CorruptPdfError,
    EncryptionError,
    SourceUnreachableError,
    RemoteFetchInterruptedError,
    TlsError,
    ReceiptVerifyError,
    UnsupportedOperationError,
)

# Import type definitions
from pdftract.types import (
    Document,
    Page,
    Span,
    Block,
    Match,
    Fingerprint,
    Classification,
    Metadata,
)

# Import typing for return annotations
from typing import Iterator

# Import subprocess fallback
from pdftract.fallback import SubprocessExtractor

# Version
__version__ = "0.1.0"

# Check native availability
if not _native_available:
    import warnings
    warnings.warn(
        f"Native module failed to import: {_import_error}. "
        "Using subprocess fallback. Performance will be significantly degraded.",
        RuntimeWarning,
        stacklevel=2,
    )

# Export public API
__all__ = [
    # Version
    "__version__",
    # Exceptions
    "PdftractError",
    "CorruptPdfError",
    "EncryptionError",
    "SourceUnreachableError",
    "RemoteFetchInterruptedError",
    "TlsError",
    "ReceiptVerifyError",
    "UnsupportedOperationError",
    # Types
    "Document",
    "Page",
    "Span",
    "Block",
    "Match",
    "Fingerprint",
    "Classification",
    "Metadata",
    # Functions
    "extract",
    "extract_text",
    "extract_markdown",
    "extract_stream",
    "search",
    "get_metadata",
    "hash",
    "classify",
    "verify_receipt",
]

# Re-export asyncio module
import pdftract.asyncio as _asyncio_module
asyncio = _asyncio_module
__all__.extend(["asyncio"])

# Module-level state for subprocess fallback
_fallback_extractor = None


def _get_extractor():
    """Get the native extractor or subprocess fallback."""
    global _fallback_extractor

    if _native_available:
        # Return native module
        import pdftract._native as native
        return native
    else:
        # Initialize subprocess fallback on first use
        if _fallback_extractor is None:
            _fallback_extractor = SubprocessExtractor()
        return _fallback_extractor


def extract(source, **options) -> Document:
    """Extract text and structure from a PDF.

    Args:
        source: Path to PDF file or URL
        **options: Extraction options (snake_case):
            - ocr (bool): Enable OCR
            - ocr_language (list[str]): OCR languages (e.g., ["eng", "fra"])
            - include_invisible (bool): Include invisible text
            - extract_forms (bool): Extract form fields
            - extract_attachments (bool): Extract attachments
            - readability_threshold (float): Readability threshold (0.0-1.0)
            - password (str | None): PDF password
            - max_decompress_gb (int): Max decompressed GB per stream
            - full_render (bool): Enable full rendering

    Returns:
        Document: Extracted document with pages, spans, blocks

    Raises:
        CorruptPdfError: PDF file is corrupted
        EncryptionError: PDF is encrypted and no/wrong password
        SourceUnreachableError: File or URL is unreachable
        PdftractError: Other extraction errors
    """
    extractor = _get_extractor()
    result = extractor.extract(source, **options)
    # Wrap raw dict from native module in typed Document
    if isinstance(result, dict):
        return Document.from_native(result)
    return result


def extract_text(source, **options) -> str:
    """Extract plain text from a PDF.

    Args:
        source: Path to PDF file or URL
        **options: Extraction options (see extract())

    Returns:
        str: Extracted plain text

    Raises:
        PdftractError: Extraction errors
    """
    extractor = _get_extractor()
    return extractor.extract_text(source, **options)


def extract_markdown(source, **options) -> str:
    """Extract Markdown from a PDF.

    Args:
        source: Path to PDF file or URL
        **options: Extraction options (see extract())
            - anchors (bool): Include anchor links (default: False)

    Returns:
        str: Extracted Markdown

    Raises:
        PdftractError: Extraction errors
    """
    extractor = _get_extractor()
    return extractor.extract_markdown(source, **options)


def extract_stream(source, **options) -> Iterator[Page]:
    """Extract pages from a PDF as a streaming iterator.

    Args:
        source: Path to PDF file or URL
        **options: Extraction options (see extract())

    Returns:
        Iterator[Page]: Iterator yielding one page at a time

    Raises:
        PdftractError: Extraction errors

    Note:
        Memory usage stays bounded regardless of PDF size.
        Only one page is resident in memory at a time.
    """
    extractor = _get_extractor()
    # Wrap raw dict iterator from native module to yield typed Page objects
    for page in extractor.extract_stream(source, **options):
        if isinstance(page, dict):
            yield Page.from_native(page)
        else:
            yield page


def search(source, pattern, **options) -> Iterator[Match]:
    """Search for a regex pattern in a PDF.

    Args:
        source: Path to PDF file or URL
        pattern: Regular expression pattern to search for
        **options: Extraction options (see extract())

    Returns:
        Iterator[Match]: Iterator yielding matches

    Raises:
        PdftractError: Extraction errors
    """
    extractor = _get_extractor()
    # Wrap raw dict iterator from native module to yield typed Match objects
    for match in extractor.search(source, pattern, **options):
        if isinstance(match, dict):
            yield Match.from_native(match)
        else:
            yield match


def get_metadata(source, **options) -> Metadata:
    """Get metadata, outline, and fingerprint from a PDF (cheap, no full extraction).

    Args:
        source: Path to PDF file or URL
        **options: Extraction options:
            - password (str | None): PDF password

    Returns:
        Metadata: Document metadata

    Raises:
        PdftractError: Extraction errors
    """
    extractor = _get_extractor()
    result = extractor.get_metadata(source, **options)
    # Wrap raw dict from native module in typed Metadata
    if isinstance(result, dict):
        return Metadata.from_native(result)
    return result


def hash(source, **options) -> Fingerprint:
    """Compute the structural fingerprint of a PDF.

    Args:
        source: Path to PDF file or URL
        **options: Extraction options:
            - password (str | None): PDF password

    Returns:
        Fingerprint: Document fingerprint

    Raises:
        PdftractError: Extraction errors
    """
    extractor = _get_extractor()
    result = extractor.hash(source, **options)
    # Wrap raw dict/string from native module in typed Fingerprint
    if isinstance(result, dict):
        return Fingerprint.from_native(result)
    elif isinstance(result, str):
        return Fingerprint.from_string(result)
    return result


def classify(source) -> Classification:
    """Classify a PDF page type.

    Args:
        source: Path to PDF file or URL

    Returns:
        Classification: Page classification

    Raises:
        PdftractError: Extraction errors
    """
    extractor = _get_extractor()
    result = extractor.classify(source)
    # Wrap raw dict from native module in typed Classification
    if isinstance(result, dict):
        return Classification.from_native(result)
    return result


def verify_receipt(path, receipt) -> bool:
    """Verify a cryptographic receipt against a PDF.

    Args:
        path: Path to PDF file
        receipt: Receipt dict (as returned by extraction with receipts enabled)

    Returns:
        bool: True if receipt verifies, False otherwise

    Raises:
        ReceiptVerifyError: Receipt verification failed
        PdftractError: Other errors
    """
    extractor = _get_extractor()
    return extractor.verify_receipt(path, receipt)
