//
//  Match.swift
//  Pdftract
//
//  Search match result from the grep command.
//

import Foundation

/// A single search match result.
///
/// Represents one occurrence of the search pattern in the PDF.
public struct Match: Codable, Equatable {
    /// Page index where the match was found.
    public let pageIndex: UInt

    /// Span index within the page.
    public let spanIndex: UInt

    /// The matched text content.
    public let text: String

    /// Bounding box of the match [x0, y0, x1, y1] in PDF user-space points.
    public let bbox: [Double]

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case pageIndex = "page_index"
        case spanIndex = "span_index"
        case text
        case bbox
    }

    /// Create a new Match structure.
    public init(
        pageIndex: UInt,
        spanIndex: UInt,
        text: String,
        bbox: [Double]
    ) {
        self.pageIndex = pageIndex
        self.spanIndex = spanIndex
        self.text = text
        self.bbox = bbox
    }
}

/// Options for search operations.
public struct SearchOptions: Codable, Equatable {
    /// Case-insensitive search.
    public let caseInsensitive: Bool

    /// Whole word search (pattern must match complete words).
    public let wholeWord: Bool

    /// Regex search (pattern is a regular expression).
    public let regex: Bool

    /// Maximum number of matches to return (0 = unlimited).
    public let maxMatches: UInt

    /// Create default search options.
    public init(
        caseInsensitive: Bool = false,
        wholeWord: Bool = false,
        regex: Bool = false,
        maxMatches: UInt = 0
    ) {
        self.caseInsensitive = caseInsensitive
        self.wholeWord = wholeWord
        self.regex = regex
        self.maxMatches = maxMatches
    }

    /// Default search options.
    public static let `default` = SearchOptions()
}
