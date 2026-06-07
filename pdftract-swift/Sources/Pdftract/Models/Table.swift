// swiftlint:disable all
// Auto-generated from pdftract schema v1.0 - do not edit manually

import Foundation

/// Table structure.
public struct Table: Codable, Sendable {
    /// Table index within the page
    public let index: Int

    /// Bounding box in PDF user-space points [x0, y0, x1, y1]
    public let bbox: [Double]

    /// Number of rows detected
    public let rowCount: Int

    /// Number of columns detected
    public let columnCount: Int

    /// Table rows
    public let rows: [Row]

    enum CodingKeys: String, CodingKey {
        case index
        case bbox
        case rowCount = "row_count"
        case columnCount = "col_count"
        case rows
    }
}

/// Table row.
public struct Row: Codable, Sendable {
    /// Bounding box in PDF user-space points [x0, y0, x1, y1]
    public let bbox: [Double]

    /// Cells in this row, ordered left-to-right
    public let cells: [Cell]

    /// Whether this row is a header row
    public let isHeader: Bool

    enum CodingKeys: String, CodingKey {
        case bbox
        case cells
        case isHeader = "is_header"
    }
}

/// Table cell.
public struct Cell: Codable, Sendable {
    /// Bounding box in PDF user-space points [x0, y0, x1, y1]
    public let bbox: [Double]

    /// Cell text content
    public let text: String

    /// References to spans in the page's spans array
    public let spans: [Int]

    /// Zero-based row index within the table
    public let row: Int

    /// Zero-based column index within the table
    public let col: Int

    /// Number of rows this cell spans (default 1)
    public let rowspan: Int

    /// Number of columns this cell spans (default 1)
    public let colspan: Int

    /// Whether this cell is in a header row
    public let isHeaderRow: Bool

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
}
