"""Type definitions for pdftract.

All types are implemented as frozen dataclasses for immutability.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Iterator, List, Optional, Tuple


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
    bbox: List[float]
    font: str
    size: float
    confidence: Optional[float] = None

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        text_preview = self.text[:20] if self.text else ""
        return f"{cls_name}(text={text_preview!r}..., font={self.font!r}, size={self.size})"

    @classmethod
    def from_native(cls, native_dict: dict) -> "Span":
        """Create a Span from a native layer dict representation."""
        return cls(
            text=native_dict.get("text", ""),
            bbox=native_dict.get("bbox", []),
            font=native_dict.get("font", ""),
            size=native_dict.get("size", 0.0),
            confidence=native_dict.get("confidence"),
        )


@dataclass(frozen=True, slots=True)
class Block:
    """A semantic block extracted from a PDF.

    Attributes:
        kind: Block type (e.g., "text", "heading", "list", "table", "figure")
        text: The block's text content
        bbox: Bounding box [x0, y0, x1, y1] in PDF user-space points
        level: Heading level (1-6) for heading blocks
        table_index: Index of the table for table-caption blocks
    """

    kind: str
    text: str
    bbox: List[float]
    level: Optional[int] = None
    table_index: Optional[int] = None

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        text_preview = self.text[:20] if self.text else ""
        return f"{cls_name}(kind={self.kind!r}, text={text_preview!r}...)"

    @classmethod
    def from_native(cls, native_dict: dict) -> "Block":
        """Create a Block from a native layer dict representation."""
        return cls(
            kind=native_dict.get("kind", ""),
            text=native_dict.get("text", ""),
            bbox=native_dict.get("bbox", []),
            level=native_dict.get("level"),
            table_index=native_dict.get("table_index"),
        )


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

    bbox: List[float]
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

    bbox: List[float]
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
    bbox: List[float]
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

    @classmethod
    def from_native(cls, native_dict: dict) -> "Page":
        """Create a Page from a native layer dict representation."""
        return cls.from_dict(native_dict)

    @classmethod
    def from_dict(cls, data: dict) -> "Page":
        """Create a Page from a dict (e.g., from subprocess output)."""
        from pdftract.types import Span, Block, Table, Row, Cell

        spans = [
            Span(
                text=s["text"],
                bbox=s["bbox"],
                font=s["font"],
                size=s["size"],
                confidence=s.get("confidence"),
            )
            for s in data.get("spans", [])
        ]

        blocks = [
            Block(
                kind=b["kind"],
                text=b["text"],
                bbox=b["bbox"],
                level=b.get("level"),
                table_index=b.get("table_index"),
            )
            for b in data.get("blocks", [])
        ]

        tables = []
        for t in data.get("tables", []):
            rows = []
            for r in t.get("rows", []):
                cells = [
                    Cell(
                        bbox=c["bbox"],
                        text=c["text"],
                        spans=c["spans"],
                        row=c["row"],
                        col=c["col"],
                        rowspan=c["rowspan"],
                        colspan=c["colspan"],
                        is_header_row=c["is_header_row"],
                    )
                    for c in r.get("cells", [])
                ]
                rows.append(
                    Row(
                        bbox=r["bbox"],
                        cells=cells,
                        is_header=r["is_header"],
                    )
                )

            tables.append(
                Table(
                    id=t["id"],
                    bbox=t["bbox"],
                    rows=rows,
                    header_rows=t["header_rows"],
                    detection_method=t["detection_method"],
                    continued=t["continued"],
                    continued_from_prev=t["continued_from_prev"],
                    page_index=t["page_index"],
                )
            )

        return cls(
            page_index=data["page_index"],
            spans=spans,
            blocks=blocks,
            tables=tables,
            error=data.get("error"),
        )


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
        creation_date: Creation date string
        mod_date: Modification date string
        fingerprint: Document fingerprint
        outline: Outline/bookmarks structure
    """

    page_count: int
    title: Optional[str] = None
    author: Optional[str] = None
    subject: Optional[str] = None
    keywords: Optional[str] = None
    creator: Optional[str] = None
    producer: Optional[str] = None
    creation_date: Optional[str] = None
    mod_date: Optional[str] = None
    fingerprint: Optional[str] = None
    outline: Optional[dict] = None

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        return f"{cls_name}(page_count={self.page_count}, title={self.title!r})"

    @classmethod
    def from_native(cls, native_dict: dict) -> "Metadata":
        """Create a Metadata from a native layer dict representation."""
        return cls(
            page_count=native_dict.get("page_count", 0),
            title=native_dict.get("title"),
            author=native_dict.get("author"),
            subject=native_dict.get("subject"),
            keywords=native_dict.get("keywords"),
            creator=native_dict.get("creator"),
            producer=native_dict.get("producer"),
            creation_date=native_dict.get("creation_date"),
            mod_date=native_dict.get("mod_date"),
            fingerprint=native_dict.get("fingerprint"),
            outline=native_dict.get("outline"),
        )


@dataclass(frozen=True, slots=True)
class Document:
    """A complete PDF document extraction result.

    Attributes:
        schema_version: Schema version identifier
        pages: List of pages in the document
        metadata: Document metadata
    """

    schema_version: str = "1.0"
    pages: List[Page] = ()
    metadata: Optional[Metadata] = None

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        return f"{cls_name}(schema_version={self.schema_version!r}, pages={len(self.pages)}, metadata={self.metadata.title if self.metadata else None!r})"

    @classmethod
    def from_native(cls, native_dict: dict) -> "Document":
        """Create a Document from a native layer dict representation."""
        return cls.from_dict(native_dict)

    @classmethod
    def from_dict(cls, data: dict) -> "Document":
        """Create a Document from a dict (e.g., from subprocess output)."""
        pages = [Page.from_dict(p) for p in data.get("pages", [])]

        md = data.get("metadata", {})
        metadata = Metadata(
            page_count=md.get("page_count", len(pages)),
            title=md.get("title"),
            author=md.get("author"),
            subject=md.get("subject"),
            keywords=md.get("keywords"),
            creator=md.get("creator"),
            producer=md.get("producer"),
            creation_date=md.get("creation_date"),
            mod_date=md.get("mod_date"),
            fingerprint=md.get("fingerprint"),
            outline=md.get("outline"),
        )

        return cls(pages=pages, metadata=metadata)


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
    bbox: Tuple[int, int, int, int]
    context: Optional[dict] = None

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        text_preview = self.text[:20] if self.text else ""
        return f"{cls_name}(text={text_preview!r}..., page={self.page})"

    @classmethod
    def from_native(cls, native_dict: dict) -> "Match":
        """Create a Match from a native layer dict representation."""
        return cls(
            text=native_dict.get("text", ""),
            page_index=native_dict.get("page_index", 0),
            span_index=native_dict.get("span_index", 0),
            bbox=native_dict.get("bbox", []),
            match_start=native_dict.get("match_start", 0),
            match_end=native_dict.get("match_end", 0),
        )


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

    @classmethod
    def from_native(cls, native_dict: dict) -> "Fingerprint":
        """Create a Fingerprint from a native layer dict representation."""
        if isinstance(native_dict, str):
            return cls.from_string(native_dict)
        return cls(
            value=native_dict.get("value", ""),
            version=native_dict.get("version", "v1"),
            fast_hash=native_dict.get("fast_hash"),
        )

    @classmethod
    def from_string(cls, value: str) -> "Fingerprint":
        """Create a Fingerprint from a string."""
        if value.startswith("pdftract-"):
            parts = value.split(":", 1)
            if len(parts) == 2:
                version = parts[0].replace("pdftract-", "")
                return cls(value=value, version=version)
        return cls(value=value, version="v1")


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
    heuristics: Optional[dict] = None

    @property
    def class_name(self) -> str:
        """Backward compatibility alias for category."""
        return self.category

    def __repr__(self) -> str:
        cls_name = self.__class__.__name__
        return f"{cls_name}(category={self.category!r}, confidence={self.confidence:.2f})"

    @classmethod
    def from_native(cls, native_dict: dict) -> "Classification":
        """Create a Classification from a native layer dict representation."""
        # Handle both category and class_name for backward compatibility
        category = native_dict.get("category") or native_dict.get("class_name", "Unknown")
        return cls(
            category=category,
            confidence=native_dict.get("confidence", 0.0),
            hybrid_cells=native_dict.get("hybrid_cells"),
        )
