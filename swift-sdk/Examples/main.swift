//
//  main.swift
//  Pdftract Examples
//
//  Demonstrates all major features of the Pdftract Swift SDK.
//

import Foundation
#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

import Pdftract

@MainActor
func runExamples() async {
    print("=== Pdftract Swift SDK Examples ===\n")

    // Note: These examples use placeholder paths.
    // Replace with actual PDF paths for testing.

    // Example 1: Basic extraction
    await example1_basicExtraction()

    // Example 2: Streaming pages
    await example2_streamingPages()

    // Example 3: Text extraction
    await example3_textExtraction()

    // Example 4: Markdown extraction
    await example4_markdownExtraction()

    // Example 5: Metadata only
    await example5_metadataOnly()

    // Example 6: URL source
    await example6_urlSource()

    // Example 7: Bytes source
    await example7_bytesSource()

    // Example 8: Custom options
    await example8_customOptions()

    // Example 9: Error handling
    await example9_errorHandling()

    // Example 10: Working with tables
    await example10_tables()

    print("\n=== Examples Complete ===")
}

// MARK: - Example 1: Basic Extraction

func example1_basicExtraction() async {
    print("\n--- Example 1: Basic Extraction ---")

    let client = Pdftract()
    let source = Source.path("/path/to/document.pdf")

    do {
        let document = try await client.extract(from: source)

        print("Schema Version: \(document.schemaVersion)")
        print("Page Count: \(document.metadata.pageCount)")
        print("Title: \(document.metadata.title ?? "none")")
        print("Author: \(document.metadata.author ?? "none")")
        print("PDF Version: \(document.metadata.pdfVersion ?? "unknown")")
        print("Encrypted: \(document.metadata.isEncrypted)")
        print("Tagged PDF: \(document.metadata.isTagged)")

        print("\nPages:")
        for page in document.pages {
            print("  Page \(page.pageNumber): \(page.pageType)")
            print("    Spans: \(page.spans.count)")
            print("    Blocks: \(page.blocks.count)")
            print("    Tables: \(page.tables.count)")
        }
    } catch {
        print("Error: \(error)")
    }
}

// MARK: - Example 2: Streaming Pages

func example2_streamingPages() async {
    print("\n--- Example 2: Streaming Pages ---")

    let client = Pdftract()
    let source = Source.path("/path/to/large.pdf")

    do {
        var pageCount = 0
        for try await page in await client.extractPages(from: source) {
            pageCount += 1
            print("Page \(page.pageNumber): \(page.spans.count) spans, \(page.blocks.count) blocks")

            // Process page immediately without waiting for full document
            for block in page.blocks {
                if block.kind == "heading" {
                    print("  Heading: \(block.text)")
                }
            }
        }
        print("Total pages streamed: \(pageCount)")
    } catch {
        print("Error: \(error)")
    }
}

// MARK: - Example 3: Text Extraction

func example3_textExtraction() async {
    print("\n--- Example 3: Text Extraction ---")

    let client = Pdftract()
    let source = Source.path("/path/to/document.pdf")

    do {
        // Extract all text
        let text = try await client.extractText(from: source)
        print("Extracted text length: \(text.count) characters")
        print("Preview: \(text.prefix(200))...")

        // Stream text page by page
        print("\nText by page:")
        for try await pageText in await client.extractTextPages(from: source) {
            let lines = pageText.split(separator: "\n").count
            print("  Page with \(lines) lines")
        }
    } catch {
        print("Error: \(error)")
    }
}

// MARK: - Example 4: Markdown Extraction

func example4_markdownExtraction() async {
    print("\n--- Example 4: Markdown Extraction ---")

    let client = Pdftract()
    let source = Source.path("/path/to/document.pdf")

    let options = MarkdownOptions(
        includeHeadings: true,
        includeLists: true,
        includeTables: true,
        includeLinks: true
    )

    do {
        let markdown = try await client.extractMarkdown(from: source, options: options)
        print("Markdown length: \(markdown.count) characters")
        print("Preview:\n\(markdown.prefix(500))...")
    } catch {
        print("Error: \(error)")
    }
}

// MARK: - Example 5: Metadata Only

func example5_metadataOnly() async {
    print("\n--- Example 5: Metadata Only ---")

    let client = Pdftract()
    let source = Source.path("/path/to/document.pdf")

    do {
        let metadata = try await client.extractMetadata(from: source)

        print("Page Count: \(metadata.pageCount)")
        print("Title: \(metadata.title ?? "none")")
        print("Author: \(metadata.author ?? "none")")
        print("Subject: \(metadata.subject ?? "none")")
        print("Keywords: \(metadata.keywords ?? "none")")
        print("Creator: \(metadata.creator ?? "none")")
        print("Producer: \(metadata.producer ?? "none")")
        print("Creation Date: \(metadata.creationDate ?? "unknown")")
        print("PDF Version: \(metadata.pdfVersion ?? "unknown")")
        print("Conformance: \(metadata.conformance)")
        print("Contains JavaScript: \(metadata.containsJavaScript)")
        print("Contains XFA: \(metadata.containsXfa)")
        print("Has OCG: \(metadata.ocgPresent)")

        if !metadata.javascriptActions.isEmpty {
            print("\nJavaScript Actions:")
            for action in metadata.javascriptActions {
                print("  - \(action.location)")
            }
        }
    } catch {
        print("Error: \(error)")
    }
}

// MARK: - Example 6: URL Source

func example6_urlSource() async {
    print("\n--- Example 6: URL Source ---")

    let client = Pdftract()
    let source = Source.url("https://example.com/document.pdf")

    do {
        let document = try await client.extract(from: source)
        print("Extracted from URL: \(document.pages.count) pages")
    } catch {
        print("Error: \(error)")
    }
}

// MARK: - Example 7: Bytes Source

func example7_bytesSource() async {
    print("\n--- Example 7: Bytes Source ---")

    let client = Pdftract()

    // Simulate reading bytes from somewhere
    let pdfData = Data(repeating: 0x25, count: 1000) // Placeholder
    let source = Source.bytes(pdfData)

    do {
        let document = try await client.extract(from: source)
        print("Extracted from bytes: \(document.pages.count) pages")
    } catch {
        print("Error: \(error)")
    }
}

// MARK: - Example 8: Custom Options

func example8_customOptions() async {
    print("\n--- Example 8: Custom Options ---")

    let client = Pdftract()
    let source = Source.path("/path/to/document.pdf")

    // Customize extraction
    let options = ExtractionOptions(
        extractSpans: true,
        extractBlocks: true,
        extractTables: true,
        extractAnnotations: false,
        extractFormFields: true,
        extractSignatures: true,
        extractAttachments: false,
        extractOutline: true,
        extractThreads: false,
        extractLinks: true,
        ocrDpi: 400,
        maxAttachmentSize: 10_000_000,
        includeQuality: true,
        includeErrors: true
    )

    do {
        let document = try await client.extract(from: source, options: options)
        print("Extracted with custom options")
        print("Quality: \(document.extractionQuality.overallQuality)")

        if let dpi = document.extractionQuality.dpiUsed {
            print("DPI used: \(dpi)")
        }

        if let ocrFrac = document.extractionQuality.ocrFraction {
            print("OCR fraction: \(ocrFrac)")
        }

        if !document.errors.isEmpty {
            print("\nDiagnostics:")
            for error in document.errors {
                print("  [\(error.severity)] \(error.code): \(error.message)")
            }
        }
    } catch {
        print("Error: \(error)")
    }
}

// MARK: - Example 9: Error Handling

func example9_errorHandling() async {
    print("\n--- Example 9: Error Handling ---")

    let client = Pdftract()
    let source = Source.path("/nonexistent/file.pdf")

    do {
        let _ = try await client.extract(from: source)
    } catch let error as PdftractError {
        print("Pdftract Error:")
        print("  Code: \(error.code)")
        print("  Description: \(error.localizedDescription)")

        // Handle specific errors
        switch error {
        case .invalidPdf(let message):
            print("  Invalid PDF: \(message)")
        case .ioError(let message):
            print("  I/O Error: \(message)")
        case .networkError(let message):
            print("  Network Error: \(message)")
        case .outOfMemory:
            print("  Out of Memory")
        case .parseError(let message):
            print("  Parse Error: \(message)")
        case .ocrError(let message):
            print("  OCR Error: \(message)")
        case .renderingError(let message):
            print("  Rendering Error: \(message)")
        case .internalError(let message):
            print("  Internal Error: \(message)")
        }
    } catch {
        print("Other error: \(error)")
    }
}

// MARK: - Example 10: Working with Tables

func example10_tables() async {
    print("\n--- Example 10: Working with Tables ---")

    let client = Pdftract()
    let source = Source.path("/path/to/document.pdf")

    do {
        let document = try await client.extract(from: source)

        var totalTables = 0
        for (pageIndex, page) in document.pages.enumerated() {
            if !page.tables.isEmpty {
                print("Page \(page.pageNumber): \(page.tables.count) tables")
                totalTables += page.tables.count

                for table in page.tables {
                    print("  Table '\(table.id)':")
                    print("    Detection method: \(table.detectionMethod)")
                    print("    Header rows: \(table.headerRows)")
                    print("    Total rows: \(table.rows.count)")
                    print("    Continued: \(table.continued)")
                    print("    Continued from prev: \(table.continuedFromPrev)")

                    // Examine first row
                    if let firstRow = table.rows.first {
                        print("    First row: \(firstRow.cells.count) cells")
                        for cell in firstRow.cells {
                            print("      [\(cell.row),\(cell.col)] \(cell.text)")
                        }
                    }
                }
            }
        }

        print("\nTotal tables: \(totalTables)")
    } catch {
        print("Error: \(error)")
    }
}

// MARK: - Additional Helper Examples

func example_workingWithSpans() async {
    print("\n--- Working with Spans ---")

    let client = Pdftract()
    let source = Source.path("/path/to/document.pdf")

    do {
        let document = try await client.extract(from: source)

        for page in document.pages {
            print("Page \(page.pageNumber):")

            for (index, span) in page.spans.enumerated() {
                print("  Span \(index):")
                print("    Text: \(span.text)")
                print("    Font: \(span.font) @ \(span.size)pt")
                print("    BBox: \(span.bbox)")

                if let color = span.color {
                    print("    Color: \(color)")
                }

                if let confidence = span.confidence {
                    print("    Confidence: \(confidence)")
                }

                if let source = span.confidenceSource {
                    print("    Source: \(source)")
                }

                if let lang = span.lang {
                    print("    Language: \(lang)")
                }

                if !span.flags.isEmpty {
                    print("    Flags: \(span.flags.joined(separator: ", "))")
                }

                if let column = span.column {
                    print("    Column: \(column)")
                }
            }
        }
    } catch {
        print("Error: \(error)")
    }
}

func example_workingWithBlocks() async {
    print("\n--- Working with Blocks ---")

    let client = Pdftract()
    let source = Source.path("/path/to/document.pdf")

    do {
        let document = try await client.extract(from: source)

        for page in document.pages {
            print("Page \(page.pageNumber):")

            for block in page.blocks {
                switch block.kind {
                case "heading":
                    if let level = block.level {
                        print("  H\(level): \(block.text)")
                    } else {
                        print("  Heading: \(block.text)")
                    }

                case "paragraph":
                    print("  Paragraph: \(block.text.prefix(50))...")

                case "list":
                    print("  List item: \(block.text)")

                case "table":
                    if let tableIndex = block.tableIndex {
                        print("  Table (index \(tableIndex)): \(block.text)")
                    } else {
                        print("  Table: \(block.text)")
                    }

                case "figure":
                    print("  Figure: \(block.text)")

                default:
                    print("  \(block.kind): \(block.text)")
                }
            }
        }
    } catch {
        print("Error: \(error)")
    }
}

func example_workingWithFormFields() async {
    print("\n--- Working with Form Fields ---")

    let client = Pdftract()
    let source = Source.path("/path/to/form.pdf")

    do {
        let document = try await client.extract(from: source)

        guard !document.formFields.isEmpty else {
            print("No form fields found")
            return
        }

        print("Form fields: \(document.formFields.count)")

        for field in document.formFields {
            print("  Field: \(field.name)")
            print("    Type: \(field.fieldType)")

            switch field.fieldType {
            case .text:
                case .text(let value):
                    print("    Value: \(value ?? "empty")")
                if let multiline = field.multiline {
                    print("    Multiline: \(multiline)")
                }
                if let maxLength = field.maxLength {
                    print("    Max length: \(maxLength)")
                }

            case .button:
                case .button(let selected):
                    print("    Selected: \(selected)")
                if let state = field.stateName {
                    print("    State: \(state)")
                }

            case .choice:
                case .choice(let choice):
                    switch choice {
                    case .single(let value):
                        print("    Selected: \(value)")
                    case .multiple(let values):
                        print("    Selected: \(values.joined(separator: ", "))")
                    }

                if let options = field.options {
                    print("    Options:")
                    for opt in options {
                        print("      \(opt[0]) - \(opt[1])")
                    }
                }

            case .signature:
                case .signature(let ref):
                    print("    Signature ref: \(ref?.description ?? "unsigned")")
            }

            print("    Required: \(field.required)")
            print("    Read-only: \(field.readOnly)")

            if let pageIndex = field.pageIndex {
                print("    Page: \(pageIndex)")
            }
        }
    } catch {
        print("Error: \(error)")
    }
}

func example_workingWithSignatures() async {
    print("\n--- Working with Signatures ---")

    let client = Pdftract()
    let source = Source.path("/path/to/signed.pdf")

    do {
        let document = try await client.extract(from: source)

        guard !document.signatures.isEmpty else {
            print("No signatures found")
            return
        }

        print("Signatures: \(document.signatures.count)")

        for sig in document.signatures {
            print("  Signature: \(sig.fieldName)")
            print("    Signer: \(sig.signerName)")

            if let date = sig.signingDate {
                print("    Date: \(date)")
            }

            if let reason = sig.reason {
                print("    Reason: \(reason)")
            }

            if let location = sig.location {
                print("    Location: \(location)")
            }

            if let subFilter = sig.subFilter {
                print("    Format: \(subFilter)")
            }

            if let byteRange = sig.byteRange {
                print("    Byte range: \(byteRange)")
            }

            if let coverage = sig.coverageFraction {
                print("    Coverage: \(Int(coverage * 100))%")
            }

            print("    Validation: \(sig.validationStatus)")
        }
    } catch {
        print("Error: \(error)")
    }
}

func example_workingWithAttachments() async {
    print("\n--- Working with Attachments ---")

    let client = Pdftract()
    let source = Source.path("/path/to/attachments.pdf")

    do {
        let document = try await client.extract(from: source)

        guard !document.attachments.isEmpty else {
            print("No attachments found")
            return
        }

        print("Attachments: \(document.attachments.count)")

        for attachment in document.attachments {
            print("  Attachment: \(attachment.name)")

            if let description = attachment.description {
                print("    Description: \(description)")
            }

            if let mimeType = attachment.mimeType {
                print("    MIME type: \(mimeType)")
            }

            print("    Size: \(attachment.size) bytes")

            if let created = attachment.created {
                print("    Created: \(created)")
            }

            if let modified = attachment.modified {
                print("    Modified: \(modified)")
            }

            if let checksum = attachment.checksumMd5 {
                print("    MD5: \(checksum)")
            }

            if attachment.truncated {
                print("    Status: Truncated (> 50 MB)")
            } else if attachment.data != nil {
                print("    Status: Included (\(attachment.data!.count) base64 chars)")
            } else {
                print("    Status: Empty")
            }
        }
    } catch {
        print("Error: \(error)")
    }
}

func example_workingWithOutline() async {
    print("\n--- Working with Outline (Bookmarks) ---")

    let client = Pdftract()
    let source = Source.path("/path/to/document.pdf")

    do {
        let document = try await client.extract(from: source)

        guard !document.outline.isEmpty else {
            print("No outline found")
            return
        }

        print("Outline entries: \(document.outline.count)")
        printOutlineTree(document.outline, level: 0)
    } catch {
        print("Error: \(error)")
    }
}

func printOutlineTree(_ nodes: [OutlineNode], level: Int) {
    let indent = String(repeating: "  ", count: level)

    for node in nodes {
        print("\(indent)- \(node.title)")

        if let pageIndex = node.pageIndex {
            print("\(indent)  → Page \(pageIndex)")
        }

        if let destination = node.destination {
            print("\(indent)  → Dest: \(destination.destType)")
        }

        if !node.children.isEmpty {
            printOutlineTree(node.children, level: level + 1)
        }
    }
}

// Run all examples
if CommandLine.arguments.count > 1 && CommandLine.arguments[1] == "run" {
    Task {
        await runExamples()
        exit(0)
    }

    // Run the async task
    RunLoop.current.run()
} else {
    print("Run with: swift run PdftractExamples run")
}
