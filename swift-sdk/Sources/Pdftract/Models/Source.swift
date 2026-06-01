//
//  Source.swift
//  Pdftract
//
//  Source enumeration for PDF input.
//
//  NOTE: The Source enum is now defined in Pdftract.swift
//  This file re-exports it for consistency and provides options structs.
//

import Foundation

/// Options for PDF extraction.
public struct ExtractionOptions: Codable, Equatable {
    /// Whether to extract spans (atomic text units).
    public var extractSpans: Bool

    /// Whether to extract blocks (semantic units).
    public var extractBlocks: Bool

    /// Whether to extract tables.
    public var extractTables: Bool

    /// Whether to extract annotations.
    public var extractAnnotations: Bool

    /// Whether to extract form fields.
    public var extractFormFields: Bool

    /// Whether to extract signatures.
    public var extractSignatures: Bool

    /// Whether to extract attachments.
    public var extractAttachments: Bool

    /// Whether to extract outline/bookmarks.
    public var extractOutline: Bool

    /// Whether to extract article threads.
    public var extractThreads: Bool

    /// Whether to extract links.
    public var extractLinks: Bool

    /// DPI to use for OCR (nil for auto-selection).
    public var ocrDpi: UInt32?

    /// Maximum attachment size in bytes (nil for no limit).
    public var maxAttachmentSize: UInt64?

    /// Whether to include extraction quality metrics.
    public var includeQuality: Bool

    /// Whether to include diagnostic errors.
    public var includeErrors: Bool

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case extractSpans = "extract_spans"
        case extractBlocks = "extract_blocks"
        case extractTables = "extract_tables"
        case extractAnnotations = "extract_annotations"
        case extractFormFields = "extract_form_fields"
        case extractSignatures = "extract_signatures"
        case extractAttachments = "extract_attachments"
        case extractOutline = "extract_outline"
        case extractThreads = "extract_threads"
        case extractLinks = "extract_links"
        case ocrDpi = "ocr_dpi"
        case maxAttachmentSize = "max_attachment_size"
        case includeQuality = "include_quality"
        case includeErrors = "include_errors"
    }

    /// Create default extraction options.
    public init(
        extractSpans: Bool = true,
        extractBlocks: Bool = true,
        extractTables: Bool = true,
        extractAnnotations: Bool = true,
        extractFormFields: Bool = true,
        extractSignatures: Bool = true,
        extractAttachments: Bool = true,
        extractOutline: Bool = true,
        extractThreads: Bool = true,
        extractLinks: Bool = true,
        ocrDpi: UInt32? = nil,
        maxAttachmentSize: UInt64? = nil,
        includeQuality: Bool = true,
        includeErrors: Bool = true
    ) {
        self.extractSpans = extractSpans
        self.extractBlocks = extractBlocks
        self.extractTables = extractTables
        self.extractAnnotations = extractAnnotations
        self.extractFormFields = extractFormFields
        self.extractSignatures = extractSignatures
        self.extractAttachments = extractAttachments
        self.extractOutline = extractOutline
        self.extractThreads = extractThreads
        self.extractLinks = extractLinks
        self.ocrDpi = ocrDpi
        self.maxAttachmentSize = maxAttachmentSize
        self.includeQuality = includeQuality
        self.includeErrors = includeErrors
    }

    /// Default extraction options with all features enabled.
    public static let `default` = ExtractionOptions()
}

/// Specialized options for text extraction.
public struct TextOptions: Codable, Equatable {
    /// Whether to preserve whitespace formatting.
    public var preserveWhitespace: Bool

    /// Whether to include font information.
    public var includeFontInfo: Bool

    /// Whether to include bounding boxes.
    public var includeBoundingBoxes: Bool

    /// Create default text options.
    public init(
        preserveWhitespace: Bool = true,
        includeFontInfo: Bool = false,
        includeBoundingBoxes: Bool = false
    ) {
        self.preserveWhitespace = preserveWhitespace
        self.includeFontInfo = includeFontInfo
        self.includeBoundingBoxes = includeBoundingBoxes
    }

    /// Default text options.
    public static let `default` = TextOptions()
}

/// Specialized options for markdown extraction.
public struct MarkdownOptions: Codable, Equatable {
    /// Whether to include headings.
    public var includeHeadings: Bool

    /// Whether to include lists.
    public var includeLists: Bool

    /// Whether to include tables as markdown tables.
    public var includeTables: Bool

    /// Whether to include links.
    public var includeLinks: Bool

    /// Create default markdown options.
    public init(
        includeHeadings: Bool = true,
        includeLists: Bool = true,
        includeTables: Bool = true,
        includeLinks: Bool = true
    ) {
        self.includeHeadings = includeHeadings
        self.includeLists = includeLists
        self.includeTables = includeTables
        self.includeLinks = includeLinks
    }

    /// Default markdown options.
    public static let `default` = MarkdownOptions()
}
