//
//  Page.swift
//  Pdftract
//
//  Page-level models for extracted PDF content.
//

import Foundation

/// Result for a single page.
public struct Page: Codable, Equatable {
    /// Zero-based page index.
    public let index: UInt

    /// 1-based page number (= index + 1).
    /// Emitted as a convenience for human-facing display.
    public let pageNumber: UInt32

    /// Human-readable label from PDF /PageLabels number tree.
    /// Examples: "iv", "A-3", "1". Null if the PDF defines no page labels.
    public var pageLabel: String?

    /// Page width in points (1/72 inch).
    public var width: Float?

    /// Page height in points (1/72 inch).
    public var height: Float?

    /// Page rotation in degrees clockwise (0, 90, 180, or 270).
    public var rotation: UInt16?

    /// Page classification from the page classifier.
    /// One of: "text", "scanned", "mixed", "broken_vector", "blank", "figure_only".
    public var type: PageType?

    /// Extracted spans (text fragments with consistent styling).
    public var spans: [Span]

    /// Extracted blocks (semantic units like paragraphs, headings).
    public var blocks: [Block]

    /// Extracted tables (cell-level structure).
    public var tables: [Table]

    /// Page-level annotations (highlights, stamps, notes, etc.).
    public var annotations: [Annotation]

    /// Error message if extraction failed for this page.
    public var error: String?

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case index
        case pageNumber = "page_number"
        case pageLabel = "page_label"
        case width
        case height
        case rotation
        case type
        case spans
        case blocks
        case tables
        case annotations
        case error
    }

    /// Create a new Page structure.
    public init(
        index: UInt,
        pageNumber: UInt32,
        pageLabel: String? = nil,
        width: Float? = nil,
        height: Float? = nil,
        rotation: UInt16? = nil,
        type: PageType? = nil,
        spans: [Span] = [],
        blocks: [Block] = [],
        tables: [Table] = [],
        annotations: [Annotation] = [],
        error: String? = nil
    ) {
        self.index = index
        self.pageNumber = pageNumber
        self.pageLabel = pageLabel
        self.width = width
        self.height = height
        self.rotation = rotation
        self.type = type
        self.spans = spans
        self.blocks = blocks
        self.tables = tables
        self.annotations = annotations
        self.error = error
    }
}

/// Page classification type.
public enum PageType: String, Codable, Equatable {
    /// Page with native vector text.
    case text = "text"
    /// Page that requires OCR (no vector text).
    case scanned = "scanned"
    /// Page with both vector text and images requiring OCR.
    case mixed = "mixed"
    /// Page with broken vector text (e.g., corrupt font data).
    case brokenVector = "broken_vector"
    /// Empty page with no content.
    case blank = "blank"
    /// Page with only figure/image content.
    case figureOnly = "figure_only"
}

/// A text span - the smallest unit of extracted text.
public struct Span: Codable, Equatable {
    /// The extracted text content.
    public let text: String

    /// Bounding box in PDF user-space points.
    /// Format: [x0, y0, x1, y1] where (x0, y0) is the bottom-left corner.
    public let bbox: [Double]

    /// Font name or identifier.
    public let font: String

    /// Font size in points.
    public let size: Double

    /// Fill color as CSS hex string (e.g., "#1a1a1a"), or null if not expressible as RGB.
    public var color: String?

    /// PDF Tr operator value (0-7) indicating the text rendering mode.
    /// 0 = fill, 1 = stroke, 2 = fill then stroke, 3 = invisible,
    /// 4 = fill to clip, 5 = stroke to clip, 6 = fill then stroke to clip, 7 = clip.
    public var renderingMode: UInt8?

    /// Optional confidence score (0.0 to 1.0).
    public var confidence: Double?

    /// Source of the confidence/text extraction.
    /// One of: "vector", "ocr", "ocr-assisted", "ocr-fallback", "repaired".
    public var confidenceSource: ConfidenceSource?

    /// BCP-47 language tag if detected.
    /// Examples: "en", "en-US", "zh-Hans".
    public var lang: String?

    /// Set of style flags applied to this span.
    /// Possible values: "bold", "italic", "smallcaps", "subscript", "superscript".
    public var flags: [String]

    /// Column index (0-based) assigned by column detection.
    public var column: UInt32?

    /// Optional cryptographic receipt for verification.
    public var receipt: Receipt?

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case text
        case bbox
        case font
        case size
        case color
        case renderingMode = "rendering_mode"
        case confidence
        case confidenceSource = "confidence_source"
        case lang
        case flags
        case column
        case receipt
    }

    /// Create a new Span structure.
    public init(
        text: String,
        bbox: [Double],
        font: String,
        size: Double,
        color: String? = nil,
        renderingMode: UInt8? = nil,
        confidence: Double? = nil,
        confidenceSource: ConfidenceSource? = nil,
        lang: String? = nil,
        flags: [String] = [],
        column: UInt32? = nil,
        receipt: Receipt? = nil
    ) {
        self.text = text
        self.bbox = bbox
        self.font = font
        self.size = size
        self.color = color
        self.renderingMode = renderingMode
        self.confidence = confidence
        self.confidenceSource = confidenceSource
        self.lang = lang
        self.flags = flags
        self.column = column
        self.receipt = receipt
    }
}

/// Source of the confidence/text extraction.
public enum ConfidenceSource: String, Codable, Equatable {
    /// Native font decoding.
    case vector = "vector"
    /// Pure OCR.
    case ocr = "ocr"
    /// OCR + vector correction.
    case ocrAssisted = "ocr-assisted"
    /// Region-level fallback.
    case ocrFallback = "ocr-fallback"
    /// Text was repaired via heuristics.
    case repaired = "repaired"
}

/// A structural block composed of one or more spans.
public struct Block: Codable, Equatable {
    /// The block kind/type.
    /// Common values: "paragraph", "heading", "list", "table", "figure".
    public let kind: String

    /// The concatenated text content of all spans in the block.
    public let text: String

    /// Bounding box in PDF user-space points.
    /// Format: [x0, y0, x1, y1] where (x0, y0) is the bottom-left corner.
    public let bbox: [Double]

    /// Optional heading level (1-6) for "heading" kind blocks.
    public var level: UInt8?

    /// Optional table index for "table" kind blocks.
    public var tableIndex: UInt?

    /// References to spans in the page's spans array.
    public var spans: [UInt]

    /// Optional cryptographic receipt for verification.
    public var receipt: Receipt?

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case kind
        case text
        case bbox
        case level
        case tableIndex = "table_index"
        case spans
        case receipt
    }

    /// Create a new Block structure.
    public init(
        kind: String,
        text: String,
        bbox: [Double],
        level: UInt8? = nil,
        tableIndex: UInt? = nil,
        spans: [UInt] = [],
        receipt: Receipt? = nil
    ) {
        self.kind = kind
        self.text = text
        self.bbox = bbox
        self.level = level
        self.tableIndex = tableIndex
        self.spans = spans
        self.receipt = receipt
    }
}
