//
//  Table.swift
//  Pdftract
//
//  Table-related models for extracted PDF content.
//

import Foundation

/// A table extracted from a PDF page.
public struct Table: Codable, Equatable {
    /// Unique identifier for this table (e.g., "table_0").
    public let id: String

    /// Bounding box in PDF user-space points.
    public let bbox: [Double]

    /// Rows in this table, ordered top-to-bottom.
    public var rows: [Row]

    /// Number of contiguous header rows at the top of the table.
    public let headerRows: UInt32

    /// Detection method used to identify this table.
    public let detectionMethod: String

    /// Whether this table continues on the next page.
    public var continued: Bool

    /// Whether this table is a continuation from the previous page.
    public var continuedFromPrev: Bool

    /// Zero-based page index where this table appears.
    public let pageIndex: UInt

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case id
        case bbox
        case rows
        case headerRows = "header_rows"
        case detectionMethod = "detection_method"
        case continued
        case continuedFromPrev = "continued_from_prev"
        case pageIndex = "page_index"
    }

    /// Create a new Table structure.
    public init(
        id: String,
        bbox: [Double],
        rows: [Row] = [],
        headerRows: UInt32 = 0,
        detectionMethod: String,
        continued: Bool = false,
        continuedFromPrev: Bool = false,
        pageIndex: UInt = 0
    ) {
        self.id = id
        self.bbox = bbox
        self.rows = rows
        self.headerRows = headerRows
        self.detectionMethod = detectionMethod
        self.continued = continued
        self.continuedFromPrev = continuedFromPrev
        self.pageIndex = pageIndex
    }
}

/// A table row containing cells.
public struct Row: Codable, Equatable {
    /// Bounding box in PDF user-space points.
    public let bbox: [Double]

    /// Cells in this row, ordered left-to-right.
    public var cells: [Cell]

    /// Whether this row is a header row.
    public let isHeader: Bool

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case bbox
        case cells
        case isHeader = "is_header"
    }

    /// Create a new Row structure.
    public init(
        bbox: [Double],
        cells: [Cell] = [],
        isHeader: Bool = false
    ) {
        self.bbox = bbox
        self.cells = cells
        self.isHeader = isHeader
    }
}

/// A table cell.
public struct Cell: Codable, Equatable {
    /// Bounding box in PDF user-space points.
    public let bbox: [Double]

    /// The concatenated text content of all spans in the cell.
    public let text: String

    /// References to spans in the page's spans array.
    public let spans: [UInt]

    /// Zero-based row index within the table.
    public let row: UInt

    /// Zero-based column index within the table.
    public let col: UInt

    /// Number of rows this cell spans (default 1).
    public let rowspan: UInt32

    /// Number of columns this cell spans (default 1).
    public let colspan: UInt32

    /// Whether this cell is in a header row.
    public let isHeaderRow: Bool

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case bbox
        case text
        case spans
        case row
        case col
        case rowspan
        case colspan
        case isHeaderRow = "is_header_row"
    }

    /// Create a new Cell structure.
    public init(
        bbox: [Double],
        text: String,
        spans: [UInt],
        row: UInt,
        col: UInt,
        rowspan: UInt32 = 1,
        colspan: UInt32 = 1,
        isHeaderRow: Bool = false
    ) {
        self.bbox = bbox
        self.text = text
        self.spans = spans
        self.row = row
        self.col = col
        self.rowspan = rowspan
        self.colspan = colspan
        self.isHeaderRow = isHeaderRow
    }
}
