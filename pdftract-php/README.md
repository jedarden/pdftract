# jedarden/pdftract

PHP subprocess SDK for pdftract document extraction.

## Installation

```bash
composer require jedarden/pdftract
```

## Requirements

- PHP 8.2 or higher
- The `pdftract` binary must be in your PATH or specified via constructor

## Usage

```php
use Jedarden\Pdftract\Client;
use Monolog\Logger;
use Monolog\Handler\StreamHandler;

// With optional PSR-3 logger
$logger = new Logger('pdftract');
$logger->pushHandler(new StreamHandler('php://stdout', Logger::DEBUG));

$client = new Client(logger: $logger);

// Extract document
$document = $client->extract('document.pdf');
echo "Pages: {$document->getPageCount()}\n";

// Extract text
$text = $client->extractText('document.pdf');

// Extract Markdown
$markdown = $client->extractMarkdown('document.pdf');

// Stream pages
foreach ($client->extractStream('document.pdf') as $page) {
    echo "Page {$page->page}: {$page->getText()}\n";
}

// Search
foreach ($client->search('document.pdf', 'invoice') as $match) {
    echo "Found at page {$match->page}\n";
}

// Get metadata
$metadata = $client->getMetadata('document.pdf');

// Hash for fingerprinting
$fingerprint = $client->hash('document.pdf');

// Classify document
$classification = $client->classify('document.pdf');

// Verify receipt
$receipt = \Jedarden\Pdftract\Models\Receipt::fromJson($receiptJson);
$valid = $client->verifyReceipt('document.pdf', $receipt);
```

## Options

Pass options as an associative array (camelCase per PHP-FIG convention):

```php
$document = $client->extract('document.pdf', [
    'ocrLanguage' => 'eng',
    'preserveLayout' => true,
    'extractImages' => true,
    'imageFormat' => 'png',
]);
```

### Available Options

**Extract Options (extract, extract_text, extract_markdown, extract_stream):**
- `ocrLanguage` (string, default: "eng") - ISO 639-3 language code for OCR
- `ocrThreshold` (float, default: 0.7) - Confidence threshold (0-1) for accepting OCR text
- `preserveLayout` (bool) - Preserve original reading order and layout
- `extractImages` (bool) - Extract embedded images
- `imageFormat` (string, default: "png") - Format: png, jpg, webp
- `minImageSize` (int, default: 64) - Minimum dimension (pixels) for image extraction

**Search Options (search):**
- `caseInsensitive` (bool) - Ignore case when matching
- `regex` (bool) - Treat pattern as regular expression
- `wholeWord` (bool) - Match only whole words
- `maxResults` (int|null) - Maximum matches (null = unlimited)

**Base Options (all methods):**
- `timeout` (int, default: 30) - Maximum seconds to wait

## Logging

The Client accepts any PSR-3 LoggerInterface for opt-in logging:

```php
use Monolog\Logger;
use Monolog\Handler\StreamHandler;

$logger = new Logger('pdftract');
$logger->pushHandler(new StreamHandler('php://stdout', Logger::DEBUG));

$client = new Client(logger: $logger);
```

Logs are emitted at:
- `DEBUG` - subprocess invocations
- `ERROR` - command failures

## Error Handling

All SDK exceptions inherit from `PdftractException`. Specific error types:

| Exit Code | Exception | Meaning |
|-----------|-----------|---------|
| 2 | `CorruptPdfError` | Corrupt PDF |
| 3 | `EncryptionError` | Encrypted PDF (password missing/wrong) |
| 4 | `SourceUnreachableError` | File or URL unreachable |
| 5 | `RemoteFetchInterruptedError` | Network interrupted |
| 6 | `TlsError` | TLS/certificate failure |
| 10 | `ReceiptVerifyError` | Receipt verification failed |

```php
try {
    $document = $client->extract('document.pdf');
} catch (\Jedarden\Pdftract\EncryptionError $e) {
    // Handle encrypted PDF
    echo "PDF is encrypted: " . $e->getMessage();
} catch (\Jedarden\Pdftract\PdftractException $e) {
    // Handle all other pdftract errors
    echo "Error: " . $e->getMessage();
}
```

## License

MIT OR Apache-2.0

## Support

- Issues: https://github.com/jedarden/pdftract-php/issues
- Upstream: https://github.com/jedarden/pdftract
