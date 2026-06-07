// swiftlint:disable all
// Auto-generated from pdftract schema v1.0 - do not edit manually

import Foundation

/// Embedded file attachment.
public struct Attachment: Codable, Sendable {
    /// Filename
    public let filename: String

    /// MIME type (e.g., "application/pdf")
    public let mimeType: String?

    /// Size in bytes
    public let size: Int?

    /// Creation date (ISO 8601)
    public let created: String?

    /// Modification date (ISO 8601)
    public let modified: String?

    enum CodingKeys: String, CodingKey {
        case filename
        case mimeType = "mime_type"
        case size
        case created
        case modified
    }
}
