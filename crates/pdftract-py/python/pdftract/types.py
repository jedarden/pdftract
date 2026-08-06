"""Type definitions for pdftract.

All types are implemented as frozen dataclasses for immutability.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple


@dataclass(frozen=True, slots=True)
class Metadata:
    """Document metadata.

    Attributes:
        page_count: Total number of pages
        title: Document title
        author: Document author
        subject: Document subject
        keywords: Document keywords
        creator: Application that created the PDF
        producer: PDF generator
        created: Creation date string (ISO 8601)
        modified: Modification date string (ISO 8601)
    """

    page_count: int = 0
    title: Optional[str] = None
    author: Optional[str] = None
    subject: Optional[str] = None
    keywords: Optional[List[str]] = None
    creator: Optional[str] = None
    producer: Optional[str] = None
    created: Optional[str] = None
    modified: Optional[str] = None

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        return f"{cls_name}(page_count={self.page_count}, title={self.title!r})"


@dataclass(frozen=True, slots=True)
class Span:
    """A text span extracted from a PDF.

    Attributes:
        text: The extracted text content
        bbox: Bounding box [x0, y0, x1, y1] in PDF user-space points
        font: Font name
        size: Font size in points
        confidence: OCR confidence score (0.0-1.0), None for non-OCR text
    """

    text: str
    bbox: Tuple[float, float, float, float]
    font: str
    size: float
    confidence: Optional[float] = None

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        text_preview = self.text[:20] if self.text else ""
        return f"{cls_name}(text={text_preview!r}..., font={self.font!r}, size={self.size})"


@dataclass(frozen=True, slots=True)
class Block:
    """A semantic block extracted from a PDF.

    Attributes:
        kind: Block type (e.g., "text", "heading", "list", "table", "figure")
        text: The block's text content
        bbox: Bounding box [x0, y0, x1, y1] in PDF user-space points
        level: Heading level (1-6) for heading blocks
    """

    kind: str
    text: str
    bbox: Tuple[float, float, float, float]
    level: Optional[int] = None

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        text_preview = self.text[:20] if self.text else ""
        return f"{cls_name}(kind={self.kind!r}, text={text_preview!r}...)"


@dataclass(frozen=True, slots=True)
class Cell:
    """A table cell.

    Attributes:
        bbox: Bounding box [x0, y0, x1, y1]
        text: Cell text content
        spans: Indices of spans within this cell
        row: Row index (0-based)
        col: Column index (0-based)
        rowspan: Row span (number of rows this cell occupies)
        colspan: Column span (number of columns this cell occupies)
        is_header_row: Whether this cell is in a header row
    """

    bbox: Tuple[float, float, float, float]
    text: str
    spans: List[int]
    row: int
    col: int
    rowspan: int
    colspan: int
    is_header_row: bool


@dataclass(frozen=True, slots=True)
class Row:
    """A table row.

    Attributes:
        bbox: Bounding box [x0, y0, x1, y1]
        cells: List of cells in this row
        is_header: Whether this is a header row
    """

    bbox: Tuple[float, float, float, float]
    cells: List[Cell]
    is_header: bool


@dataclass(frozen=True, slots=True)
class Table:
    """A table extracted from a PDF.

    Attributes:
        id: Table identifier
        bbox: Bounding box [x0, y0, x1, y1]
        rows: List of rows in the table
        header_rows: Number of header rows
        detection_method: Method used to detect the table
        continued: Whether this table continues on the next page
        continued_from_prev: Whether this table continues from the previous page
        page_index: Page index where this table appears
    """

    id: str
    bbox: Tuple[float, float, float, float]
    rows: List[Row]
    header_rows: int
    detection_method: str
    continued: bool
    continued_from_prev: bool
    page_index: int


@dataclass(frozen=True, slots=True)
class Page:
    """A page extracted from a PDF.

    Attributes:
        page: One-based page number
        width: Page width in points (1/72 inch)
        height: Page height in points
        rotation: Page rotation in degrees (0, 90, 180, 270)
        spans: List of text spans on this page
        blocks: List of semantic blocks on this page
    """

    page: int
    width: int
    height: int
    rotation: int = 0
    spans: List[Span] = ()
    blocks: List[Block] = ()

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        return f"{cls_name}(page={self.page}, width={self.width}, height={self.height}, spans={len(self.spans)}, blocks={len(self.blocks)})"


@dataclass(frozen=True, slots=True)
class Document:
    """A complete PDF document extraction result.

    Attributes:
        schema_version: Schema version identifier
        pages: List of pages in the document
        metadata: Document metadata
    """

    schema_version: str
    pages: List[Page]
    metadata: Metadata

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        return f"{cls_name}(schema_version={self.schema_version!r}, pages={len(self.pages)}, metadata={self.metadata.title if self.metadata else None!r})"


@dataclass(frozen=True, slots=True)
class Match:
    """A regex match result from search.

    Attributes:
        text: The matched text
        page: Page number where the match occurred
        bbox: Bounding box of the match [x0, y0, x1, y1]
        context: Context before and after the match
    """

    text: str
    page: int
    bbox: Tuple[float, float, float, float]
    context: Optional[Dict[str, str]] = None

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        text_preview = self.text[:20] if self.text else ""
        return f"{cls_name}(text={text_preview!r}..., page={self.page})"


@dataclass(frozen=True, slots=True)
class Fingerprint:
    """A PDF structural fingerprint.

    Attributes:
        hash: SHA-256 hex of document content
        fast_hash: BLAKE3 hex of first 10KB
        page_count: Total number of pages
        metadata: Document metadata
    """

    hash: str
    fast_hash: str
    page_count: int = 0
    metadata: Optional[Metadata] = None

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        hash_preview = self.hash[:12] if self.hash else ""
        return f"{cls_name}(hash={hash_preview!r}..., page_count={self.page_count})"


@dataclass(frozen=True, slots=True)
class Classification:
    """A page classification result.

    Attributes:
        category: Classification category name
        confidence: Confidence score [0.0, 1.0]
        tags: Classification tags
        heuristics: Individual feature detections
    """

    category: str
    confidence: float
    tags: List[str] = ()
    heuristics: Optional[Dict[str, bool]] = None

    @property
    def class_name(self) -> str:
        """Backward compatibility alias for category."""
        return self.category

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        return f"{cls_name}(category={self.category!r}, confidence={self.confidence:.2f})"
