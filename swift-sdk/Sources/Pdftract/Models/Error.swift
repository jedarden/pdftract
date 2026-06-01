//
//  Error.swift
//  Pdftract
//
//  Error types for pdftract operations.
//

import Foundation

/// Pdftract error types.
public enum PdftractError: Error, Equatable {
    /// Invalid PDF file format.
    case invalidPdf(String)

    /// I/O error reading or writing files.
    case ioError(String)

    /// Network error when fetching from URL.
    case networkError(String)

    /// Memory allocation failure.
    case outOfMemory

    /// Parse error in PDF structure.
    case parseError(String)

    /// OCR processing error.
    case ocrError(String)

    /// Rendering error when generating page images.
    case renderingError(String)

    /// Generic internal error.
    case internalError(String)

    /// User-friendly error description.
    public var localizedDescription: String {
        switch self {
        case .invalidPdf(let message):
            return "Invalid PDF: \(message)"
        case .ioError(let message):
            return "I/O error: \(message)"
        case .networkError(let message):
            return "Network error: \(message)"
        case .outOfMemory:
            return "Out of memory"
        case .parseError(let message):
            return "Parse error: \(message)"
        case .ocrError(let message):
            return "OCR error: \(message)"
        case .renderingError(let message):
            return "Rendering error: \(message)"
        case .internalError(let message):
            return "Internal error: \(message)"
        }
    }

    /// Error code for programmatic handling.
    public var code: String {
        switch self {
        case .invalidPdf:
            return "INVALID_PDF"
        case .ioError:
            return "IO_ERROR"
        case .networkError:
            return "NETWORK_ERROR"
        case .outOfMemory:
            return "OUT_OF_MEMORY"
        case .parseError:
            return "PARSE_ERROR"
        case .ocrError:
            return "OCR_ERROR"
        case .renderingError:
            return "RENDERING_ERROR"
        case .internalError:
            return "INTERNAL_ERROR"
        }
    }

    /// Equatable conformance
    public static func == (lhs: PdftractError, rhs: PdftractError) -> Bool {
        switch (lhs, rhs) {
        case (.invalidPdf(let a), .invalidPdf(let b)),
             (.ioError(let a), .ioError(let b)),
             (.networkError(let a), .networkError(let b)),
             (.parseError(let a), .parseError(let b)),
             (.ocrError(let a), .ocrError(let b)),
             (.renderingError(let a), .renderingError(let b)),
             (.internalError(let a), .internalError(let b)):
            return a == b
        case (.outOfMemory, .outOfMemory):
            return true
        default:
            return false
        }
    }
}

/// Custom error type for JSON decoding failures with context.
public struct DecodingErrorWrapper: Error {
    /// The underlying decoding error.
    public let underlyingError: Error

    /// Context about what was being decoded.
    public let context: String

    public init(underlyingError: Error, context: String) {
        self.underlyingError = underlyingError
        self.context = context
    }

    public var localizedDescription: String {
        return "Failed to decode \(context): \(underlyingError.localizedDescription)"
    }
}
