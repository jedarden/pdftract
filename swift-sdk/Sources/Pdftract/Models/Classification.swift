//
//  Classification.swift
//  Pdftract
//
//  Document classification result from the classify command.
//

import Foundation

/// Classification result from document type classifier.
///
/// Contains the detected document type, confidence score, and
/// reasons for the classification.
public struct Classification: Codable, Equatable {
    /// The classified document type.
    /// Examples: "scientific_paper", "invoice", "contract", "misc"
    public let documentType: String

    /// Confidence score [0.0, 1.0].
    public let confidence: Float

    /// Human-readable reasons for the classification (top-K matched predicates).
    public let reasons: [String]

    /// Runner-up profile type (second-highest score), if any.
    public let runnerUp: String?

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case documentType = "document_type"
        case confidence
        case reasons
        case runnerUp = "runner_up"
    }

    /// Create a new Classification structure.
    public init(
        documentType: String,
        confidence: Float,
        reasons: [String] = [],
        runnerUp: String? = nil
    ) {
        self.documentType = documentType
        self.confidence = confidence
        self.reasons = reasons
        self.runnerUp = runnerUp
    }
}

/// Options for classification.
public struct ClassificationOptions: Codable, Equatable {
    /// Number of top reasons to include (default: all).
    public let topK: UInt

    /// Whether to exit with code 1 if document type is unknown.
    public let exitOnUnknown: Bool

    /// Create default classification options.
    public init(
        topK: UInt = 0,
        exitOnUnknown: Bool = false
    ) {
        self.topK = topK
        self.exitOnUnknown = exitOnUnknown
    }

    /// Default classification options.
    public static let `default` = ClassificationOptions()
}
