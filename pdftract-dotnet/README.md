# Pdftract .NET SDK

The .NET SDK for [pdftract](https://github.com/jedarden/pdftract) — a subprocess wrapper around the `pdftract` binary for PDF text extraction, OCR, search, and metadata.

## Installation

```bash
dotnet add package Pdftract
```

## Quick Start

```csharp
using Pdftract;
using Pdftract.Models;

var client = new Pdftract();

// Extract structured data
var doc = await client.ExtractAsync(Source.FromPath("document.pdf"));
Console.WriteLine($"Pages: {doc.Pages.Count}");

// Extract plain text
var text = await client.ExtractTextAsync(Source.FromPath("document.pdf"));

// Extract markdown
var md = await client.ExtractMarkdownAsync(Source.FromPath("document.pdf"));

// Get metadata
var metadata = await client.GetMetadataAsync(Source.FromPath("document.pdf"));
Console.WriteLine($"Title: {metadata.Title}");
```

## Features

- **Extract**: Structured data, plain text, or markdown from PDFs
- **Search**: Full-text search with regex and whole-word options
- **Metadata**: Extract document metadata (title, author, page count, etc.)
- **Hash**: Compute document fingerprints for deduplication
- **Classify**: Automatic document classification
- **OCR**: Built-in OCR support for scanned documents
- **Async-first**: All methods return `Task<T>` or `IAsyncEnumerable<T>`
- **AOT-compatible**: Works with Native AOT compilation

## Supported Platforms

- .NET 9.0 (recommended)
- .NET 8.0

.NET Framework 4.x is **not supported**.

## API Reference

### Source Types

```csharp
// From file path
var source = Source.FromPath("document.pdf");

// From URL string
var source = Source.FromUrl("https://example.com/document.pdf");

// From URI
var uri = new Uri("https://example.com/document.pdf");
var source = Source.FromUri(uri);

// From bytes
var data = await File.ReadAllBytesAsync("document.pdf");
var source = Source.FromBytes(data);
```

### Extraction Methods

```csharp
// Structured data with pages, spans, and blocks
var doc = await client.ExtractAsync(source, new ExtractOptions
{
    OcrLanguage = "eng",
    PreserveLayout = true
});

// Plain text
var text = await client.ExtractTextAsync(source);

// Markdown
var md = await client.ExtractMarkdownAsync(source);

// Streaming pages
await foreach (var page in client.ExtractStreamAsync(source))
{
    Console.WriteLine($"Page {page.PageIndex}: {page.Blocks.Count} blocks");
}
```

### Search

```csharp
await foreach (var match in client.SearchAsync(source, "pattern", new SearchOptions
{
    CaseInsensitive = true,
    Regex = true,
    WholeWord = false,
    MaxResults = 100
}))
{
    Console.WriteLine($"{match.Page}: {match.Text}");
    Console.WriteLine($"  Context: {match.Context.Before}[MATCH]{match.Context.After}");
}
```

### Metadata

```csharp
var metadata = await client.GetMetadataAsync(source);
Console.WriteLine($"Title: {metadata.Title}");
Console.WriteLine($"Author: {metadata.Author}");
Console.WriteLine($"Page Count: {metadata.PageCount}");
Console.WriteLine($"Created: {metadata.Created}");
```

### Hash

```csharp
var fingerprint = await client.HashAsync(source);
Console.WriteLine($"Hash: {fingerprint.Hash}");
Console.WriteLine($"Fast Hash: {fingerprint.FastHash}");
```

### Classification

```csharp
var classification = await client.ClassifyAsync(source);
Console.WriteLine($"Category: {classification.Category}");
Console.WriteLine($"Confidence: {classification.Confidence}");
Console.WriteLine($"Tags: {string.Join(", ", classification.Tags)}");
```

## Options

### ExtractOptions

| Option | Type | Description |
|--------|------|-------------|
| `Password` | `string?` | Password for encrypted PDFs |
| `OcrLanguage` | `string?` | ISO 639-3 language code for OCR |
| `OcrThreshold` | `double?` | Confidence threshold for OCR (0-1) |
| `PreserveLayout` | `bool?` | Preserve original reading order and layout |
| `ExtractImages` | `bool?` | Extract embedded images |
| `ImageFormat` | `string?` | Format for extracted images (png, jpg, webp) |
| `MinImageSize` | `int?` | Minimum dimension for image extraction |
| `Timeout` | `int?` | Maximum seconds to wait for the operation |

### SearchOptions

| Option | Type | Description |
|--------|------|-------------|
| `CaseInsensitive` | `bool?` | Ignore case when matching |
| `Regex` | `bool?` | Treat pattern as regular expression |
| `WholeWord` | `bool?` | Match only whole words |
| `MaxResults` | `int?` | Maximum matches to return |

### HashOptions

| Option | Type | Description |
|--------|------|-------------|
| `Password` | `string?` | Password for encrypted PDFs |

## Error Handling

The SDK provides specific exception types for different error conditions:

```csharp
try
{
    var doc = await client.ExtractAsync(source);
}
catch (CorruptPdfException ex)
{
    Console.WriteLine($"PDF is corrupt: {ex.Message}");
}
catch (EncryptionException ex)
{
    Console.WriteLine($"PDF is encrypted: {ex.Message}");
}
catch (SourceUnreachableException ex)
{
    Console.WriteLine($"Cannot read source: {ex.Message}");
}
catch (RemoteFetchInterruptedException ex)
{
    Console.WriteLine($"Network error: {ex.Message}");
}
catch (TlsException ex)
{
    Console.WriteLine($"TLS error: {ex.Message}");
}
catch (ReceiptVerifyException ex)
{
    Console.WriteLine($"Receipt verification failed: {ex.Message}");
}
catch (PdftractException ex)
{
    Console.WriteLine($"pdftract error (exit {ex.ExitCode}): {ex.Message}");
}
```

## Conformance

The SDK ships a conformance test suite that verifies compliance with the pdftract contract. See the [conformance documentation](https://github.com/jedarden/pdftract/blob/main/docs/conformance/sdk-contract.md) for details.

## Native AOT

This SDK is designed to work with Native AOT compilation. Ensure your project uses source-generated JSON serialization:

```xml
<PropertyGroup>
  <PublishAot>true</PublishAot>
</PropertyGroup>
```

## License

MIT

## Links

- [pdftract](https://github.com/jedarden/pdftract)
- [Documentation](https://github.com/jedarden/pdftract/tree/main/docs)
- [Conformance](https://github.com/jedarden/pdftract/blob/main/docs/conformance/sdk-contract.md)
