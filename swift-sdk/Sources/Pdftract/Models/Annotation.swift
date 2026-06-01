//
//  Annotation.swift
//  Pdftract
//
//  Annotation models for extracted PDF content.
//

import Foundation

/// A hyperlink annotation (URI or internal destination).
public struct Link: Codable, Equatable {
    /// Zero-based page index containing this link.
    public let pageIndex: UInt

    /// Bounding box in PDF user-space points.
    public let rect: [Float]

    /// The URI target for external links.
    public var uri: String?

    /// The internal destination name (from /Dest as a name string).
    public var dest: String?

    /// Explicit destination array (from /Dest as an array or resolved name tree).
    public var destArray: DestinationArray?

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case pageIndex = "page_index"
        case rect
        case uri
        case dest
        case destArray = "dest_array"
    }

    /// Create a new Link structure.
    public init(
        pageIndex: UInt,
        rect: [Float],
        uri: String? = nil,
        dest: String? = nil,
        destArray: DestinationArray? = nil
    ) {
        self.pageIndex = pageIndex
        self.rect = rect
        self.uri = uri
        self.dest = dest
        self.destArray = destArray
    }
}

/// An explicit destination array.
public struct DestinationArray: Codable, Equatable {
    /// Zero-based page index within the document.
    public let pageIndex: UInt

    /// Destination type and coordinates.
    public let dest: DestinationType

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case pageIndex = "page_index"
        case dest
    }

    /// Create a new DestinationArray structure.
    public init(pageIndex: UInt, dest: DestinationType) {
        self.pageIndex = pageIndex
        self.dest = dest
    }
}

/// Destination type with coordinates.
public enum DestinationType: Codable, Equatable {
    case xyz(left: Double?, top: Double?, zoom: Double?)
    case fit
    case fitH(top: Double?)
    case fitV(left: Double?)
    case fitR(left: Double, bottom: Double, right: Double, top: Double)
    case fitB
    case fitBH(top: Double?)
    case fitBV(left: Double?)

    /// Custom coding for tag-based representation
    enum CodingKeys: String, CodingKey {
        case fit
        case left
        case top
        case zoom
        case bottom
        case right
    }

    /// Create a new DestinationType from a decoder.
    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let fit = try container.decode(String.self, forKey: .fit)

        switch fit.lowercased() {
        case "xyz":
            let left = try container.decodeIfPresent(Double.self, forKey: .left)
            let top = try container.decodeIfPresent(Double.self, forKey: .top)
            let zoom = try container.decodeIfPresent(Double.self, forKey: .zoom)
            self = .xyz(left: left, top: top, zoom: zoom)
        case "fit":
            self = .fit
        case "fith":
            let top = try container.decodeIfPresent(Double.self, forKey: .top)
            self = .fitH(top: top)
        case "fitv":
            let left = try container.decodeIfPresent(Double.self, forKey: .left)
            self = .fitV(left: left)
        case "fitr":
            let left = try container.decode(Double.self, forKey: .left)
            let bottom = try container.decode(Double.self, forKey: .bottom)
            let right = try container.decode(Double.self, forKey: .right)
            let top = try container.decode(Double.self, forKey: .top)
            self = .fitR(left: left, bottom: bottom, right: right, top: top)
        case "fitb":
            self = .fitB
        case "fitbh":
            let top = try container.decodeIfPresent(Double.self, forKey: .top)
            self = .fitBH(top: top)
        case "fitbv":
            let left = try container.decodeIfPresent(Double.self, forKey: .left)
            self = .fitBV(left: left)
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .fit,
                in: container,
                debugDescription: "Invalid fit value: \(fit)"
            )
        }
    }

    /// Encode a DestinationType to an encoder.
    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)

        switch self {
        case .xyz(let left, let top, let zoom):
            try container.encode("xyz", forKey: .fit)
            try container.encodeIfPresent(left, forKey: .left)
            try container.encodeIfPresent(top, forKey: .top)
            try container.encodeIfPresent(zoom, forKey: .zoom)
        case .fit:
            try container.encode("fit", forKey: .fit)
        case .fitH(let top):
            try container.encode("fith", forKey: .fit)
            try container.encodeIfPresent(top, forKey: .top)
        case .fitV(let left):
            try container.encode("fitv", forKey: .fit)
            try container.encodeIfPresent(left, forKey: .left)
        case .fitR(let left, let bottom, let right, let top):
            try container.encode("fitr", forKey: .fit)
            try container.encode(left, forKey: .left)
            try container.encode(bottom, forKey: .bottom)
            try container.encode(right, forKey: .right)
            try container.encode(top, forKey: .top)
        case .fitB:
            try container.encode("fitb", forKey: .fit)
        case .fitBH(let top):
            try container.encode("fitbh", forKey: .fit)
            try container.encodeIfPresent(top, forKey: .top)
        case .fitBV(let left):
            try container.encode("fitbv", forKey: .fit)
            try container.encodeIfPresent(left, forKey: .left)
        }
    }
}

/// A non-link annotation (highlight, text note, stamp, etc.).
public struct Annotation: Codable, Equatable {
    /// Annotation subtype (e.g., "Text", "Highlight", "Stamp", "FreeText").
    public let subtype: String

    /// Bounding box in PDF user-space points.
    public var rect: [Float]?

    /// The annotation's content text (from /Contents).
    public var contents: String?

    /// The annotation's author (from /T).
    public var author: String?

    /// The modification date (from /M) as an ISO 8601 string.
    public var modified: String?

    /// The color array (from /C) as RGB/Grayscale components.
    public var color: [Float]?

    /// The opacity (from /CA).
    public var opacity: Float?

    /// The name identifier (from /NM).
    public var nameId: String?

    /// The subject (from /Subj).
    public var subject: String?

    /// Subtype-specific fields.
    public var specific: AnnotationSpecific?

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case subtype = "type"
        case rect
        case contents
        case author
        case modified
        case color
        case opacity
        case nameId = "name_id"
        case subject
        case specific
    }

    /// Create a new Annotation structure.
    public init(
        subtype: String,
        rect: [Float]? = nil,
        contents: String? = nil,
        author: String? = nil,
        modified: String? = nil,
        color: [Float]? = nil,
        opacity: Float? = nil,
        nameId: String? = nil,
        subject: String? = nil,
        specific: AnnotationSpecific? = nil
    ) {
        self.subtype = subtype
        self.rect = rect
        self.contents = contents
        self.author = author
        self.modified = modified
        self.color = color
        self.opacity = opacity
        self.nameId = nameId
        self.subject = subject
        self.specific = specific
    }
}

/// Subtype-specific annotation fields.
public enum AnnotationSpecific: Codable, Equatable {
    case textMarkup(quads: [[Float]])
    case stamp(name: String?)
    case freeText(da: String?)
    case text(open: Bool?, state: String?, stateModel: String?)
    case ink(strokes: [[[Float]]])
    case line(endpoints: [Float]?)
    case polygon(vertices: [[Float]])
    case fileAttachment(fsRef: UInt32?)
    case other

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case kind
        case quads
        case name
        case da
        case open
        case state
        case stateModel = "state_model"
        case strokes
        case endpoints
        case vertices
        case fsRef = "fs_ref"
    }

    /// Create a new AnnotationSpecific from a decoder.
    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let kind = try container.decode(String.self, forKey: .kind)

        switch kind {
        case "text_markup":
            let quads = try container.decode([[Float]].self, forKey: .quads)
            self = .textMarkup(quads: quads)
        case "stamp":
            let name = try container.decodeIfPresent(String.self, forKey: .name)
            self = .stamp(name: name)
        case "free_text":
            let da = try container.decodeIfPresent(String.self, forKey: .da)
            self = .freeText(da: da)
        case "text":
            let open = try container.decodeIfPresent(Bool.self, forKey: .open)
            let state = try container.decodeIfPresent(String.self, forKey: .state)
            let stateModel = try container.decodeIfPresent(String.self, forKey: .stateModel)
            self = .text(open: open, state: state, stateModel: stateModel)
        case "ink":
            let strokes = try container.decode([[[Float]]].self, forKey: .strokes)
            self = .ink(strokes: strokes)
        case "line":
            let endpoints = try container.decodeIfPresent([Float].self, forKey: .endpoints)
            self = .line(endpoints: endpoints)
        case "polygon":
            let vertices = try container.decode([[Float]].self, forKey: .vertices)
            self = .polygon(vertices: vertices)
        case "file_attachment":
            let fsRef = try container.decodeIfPresent(UInt32.self, forKey: .fsRef)
            self = .fileAttachment(fsRef: fsRef)
        default:
            self = .other
        }
    }

    /// Encode an AnnotationSpecific to an encoder.
    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)

        switch self {
        case .textMarkup(let quads):
            try container.encode("text_markup", forKey: .kind)
            try container.encode(quads, forKey: .quads)
        case .stamp(let name):
            try container.encode("stamp", forKey: .kind)
            try container.encodeIfPresent(name, forKey: .name)
        case .freeText(let da):
            try container.encode("free_text", forKey: .kind)
            try container.encodeIfPresent(da, forKey: .da)
        case .text(let open, let state, let stateModel):
            try container.encode("text", forKey: .kind)
            try container.encodeIfPresent(open, forKey: .open)
            try container.encodeIfPresent(state, forKey: .state)
            try container.encodeIfPresent(stateModel, forKey: .stateModel)
        case .ink(let strokes):
            try container.encode("ink", forKey: .kind)
            try container.encode(strokes, forKey: .strokes)
        case .line(let endpoints):
            try container.encode("line", forKey: .kind)
            try container.encodeIfPresent(endpoints, forKey: .endpoints)
        case .polygon(let vertices):
            try container.encode("polygon", forKey: .kind)
            try container.encode(vertices, forKey: .vertices)
        case .fileAttachment(let fsRef):
            try container.encode("file_attachment", forKey: .kind)
            try container.encodeIfPresent(fsRef, forKey: .fsRef)
        case .other:
            try container.encode("other", forKey: .kind)
        }
    }
}
