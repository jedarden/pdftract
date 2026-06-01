//
//  Fingerprint.swift
//  Pdftract
//
//  Cryptographic fingerprint (hash) of a PDF document.
//

import Foundation

/// Cryptographic fingerprint of a PDF document.
///
/// Contains the PDF fingerprint identifier used for receipt generation.
/// The fingerprint is in the format "pdftract-v1:<sha256_prefix>".
public struct Fingerprint: Codable, Equatable {
    /// Fingerprint identifier in format "pdftract-v1:<sha256_prefix>".
    public let id: String

    /// Create a new Fingerprint structure.
    public init(id: String) {
        self.id = id
    }
}

/// Options for hash computation.
public struct HashOptions: Codable, Equatable {
    /// Whether to compute MD5 hash (default: true).
    public let includeMd5: Bool

    /// Whether to compute structural hash (default: true).
    public let includeStructure: Bool

    /// Create default hash options.
    public init(
        includeMd5: Bool = true,
        includeStructure: Bool = true
    ) {
        self.includeMd5 = includeMd5
        self.includeStructure = includeStructure
    }

    /// Default hash options.
    public static let `default` = HashOptions()
}
