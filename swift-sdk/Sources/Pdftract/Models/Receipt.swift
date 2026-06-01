//
//  Receipt.swift
//  Pdftract
//
//  Visual citation receipt for extracted text.
//

import Foundation

/// A visual citation receipt for extracted text.
///
/// Receipts provide cryptographic proof that a piece of extracted text
/// originated from a specific region in a specific PDF. They can be
/// verified independently by re-running pdftract on the original file.
///
/// # Lite mode
///
/// In lite mode, `svgClip` is `nil` and the JSON output does not
/// include the key at all. This keeps receipts small (~120-180 bytes)
/// for high-volume use cases like RAG citation pipelines.
///
/// # SVG mode
///
/// In SVG mode, `svgClip` contains a self-contained SVG element
/// that renders only the glyphs whose bboxes fall within the receipt
/// bbox. The SVG is normalized to the bbox coordinate system and
/// can be rendered standalone in any browser.
public struct Receipt: Codable, Equatable {
    /// PDF fingerprint in format "pdftract-v1:<hex>".
    public let pdfFingerprint: String

    /// 0-based page index in the source PDF.
    public let pageIndex: UInt

    /// Bounding box in PDF user-space points [x0, y0, x1, y1].
    public let bbox: [Double]

    /// SHA-256 hash of the NFC-normalized text content.
    /// Format: "sha256:<hex>".
    public let contentHash: String

    /// The pdftract version that produced this receipt.
    public let extractionVersion: String

    /// Optional SVG clip rendering the glyphs in this receipt.
    ///
    /// - `nil` in lite mode (the key is omitted from JSON entirely)
    /// - SVG string in SVG mode, where the SVG is self-contained
    public let svgClip: String?

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case pdfFingerprint = "pdf_fingerprint"
        case pageIndex = "page_index"
        case bbox
        case contentHash = "content_hash"
        case extractionVersion = "extraction_version"
        case svgClip = "svg_clip"
    }

    /// Create a new Receipt structure.
    public init(
        pdfFingerprint: String,
        pageIndex: UInt,
        bbox: [Double],
        contentHash: String,
        extractionVersion: String,
        svgClip: String? = nil
    ) {
        self.pdfFingerprint = pdfFingerprint
        self.pageIndex = pageIndex
        self.bbox = bbox
        self.contentHash = contentHash
        self.extractionVersion = extractionVersion
        self.svgClip = svgClip
    }
}
