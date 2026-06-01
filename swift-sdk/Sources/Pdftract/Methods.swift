//
//  Methods.swift
//  Pdftract
//
//  The 9 contract methods for PDF extraction.
//

import Foundation

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

// MARK: - Pdftract Method Extensions

extension Pdftract {

    /// MARK: - 1. Extract Full Structured Data

    /// Extract full structured data from a PDF.
    ///
    /// - Parameters:
    ///   - source: The PDF source (path, URL, or bytes).
    ///   - options: Extraction options controlling what to extract.
    /// - Returns: A fully parsed `Document` with all requested content.
    /// - Throws: `PdftractError` if extraction fails.
    public func extract(
        from source: Source,
        options: ExtractionOptions = .default
    ) async throws -> Document {
        let arguments = buildArguments(for: source, options: options)
        let jsonData = try await runProcess(arguments: arguments, stdin: dataForSource(source))

        let decoder = JSONDecoder()
        do {
            return try decoder.decode(Document.self, from: jsonData)
        } catch let DecodingError.dataCorrupted(context) {
            throw PdftractError.parseError("Data corrupted: \(context.debugDescription)")
        } catch let DecodingError.keyNotFound(key, context) {
            throw PdftractError.parseError("Key '\(key.stringValue)' not found: \(context.debugDescription)")
        } catch let DecodingError.typeMismatch(type, context) {
            throw PdftractError.parseError("Type mismatch for \(type): \(context.debugDescription)")
        } catch let DecodingError.valueNotFound(type, context) {
            throw PdftractError.parseError("Value not found for \(type): \(context.debugDescription)")
        } catch {
            throw PdftractError.parseError("Failed to decode document: \(error.localizedDescription)")
        }
    }

    /// MARK: - 2. Extract Text

    /// Extract only text content from a PDF.
    ///
    /// Returns concatenated text from all pages, preserving whitespace
    /// and basic formatting.
    ///
    /// - Parameters:
    ///   - source: The PDF source (path, URL, or bytes).
    ///   - options: Text extraction options.
    /// - Returns: The extracted text as a string.
    /// - Throws: `PdftractError` if extraction fails.
    public func extractText(
        from source: Source,
        options: TextOptions = .default
    ) async throws -> String {
        let arguments = buildTextArguments(for: source, options: options)
        let outputData = try await runProcess(arguments: arguments, stdin: dataForSource(source))

        guard let text = String(data: outputData, encoding: .utf8) else {
            throw PdftractError.parseError("Failed to decode text output")
        }

        return text
    }

    /// MARK: - 3. Extract Markdown

    /// Extract content from a PDF as Markdown.
    ///
    /// Converts PDF structure (headings, lists, tables, links) to Markdown format.
    ///
    /// - Parameters:
    ///   - source: The PDF source (path, URL, or bytes).
    ///   - options: Markdown extraction options.
    /// - Returns: The extracted content as Markdown.
    /// - Throws: `PdftractError` if extraction fails.
    public func extractMarkdown(
        from source: Source,
        options: MarkdownOptions = .default
    ) async throws -> String {
        let arguments = buildMarkdownArguments(for: source, options: options)
        let outputData = try await runProcess(arguments: arguments, stdin: dataForSource(source))

        guard let markdown = String(data: outputData, encoding: .utf8) else {
            throw PdftractError.parseError("Failed to decode markdown output")
        }

        return markdown
    }

    /// MARK: - 4. Extract Stream (Async Pages)

    /// Extract full structured data with async streaming of pages.
    ///
    /// This method yields pages as they are extracted, rather than waiting
    /// for the entire document to complete. Useful for large PDFs.
    ///
    /// - Parameters:
    ///   - source: The PDF source (path, URL, or bytes).
    ///   - options: Extraction options controlling what to extract.
    /// - Returns: An `AsyncThrowingStream` that yields `Page` objects.
    public func extractStream(
        from source: Source,
        options: ExtractionOptions = .default
    ) -> AsyncThrowingStream<Page, Error> {
        let arguments = buildArguments(for: source, options: options)
        let stdinData = dataForSource(source)

        return AsyncThrowingStream { continuation in
            Task {
                do {
                    let process = Process()
                    let stdinPipe = Pipe()
                    let stdoutPipe = Pipe()
                    let stderrPipe = Pipe()

                    process.executableURL = URL(fileURLWithPath: binaryPath)
                    process.arguments = arguments
                    process.standardInput = stdinPipe
                    process.standardOutput = stdoutPipe
                    process.standardError = stderrPipe

                    // Launch process
                    process.launch()

                    // Write stdin data if needed
                    if let data = stdinData {
                        stdinPipe.fileHandleForWriting.write(data)
                        stdinPipe.fileHandleForWriting.closeFile()
                    } else {
                        stdinPipe.fileHandleForWriting.closeFile()
                    }

                    // Read NDJSON lines from stdout
                    let stdoutHandle = stdoutPipe.fileHandleForReading
                    let decoder = JSONDecoder()
                    var buffer = Data()

                    while true {
                        let chunk = stdoutHandle.availableData
                        if chunk.isEmpty {
                            break
                        }

                        buffer.append(chunk)

                        // Process complete lines
                        while let lineEnd = buffer.firstIndex(of: UInt8(0x0A)) {
                            let lineData = buffer[..<lineEnd]
                            buffer.removeSubrange(...lineEnd)

                            // Skip empty lines
                            if lineData.isEmpty { continue }

                            // Decode JSON object
                            do {
                                let page = try decoder.decode(Page.self, from: lineData)
                                continuation.yield(page)
                            } catch {
                                continuation.finish(throwing: PdftractError.parseError(
                                    "Failed to decode page: \(error.localizedDescription)"
                                ))
                                return
                            }
                        }
                    }

                    // Wait for process to finish
                    process.waitUntilExit()

                    let exitCode = process.terminationStatus
                    if exitCode != 0 {
                        let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
                        if let stderr = String(data: stderrData, encoding: .utf8), !stderr.isEmpty {
                            continuation.finish(throwing: PdftractError.internalError(stderr))
                        } else {
                            continuation.finish(throwing: PdftractError.internalError("Process exited with code \(exitCode)"))
                        }
                        return
                    }

                    continuation.finish()

                } catch {
                    continuation.finish(throwing: error)
                }
            }
        }
    }

    /// MARK: - 5. Search

    /// Search for a pattern in a PDF.
    ///
    /// Returns matches as they are found via async streaming.
    ///
    /// - Parameters:
    ///   - source: The PDF source (path, URL, or bytes).
    ///   - pattern: The search pattern (text or regex).
    ///   - options: Search options.
    /// - Returns: An `AsyncThrowingStream` that yields `Match` objects.
    public func search(
        source: Source,
        pattern: String,
        options: SearchOptions = .default
    ) -> AsyncThrowingStream<Match, Error> {
        let arguments = buildSearchArguments(for: source, pattern: pattern, options: options)
        let stdinData = dataForSource(source)

        return AsyncThrowingStream { continuation in
            Task {
                do {
                    let process = Process()
                    let stdinPipe = Pipe()
                    let stdoutPipe = Pipe()
                    let stderrPipe = Pipe()

                    process.executableURL = URL(fileURLWithPath: binaryPath)
                    process.arguments = arguments
                    process.standardInput = stdinPipe
                    process.standardOutput = stdoutPipe
                    process.standardError = stderrPipe

                    // Launch process
                    process.launch()

                    // Write stdin data if needed
                    if let data = stdinData {
                        stdinPipe.fileHandleForWriting.write(data)
                        stdinPipe.fileHandleForWriting.closeFile()
                    } else {
                        stdinPipe.fileHandleForWriting.closeFile()
                    }

                    // Read NDJSON lines from stdout
                    let stdoutHandle = stdoutPipe.fileHandleForReading
                    let decoder = JSONDecoder()
                    var buffer = Data()

                    while true {
                        let chunk = stdoutHandle.availableData
                        if chunk.isEmpty {
                            break
                        }

                        buffer.append(chunk)

                        // Process complete lines
                        while let lineEnd = buffer.firstIndex(of: UInt8(0x0A)) {
                            let lineData = buffer[..<lineEnd]
                            buffer.removeSubrange(...lineEnd)

                            // Skip empty lines
                            if lineData.isEmpty { continue }

                            // Decode JSON object
                            do {
                                let match = try decoder.decode(Match.self, from: lineData)
                                continuation.yield(match)
                            } catch {
                                continuation.finish(throwing: PdftractError.parseError(
                                    "Failed to decode match: \(error.localizedDescription)"
                                ))
                                return
                            }
                        }
                    }

                    // Wait for process to finish
                    process.waitUntilExit()

                    let exitCode = process.terminationStatus
                    if exitCode != 0 {
                        let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
                        if let stderr = String(data: stderrData, encoding: .utf8), !stderr.isEmpty {
                            continuation.finish(throwing: PdftractError.internalError(stderr))
                        } else {
                            continuation.finish(throwing: PdftractError.internalError("Process exited with code \(exitCode)"))
                        }
                        return
                    }

                    continuation.finish()

                } catch {
                    continuation.finish(throwing: error)
                }
            }
        }
    }

    /// MARK: - 6. Get Metadata

    /// Extract only document metadata (no page content).
    ///
    /// Useful for quick inspection of PDF properties like page count,
    /// title, author, PDF version, etc.
    ///
    /// - Parameter source: The PDF source (path, URL, or bytes).
    /// - Returns: The document metadata.
    /// - Throws: `PdftractError` if extraction fails.
    public func getMetadata(from source: Source) async throws -> ExtractionMetadata {
        // Extract with minimal options to get metadata
        let minimalOptions = ExtractionOptions(
            extractSpans: false,
            extractBlocks: false,
            extractTables: false,
            extractAnnotations: false,
            extractFormFields: false,
            extractSignatures: false,
            extractAttachments: false,
            extractOutline: false,
            extractThreads: false,
            extractLinks: false,
            includeQuality: false,
            includeErrors: false
        )

        let document = try await extract(from: source, options: minimalOptions)
        return document.metadata
    }

    /// MARK: - 7. Hash (Fingerprint)

    /// Compute cryptographic fingerprint (hash) of a PDF.
    ///
    /// Returns the PDF fingerprint identifier for receipt generation.
    /// The fingerprint is in the format "pdftract-v1:<sha256_prefix>".
    ///
    /// - Parameter source: The PDF source (path, URL, or bytes).
    /// - Returns: A `Fingerprint` containing the PDF fingerprint identifier.
    /// - Throws: `PdftractError` if hashing fails.
    public func hash(source: Source) async throws -> Fingerprint {
        let arguments = buildHashArguments(for: source)
        let outputData = try await runProcess(arguments: arguments, stdin: dataForSource(source))

        guard let fingerprint = String(data: outputData, encoding: .utf8) else {
            throw PdftractError.parseError("Failed to decode fingerprint output")
        }

        return Fingerprint(id: fingerprint.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    /// MARK: - 8. Classify

    /// Classify a PDF document type.
    ///
    /// Determines the document type (e.g., scientific_paper, invoice, contract, misc)
    /// with a confidence score and reasons.
    ///
    /// - Parameter source: The PDF source (path, URL, or bytes).
    /// - Returns: A `Classification` with document type and confidence.
    /// - Throws: `PdftractError` if classification fails.
    public func classify(source: Source) async throws -> Classification {
        let arguments = buildClassifyArguments(for: source)
        let jsonData = try await runProcess(arguments: arguments, stdin: dataForSource(source))

        let decoder = JSONDecoder()
        do {
            return try decoder.decode(Classification.self, from: jsonData)
        } catch {
            throw PdftractError.parseError("Failed to decode classification: \(error.localizedDescription)")
        }
    }

    /// MARK: - 9. Verify Receipt

    /// Verify a receipt for a PDF document.
    ///
    /// Validates that a receipt matches the PDF fingerprint and content.
    ///
    /// - Parameters:
    ///   - path: Path to the PDF file.
    ///   - receipt: The receipt to verify.
    /// - Returns: `true` if the receipt is valid, `false` otherwise.
    /// - Throws: `PdftractError` if verification fails.
    public func verifyReceipt(path: String, receipt: String) async throws -> Bool {
        let arguments = buildVerifyReceiptArguments(path: path, receipt: receipt)
        let outputData = try await runProcess(arguments: arguments, stdin: nil)

        guard let output = String(data: outputData, encoding: .utf8) else {
            throw PdftractError.parseError("Failed to decode verification output")
        }

        // Parse output format: "valid: true" or "valid: false"
        let trimmed = output.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.contains("true") {
            return true
        } else if trimmed.contains("false") {
            return false
        } else {
            throw PdftractError.parseError("Unexpected verification output: \(trimmed)")
        }
    }

    /// MARK: - Helper Methods

    /// Run a process and return stdout data.
    private func runProcess(arguments: [String], stdin: Data?) async throws -> Data {
        let process = Process()
        let stdinPipe = Pipe()
        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()

        process.executableURL = URL(fileURLWithPath: binaryPath)
        process.arguments = arguments
        process.standardInput = stdinPipe
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe

        // Launch process
        process.launch()

        // Write stdin data if needed
        if let data = stdin {
            stdinPipe.fileHandleForWriting.write(data)
            stdinPipe.fileHandleForWriting.closeFile()
        } else {
            stdinPipe.fileHandleForWriting.closeFile()
        }

        // Wait for process to finish
        process.waitUntilExit()

        // Read stdout
        let stdoutData = stdoutPipe.fileHandleForReading.readDataToEndOfFile()

        let exitCode = process.terminationStatus
        if exitCode != 0 {
            let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
            if let stderr = String(data: stderrData, encoding: .utf8), !stderr.isEmpty {
                throw PdftractError.internalError(stderr)
            } else {
                throw PdftractError.internalError("Process exited with code \(exitCode)")
            }
        }

        return stdoutData
    }

    /// Get stdin data for a source (nil for path/url sources, Data for bytes).
    private func dataForSource(_ source: Source) -> Data? {
        switch source {
        case .path, .url:
            return nil
        case .bytes(let data):
            return data
        }
    }

    /// MARK: - Argument Builders

    /// Build command-line arguments for full extraction.
    private func buildArguments(
        for source: Source,
        options: ExtractionOptions
    ) -> [String] {
        var args = ["extract", "--output-format", "json"]

        // Add source argument
        switch source {
        case .path(let path):
            args.append(path)
        case .url(let url):
            args.append("--url")
            args.append(url.absoluteString)
        case .bytes:
            // For bytes, we'll read from stdin
            args.append("--stdin")
        }

        // Add extraction options
        if !options.extractSpans { args.append("--no-spans") }
        if !options.extractBlocks { args.append("--no-blocks") }
        if !options.extractTables { args.append("--no-tables") }
        if !options.extractAnnotations { args.append("--no-annotations") }
        if !options.extractFormFields { args.append("--no-form-fields") }
        if !options.extractSignatures { args.append("--no-signatures") }
        if !options.extractAttachments { args.append("--no-attachments") }
        if !options.extractOutline { args.append("--no-outline") }
        if !options.extractThreads { args.append("--no-threads") }
        if !options.extractLinks { args.append("--no-links") }

        if let dpi = options.ocrDpi {
            args.append("--ocr-dpi")
            args.append(String(dpi))
        }

        if let maxSize = options.maxAttachmentSize {
            args.append("--max-attachment-size")
            args.append(String(maxSize))
        }

        if !options.includeQuality { args.append("--no-quality") }
        if !options.includeErrors { args.append("--no-errors") }

        return args
    }

    /// Build command-line arguments for text extraction.
    private func buildTextArguments(
        for source: Source,
        options: TextOptions
    ) -> [String] {
        var args = ["extract", "--output-format", "text"]

        // Add source
        switch source {
        case .path(let path):
            args.append(path)
        case .url(let url):
            args.append("--url")
            args.append(url.absoluteString)
        case .bytes:
            args.append("--stdin")
        }

        // Add text options
        if !options.preserveWhitespace { args.append("--no-preserve-whitespace") }
        if options.includeFontInfo { args.append("--include-font-info") }
        if options.includeBoundingBoxes { args.append("--include-bboxes") }

        return args
    }

    /// Build command-line arguments for markdown extraction.
    private func buildMarkdownArguments(
        for source: Source,
        options: MarkdownOptions
    ) -> [String] {
        var args = ["extract", "--output-format", "markdown"]

        // Add source
        switch source {
        case .path(let path):
            args.append(path)
        case .url(let url):
            args.append("--url")
            args.append(url.absoluteString)
        case .bytes:
            args.append("--stdin")
        }

        // Add markdown options
        if !options.includeHeadings { args.append("--no-headings") }
        if !options.includeLists { args.append("--no-lists") }
        if !options.includeTables { args.append("--no-tables") }
        if !options.includeLinks { args.append("--no-links") }

        return args
    }

    /// Build command-line arguments for search.
    private func buildSearchArguments(
        for source: Source,
        pattern: String,
        options: SearchOptions
    ) -> [String] {
        var args = ["grep", "--output-format", "json"]

        // Add pattern
        args.append("--pattern")
        args.append(pattern)

        // Add search options
        if options.caseInsensitive { args.append("--case-insensitive") }
        if options.wholeWord { args.append("--whole-word") }
        if options.regex { args.append("--regex") }
        if options.maxMatches > 0 {
            args.append("--max-matches")
            args.append(String(options.maxMatches))
        }

        // Add source
        switch source {
        case .path(let path):
            args.append(path)
        case .url(let url):
            args.append("--url")
            args.append(url.absoluteString)
        case .bytes:
            args.append("--stdin")
        }

        return args
    }

    /// Build command-line arguments for hash.
    private func buildHashArguments(for source: Source) -> [String] {
        var args = ["hash"]

        // Add source
        switch source {
        case .path(let path):
            args.append(path)
        case .url(let url):
            args.append("--url")
            args.append(url.absoluteString)
        case .bytes:
            args.append("--stdin")
        }

        return args
    }

    /// Build command-line arguments for classify.
    private func buildClassifyArguments(for source: Source) -> [String] {
        var args = ["classify", "--output-format", "json"]

        // Add source
        switch source {
        case .path(let path):
            args.append(path)
        case .url(let url):
            args.append("--url")
            args.append(url.absoluteString)
        case .bytes:
            args.append("--stdin")
        }

        return args
    }

    /// Build command-line arguments for verify-receipt.
    private func buildVerifyReceiptArguments(path: String, receipt: String) -> [String] {
        return [
            "verify-receipt",
            "--path",
            path,
            "--receipt",
            receipt
        ]
    }
}
