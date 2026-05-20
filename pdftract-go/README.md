# pdftract-go

Go SDK for pdftract — subprocess-based client for extracting structured data from PDFs.

## Installation

```go
go get github.com/jedarden/pdftract-go
```

The SDK requires the `pdftract` binary to be installed and available in your PATH. See [jedarden/pdftract](https://github.com/jedarden/pdftract) for installation instructions.

## Quick Start

```go
package main

import (
    "context"
    "fmt"
    "log"

    "github.com/jedarden/pdftract-go"
)

func main() {
    // Create a client (searches PATH for pdftract binary)
    client, err := pdftract.NewClient("")
    if err != nil {
        log.Fatal(err)
    }

    // Extract structured data from a PDF
    doc, err := client.Extract(context.Background(), pdftract.FileSource("document.pdf"), nil)
    if err != nil {
        log.Fatal(err)
    }

    fmt.Printf("Pages: %d\n", len(doc.Pages))
    fmt.Printf("Title: %s\n", doc.Metadata.Title)
}
```

## Sources

The SDK accepts three types of PDF sources:

```go
// Local file path
client.Extract(ctx, pdftract.FileSource("path/to/file.pdf"), opts)

// Remote URL
client.Extract(ctx, pdftract.RemoteSource("https://example.com/doc.pdf"), opts)

// In-memory bytes
data, _ := os.ReadFile("document.pdf")
client.Extract(ctx, pdftract.MemorySource(data), opts)
```

## API Methods

### Extract

Extract structured data from a PDF:

```go
opts := &pdftract.ExtractOptions{
    OCRLanguage:    "eng",
    OCRThreshold:   0.7,
    PreserveLayout: false,
    ExtractImages:  false,
}

doc, err := client.Extract(ctx, pdftract.FileSource("doc.pdf"), opts)
```

### ExtractText

Extract plain text:

```go
text, err := client.ExtractText(ctx, source, opts)
```

### ExtractMarkdown

Extract Markdown-formatted text:

```go
md, err := client.ExtractMarkdown(ctx, source, opts)
```

### ExtractStream

Stream pages one at a time:

```go
resultChan, err := client.ExtractStream(ctx, source, opts)
for result := range resultChan {
    if result.Err != nil {
        log.Printf("Error: %v", result.Err)
        continue
    }
    page := result.Page
    fmt.Printf("Page %d: %d spans\n", page.Page, len(page.Spans))
}
```

### Search

Search for a pattern in a PDF:

```go
opts := &pdftract.SearchOptions{
    CaseInsensitive: true,
    Regex:           false,
    WholeWord:       false,
}

resultChan, err := client.Search(ctx, source, "invoice", opts)
for result := range resultChan {
    if result.Err != nil {
        log.Printf("Error: %v", result.Err)
        continue
    }
    match := result.Match
    fmt.Printf("Match on page %d: %s\n", match.Page, match.Text)
}
```

### GetMetadata

Extract document metadata:

```go
meta, err := client.GetMetadata(ctx, source, nil)
fmt.Printf("Title: %s\n", meta.Title)
fmt.Printf("Author: %s\n", meta.Author)
fmt.Printf("Page count: %d\n", meta.PageCount)
```

### Hash

Compute document fingerprint:

```go
fp, err := client.Hash(ctx, source, nil)
fmt.Printf("SHA-256: %s\n", fp.Hash)
fmt.Printf("BLAKE3: %s\n", fp.FastHash)
```

### Classify

Classify document type:

```go
cls, err := client.Classify(ctx, source)
fmt.Printf("Category: %s\n", cls.Category)
fmt.Printf("Confidence: %.2f\n", cls.Confidence)
fmt.Printf("Tags: %v\n", cls.Tags)
```

### VerifyReceipt

Verify a cryptographic receipt:

```go
valid, err := client.VerifyReceipt(ctx, "document.pdf", receipt)
```

## Context Cancellation

All methods accept `context.Context` for cancellation:

```go
ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
defer cancel()

doc, err := client.Extract(ctx, source, opts)
if errors.Is(err, context.DeadlineExceeded) {
    // Handle timeout
}
```

## Error Handling

The SDK maps CLI exit codes to specific error types:

```go
doc, err := client.Extract(ctx, source, opts)
if err != nil {
    if corruptErr, ok := pdftract.AsCorruptPdfError(err); ok {
        // Handle corrupt PDF
        fmt.Printf("Corrupt PDF: %s\n", corruptErr.Message)
    } else if encErr, ok := pdftract.AsEncryptionError(err); ok {
        // Handle encrypted PDF
        fmt.Printf("Encrypted PDF: %s\n", encErr.Message)
    } else {
        // Handle other errors
        log.Fatal(err)
    }
}
```

Available error types:
- `CorruptPdfError` — Exit code 2
- `EncryptionError` — Exit code 3
- `SourceUnreachableError` — Exit code 4
- `RemoteFetchInterruptedError` — Exit code 5
- `TlsError` — Exit code 6
- `ReceiptVerifyError` — Exit code 10

## Options

### ExtractOptions

```go
type ExtractOptions struct {
    Password       string  // PDF password
    OCRLanguage    string  // OCR language code (default: "eng")
    OCRThreshold   float64 // OCR confidence threshold (default: 0.7)
    PreserveLayout bool    // Preserve original reading order
    ExtractImages  bool    // Extract embedded images
    ImageFormat    string  // Image format: "png", "jpg", "webp"
    MinImageSize   int     // Minimum image dimension in pixels
}
```

### SearchOptions

```go
type SearchOptions struct {
    CaseInsensitive bool // Ignore case when matching
    Regex           bool // Treat pattern as regex
    WholeWord       bool // Match only whole words
    MaxResults      *int // Maximum matches (nil = unlimited)
}
```

### HashOptions

```go
type HashOptions struct {
    Password string // PDF password
}
```

## Go Version

This module requires Go 1.22 or later.

## Conformance

This SDK passes the official pdftract conformance suite. Run tests with:

```bash
go test ./...
```

## License

MIT

## Links

- [pdftract CLI](https://github.com/jedarden/pdftract)
- [SDK Contract](https://github.com/jedarden/pdftract/blob/main/docs/notes/sdk-contract.md)
- [pkg.go.dev](https://pkg.go.dev/github.com/jedarden/pdftract-go)
