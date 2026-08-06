"""Type definitions for pdftract.

All types are implemented as frozen dataclasses for immutability.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, List, Optional, Self, Tuple


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

    @classmethod
    def from_native(cls, native_dict: dict) -> Self:
        return cls(
            page_count=native_dict.get("page_count", 0),
            title=native_dict.get("title"),
            author=native_dict.get("author"),
            subject=native_dict.get("subject"),
            keywords=native_dict.get("keywords"),
            creator=native_dict.get("creator"),
            producer=native_dict.get("producer"),
            created=native_dict.get("created"),
            modified=native_dict.get("modified"),
        )

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

    @classmethod
    def from_native(cls, native_dict: dict) -> Self:
        return cls(
            text=native_dict["text"],
            bbox=tuple(native_dict["bbox"]),
            font=native_dict["font"],
            size=native_dict["size"],
            confidence=native_dict.get("confidence"),
        )

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

    @classmethod
    def from_native(cls, native_dict: dict) -> Self:
        return cls(
            kind=native_dict["kind"],
            text=native_dict["text"],
            bbox=tuple(native_dict["bbox"]),
            level=native_dict.get("level"),
        )

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

    @classmethod
    def from_native(cls, native_dict: dict) -> Self:
        return cls(
            bbox=tuple(native_dict["bbox"]),
            text=native_dict["text"],
            spans=list(native_dict["spans"]),
            row=int(native_dict["row"]),
            col=int(native_dict["col"]),
            rowspan=int(native_dict["rowspan"]),
            colspan=int(native_dict["colspan"]),
            is_header_row=bool(native_dict["is_header_row"]),
        )


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

    @classmethod
    def from_native(cls, native_dict: dict) -> Self:
        return cls(
            bbox=tuple(native_dict["bbox"]),
            cells=[Cell.from_native(cell_dict) for cell_dict in native_dict["cells"]],
            is_header=bool(native_dict["is_header"]),
        )


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

    @classmethod
    def from_native(cls, native_dict: dict) -> Self:
        return cls(
            id=native_dict["id"],
            bbox=tuple(native_dict["bbox"]),
            rows=[Row.from_native(row_dict) for row_dict in native_dict["rows"]],
            header_rows=int(native_dict["header_rows"]),
            detection_method=native_dict["detection_method"],
            continued=bool(native_dict["continued"]),
            continued_from_prev=bool(native_dict["continued_from_prev"]),
            page_index=int(native_dict["page_index"]),
        )


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

    @classmethod
    def from_native(cls, native_dict: dict) -> Self:
        return cls(
            page=int(native_dict["page"]),
            width=int(native_dict["width"]),
            height=int(native_dict["height"]),
            rotation=int(native_dict.get("rotation", 0)),
            spans=tuple(Span.from_native(span_dict) for span_dict in native_dict.get("spans", [])),
            blocks=tuple(Block.from_native(block_dict) for block_dict in native_dict.get("blocks", [])),
        )

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

    @classmethod
    def from_native(cls, native_dict: dict) -> Self:
        return cls(
            schema_version=native_dict["schema_version"],
            pages=[Page.from_native(page_dict) for page_dict in native_dict["pages"]],
            metadata=Metadata.from_native(native_dict["metadata"]),
        )

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

    @classmethod
    def from_native(cls, native_dict: dict) -> Self:
        return cls(
            text=native_dict["text"],
            page=int(native_dict["page"]),
            bbox=tuple(native_dict["bbox"]),
            context=native_dict.get("context"),
        )

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

    @classmethod
    def from_native(cls, native_dict: dict) -> Self:
        metadata_dict = native_dict.get("metadata")
        return cls(
            hash=native_dict["hash"],
            fast_hash=native_dict["fast_hash"],
            page_count=int(native_dict.get("page_count", 0)),
            metadata=Metadata.from_native(metadata_dict) if metadata_dict else None,
        )

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

    @classmethod
    def from_native(cls, native_dict: dict) -> Self:
        return cls(
            category=native_dict["category"],
            confidence=float(native_dict["confidence"]),
            tags=tuple(native_dict.get("tags", [])),
            heuristics=native_dict.get("heuristics"),
        )

    @property
    def class_name(self) -> str:
        """Backward compatibility alias for category."""
        return self.category

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        return f"{cls_name}(category={self.category!r}, confidence={self.confidence:.2f})"
