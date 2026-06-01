# Pdftract Swift SDK - Complete Package Structure

## Overview

This document describes the complete Swift package structure for the pdftract SDK, designed according to the JSON schema contract (`docs/schema/v1.0/pdftract.schema.json`).

## Package Structure

```
swift-sdk/
├── Package.swift                          # SPM manifest with .macOS(.v13), .linux
├── README.md                              # User-facing documentation
├── .gitignore                             # Git ignore patterns
├── STRUCTURE.md                           # This file
│
├── Sources/Pdftract/
│   ├── Pdftract.swift                     # Main client class (actor)
│   ├── PdftractExport.swift               # Public API exports
│   │
│   └── Models/
│       ├── Document.swift                 # Document, Metadata
│       ├── Page.swift                     # Page, Span, Block
│       ├── Table.swift                    # Table, Row, Cell
│       ├── Annotation.swift              # Link, DestinationArray, DestinationType, Annotation, AnnotationSpecific
│       ├── Signature.swift                # Signature
│       ├── FormField.swift                # FormField, FormFieldType, FormFieldValue, ChoiceValue
│       ├── Attachment.swift              # Attachment, Thread, Bead, OutlineNode, Destination
│       ├── Quality.swift                  # ExtractionQuality, Diagnostic, ObjectLocation, JavascriptAction
│       ├── Source.swift                   # Source enum, ExtractionOptions, TextOptions, MarkdownOptions
│       └── Error.swift                    # PdftractError (8 cases), DecodingErrorWrapper
│
├── Tests/PdftractTests/
│   └── PdftractTests.swift                # Comprehensive unit tests
│
└── Examples/
    └── main.swift                         # Usage examples for all features
```

## File-by-File Breakdown

### 1. Package.swift

```swift
// swift-tools-version: 5.9
// Platforms: .macOS(.v13), .linux
// Products: Pdftract library
// Targets: Pdftract (source), PdftractTests (tests)
```

**Key Features:**
- Swift 5.9+ for modern concurrency support
- Multi-platform: macOS 13+, Linux
- No external dependencies (standalone)

### 2. Sources/Pdftract/Pdftract.swift

**Main Client Class (Actor):**

```swift
public actor Pdftract {
    // Full structured extraction
    public func extract(from:source, options:) async throws -> Document

    // Streaming extraction
    public func extractPages(from:source, options:) async -> AsyncThrowingStream<Page, Error>

    // Text extraction
    public func extractText(from:source, options:) async throws -> String
    public func extractTextPages(from:source, options:) async -> AsyncThrowingStream<String, Error>

    // Markdown extraction
    public func extractMarkdown(from:source, options:) async throws -> String

    // Hashing
    public func hash(source:) async throws -> (md5: String, sha256: String)

    // Metadata only
    public func extractMetadata(from:) async throws -> Metadata
}
```

**Design Decisions:**
- **Actor** for thread-safe access to underlying extractor
- **Async/await** for all I/O operations
- **AsyncThrowingStream** for incremental processing of large PDFs
- **Throws** typed `PdftractError` for all failures

### 3. Models/Document.swift

**Structures:**

```swift
public struct Document {
    public let schemaVersion: String           // "1.0"
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

public struct Metadata {
    public var title: String?
    public var author: String?
    public var subject: String?
    public var keywords: String?
    public var creator: String?
    public var producer: String?
    public var creationDate: String?
    public var modificationDate: String?
    public let pageCount: UInt32
    public var pdfVersion: String?
    public let isTagged: Bool
    public let isEncrypted: Bool
    public var conformance: String              // "none", "PDF-A-1a", etc.
    public let containsJavaScript: Bool
    public var javascriptActions: [JavascriptAction]
    public let containsXfa: Bool
    public let ocgPresent: Bool
    public var generator: String?
}
```

### 4. Models/Page.swift

**Structures:**

```swift
public struct Page {
    public let pageIndex: UInt                 // 0-based
    public let pageNumber: UInt32              // 1-based
    public var pageLabel: String?
    public let width: Float
    public let height: Float
    public let rotation: UInt16                // 0, 90, 180, 270
    public let pageType: String                // "text", "scanned", "mixed", etc.
    public var spans: [Span]
    public var blocks: [Block]
    public var tables: [Table]
    public var annotations: [Annotation]
}

public struct Span {
    public let text: String
    public let bbox: [Double]                   // [x0, y0, x1, y1]
    public let font: String
    public let size: Double
    public var color: String?
    public var renderingMode: UInt8?
    public var confidence: Double?
    public var confidenceSource: String?        // "vector", "ocr", etc.
    public var lang: String?
    public var flags: [String]                  // "bold", "italic", etc.
    public var column: UInt32?
}

public struct Block {
    public let kind: String                     // "paragraph", "heading", etc.
    public let text: String
    public let bbox: [Double]
    public var level: UInt8?                    // For headings (1-6)
    public var tableIndex: UInt?                // For tables
    public var spans: [UInt]                    // Indices into page.spans
}
```

### 5. Models/Table.swift

**Structures:**

```swift
public struct Table {
    public let id: String                       // "table_0"
    public let bbox: [Double]
    public var rows: [Row]
    public let headerRows: UInt32
    public let detectionMethod: String         // "line_based", "borderless"
    public var continued: Bool
    public var continuedFromPrev: Bool
    public let pageIndex: UInt
}

public struct Row {
    public let bbox: [Double]
    public var cells: [Cell]
    public let isHeader: Bool
}

public struct Cell {
    public let bbox: [Double]
    public let text: String
    public let spans: [UInt]
    public let row: UInt
    public let col: UInt
    public let rowspan: UInt32
    public let colspan: UInt32
    public let isHeaderRow: Bool
}
```

### 6. Models/Annotation.swift

**Structures:**

```swift
public struct Link {
    public let pageIndex: UInt
    public let rect: [Float]
    public var uri: String?
    public var dest: String?
    public var destArray: DestinationArray?
}

public struct DestinationArray {
    public let pageIndex: UInt
    public let dest: DestinationType
}

public enum DestinationType: Codable {
    case xyz(left: Double?, top: Double?, zoom: Double?)
    case fit
    case fitH(top: Double?)
    case fitV(left: Double?)
    case fitR(left: Double, bottom: Double, right: Double, top: Double)
    case fitB
    case fitBH(top: Double?)
    case fitBV(left: Double?)
}

public struct Annotation {
    public let subtype: String                 // "Highlight", "Text", etc.
    public var rect: [Float]?
    public var contents: String?
    public var author: String?
    public var modified: String?
    public var color: [Float]?
    public var opacity: Float?
    public var nameId: String?
    public var subject: String?
    public var specific: AnnotationSpecific?
}

public enum AnnotationSpecific: Codable {
    case textMarkup(quads: [[Float]])
    case stamp(name: String?)
    case freeText(da: String?)
    case text(open: Bool?, state: String?, stateModel: String?)
    case ink(strokes: [[[Float]]])
    case line(endpoints: [Float]?)
    case polygon(vertices: [[Float]])
    case fileAttachment(fsRef: UInt32?)
    case other
}
```

### 7. Models/Signature.swift

**Structure:**

```swift
public struct Signature {
    public let fieldName: String
    public let signerName: String
    public var signingDate: String?
    public var reason: String?
    public var location: String?
    public var subFilter: String?
    public var byteRange: [UInt64]?
    public var coverageFraction: Double?
    public let validationStatus: String        // Always "not_checked" in v1
}
```

### 8. Models/FormField.swift

**Structures:**

```swift
public struct FormField {
    public let name: String
    public let fieldType: FormFieldType
    public var value: FormFieldValue
    public var defaultValue: FormFieldValue?
    public var pageIndex: UInt?
    public var rect: [Float]?
    public let required: Bool
    public let readOnly: Bool
    public var multiline: Bool?
    public var maxLength: UInt32?
    public var options: [[String]]?             // [[export_value, display_name], ...]
    public var multiSelect: Bool?
    public var selected: Bool?
    public var stateName: String?
    public var pushbutton: Bool?
    public var radio: Bool?
}

public enum FormFieldType: String, Codable {
    case text, button, choice, signature
}

public enum FormFieldValue: Codable, Equatable {
    case text(String?)
    case button(Bool)
    case choice(ChoiceValue)
    case signature(UInt32?)
}

public enum ChoiceValue: Codable, Equatable {
    case single(String)
    case multiple([String])
}
```

### 9. Models/Attachment.swift

**Structures:**

```swift
public struct Attachment {
    public let name: String
    public var description: String?
    public var mimeType: String?
    public let size: UInt64
    public var created: String?
    public var modified: String?
    public var checksumMd5: String?
    public var data: String?                     // Base64 or nil if truncated
    public let truncated: Bool                  // true if > 50 MB
}

public struct Thread {
    public var title: String?
    public var author: String?
    public var subject: String?
    public var keywords: String?
    public var beads: [Bead]
}

public struct Bead {
    public let pageIndex: UInt
    public let rect: [Float]
}

public struct OutlineNode {
    public let title: String
    public let level: UInt8
    public var pageIndex: UInt32?
    public var destination: Destination?
    public var children: [OutlineNode]
}

public struct Destination {
    public let destType: String
    public var left: Double?
    public var top: Double?
    public var right: Double?
    public var bottom: Double?
    public var zoom: Double?
}
```

### 10. Models/Quality.swift

**Structures:**

```swift
public struct ExtractionQuality {
    public var overallQuality: String           // "high", "medium", "low", "none"
    public var dpiUsed: UInt32?
    public var ocrFraction: Float?
    public var minConfidence: Float?
    public var avgConfidence: Float?
    public var readability: Float?
}

public struct Diagnostic {
    public let code: String                     // "FONT_GLYPH_UNMAPPED"
    public let message: String
    public let severity: String                 // "info", "warning", "error", "fatal"
    public var pageIndex: UInt?
    public var location: ObjectLocation?
    public var hint: String?
}

public struct ObjectLocation {
    public let objectNumber: UInt32
    public let generationNumber: UInt16
}

public struct JavascriptAction {
    public let location: String                 // "catalog.openaction", etc.
    public let codeExcerpt: String              // First 200 chars
}
```

### 11. Models/Source.swift

**Enumerations and Options:**

```swift
public enum Source {
    case path(String)
    case url(String)
    case bytes(Data)
    case bytesStream(AsyncStream<Data>)
}

public struct ExtractionOptions: Codable {
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

public struct TextOptions: Codable {
    public var preserveWhitespace: Bool
    public var includeFontInfo: Bool
    public var includeBoundingBoxes: Bool
}

public struct MarkdownOptions: Codable {
    public var includeHeadings: Bool
    public var includeLists: Bool
    public var includeTables: Bool
    public var includeLinks: Bool
}
```

### 12. Models/Error.swift

**Error Types:**

```swift
public enum PdftractError: Error, Equatable {
    case invalidPdf(String)                     // Invalid PDF file format
    case ioError(String)                        // I/O error reading/writing files
    case networkError(String)                   // Network error fetching from URL
    case outOfMemory                             // Memory allocation failure
    case parseError(String)                      // PDF structure parse error
    case ocrError(String)                        // OCR processing error
    case renderingError(String)                 // Page rendering error
    case internalError(String)                   // Generic internal error

    public var localizedDescription: String { /* ... */ }
    public var code: String { /* ... */ }        // "INVALID_PDF", etc.
}
```

### 13. Tests/PdftractTests.swift

**Test Coverage:**

- `DocumentTests`: Document initialization, JSON encoding/decoding
- `PageTests`: Page, Span, Block initialization
- `TableTests`: Table, Row, Cell with merged cells
- `AnnotationTests`: Links (internal/external), annotations
- `FormFieldTests`: Text, button, choice (single/multiple), signature fields
- `SignatureTests`: Signed and unsigned signatures
- `AttachmentTests`: Regular and truncated attachments
- `ExtractionQualityTests`: Quality metrics
- `DiagnosticTests`: Diagnostic with context
- `SourceTests`: Path, URL, bytes sources
- `ExtractionOptionsTests`: Default and custom options
- `ErrorTests`: Error descriptions, codes, equality

**Run Tests:**
```bash
swift test
```

### 14. Examples/main.swift

**Example Functions:**

1. `example1_basicExtraction()` - Basic document extraction
2. `example2_streamingPages()` - Stream pages incrementally
3. `example3_textExtraction()` - Extract all text or by page
4. `example4_markdownExtraction()` - Convert to Markdown
5. `example5_metadataOnly()` - Quick metadata inspection
6. `example6_urlSource()` - Extract from URL
7. `example7_bytesSource()` - Extract from in-memory bytes
8. `example8_customOptions()` - Custom extraction options
9. `example9_errorHandling()` - Handle specific errors
10. `example10_tables()` - Work with tables
11. `example_workingWithSpans()` - Detailed span inspection
12. `example_workingWithBlocks()` - Block-level processing
13. `example_workingWithFormFields()` - Form field handling
14. `example_workingWithSignatures()` - Signature inspection
15. `example_workingWithAttachments()` - Attachment handling
16. `example_workingWithOutline()` - Outline/bookmark traversal

**Run Examples:**
```bash
swift run PdftractExamples run
```

## Naming Conventions

### Swift Naming (camelCase)

- **Methods**: `extract(from:options:)`, `extractText(from:options:)`
- **Properties**: `schemaVersion`, `pageCount`, `extractionQuality`
- **Parameters**: `from source`, `options: ExtractionOptions`
- **Variables**: `let pageIndex`, `var pageNumber`

### JSON Keys (snake_case)

All `CodingKeys` map Swift camelCase to JSON snake_case:

```swift
enum CodingKeys: String, CodingKey {
    case schemaVersion = "schema_version"
    case pageCount = "page_count"
    case extractionQuality = "extraction_quality"
}
```

## Key Design Decisions

### 1. Actor Concurrency

The `Pdftract` client is an `actor` for thread-safe access:

```swift
public actor Pdftract {
    private var extractor: ExtractorBridge?

    public func extract(from source: Source) async throws -> Document {
        // Actor ensures thread-safe access to extractor
    }
}
```

### 2. AsyncThrowingStream for Streaming

Large PDFs can be processed incrementally:

```swift
public func extractPages(from source: Source)
    async -> AsyncThrowingStream<Page, Error>
```

Consumers can process pages as they arrive:

```swift
for try await page in await client.extractPages(from: source) {
    // Process page immediately
}
```

### 3. Codable for All Models

Every model is `Codable` for JSON serialization:

```swift
let document = try decoder.decode(Document.self, from: jsonData)
let json = try encoder.encode(document)
```

### 4. Optionals for Schema Conditionals

Fields that are `null` in the schema are Swift `Optionals`:

```swift
public var level: UInt8?           // null for non-heading blocks
public var tableIndex: UInt?        // null for non-table blocks
```

### 5. Enum Discriminated Unions

Complex types use Swift enums with associated values:

```swift
public enum FormFieldValue: Codable {
    case text(String?)
    case button(Bool)
    case choice(ChoiceValue)
    case signature(UInt32?)
}
```

### 6. Type-Safe Errors

`PdftractError` provides typed errors with codes:

```swift
catch let error as PdftractError {
    switch error {
    case .invalidPdf(let msg):
        // Handle invalid PDF
    case .networkError(let msg):
        // Handle network error
    }
}
```

## Schema Compliance

All models comply with `docs/schema/v1.0/pdftract.schema.json`:

- **Required fields**: Non-optional Swift properties
- **Optional fields**: Swift `Optional` (`Type?`)
- **Arrays**: Swift arrays (`[Type]`)
- **Null handling**: `nil` in Swift, `null` in JSON
- **Enums**: Swift enums with `String` raw values or custom `Codable`

## Integration Notes

### Placeholder Implementation

The current implementation uses a placeholder `ExtractorBridge` actor. In production, this would be replaced with:

1. **C FFI**: Call into compiled Rust library
2. **HTTP Client**: Call pdftract server API
3. **CLI Wrapper**: Execute pdftract binary

### Cross-Platform Networking

Conditional import for Linux compatibility:

```swift
#if canImport(FoundationNetworking)
import FoundationNetworking
#endif
```

### Memory Management

- All structs are value types (no reference counting)
- `actor` provides thread-safe access
- `AsyncThrowingStream` handles backpressure
- Large data (attachments) truncated at 50 MB

## File Paths Summary

| File | Lines | Purpose |
|------|-------|---------|
| `Package.swift` | 25 | SPM manifest |
| `Sources/Pdftract/Pdftract.swift` | ~200 | Main client |
| `Sources/Pdftract/Models/Document.swift` | ~150 | Document, Metadata |
| `Sources/Pdftract/Models/Page.swift` | ~120 | Page, Span, Block |
| `Sources/Pdftract/Models/Table.swift` | ~100 | Table, Row, Cell |
| `Sources/Pdftract/Models/Annotation.swift` | ~200 | Links, Annotations |
| `Sources/Pdftract/Models/Signature.swift` | ~50 | Signature |
| `Sources/Pdftract/Models/FormField.swift` | ~120 | Form fields |
| `Sources/Pdftract/Models/Attachment.swift` | ~150 | Attachments, threads, outline |
| `Sources/Pdftract/Models/Quality.swift` | ~100 | Quality, diagnostics |
| `Sources/Pdftract/Models/Source.swift` | ~100 | Source enum, options |
| `Sources/Pdftract/Models/Error.swift` | ~50 | Error types |
| `Tests/PdftractTests.swift` | ~500 | Unit tests |
| `Examples/main.swift` | ~600 | Usage examples |

**Total**: ~2,465 lines of Swift code

## Next Steps

1. **Implement `ExtractorBridge`**: Connect to actual pdftract core
   - Option A: C FFI to compiled Rust library
   - Option B: HTTP client to pdftract server
   - Option C: Command-line wrapper

2. **Add CI/CD**: GitHub Actions for macOS/Linux testing

3. **Documentation**: Generate DocC documentation

4. **Binary Framework**: Distribute as `.xcframework` for non-SPM use

5. **Performance Testing**: Benchmark large PDF handling

## References

- JSON Schema: `/home/coding/pdftract/docs/schema/v1.0/pdftract.schema.json`
- Rust Models: `/home/coding/pdftract/crates/pdftract-core/src/schema/mod.rs`
- Plan: `/home/coding/pdftract/docs/plan/plan.md` (lines 1-3825)
