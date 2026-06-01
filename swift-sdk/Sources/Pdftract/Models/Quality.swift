//
//  Quality.swift
//  Pdftract
//
//  Extraction quality and diagnostic models.
//

import Foundation

/// Extraction quality metrics for the document.
public struct ExtractionQuality: Codable, Equatable {
    /// Overall quality assessment: "high", "medium", "low", or "none".
    public var overallQuality: String

    /// DPI used for OCR rendering (Phase 5.2).
    public var dpiUsed: UInt32?

    /// Fraction of pages that required OCR fallback [0.0, 1.0].
    public var ocrFraction: Float?

    /// Minimum confidence score across all spans [0.0, 1.0].
    public var minConfidence: Float?

    /// Average confidence score across all spans [0.0, 1.0].
    public var avgConfidence: Float?

    /// Per-page readability score (char-weighted median of span scores) [0.0, 1.0].
    public var readability: Float?

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case overallQuality = "overall_quality"
        case dpiUsed = "dpi_used"
        case ocrFraction = "ocr_fraction"
        case minConfidence = "min_confidence"
        case avgConfidence = "avg_confidence"
        case readability
    }

    /// Create a new ExtractionQuality structure.
    public init(
        overallQuality: String = "none",
        dpiUsed: UInt32? = nil,
        ocrFraction: Float? = nil,
        minConfidence: Float? = nil,
        avgConfidence: Float? = nil,
        readability: Float? = nil
    ) {
        self.overallQuality = overallQuality
        self.dpiUsed = dpiUsed
        self.ocrFraction = ocrFraction
        self.minConfidence = minConfidence
        self.avgConfidence = avgConfidence
        self.readability = readability
    }
}

/// A diagnostic error emitted during extraction.
public struct Diagnostic: Codable, Equatable {
    /// Stable string identifier for this diagnostic.
    public let code: String

    /// Human-readable description of the diagnostic.
    public let message: String

    /// Severity level: "info", "warning", "error", or "fatal".
    public let severity: String

    /// Page index where this diagnostic occurred, or null for document-level events.
    public var pageIndex: UInt?

    /// PDF object reference where the issue originated, if applicable.
    public var location: ObjectLocation?

    /// Optional hint for resolving the diagnostic.
    public var hint: String?

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case code
        case message
        case severity
        case pageIndex = "page_index"
        case location
        case hint
    }

    /// Create a new Diagnostic structure.
    public init(
        code: String,
        message: String,
        severity: String,
        pageIndex: UInt? = nil,
        location: ObjectLocation? = nil,
        hint: String? = nil
    ) {
        self.code = code
        self.message = message
        self.severity = severity
        self.pageIndex = pageIndex
        self.location = location
        self.hint = hint
    }
}

/// A PDF object reference.
public struct ObjectLocation: Codable, Equatable {
    /// Object number (zero-based index in the xref table).
    public let objectNumber: UInt32

    /// Generation number (incremented on each save).
    public let generationNumber: UInt16

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case objectNumber = "object_number"
        case generationNumber = "generation_number"
    }

    /// Create a new ObjectLocation structure.
    public init(objectNumber: UInt32, generationNumber: UInt16) {
        self.objectNumber = objectNumber
        self.generationNumber = generationNumber
    }
}

/// A JavaScript action found in a PDF.
public struct JavascriptAction: Codable, Equatable {
    /// Location of the JavaScript action in the PDF structure.
    public let location: String

    /// Truncated excerpt of the JavaScript code (first 200 characters).
    public let codeExcerpt: String

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case location
        case codeExcerpt = "code_excerpt"
    }

    /// Create a new JavascriptAction structure.
    public init(location: String, codeExcerpt: String) {
        self.location = location
        self.codeExcerpt = codeExcerpt
    }
}
