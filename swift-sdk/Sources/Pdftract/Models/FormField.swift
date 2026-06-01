//
//  FormField.swift
//  Pdftract
//
//  Form field models.
//

import Foundation

/// A form field extracted from a PDF's AcroForm or XFA data.
public struct FormField: Codable, Equatable {
    /// The absolute (dot-joined) field name from the AcroForm.
    public let name: String

    /// The field type.
    public let fieldType: FormFieldType

    /// The current value of the form field.
    public var value: FormFieldValue

    /// The default value (/DV entry) if present.
    public var defaultValue: FormFieldValue?

    /// Zero-based page index where this field's widget appears.
    public var pageIndex: UInt?

    /// Bounding box in PDF user-space points.
    public var rect: [Float]?

    /// Whether this field is required (bit 2 of /Ff flags).
    public let required: Bool

    /// Whether this field is read-only (bit 1 of /Ff flags).
    public let readOnly: Bool

    /// Whether this text field supports multiple lines.
    public var multiline: Bool?

    /// Maximum length for text fields (/MaxLen entry).
    public var maxLength: UInt32?

    /// Available options for choice fields.
    public var options: [[String]]?

    /// Whether this choice field supports multiple selections.
    public var multiSelect: Bool?

    /// Selected state for button fields.
    public var selected: Bool?

    /// Appearance state name for button fields.
    public var stateName: String?

    /// Whether this button is a pushbutton (bit 26 of /Ff).
    public var pushbutton: Bool?

    /// Whether this button is a radio button (bit 25 of /Ff).
    public var radio: Bool?

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case name
        case fieldType = "type"
        case value
        case defaultValue = "default"
        case pageIndex = "page_index"
        case rect
        case required
        case readOnly = "read_only"
        case multiline
        case maxLength = "max_length"
        case options
        case multiSelect = "multi_select"
        case selected
        case stateName = "state_name"
        case pushbutton
        case radio
    }

    /// Create a new FormField structure.
    public init(
        name: String,
        fieldType: FormFieldType,
        value: FormFieldValue,
        defaultValue: FormFieldValue? = nil,
        pageIndex: UInt? = nil,
        rect: [Float]? = nil,
        required: Bool = false,
        readOnly: Bool = false,
        multiline: Bool? = nil,
        maxLength: UInt32? = nil,
        options: [[String]]? = nil,
        multiSelect: Bool? = nil,
        selected: Bool? = nil,
        stateName: String? = nil,
        pushbutton: Bool? = nil,
        radio: Bool? = nil
    ) {
        self.name = name
        self.fieldType = fieldType
        self.value = value
        self.defaultValue = defaultValue
        self.pageIndex = pageIndex
        self.rect = rect
        self.required = required
        self.readOnly = readOnly
        self.multiline = multiline
        self.maxLength = maxLength
        self.options = options
        self.multiSelect = multiSelect
        self.selected = selected
        self.stateName = stateName
        self.pushButton = pushbutton
        self.radio = radio
    }
}

/// Form field type discriminator.
public enum FormFieldType: String, Codable {
    case text
    case button
    case choice
    case signature
}

/// Form field value representation.
public enum FormFieldValue: Codable, Equatable {
    case text(String?)
    case button(Bool)
    case choice(ChoiceValue)
    case signature(UInt32?)

    /// Create a new FormFieldValue from a decoder.
    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()

        if let stringValue = try? container.decode(String.self) {
            self = .text(stringValue)
        } else if let boolValue = try? container.decode(Bool.self) {
            self = .button(boolValue)
        } else if let arrayValue = try? container.decode([String].self) {
            self = .choice(.multiple(arrayValue))
        } else if let intValue = try? container.decode(UInt32.self) {
            self = .signature(intValue)
        } else if container.decodeNil() {
            // Need context to determine which type of nil
            // Default to text(nil) for backward compatibility
            self = .text(nil)
        } else {
            throw DecodingError.dataCorruptedError(
                in: container,
                debugDescription: "FormFieldValue cannot be decoded"
            )
        }
    }

    /// Encode a FormFieldValue to an encoder.
    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()

        switch self {
        case .text(let value):
            try container.encode(value)
        case .button(let value):
            try container.encode(value)
        case .choice(let choiceValue):
            switch choiceValue {
            case .single(let value):
                try container.encode(value)
            case .multiple(let values):
                try container.encode(values)
            }
        case .signature(let value):
            try container.encode(value)
        }
    }
}

/// Choice field value representation.
public enum ChoiceValue: Codable, Equatable {
    case single(String)
    case multiple([String])

    /// Create a new ChoiceValue from a decoder.
    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()

        if let stringValue = try? container.decode(String.self) {
            self = .single(stringValue)
        } else if let arrayValue = try? container.decode([String].self) {
            self = .multiple(arrayValue)
        } else {
            throw DecodingError.dataCorruptedError(
                in: container,
                debugDescription: "ChoiceValue cannot be decoded"
            )
        }
    }

    /// Encode a ChoiceValue to an encoder.
    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()

        switch self {
        case .single(let value):
            try container.encode(value)
        case .multiple(let values):
            try container.encode(values)
        }
    }
}
