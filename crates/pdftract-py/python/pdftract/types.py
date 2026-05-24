"""Type definitions for pdftract.

All types are implemented as frozen dataclasses for immutability.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Iterator, List, Optional


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
        page_index: Zero-based page index
        spans: List of text spans on this page
        blocks: List of semantic blocks on this page
        tables: List of tables on this page
        error: Error message if extraction failed for this page
    """

    page_index: int
    spans: List[Span]
    blocks: List[Block]
    tables: List[Table]
    error: Optional[str] = None

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


@dataclass(frozen=True, slots=True)
class Document:
    """A complete PDF document extraction result.

    Attributes:
        pages: List of pages in the document
        metadata: Document metadata
    """

    pages: List[Page]
    metadata: Metadata

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
        page_index: Page index where the match occurred
        span_index: Index of the span containing the match
        bbox: Bounding box of the match
        match_start: Start position within the span text
        match_end: End position within the span text
    """

    text: str
    page_index: int
    span_index: int
    bbox: List[float]
    match_start: int
    match_end: int


@dataclass(frozen=True, slots=True)
class Fingerprint:
    """A PDF structural fingerprint.

    Attributes:
        value: The fingerprint string (e.g., "pdftract-v1:abc123...")
        version: Fingerprint algorithm version
    """

    value: str
    version: str = "v1"

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
        class_name: Classification class name
        confidence: Confidence score [0.0, 1.0]
        hybrid_cells: For Hybrid pages, set of scanned cell indexes
    """

    class_name: str
    confidence: float
    hybrid_cells: Optional[set[int]] = None
