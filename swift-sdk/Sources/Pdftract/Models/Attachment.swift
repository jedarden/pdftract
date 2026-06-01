//
//  Attachment.swift
//  Pdftract
//
//  Attachment models.
//

import Foundation

/// An embedded file attachment extracted from a PDF.
public struct Attachment: Codable, Equatable {
    /// Attachment filename from /UF (Unicode, preferred) or /F (system-independent).
    public let name: String

    /// Description from /Desc (None if absent, not empty string).
    public var description: String?

    /// MIME type from stream /Subtype (None if absent, no guessing from extension).
    public var mimeType: String?

    /// Original decoded size in bytes (always populated, even when truncated).
    public let size: UInt64

    /// Creation date from /Params /CreationDate as ISO 8601 string (None if absent).
    public var created: String?

    /// Modification date from /Params /ModDate as ISO 8601 string (None if absent).
    public var modified: String?

    /// MD5 checksum from /Params /CheckSum as hex string (None if absent).
    public var checksumMd5: String?

    /// Base64-encoded attachment content (null if truncated or empty).
    public var data: String?

    /// Whether the attachment content was truncated due to the 50 MB size limit.
    public let truncated: Bool

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case name
        case description
        case mimeType = "mime_type"
        case size
        case created
        case modified
        case checksumMd5 = "checksum_md5"
        case data
        case truncated
    }

    /// Create a new Attachment structure.
    public init(
        name: String,
        description: String? = nil,
        mimeType: String? = nil,
        size: UInt64,
        created: String? = nil,
        modified: String? = nil,
        checksumMd5: String? = nil,
        data: String? = nil,
        truncated: Bool = false
    ) {
        self.name = name
        self.description = description
        self.mimeType = mimeType
        self.size = size
        self.created = created
        self.modified = modified
        self.checksumMd5 = checksumMd5
        self.data = data
        self.truncated = truncated
    }
}

/// An article thread extracted from the PDF's /Threads array.
public struct Thread: Codable, Equatable {
    /// Thread title from /I/Title.
    public var title: String?

    /// Thread author from /I/Author.
    public var author: String?

    /// Thread subject from /I/Subject.
    public var subject: String?

    /// Thread keywords from /I/Keywords.
    public var keywords: String?

    /// Beads in this thread chain, in traversal order.
    public var beads: [Bead]

    /// Create a new Thread structure.
    public init(
        title: String? = nil,
        author: String? = nil,
        subject: String? = nil,
        keywords: String? = nil,
        beads: [Bead] = []
    ) {
        self.title = title
        self.author = author
        self.subject = subject
        self.keywords = keywords
        self.beads = beads
    }
}

/// A single bead in an article thread chain.
public struct Bead: Codable, Equatable {
    /// 0-based page index where this bead is located.
    public let pageIndex: UInt

    /// Bounding rectangle in PDF user-space coordinates [x0, y0, x1, y1].
    public let rect: [Float]

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case pageIndex = "page_index"
        case rect
    }

    /// Create a new Bead structure.
    public init(pageIndex: UInt, rect: [Float]) {
        self.pageIndex = pageIndex
        self.rect = rect
    }
}

/// An outline node (bookmark) from the document's outline hierarchy.
public struct OutlineNode: Codable, Equatable {
    /// The outline title text (decoded to UTF-8).
    public let title: String

    /// Hierarchical level in the outline tree (0-based, root is 0).
    public let level: UInt8

    /// Zero-based page index this outline points to, if resolved.
    public var pageIndex: UInt32?

    /// Destination type and coordinates within the page.
    public var destination: Destination?

    /// Nested child outlines (empty array for leaf nodes).
    public var children: [OutlineNode]

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case title
        case level
        case pageIndex = "page_index"
        case destination = "dest"
        case children
    }

    /// Create a new OutlineNode structure.
    public init(
        title: String,
        level: UInt8 = 0,
        pageIndex: UInt32? = nil,
        destination: Destination? = nil,
        children: [OutlineNode] = []
    ) {
        self.title = title
        self.level = level
        self.pageIndex = pageIndex
        self.destination = destination
        self.children = children
    }
}

/// A destination anchor describing a specific location within a PDF page.
public struct Destination: Codable, Equatable {
    /// Destination type: "xyz", "fit", "fith", "fitv", "fitr", "fitb", "fitbh", "fitbv".
    public let destType: String

    /// Left coordinate (user-space points), present for "xyz", "fitv", "fitr", "fitbv".
    public var left: Double?

    /// Top coordinate (user-space points), present for "xyz", "fith", "fitr", "fitbh".
    public var top: Double?

    /// Right coordinate (user-space points), present only for "fitr".
    public var right: Double?

    /// Bottom coordinate (user-space points), present only for "fitr".
    public var bottom: Double?

    /// Zoom factor, present only for "xyz".
    public var zoom: Double?

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case destType = "type"
        case left
        case top
        case right
        case bottom
        case zoom
    }

    /// Create a new Destination structure.
    public init(
        destType: String,
        left: Double? = nil,
        top: Double? = nil,
        right: Double? = nil,
        bottom: Double? = nil,
        zoom: Double? = nil
    ) {
        self.destType = destType
        self.left = left
        self.top = top
        self.right = right
        self.bottom = bottom
        self.zoom = zoom
    }
}
