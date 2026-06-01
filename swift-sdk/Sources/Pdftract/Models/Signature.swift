//
//  Signature.swift
//  Pdftract
//
//  Digital signature models.
//

import Foundation

/// A digital signature extracted from a PDF signature field.
public struct Signature: Codable, Equatable {
    /// The absolute (dot-joined) field name from the AcroForm.
    public let fieldName: String

    /// The signer's name from the /Name entry in the signature dictionary.
    public let signerName: String

    /// The signing date as an ISO 8601 string (RFC 3339 format).
    public var signingDate: String?

    /// The reason for signing from the /Reason entry.
    public var reason: String?

    /// The location of signing from the /Location entry.
    public var location: String?

    /// The signature format / filter from the /SubFilter entry.
    public var subFilter: String?

    /// The /ByteRange array defining which bytes of the file are signed.
    public var byteRange: [UInt64]?

    /// Fraction of the file covered by the signature (0.0 to 1.0).
    public var coverageFraction: Double?

    /// Validation status — always "not_checked" in v1.
    public let validationStatus: String

    /// Coding keys for custom serialization
    enum CodingKeys: String, CodingKey {
        case fieldName = "field_name"
        case signerName = "signer_name"
        case signingDate = "signing_date"
        case reason
        case location
        case subFilter = "sub_filter"
        case byteRange = "byte_range"
        case coverageFraction = "coverage_fraction"
        case validationStatus = "validation_status"
    }

    /// Create a new Signature structure.
    public init(
        fieldName: String,
        signerName: String,
        signingDate: String? = nil,
        reason: String? = nil,
        location: String? = nil,
        subFilter: String? = nil,
        byteRange: [UInt64]? = nil,
        coverageFraction: Double? = nil,
        validationStatus: String = "not_checked"
    ) {
        self.fieldName = fieldName
        self.signerName = signerName
        self.signingDate = signingDate
        self.reason = reason
        self.location = location
        self.subFilter = subFilter
        self.byteRange = byteRange
        self.coverageFraction = coverageFraction
        self.validationStatus = validationStatus
    }
}
