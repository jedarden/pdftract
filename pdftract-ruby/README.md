# pdftract-ruby

Ruby SDK for pdftract - PDF extraction and conformance testing.

## Installation

```bash
gem install pdftract
```

Or in your Gemfile:

```ruby
gem 'pdftract', '~> 1.0.0'
```

## Usage

### Basic extract

```ruby
require 'pdftract'

client = Pdftract.client
doc = client.extract('document.pdf')
puts "Pages: #{doc.pages.length}"
```

### Extract with OCR

```ruby
doc = client.extract('scanned.pdf', { ocr_language: 'eng', ocr_threshold: 0.7 })
```

### Extract text

```ruby
text = client.extract_text('document.pdf')
puts text
```

### Extract Markdown

```ruby
markdown = client.extract_markdown('document.pdf')
puts markdown
```

### Stream extraction

```ruby
client.extract_stream('large.pdf').each do |page|
  puts "Page #{page.page}: #{page.blocks&.length || 0} blocks"
end
```

### Search

```ruby
client.search('document.pdf', 'invoice').each do |match|
  puts "Found on page #{match.page}: #{match.text}"
end
```

### Get metadata

```ruby
metadata = client.get_metadata('document.pdf')
puts "Title: #{metadata.title}"
puts "Pages: #{metadata.page_count}"
```

### Hash

```ruby
fingerprint = client.hash('document.pdf')
puts "SHA-256: #{fingerprint.hash}"
puts "Fast hash: #{fingerprint.fast_hash}"
```

### Classify

```ruby
classification = client.classify('document.pdf')
puts "Category: #{classification.category}"
puts "Confidence: #{classification.confidence}"
```

### Verify receipt

```ruby
valid = client.verify_receipt('document.pdf', 'receipt-data')
puts "Valid: #{valid}"
```

## Binary version compatibility

This SDK requires pdftract 1.0.0 or later. Download from:
https://github.com/jedarden/pdftract/releases

## Troubleshooting

### Binary not found
Ensure `pdftract` is on your PATH. The SDK probes PATH for the executable.

### Version mismatch
The SDK will refuse to invoke mismatched binary versions. Install the correct version.

### Network failure
For remote URLs, check your network connection and TLS certificate chain.
