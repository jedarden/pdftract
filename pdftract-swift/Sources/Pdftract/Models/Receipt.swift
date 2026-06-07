// swiftlint:disable all
// Auto-generated from pdftract schema v1.0 - do not edit manually

import Foundation

/// Cryptographic receipt for text verification.
public struct Receipt: Codable, Sendable {
    /// Phase 1.7 fingerprint of the source PDF (format: "pdftract-v1:" + hex(SHA-256))
    public let pdfFingerprint: String

    /// 0-based page index in the source PDF
    public let pageIndex: Int

    /// Bounding box in PDF user-space points [x0, y0, x1, y1]
    public let bbox: [Double]

    /// SHA-256 hash of the NFC-normalized text content (format: "sha256:" + hex)
    public let contentHash: String

    /// The pdftract version that produced this receipt (semver string)
    public let extractionVersion: String

    /// Optional SVG clip rendering the glyphs in this receipt
    public let svgClip: String?

    enum CodingKeys: String, CodingKey {
        case pdfFingerprint = "pdf_fingerprint"
        case pageIndex = "page_index"
        case bbox
        case contentHash = "content_hash"
        case extractionVersion = "extraction_version"
        case svgClip = "svg_clip"
    }
}

/// Receipt for document verification (verify_receipt method).
public struct DocumentReceipt: Codable, Sendable {
    /// SHA-256 hash of document content
    public let hash: String

    /// Cryptographic signature
    public let signature: String

    /// Timestamp (ISO 8601)
    public let timestamp: String
}
