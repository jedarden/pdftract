// swiftlint:disable all
// Auto-generated from pdftract schema v1.0 - do not edit manually

import Foundation

/// Extraction quality metrics.
public struct ExtractionQuality: Codable, Sendable {
    /// Overall quality score (0-1)
    public let score: Double

    /// Whether OCR was used
    public let ocrUsed: Bool

    /// Percentage of pages with vector text
    public let vectorTextCoverage: Double

    enum CodingKeys: String, CodingKey {
        case score
        case ocrUsed = "ocr_used"
        case vectorTextCoverage = "vector_text_coverage"
    }
}
