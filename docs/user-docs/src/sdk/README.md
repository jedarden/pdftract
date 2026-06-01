# SDK Quickstarts

Getting started guides for using pdftract from various programming languages. Each SDK implements the same 9-method contract: `extract`, `extract_text`, `extract_markdown`, `extract_stream`, `search`, `get_metadata`, `hash`, `classify`, and `verify_receipt`.

## Available SDKs

- **[Rust](./rust.md)** — The `pdftract-core` crate with native zero-copy PDF processing
- **[Python](./python.md)** — Native Python bindings with PyO3, plus subprocess fallback
- **[JavaScript/TypeScript](./javascript.md)** — npm package with Node.js and browser support
- **[Go](./go.md)** — Go module with native bindings

## Choosing an SDK

- **Rust** — Best for performance-critical applications and CLI tools
- **Python** — Best for data science, ML pipelines, and scripting
- **JavaScript** — Best for web applications and serverless functions
- **Go** — Best for microservices and cloud-native applications

All SDKs support:
- Remote PDFs via HTTP/HTTPS URLs
- Encrypted PDFs with password
- OCR for scanned documents (with feature flag)
- Streaming extraction for large documents
- Cryptographic receipt verification

## See Also

- [JSON Schema Reference](../json-schema-reference.md)
- [CLI Reference](../cli/README.md)
- [Installation Guide](../installation.md)
