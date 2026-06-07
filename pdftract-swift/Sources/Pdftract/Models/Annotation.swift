// swiftlint:disable all
// Auto-generated from pdftract schema v1.0 - do not edit manually

import Foundation

/// PDF annotation.
public struct Annotation: Codable, Sendable {
    /// Annotation subtype (e.g., "Text", "Highlight", "Stamp", "FreeText", "Link")
    public let type: String

    /// Bounding box in PDF user-space points [x0, y0, x1, y1]
    public let rect: [Double]?

    /// Annotation author (from /T)
    public let author: String?

    /// Annotation content text (from /Contents)
    public let contents: String?

    /// Subject (from /Subj)
    public let subject: String?

    /// Modification date (from /M) as ISO 8601 string
    public let modified: String?

    /// Name identifier (from /NM)
    public let nameId: String?

    /// Color array as RGB/Grayscale components
    public let color: [Double]?

    /// Opacity (from /CA)
    public let opacity: Double?

    /// Subtype-specific fields
    public let specific: AnnotationSpecific?
}

/// Subtype-specific annotation fields.
public enum AnnotationSpecific: Codable, Sendable {
    /// Text markup annotations (Highlight, Squiggly, StrikeOut, Underline) with quad points
    case textMarkup(quads: [[Double]])

    /// Stamp annotation with icon name
    case stamp(name: String?)

    /// FreeText annotation with default appearance string
    case freeText(da: String?)

    /// Text (sticky note) annotation
    case text(open: Bool?, state: String?, stateModel: String?)

    /// Other annotation type (raw JSON for extensibility)
    case other

    // Custom decoding to handle the discriminated union
    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let kind = try container.decodeIfPresent(String.self, forKey: .kind)

        switch kind {
        case "text_markup":
            let quads = try container.decode([[Double]].self, forKey: .quads)
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
        default:
            self = .other
        }
    }

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
        case .other:
            break
        }
    }

    private enum CodingKeys: String, CodingKey {
        case kind, quads, name, da, open, state, stateModel
    }
}
