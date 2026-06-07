// swiftlint:disable all
// Auto-generated from pdftract schema v1.0 - do not edit manually

import Foundation

/// Diagnostic message emitted during extraction.
public struct Diagnostic: Codable, Sendable {
    /// Severity level: "info", "warning", "error"
    public let severity: String

    /// Diagnostic message
    public let message: String

    /// Optional page index (0-based)
    public let pageIndex: Int?

    enum CodingKeys: String, CodingKey {
        case severity
        case message
        case pageIndex = "page_index"
    }
}
