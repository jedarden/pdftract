"""pdftract — PDF text extraction library.

This module provides Python bindings for the pdftract-core library,
with idiomatic Python ergonomics including exception hierarchy,
dataclass types, and optional asyncio wrappers.

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
try:
    from pdftract._native import *
    _native_available = True
except ImportError as e:
    _native_available = False
    _import_error = str(e)

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


def extract(source, **options):
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
    return extractor.extract(source, **options)


def extract_text(source, **options):
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


def extract_markdown(source, **options):
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


def extract_stream(source, **options):
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
    return extractor.extract_stream(source, **options)


def search(source, pattern, **options):
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
    return extractor.search(source, pattern, **options)


def get_metadata(source, **options):
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
    return extractor.get_metadata(source, **options)


def hash(source, **options):
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
    return extractor.hash(source, **options)


def classify(source):
    """Classify a PDF page type.

    Args:
        source: Path to PDF file or URL

    Returns:
        Classification: Page classification

    Raises:
        PdftractError: Extraction errors
    """
    extractor = _get_extractor()
    return extractor.classify(source)


def verify_receipt(path, receipt):
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
