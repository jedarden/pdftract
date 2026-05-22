# pdftract Java SDK

[![Maven Central](https://img.shields.io/maven-central/v/com.jedarden/pdftract)](https://central.sonatype.com/search?q=com.jedarden:pdftract)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Java/Kotlin SDK for [pdftract](https://github.com/jedarden/pdftract) — PDF extraction and analysis library.

## Features

- **9 contract methods**: extract, extractText, extractMarkdown, extractStream, search, getMetadata, hash, classify, verifyReceipt
- **AutoCloseable client**: Use with try-with-resources for automatic cleanup
- **8 typed exceptions**: CorruptPdfException, EncryptionException, SourceUnreachableException, etc.
- **Kotlin extensions**: Idiomatic Kotlin syntax in the same artifact
- **Java 17+**: Modern Java with records and pattern matching

## Installation

Add to your `pom.xml`:

```xml
<dependency>
    <groupId>com.jedarden</groupId>
    <artifactId>pdftract</artifactId>
    <version>0.1.0</version>
</dependency>
```

Or for Gradle:

```groovy
implementation 'com.jedarden:pdftract:0.1.0'
```

## Requirements

- Java 17 or higher
- The `pdftract` binary must be available on your PATH (or specify custom path)
  - Download from [GitHub Releases](https://github.com/jedarden/pdftract/releases)

## Java Usage

### Basic extraction

```java
import com.jedarden.pdftract.*;
import com.jedarden.pdftract.codegen.*;
import java.nio.file.Path;

try (Pdftract client = new Pdftract()) {
    // Extract structured data
    Document doc = client.extract(
        Source.fromPath("document.pdf"),
        null
    );

    System.out.println("Pages: " + doc.pages().size());
    System.out.println("Title: " + doc.metadata().title());

    // Access pages, blocks, and spans
    for (Page page : doc.pages()) {
        System.out.println("Page " + page.pageIndex() + ": " + page.width() + "x" + page.height());
        for (Block block : page.blocks()) {
            System.out.println("  " + block.kind() + ": " + block.text());
        }
    }
}
```

### Extract plain text

```java
try (Pdftract client = new Pdftract()) {
    String text = client.extractText(
        Source.fromPath("document.pdf"),
        null
    );
    System.out.println(text);
}
```

### Extract Markdown

```java
try (Pdftract client = new Pdftract()) {
    String markdown = client.extractMarkdown(
        Source.fromPath("document.pdf"),
        null
    );
    System.out.println(markdown);
}
```

### OCR options

```java
ExtractOptions options = new ExtractOptions()
    .setOcrLanguage("eng")
    .setOcrThreshold(0.7);

Document doc = client.extract(Source.fromPath("scanned.pdf"), options);
```

### Password-protected PDFs

```java
BaseOptions options = new BaseOptions()
    .setPassword("secret");

Document doc = client.extract(Source.fromPath("protected.pdf"), options);
```

### Stream pages (for large PDFs)

```java
try (Pdftract client = new Pdftract()) {
    client.extractStream(Source.fromPath("large.pdf"), null)
        .forEach(page -> {
            System.out.println("Page " + page.pageIndex());
            // Process each page as it arrives
        });
}
```

### Search for text

```java
try (Pdftract client = new Pdftract()) {
    SearchOptions options = new SearchOptions()
        .setMaxResults(100)
        .setWholeWord(true);

    client.search(Source.fromPath("document.pdf"), "invoice", options)
        .forEach(match -> {
            System.out.println("Found at page " + match.page() + ": " + match.text());
        });
}
```

### Get metadata

```java
try (Pdftract client = new Pdftract()) {
    Metadata metadata = client.getMetadata(
        Source.fromPath("document.pdf"),
        null
    );

    System.out.println("Pages: " + metadata.pageCount());
    System.out.println("Title: " + metadata.title());
    System.out.println("Author: " + metadata.author());
}
```

### Compute fingerprint

```java
try (Pdftract client = new Pdftract()) {
    Fingerprint fp = client.hash(
        Source.fromPath("document.pdf"),
        null
    );

    System.out.println("SHA-256: " + fp.hash());
    System.out.println("Fast hash: " + fp.fastHash());
}
```

### Classify document

```java
try (Pdftract client = new Pdftract()) {
    Classification cls = client.classify(
        Source.fromPath("unknown.pdf")
    );

    System.out.println("Category: " + cls.category());
    System.out.println("Confidence: " + cls.confidence());
}
```

### Verify receipt

```java
try (Pdftract client = new Pdftract()) {
    Receipt receipt = new Receipt(
        "abc123def456",  // fingerprint
        "sig789xyz012"   // signature
    );

    boolean valid = client.verifyReceipt(
        Path.of("receipt.pdf"),
        receipt
    );

    System.out.println("Valid: " + valid);
}
```

### URL sources

```java
try (Pdftract client = new Pdftract()) {
    Document doc = client.extract(
        Source.fromUrl("https://example.com/document.pdf"),
        null
    );
}
```

### Byte sources

```java
byte[] pdfBytes = Files.readAllBytes(Path.of("document.pdf"));

try (Pdftract client = new Pdftract()) {
    Document doc = client.extract(
        Source.fromBytes(pdfBytes),
        null
    );
}
```

### Custom binary path

```java
try (Pdftract client = new Pdftract("/path/to/pdftract")) {
    Document doc = client.extract(Source.fromPath("doc.pdf"), null);
}
```

## Kotlin Usage

The Kotlin extensions provide idiomatic syntax with lambda-based options:

```kotlin
import com.jedarden.pdftract.*
import com.jedarden.pdftract.codegen.*
import java.nio.file.Path

// Use with invoke operator (use-with-resources pattern)
pdftract {
    val doc = extract(Path.of("document.pdf")) {
        ocrLanguage = "eng"
        ocrThreshold = 0.7
    }

    println("Pages: ${doc.pages.size}")
}

// Or use try-with-resources explicitly
Pdftract().use { client ->
    val doc = client.extract(Path.of("document.pdf"))
    println(doc.metadata.title)
}

// Extract text
Pdftract().use { client ->
    val text = client.extractText(Path.of("document.pdf")) {
        ocrLanguage = "eng"
    }
    println(text)
}

// Search with options
Pdftract().use { client ->
    client.search(Path.of("document.pdf"), "invoice") {
        maxResults = 100
        wholeWord = true
    }.forEach { match ->
        println("Found at page ${match.page}: ${match.text}")
    }
}

// Stream pages (converts to Sequence)
Pdftract().use { client ->
    client.extractStream(Path.of("large.pdf")) {
        ocrLanguage = "eng"
    }.forEach { page ->
        println("Page ${page.pageIndex}")
    }
}
```

## Exception Handling

All methods throw `PdftractException` or its subclasses:

```java
try (Pdftract client = new Pdftract()) {
    Document doc = client.extract(Source.fromPath("doc.pdf"), null);
} catch (CorruptPdfException e) {
    System.err.println("PDF is corrupt: " + e.getMessage());
} catch (EncryptionException e) {
    System.err.println("PDF is encrypted: " + e.getMessage());
} catch (SourceUnreachableException e) {
    System.err.println("Cannot read source: " + e.getMessage());
} catch (TlsException e) {
    System.err.println("TLS error: " + e.getMessage());
} catch (PdftractException e) {
    System.err.println("Error (exit code " + e.getExitCode() + "): " + e.getMessage());
}
```

Exception types:
- `PdftractException` — Base exception
- `CorruptPdfException` — PDF is corrupt (exit code 2)
- `EncryptionException` — PDF is encrypted (exit code 3)
- `SourceUnreachableException` — Cannot read source (exit code 4)
- `RemoteFetchInterruptedException` — Network interrupted (exit code 5)
- `TlsException` — TLS certificate error (exit code 6)
- `ReceiptVerifyException` — Receipt verification failed (exit code 10)

## Data Types

### Source
Sealed interface for PDF input sources:
- `Source.fromPath(Path)` — Local file path
- `Source.fromUrl(String)` — Remote URL
- `Source.fromBytes(byte[])` — Raw bytes

### Document
```java
public record Document(
    String schemaVersion,
    DocumentMetadata metadata,
    List<Page> pages,
    List<ProcessingError> errors
)
```

### Page
```java
public record Page(
    int pageIndex,
    double width,
    double height,
    int rotation,
    String pageType,  // "vector" or "scanned"
    List<Span> spans,
    List<Block> blocks
)
```

### Block
```java
public record Block(
    String kind,  // "paragraph", "heading", "table", "figure", "list"
    List<Double> bbox,  // [x1, y1, x2, y2]
    List<Line> lines
)
```

### Options
- `ExtractOptions` — Extends `BaseOptions`, adds OCR settings
- `SearchOptions` — Extends `BaseOptions`, adds search settings
- `BaseOptions` — Password and common settings

## Conformance

This SDK passes the [pdftract conformance suite](https://github.com/jedarden/pdftract/tree/main/tests/sdk-conformance).

Run tests:
```bash
mvn test
```

## License

MIT License — see [LICENSE](LICENSE) for details.

## Links

- [GitHub](https://github.com/jedarden/pdftract-java)
- [pdftract CLI](https://github.com/jedarden/pdftract)
- [Conformance Report](https://github.com/jedarden/pdftract/releases/latest)
