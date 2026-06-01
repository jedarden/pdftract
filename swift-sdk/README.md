# Pdftract Swift SDK

Swift SDK for the pdftract PDF extraction library. This SDK provides type-safe, async/await-based access to pdftract's full structured extraction, text-only, and markdown output.

## Features

- **Full structured extraction**: Complete document model with pages, spans, blocks, tables, annotations, form fields, signatures, and attachments
- **Text-only extraction**: Fast text extraction with optional formatting
- **Markdown extraction**: Convert PDFs to Markdown format
- **Async/await support**: All operations are asynchronous and non-blocking
- **Async streaming**: Stream pages or text incrementally for large PDFs
- **Type-safe models**: All JSON types are represented as native Swift structs
- **Comprehensive error handling**: Detailed error types with context

## Platform Support

**Supported**: macOS 13+, Linux (server-side Swift only)
**Unsupported**: iOS (Apple does not allow spawning subprocesses in App Store apps)

> **Note for iOS users**: Use `pdftract serve` over HTTP from your iOS client. Run the server with the Swift SDK on a macOS/Linux backend and make HTTP requests from your iOS app.

## Requirements

- macOS 13.0+ / Linux
- Swift 5.10+

## Installation

### Swift Package Manager

Add `Pdftract` to your `Package.swift` dependencies:

```swift
dependencies: [
    .package(url: "https://github.com/jedarden/pdftract-swift.git", from: "1.0.0")
]
```

Or in Xcode: File > Add Package Dependency > Enter repository URL

## Quick Start

```swift
import Pdftract

// Create a client
let client = Pdftract()

// Extract a PDF from a file path
let source = Source.path("/path/to/document.pdf")
do {
    let document = try await client.extract(from: source)
    print("Extracted \(document.pages.count) pages")
    print("Title: \(document.metadata.title ?? "none")")

    // Access page content
    for page in document.pages {
        print("Page \(page.pageNumber): \(page.spans.count) spans")
        for block in page.blocks {
            print("  \(block.kind): \(block.text)")
        }
    }
} catch {
    print("Error: \(error.localizedDescription)")
}
```

## Usage

### Full Structured Extraction

```swift
let client = Pdftract()
let source = Source.path("/path/to/document.pdf")

// Customize extraction options
let options = ExtractionOptions(
    extractTables: true,
    extractAnnotations: true,
    ocrDpi: 300
)

let document = try await client.extract(from: source, options: options)
```

### Stream Pages Incrementally

For large PDFs, stream pages as they're extracted:

```swift
let client = Pdftract()
let source = Source.path("/path/to/large.pdf")

for try await page in await client.extractPages(from: source) {
    print("Page \(page.pageNumber): \(page.spans.count) spans")
    // Process page immediately without waiting for full document
}
```

### Text Extraction

Extract only text content:

```swift
let client = Pdftract()
let source = Source.path("/path/to/document.pdf")

// Extract all text
let text = try await client.extractText(from: source)
print(text)

// Stream text page by page
for try await pageText in await client.extractTextPages(from: source) {
    print(pageText)
}
```

### Markdown Extraction

Convert PDF to Markdown:

```swift
let client = Pdftract()
let source = Source.path("/path/to/document.pdf")

let options = MarkdownOptions(
    includeTables: true,
    includeLinks: true
)

let markdown = try await client.extractMarkdown(from: source, options: options)
print(markdown)
```

### Working with URLs

Extract from a URL:

```swift
let client = Pdftract()
let source = Source.url("https://example.com/document.pdf")
let document = try await client.extract(from: source)
```

### Working with Bytes

Extract from in-memory bytes:

```swift
let client = Pdftract()
let pdfData = try Data(contentsOf: url)
let source = Source.bytes(pdfData)
let document = try await client.extract(from: source)
```

### Metadata Only

Quick inspection without full extraction:

```swift
let client = Pdftract()
let source = Source.path("/path/to/document.pdf")
let metadata = try await client.extractMetadata(from: source)

print("Pages: \(metadata.pageCount)")
print("Title: \(metadata.title ?? "none")")
print("Author: \(metadata.author ?? "none")")
print("PDF Version: \(metadata.pdfVersion ?? "unknown")")
print("Encrypted: \(metadata.isEncrypted)")
```

### Cryptographic Hashing

Compute PDF fingerprints:

```swift
let client = Pdftract()
let source = Source.path("/path/to/document.pdf")
let (md5, sha256) = try await client.hash(source: source)

print("MD5: \(md5)")
print("SHA-256: \(sha256)")
```

## Data Models

### Document

Top-level structure containing metadata and pages:

```swift
public struct Document {
    public let schemaVersion: String
    public let metadata: Metadata
    public var outline: [OutlineNode]
    public var threads: [Thread]
    public var attachments: [Attachment]
    public var signatures: [Signature]
    public var formFields: [FormField]
    public var links: [Link]
    public var pages: [Page]
    public var extractionQuality: ExtractionQuality
    public var errors: [Diagnostic]
}
```

### Page

Single page with extracted content:

```swift
public struct Page {
    public let pageIndex: UInt
    public let pageNumber: UInt32
    public var pageLabel: String?
    public let width: Float
    public let height: Float
    public let rotation: UInt16
    public let pageType: String
    public var spans: [Span]
    public var blocks: [Block]
    public var tables: [Table]
    public var annotations: [Annotation]
}
```

### Span

Atomic text unit with consistent font and styling:

```swift
public struct Span {
    public let text: String
    public let bbox: [Double]
    public let font: String
    public let size: Double
    public var color: String?
    public var confidence: Double?
    public var confidenceSource: String?
    public var lang: String?
    public var flags: [String]
    public var column: UInt32?
}
```

### Block

Semantic block composed of spans:

```swift
public struct Block {
    public let kind: String  // "paragraph", "heading", "list", "table", "figure"
    public let text: String
    public let bbox: [Double]
    public var level: UInt8?  // For headings
    public var tableIndex: UInt?  // For tables
    public var spans: [UInt]
}
```

### Table

Extracted table with cell-level structure:

```swift
public struct Table {
    public let id: String
    public let bbox: [Double]
    public var rows: [Row]
    public let headerRows: UInt32
    public let detectionMethod: String
    public var continued: Bool
    public var continuedFromPrev: Bool
    public let pageIndex: UInt
}
```

### Annotation

Hyperlinks and markup annotations:

```swift
public struct Link {
    public let pageIndex: UInt
    public let rect: [Float]
    public var uri: String?
    public var dest: String?
    public var destArray: DestinationArray?
}

public struct Annotation {
    public let subtype: String
    public var rect: [Float]?
    public var contents: String?
    public var author: String?
    public var specific: AnnotationSpecific?
}
```

### FormField

AcroForm/XFA form fields:

```swift
public struct FormField {
    public let name: String
    public let fieldType: FormFieldType
    public var value: FormFieldValue
    public var pageIndex: UInt?
    public var rect: [Float]?
    public let required: Bool
    public let readOnly: Bool
    // ... type-specific fields
}
```

### Signature

Digital signature metadata:

```swift
public struct Signature {
    public let fieldName: String
    public let signerName: String
    public var signingDate: String?
    public var reason: String?
    public var location: String?
    public var validationStatus: String  // Always "not_checked" in v1
}
```

## Error Handling

All operations can throw `PdftractError`:

```swift
public enum PdftractError: Error {
    case invalidPdf(String)      // Invalid PDF file format
    case ioError(String)         // I/O error reading/writing files
    case networkError(String)    // Network error fetching from URL
    case outOfMemory             // Memory allocation failure
    case parseError(String)      // PDF structure parse error
    case ocrError(String)        // OCR processing error
    case renderingError(String)  // Page rendering error
    case internalError(String)   // Generic internal error
}
```

Example:

```swift
do {
    let document = try await client.extract(from: source)
} catch let error as PdftractError {
    print("Error code: \(error.code)")
    print("Description: \(error.localizedDescription)")
}
```

## Extraction Options

Control what to extract:

```swift
public struct ExtractionOptions {
    public var extractSpans: Bool
    public var extractBlocks: Bool
    public var extractTables: Bool
    public var extractAnnotations: Bool
    public var extractFormFields: Bool
    public var extractSignatures: Bool
    public var extractAttachments: Bool
    public var extractOutline: Bool
    public var extractThreads: Bool
    public var extractLinks: Bool
    public var ocrDpi: UInt32?
    public var maxAttachmentSize: UInt64?
    public var includeQuality: Bool
    public var includeErrors: Bool
}
```

Example:

```swift
let options = ExtractionOptions(
    extractTables: false,
    extractAnnotations: false,
    ocrDpi: 400
)
```

## Testing

Run tests with Swift Package Manager:

```bash
swift test
```

Or in Xcode: Cmd + U

## License

MIT License - see LICENSE file for details

## Contributing

Contributions are welcome! Please read CONTRIBUTING.md for guidelines.

## Support

- Issues: https://github.com/jedarden/pdftract-swift/issues
- Discussions: https://github.com/jedarden/pdftract-swift/discussions
- Documentation: https://pdftract.com/docs
