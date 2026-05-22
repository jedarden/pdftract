# @pdftract/sdk

Node.js SDK for pdftract - PDF extraction and conformance testing.

## Installation

```bash
npm install @pdftract/sdk@1.0.0
```

## Usage

### Basic extract

```typescript
import { Client, path } from '@pdftract/sdk';

const client = new Client();
const doc = await client.extract(path('document.pdf'));
console.log(`Pages: ${doc.pages.length}`);
```

### Extract with OCR

```typescript
import { Client, path } from '@pdftract/sdk';

const client = new Client();
const doc = await client.extract(path('scanned.pdf'), {
  ocrLanguage: 'eng',
  ocrThreshold: 0.7
});
```

### Search

```typescript
import { Client, path } from '@pdftract/sdk';

const client = new Client();
for await (const match of client.search(path('document.pdf'), 'invoice')) {
  console.log(`Found on page ${match.page}: ${match.text}`);
}
```

### Stream extraction

```typescript
import { Client, path } from '@pdftract/sdk';

const client = new Client();
for await (const page of client.extractStream(path('large.pdf'))) {
  console.log(`Page ${page.page}: ${page.blocks.length} blocks`);
}
```

## Binary version compatibility

This SDK requires pdftract 1.0.0. Download from:
https://github.com/jedarden/pdftract/releases/tag/v1.0.0

## Troubleshooting

### Binary not found
Ensure `pdftract` is on your PATH. The SDK probes PATH for the executable.

### Version mismatch
The SDK will refuse to invoke mismatched binary versions. Install the correct version.

### Network failure
For remote URLs, check your network connection and TLS certificate chain.
