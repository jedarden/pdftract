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
echo "Pages: {$document->pageCount}\n";

// Extract text
$text = $client->extractText('document.pdf');

// Extract Markdown
$markdown = $client->extractMarkdown('document.pdf');

// Stream pages
foreach ($client->extractStream('document.pdf') as $page) {
    echo "Page {$page->number}: {$page->text}\n";
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
$valid = $client->verifyReceipt('document.pdf', $receipt);
```

## Options

Pass options as an associative array:

```php
$document = $client->extract('document.pdf', [
    'ocrLanguage' => 'eng',
    'structure' => true,
]);
```

## Logging

The Client accepts any PSR-3 LoggerInterface:

```php
$client = new Client(logger: $myLogger);
```

## License

MIT

## Support

- Issues: https://github.com/jedarden/pdftract-php/issues
- Upstream: https://github.com/jedarden/pdftract
