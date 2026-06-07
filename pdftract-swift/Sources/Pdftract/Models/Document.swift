// swiftlint:disable all
// Auto-generated from pdftract schema v1.0 - do not edit manually

import Foundation

/// PDF document with pages and metadata.
public struct Document: Codable, Sendable {
    /// Schema version (e.g., "1.0")
    public let schemaVersion: String

    /// Pages in the document
    public let pages: [Page]

    /// Document metadata
    public let metadata: Metadata

    /// Embedded file attachments
    public let attachments: [Attachment]

    /// Diagnostics emitted during extraction
    public let errors: [Diagnostic]

    /// Extraction quality metrics
    public let extractionQuality: ExtractionQuality?

    /// Document outlines (bookmarks)
    public let outlines: [OutlineNode]?

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case pages
        case metadata
        case attachments
        case errors
        case extractionQuality = "extraction_quality"
        case outlines
    }
}

/// Single page in the document.
public struct Page: Codable, Sendable {
    /// Zero-based page index (canonical for programmatic use)
    public let pageIndex: Int

    /// One-based page number (= pageIndex + 1)
    public let pageNumber: Int

    /// Human-readable label from PDF /PageLabels (e.g., "iv", "A-3")
    public let pageLabel: String?

    /// Page width in points (1/72 inch)
    public let width: Double

    /// Page height in points (1/72 inch)
    public let height: Double

    /// Page rotation in degrees clockwise (0, 90, 180, or 270)
    public let rotation: Int

    /// Page classification: "text", "scanned", "mixed", "broken_vector", "blank", "figure_only"
    public let type: String

    /// Text spans (atomic units with consistent font and styling)
    public let spans: [Span]

    /// Semantic blocks (paragraphs, headings, lists, tables, etc.)
    public let blocks: [Block]

    /// Table structures
    public let tables: [Table]

    /// Page-level annotations (highlights, stamps, notes, links)
    public let annotations: [Annotation]

    enum CodingKeys: String, CodingKey {
        case pageIndex = "page_index"
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
    }
}

/// Text span with font and position information.
public struct Span: Codable, Sendable {
    /// The extracted text content
    public let text: String

    /// Bounding box in PDF user-space points [x0, y0, x1, y1]
    public let bbox: [Double]

    /// Font name or identifier
    public let font: String

    /// Font size in points
    public let size: Double

    /// Fill color as CSS hex string (e.g., "#1a1a1a"), or null if not expressible as RGB
    public let color: String?

    /// PDF Tr operator value (0-7) indicating text rendering mode
    public let renderingMode: Int?

    /// Optional confidence score (0.0 to 1.0)
    public let confidence: Double?

    /// Source of confidence/text extraction: "native", "heuristic", "ocr"
    public let confidenceSource: String?

    /// BCP-47 language tag if detected (e.g., "en", "en-US", "zh-Hans")
    public let lang: String?

    /// Set of style flags: "bold", "italic", "smallcaps", "subscript", "superscript"
    public let flags: [String]

    /// Optional cryptographic receipt for verification
    public let receipt: Receipt?

    /// Column index (0-based) assigned by column detection
    public let column: Int?

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
        case receipt
        case column
    }
}

/// Structural block (paragraph, heading, list, table, figure).
public struct Block: Codable, Sendable {
    /// Block kind/type: "paragraph", "heading", "list", "table", "figure"
    public let kind: String

    /// The concatenated text content of all spans in the block
    public let text: String

    /// Bounding box in PDF user-space points [x0, y0, x1, y1]
    public let bbox: [Double]

    /// Optional heading level (1-6) for heading blocks
    public let level: Int?

    /// References to spans in the page's spans array
    public let spans: [Int]

    /// Optional table index for table blocks
    public let tableIndex: Int?

    /// Optional cryptographic receipt for verification
    public let receipt: Receipt?

    enum CodingKeys: String, CodingKey {
        case kind
        case text
        case bbox
        case level
        case spans
        case tableIndex = "table_index"
        case receipt
    }
}

/// Match result from search operations.
public struct Match: Codable, Sendable {
    /// The matched text
    public let text: String

    /// Page number where match occurred
    public let page: Int

    /// Location of the match [x0, y0, x1, y1]
    public let bbox: [Double]

    /// Surrounding text context (50 chars before/after)
    public let context: MatchContext
}

/// Context for search matches.
public struct MatchContext: Codable, Sendable {
    /// Text before the match
    public let before: String

    /// Text after the match
    public let after: String
}

/// Fingerprint hash information.
public struct Fingerprint: Codable, Sendable {
    /// SHA-256 hex of document content
    public let hash: String

    /// Number of pages
    public let pageCount: Int

    /// BLAKE3 hex of first 10KB
    public let fastHash: String

    /// Document metadata
    public let metadata: Metadata

    enum CodingKeys: String, CodingKey {
        case hash
        case pageCount = "page_count"
        case fastHash = "fast_hash"
        case metadata
    }
}

/// Classification result for a document.
public struct Classification: Codable, Sendable {
    /// Primary category
    public let category: String

    /// Confidence score (0-1)
    public let confidence: Double

    /// Tags associated with the document
    public let tags: [String]

    /// Individual feature detections
    public let heuristics: [String: Bool]
}

/// Document metadata.
public struct Metadata: Codable, Sendable {
    /// Document title
    public let title: String?

    /// Document author
    public let author: String?

    /// Document subject
    public let subject: String?

    /// Keywords
    public let keywords: [String]?

    /// Creator application
    public let creator: String?

    /// Producer application
    public let producer: String?

    /// Creation date (ISO 8601)
    public let created: String?

    /// Modification date (ISO 8601)
    public let modified: String?

    /// Number of pages
    public let pageCount: Int

    /// Whether the PDF is encrypted
    public let isEncrypted: Bool?

    enum CodingKeys: String, CodingKey {
        case title
        case author
        case subject
        case keywords
        case creator
        case producer
        case created
        case modified
        case pageCount = "page_count"
        case isEncrypted = "is_encrypted"
    }
}
