//
//  Pdftract.swift
//  Pdftract
//
//  Main Pdftract client struct with public API.
//

import Foundation

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

/// Main Pdftract client for PDF extraction.
///
/// This struct provides async methods for extracting content from PDFs
/// by spawning a pdftract binary subprocess and parsing its JSON output.
public struct Pdftract {
    /// Path to the pdftract executable.
    private let binaryPath: String

    /// Create a new Pdftract client.
    ///
    /// - Parameter binaryPath: Path to the pdftract binary (default: "pdftract").
    public init(binaryPath: String = "pdftract") {
        self.binaryPath = binaryPath
    }
}

/// Source enum for PDF input.
public enum Source {
    /// PDF from a file path.
    case path(String)

    /// PDF from a URL.
    case url(URL)

    /// PDF from raw bytes.
    case bytes(Data)
}
