# pdftract-go

Go SDK for pdftract — subprocess-based client for extracting structured data from PDFs.

## Installation

```go
go get github.com/jedarden/pdftract-go
```

## Quick Start

```go
package main

import (
    "fmt"
    "github.com/jedarden/pdftract-go"
)

func main() {
    // From a local file
    source := pdftract.PathSource("document.pdf")
    fmt.Printf("Source type: %s\n", source.sourceType())

    // From a URL
    url := pdftract.URLSource("https://example.com/doc.pdf")
    fmt.Printf("Source type: %s\n", url.sourceType())

    // From raw bytes
    data := []byte{/* PDF bytes */}
    bytes := pdftract.BytesSource(data)
    fmt.Printf("Source type: %s\n", bytes.sourceType())
}
```

## Module Documentation

[![Go Reference](https://pkg.go.dev/badge/github.com/jedarden/pdftract-go.svg)](https://pkg.go.dev/github.com/jedarden/pdftract-go)

## License

MIT
