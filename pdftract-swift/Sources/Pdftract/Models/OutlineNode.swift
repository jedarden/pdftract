// swiftlint:disable all
// Auto-generated from pdftract schema v1.0 - do not edit manually

import Foundation

/// Outline node (bookmark/hierarchy entry).
public struct OutlineNode: Codable, Sendable {
    /// Title of the outline entry
    public let title: String

    /// Zero-based page index this points to
    public let pageIndex: Int

    /// Zero-based character offset within the page
    public let destIndex: Int?

    /// Child outline entries
    public let children: [OutlineNode]

    enum CodingKeys: String, CodingKey {
        case title
        case pageIndex = "page_index"
        case destIndex = "dest_index"
        case children
    }
}
