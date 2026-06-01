//
//  Document.swift
//  Pdftract
//
//  Core document model representing a fully extracted PDF.
//

import Foundation

/// Top-level output structure for PDF extraction.
///
/// This is the canonical JSON output format, containing document-level
/// metadata and an array of page objects.
public struct Document: Codable, Equatable {
    /// The PDF fingerprint (for receipt generation).
    public let fingerprint: String

    /// Extracted pages, each containing spans and blocks.
    public var pages: [Page]

    /// Metadata about the extraction.
    public let metadata: ExtractionMetadata

    /// Digital signatures extracted from the document.
    public var signatures: [Signature]

    /// Interactive form fields extracted from the document.
    public var formFields: [FormField]

    /// Document-scoped hyperlinks extracted from the document.
    public var links: [Link]

    /// Embedded file attachments extracted from the document.
    public var attachments: [Attachment]

    /// Article thread chains extracted from the document.
    public var threads: [Thread]

    /// JavaScript actions detected in the document.
    public var javascriptActions: [JavascriptAction]

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case fingerprint
        case pages
        case metadata
        case signatures
        case formFields = "form_fields"
        case links
        case attachments
        case threads
        case javascriptActions = "javascript_actions"
    }

    /// Create a new Document structure.
    public init(
        fingerprint: String,
        pages: [Page] = [],
        metadata: ExtractionMetadata,
        signatures: [Signature] = [],
        formFields: [FormField] = [],
        links: [Link] = [],
        attachments: [Attachment] = [],
        threads: [Thread] = [],
        javascriptActions: [JavascriptAction] = []
    ) {
        self.fingerprint = fingerprint
        self.pages = pages
        self.metadata = metadata
        self.signatures = signatures
        self.formFields = formFields
        self.links = links
        self.attachments = attachments
        self.threads = threads
        self.javascriptActions = javascriptActions
    }
}

/// Metadata about the extraction process.
public struct ExtractionMetadata: Codable, Equatable {
    /// Total number of pages in the document.
    public let pageCount: UInt

    /// Receipts mode used for this extraction.
    public let receiptsMode: ReceiptsMode

    /// Number of spans extracted.
    public let spanCount: UInt

    /// Number of blocks extracted.
    public let blockCount: UInt

    /// Number of pages that failed to extract.
    public let errorCount: UInt

    /// Diagnostics emitted during extraction (coverage warnings, etc.)
    public var diagnostics: [String]

    /// Cache status: "hit", "miss", or "skipped".
    public var cacheStatus: String?

    /// Cache entry age in seconds (only present when cache_status == "hit").
    public var cacheAgeSeconds: UInt64?

    /// Reading order algorithm used for this extraction.
    public var readingOrderAlgorithm: String?

    /// Profile name if a profile was applied (Phase 7.10).
    public var profileName: String?

    /// Profile version if a profile was applied (Phase 7.10).
    public var profileVersion: String?

    /// Extracted fields from profile if a profile was applied (Phase 7.10).
    public var profileFields: [String: String]?

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case pageCount = "page_count"
        case receiptsMode = "receipts_mode"
        case spanCount = "span_count"
        case blockCount = "block_count"
        case errorCount = "error_count"
        case diagnostics
        case cacheStatus = "cache_status"
        case cacheAgeSeconds = "cache_age_seconds"
        case readingOrderAlgorithm = "reading_order_algorithm"
        case profileName = "profile_name"
        case profileVersion = "profile_version"
        case profileFields = "profile_fields"
    }

    /// Create a new ExtractionMetadata structure.
    public init(
        pageCount: UInt,
        receiptsMode: ReceiptsMode,
        spanCount: UInt,
        blockCount: UInt,
        errorCount: UInt,
        diagnostics: [String] = [],
        cacheStatus: String? = nil,
        cacheAgeSeconds: UInt64? = nil,
        readingOrderAlgorithm: String? = nil,
        profileName: String? = nil,
        profileVersion: String? = nil,
        profileFields: [String: String]? = nil
    ) {
        self.pageCount = pageCount
        self.receiptsMode = receiptsMode
        self.spanCount = spanCount
        self.blockCount = blockCount
        self.errorCount = errorCount
        self.diagnostics = diagnostics
        self.cacheStatus = cacheStatus
        self.cacheAgeSeconds = cacheAgeSeconds
        self.readingOrderAlgorithm = readingOrderAlgorithm
        self.profileName = profileName
        self.profileVersion = profileVersion
        self.profileFields = profileFields
    }
}

/// Receipt generation mode.
public enum ReceiptsMode: String, Codable, Equatable {
    /// No receipts generated (default).
    case off = "off"
    /// Lite mode: minimal receipts (~120 bytes each) with fingerprint, page index, bbox, and content hash.
    case lite = "lite"
    /// SVG mode: extended receipts that include an SVG clip rendering the glyphs.
    case svg = "svg"
}

/// JavaScript action found in a PDF.
public struct JavascriptAction: Codable, Equatable {
    /// Location of the JavaScript action in the PDF structure.
    /// Examples: "catalog.openaction", "page.0.aa.O", "page.1.annot.0.A".
    public let location: String

    /// Truncated excerpt of the JavaScript code (first 200 characters).
    public let codeExcerpt: String

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case location
        case codeExcerpt = "code_excerpt"
    }

    /// Create a new JavascriptAction structure.
    public init(
        location: String,
        codeExcerpt: String
    ) {
        self.location = location
        self.codeExcerpt = codeExcerpt
    }
}
