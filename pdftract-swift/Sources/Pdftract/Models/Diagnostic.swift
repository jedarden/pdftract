// swiftlint:disable all
// Auto-generated from pdftract schema v1.0 - do not edit manually

import Foundation

/// Canonical structured diagnostic emitted during extraction.
///
/// Full JSON `errors` and the NDJSON footer use this model. The compact Rust
/// metadata API retains its legacy `[String]` diagnostics array separately;
/// use `Diagnostic` when code or typed context is required.
public struct Diagnostic: Codable, Sendable {
    /// Stable diagnostic code, such as `STREAM_DECODE_ERROR`.
    public let code: String

    /// Human-readable diagnostic message.
    public let message: String

    /// Severity level: "info", "warning", "error", or "fatal".
    public let severity: String

    /// Optional zero-based page index. Missing values are omitted when encoded.
    public let pageIndex: Int?

    /// Optional PDF indirect-object location.
    public let location: DiagnosticLocation?

    /// Optional catalog hint. Missing values are omitted when encoded.
    public let hint: String?

    public init(
        code: String,
        message: String,
        severity: String,
        pageIndex: Int? = nil,
        location: DiagnosticLocation? = nil,
        hint: String? = nil
    ) {
        self.code = code
        self.message = message
        self.severity = severity
        self.pageIndex = pageIndex
        self.location = location
        self.hint = hint
    }

    enum CodingKeys: String, CodingKey {
        case code
        case message
        case severity
        case pageIndex = "page_index"
        case location
        case hint
    }
}

/// PDF indirect-object location attached to a diagnostic.
public struct DiagnosticLocation: Codable, Sendable {
    public let objectNumber: Int
    public let generationNumber: Int

    public init(objectNumber: Int, generationNumber: Int) {
        self.objectNumber = objectNumber
        self.generationNumber = generationNumber
    }

    enum CodingKeys: String, CodingKey {
        case objectNumber = "object_number"
        case generationNumber = "generation_number"
    }
}
