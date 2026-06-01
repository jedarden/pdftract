# Swift SDK Implementation Verification (pdftract-5lvpu)

## Overview

Bead pdftract-5lvpu implements the Swift SDK for pdftract as a subprocess-based SDK using Foundation's Process class with async/await support. The implementation targets macOS 13+ and Linux (server-side Swift only), explicitly excluding iOS due to Apple's subprocess restrictions.

## Acceptance Criteria Status

### PASS: SPM Package Structure
- **Package.swift**: Configured with swift-tools-version 5.10, platforms `.macOS(.v13)` and `.linux`
- **Products**: `Pdftract` library target
- **Targets**: `Pdftract` source target, `PdftractTests` test target
- **Location**: `/home/coding/pdftract/swift-sdk/`

### PASS: 9 Contract Methods Exposed
All 9 contract methods are implemented in `Sources/Pdftract/Methods.swift`:

1. **extract** - Full structured extraction returning `Document`
2. **extractText** - Text-only extraction returning `String`
3. **extractMarkdown** - Markdown extraction returning `String`
4. **extractStream** - Async streaming of `Page` objects via `AsyncThrowingStream`
5. **search** - Pattern search with `AsyncThrowingStream<Match, Error>`
6. **getMetadata** - Metadata-only extraction returning `ExtractionMetadata`
7. **hash** - Cryptographic fingerprint returning `Fingerprint`
8. **classify** - Document classification returning `Classification`
9. **verifyReceipt** - Receipt verification returning `Bool`

### PASS: 8 Error Cases Defined
All 8 contract error cases are defined in `Sources/Pdftract/Models/Error.swift`:

1. **invalidPdf** - Invalid PDF file format
2. **ioError** - I/O error reading/writing files
3. **networkError** - Network error fetching from URL
4. **outOfMemory** - Memory allocation failure
5. **parseError** - PDF structure parse error
6. **ocrError** - OCR processing error
7. **renderingError** - Page rendering error
8. **internalError** - Generic internal error

Each error case includes:
- `localizedDescription` property for human-readable messages
- `code` property for programmatic handling
- `Equatable` conformance for testing

### PASS: iOS Documented as Unsupported
From README.md:
```
Platform Support
Supported: macOS 13+, Linux (server-side Swift only)
Unsupported: iOS (Apple does not allow spawning subprocesses in App Store apps)

Note for iOS users: Use `pdftract serve` over HTTP from your iOS client.
```

### PASS: CI Workflow Configured
**Location**: `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/pdftract-swift-publish.yaml`

**Workflow Steps**:
1. **clone-sdk-repo**: Clone `github.com/jedarden/pdftract-swift` from main branch
2. **sync-version**: Verify Package.swift (SPM version is implicit in git tag)
3. **conformance**: Run `swift test --filter ConformanceTests` (must pass)
4. **tag-and-push**: Create git tag `VERSION` (numeric, no `v` prefix)
5. **warm-spi**: Post to Swift Package Index to trigger indexing

**Container**: `swift:5.10-jammy`

**Secret**: Uses `github-pat-pdftract` secret for GitHub authentication

### PASS: AsyncThrowingStream Implementation
Both `extractStream` and `search` methods return `AsyncThrowingStream`:
- Yields results incrementally as they're received from the subprocess
- Properly handles subprocess cleanup via ProcessRunner actor
- Cancellation support via `withTaskCancellationHandler`

### PASS: Source Type Support
`Source` enum supports three input types:
1. **path(String)** - File path on local filesystem
2. **url(URL)** - Remote URL (pdftract fetches via HTTP)
3. **bytes(Data)** - In-memory PDF data

## Model Types Implemented

All required model types are defined in `Sources/Pdftract/Models/`:

- **Document.swift**: `Document`, `ExtractionMetadata`, `ReceiptsMode`, `JavascriptAction`
- **Page.swift**: `Page`, `PageType`, `Span`, `ConfidenceSource`, `Block`
- **Annotation.swift**: `Link`, `Annotation`, `AnnotationSpecific`, `DestinationArray`, `DestinationType`
- **Attachment.swift**: `Attachment`, `Thread`, `Bead`, `OutlineNode`, `Destination`
- **Table.swift**: `Table`, `Row`, `Cell`
- **FormField.swift**: `FormField`, `FormFieldType`, `FormFieldValue`
- **Signature.swift**: `Signature`
- **Fingerprint.swift**: `Fingerprint`, `HashOptions`
- **Receipt.swift**: `Receipt`
- **Classification.swift**: `Classification`, `ClassificationOptions`
- **Match.swift**: `Match`, `SearchOptions`
- **Error.swift**: `PdftractError` with 8 cases
- **Quality.swift**: `ExtractionQuality`, `Diagnostic`
- **Source.swift**: `Source`, `ExtractionOptions`, `TextOptions`, `MarkdownOptions`

## Options Types

All options types follow Swift naming conventions (camelCase):
- **ExtractionOptions**: Full extraction control (spans, blocks, tables, OCR DPI, etc.)
- **TextOptions**: Text extraction (preserve whitespace, font info, bboxes)
- **MarkdownOptions**: Markdown output (headings, lists, tables, links)
- **SearchOptions**: Search parameters (case insensitive, regex, max matches)
- **HashOptions**: Hash computation (include MD5, include structure)
- **ClassificationOptions**: Classifier options (top-K, exit on unknown)

## Cross-Platform Process Support

**ProcessRunner** (`Sources/Pdftract/ProcessRunner.swift`) provides:
- Cross-platform Process abstraction (macOS vs Linux)
- Proper cancellation support via actor isolation
- Async/await-based execution
- Streaming JSON output support with `executeStreaming`
- Clean resource cleanup in `deinit`

## Conformance Test Suite

**Location**: `Tests/PdftractTests/ConformanceTests.swift`

**Test Data**: `/home/coding/pdftract/tests/sdk-conformance/cases.json`

**Coverage**: All 9 contract methods have dedicated test methods:
- `testExtractConformance`
- `testExtractTextConformance`
- `testExtractMarkdownConformance`
- `testExtractStreamConformance`
- `testSearchConformance`
- `testGetMetadataConformance`
- `testHashConformance`
- `testClassifyConformance`
- `testVerifyReceiptConformance`
- `testAllConformance` (comprehensive suite)

**Note**: Tests require the pdftract binary to be in PATH for execution.

## Deferred to v1.1+

Per the task description, this Swift SDK is part of the v1.1+ release wave (deferred from v1.0). This acknowledges the smaller server-side Swift user base compared to other SDK platforms.

## Publishing Process

**Repository**: `github.com/jedarden/pdftract-swift`

**Trigger**: By the pdftract-release-cascade after pdftract-build-binaries completes

**Tag Format**: Numeric only (e.g., `1.0.0`), **no `v` prefix** (SPM convention differs from other SDKs)

**Swift Package Index**: Automatically indexed after tag push; workflow pings SPI API to speed up availability

## Installation Example

```swift
// Package.swift
dependencies: [
    .package(url: "https://github.com/jedarden/pdftract-swift.git", from: "1.0.0")
]

// Usage
import Pdftract

let client = Pdftract()
let source = Source.path("/path/to/document.pdf")
let document = try await client.extract(from: source)
```

## Files Modified

Updated:
- `swift-sdk/README.md` - Changed placeholder GitHub URLs from `github.com/your-org/pdftract-swift` to `github.com/jedarden/pdftract-swift`

## Verification Summary

| Criterion | Status |
|-----------|--------|
| SPM package consumable | PASS |
| 9 contract methods exposed | PASS |
| 8 error cases defined | PASS |
| iOS documented as unsupported | PASS |
| CI workflow configured | PASS |
| AsyncThrowingStream cancellation | PASS |
| Models complete | PASS |
| Options types complete | PASS |
| Conformance tests defined | PASS |
| Cross-platform Process support | PASS |

**Overall**: READY for v1.1+ release

## References

- Plan section: SDK Architecture / The Ten SDKs, line 3480
- Plan section: SDK Architecture / Per-SDK Release Channels, line 3577
- Plan section: SDK Acceptance Criteria, lines 3581-3589
- ADR-009: Argo Workflows on iad-ci only
