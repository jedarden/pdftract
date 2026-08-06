# Pdftract .NET SDK

The .NET SDK for Pdftract provides a subprocess-based wrapper around the pdftract binary for PDF text extraction, OCR, search, and metadata.

## Installation

```bash
dotnet add package Pdftract
```

## Basic Usage

```csharp
using Pdftract;

// Create a client
var client = new PdftractClient();

// Extract text from a PDF
var result = await client.ExtractTextAsync("document.pdf");

// Search for text
var matches = await client.SearchAsync("document.pdf", "search term");

// Get metadata
var metadata = await client.GetMetadataAsync("document.pdf");
```

## Documentation

For full documentation, see the main [pdftract repository](https://github.com/jedarden/pdftract).

## License

MIT OR Apache-2.0
