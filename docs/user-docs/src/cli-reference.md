> This page is auto-generated from the clap command tree.
> Run `cargo run --manifest-path=xtask/Cargo.toml --bin gen_cli_reference` to regenerate.

# CLI Reference

This page provides comprehensive documentation for all pdftract CLI commands and flags.

## Usage

```bash
pdftract [OPTIONS] <COMMAND>
```

## Global Options

These options are available across all subcommands:

- `-h, --help` - Print help information
- `-V, --version` - Print version information

## Commands

### `pdftract`

pdftract CLI - PDF extraction and conformance testing

pdftract is a command-line tool for extracting text and structure from PDF files.
It supports JSON, Markdown, plain text, and NDJSON output formats, with
advanced features like OCR, document classification, and conformance testing.

**Usage:**

```bash
pdftract pdftract
```

**Options:**

- `-h, --help` - Print help information
- `-V, --version` - Print version information

  #### `extract`

Extract text and structure from a PDF file

Extract content from PDF files in multiple formats.
Supports local files, remote URLs, and stdin input.

**Usage:**

```bash
pdftract extract
```

**Arguments:**

- `<input>` - Path to the PDF file (use '-' for stdin) (required)

**Options:**

- `--password-stdin` - Read password from stdin (one line, terminated by newline)
- `--password` <PASSWORD> - PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)
- `--header` <HEADER:VALUE> - Custom HTTP headers for remote sources (repeatable; format: HEADER:VALUE)
- `--pages` <RANGE> - Page range to extract (1-based, comma-separated: 1-5,7,12-)
- `--json` <PATH> - Output JSON to PATH (use '-' for stdout)
- `--md` <PATH> - Output Markdown to PATH (use '-' for stdout)
- `--text` <PATH> - Output plain text to PATH (use '-' for stdout)
- `--ndjson` - Output NDJSON to stdout (mutually exclusive with other formats)
- `--format` <FORMATS> - Output formats (comma-separated: json,markdown,text,ndjson)
- `-o, --output` <BASE> - Base path for auto-named outputs (used with --format)
- `--receipts` <MODE> - Receipt mode: off (default), lite, or svg (default: `off`)
- `--ocr` - Enable OCR for scanned pages (requires 'ocr' feature)
- `--ocr-language` <LANGS> - OCR language codes (comma-separated, e.g., 'eng,fra,deu')
- `--cache-dir` <DIR> - Enable cache at this directory (creates if absent)
- `--cache-size` <SIZE> - Set cache size limit (default 1 GiB; accepts KiB, MiB, GiB suffixes) (default: `1 GiB`)
- `--no-cache` - Disable cache for this extraction (even if --cache-dir is set)
- `--md-anchors` - Emit HTML comment anchors before each block in Markdown output
- `--auto` - Auto-detect document type and apply appropriate profile
- `--profile` <NAME|PATH> - Force-apply a specific profile (by name or YAML file path)
- `--include-headers` - Include header blocks in output
- `--include-footers` - Include footer blocks in output
- `--include-headers-footers` - Include both header and footer blocks in output
- `--include-invisible-text` - Include invisible text spans in output (rendering_mode == 3)
- `--include-hidden-layers` - Include hidden-layer text spans in output (OCG-controlled)
- `--include-watermarks` - Include watermark blocks in output (no-op until Phase 7)

  #### `classify`

Classify document type

Runs metadata + signal extraction to classify document type.
Not full text extraction - suitable for quick categorization.

**Usage:**

```bash
pdftract classify
```

**Arguments:**

- `<input>` - Path to the PDF file (required)

**Options:**

- `--password-stdin` - Read password from stdin (one line, terminated by newline)
- `--password` <PASSWORD> - PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)
- `--profiles` <DIR> - Directory containing custom profile YAML files
- `--pretty` - Pretty-print JSON output
- `--top-k` <N> - Number of top reasons to include (default: all) (default: `0`)
- `--exit-on-unknown` - Exit with code 1 if document type is unknown

  #### `grep`

Search for text patterns in PDF files

Search for text patterns with bounding-box results.
Requires the 'grep' feature flag.

**Usage:**

```bash
pdftract grep
```

**Arguments:**

- `<pattern>` - Regular expression pattern to search for (required)
- `<paths>` - PDF files or directories to search (required)

**Options:**

- `-C, --context` <LINES> - Number of context lines to show (default: `0`)
- `-i, --ignore-case` - Case-insensitive search
- `--json` - Output results as JSON

  #### `inspect`

Inspect a PDF file in a local web browser

Launch a local web server with debugging overlays for PDF inspection.
Provides visual feedback on extraction accuracy and layout analysis.
Requires the 'inspect' feature flag.

**Usage:**

```bash
pdftract inspect
```

**Arguments:**

- `<input>` - Path to the PDF file (required)

**Options:**

- `-b, --bind` <ADDR> - Bind address for the inspector server (use 0.0.0.0:0 for accessibility from other devices) (default: `127.0.0.1:0`)
- `--password` <PASSWORD> - PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)
- `--ocr` - Enable OCR for scanned pages (requires 'ocr' feature)
- `--no-browser` - Don't automatically open browser

  #### `serve`

Start the HTTP server for extraction

Start an HTTP server for PDF extraction via REST API.

**Security Model:** pdftract serve has no built-in authentication. Deploy behind a reverse proxy (nginx, Traefik, Caddy) for production use.

**Endpoints:**
- POST /extract - Extract PDF and return JSON with metadata
- POST /extract/text - Extract PDF and return plain text
- POST /extract/stream - Extract PDF and return streaming NDJSON
- GET /health - Health check

Requires the 'serve' feature flag.

**Usage:**

```bash
pdftract serve
```

**Options:**

- `-b, --bind` <ADDR> - Bind address (e.g., "127.0.0.1:8080", "[::1]:9000", "0.0.0.0:3000") (default: `127.0.0.1:8080`)
- `--cache-dir` <DIR> - Enable cache at this directory
- `--cache-size` <SIZE> - Set cache size limit (default 1 GiB; accepts KiB, MiB, GiB suffixes) (default: `1 GiB`)
- `--no-cache` - Disable cache
- `--max-upload-mb` <MB> - Maximum request body size in MB (default: 256, max: 4096) (default: `256`)
- `--max-decompress-gb` <GB> - Maximum decompression size in GB (default: 1) (default: `1`)
- `--audit-log` <FILE> - Write per-request audit log to FILE (NDJSON; use "-" for stdout)
- `--trust-forwarded-for` - Trust X-Forwarded-For header for client IP detection (DANGER: enables IP spoofing if not behind a trusted proxy)
- `--profile-dir` <DIR> - Directory containing custom profile YAML files (repeatable)
- `--profile-hot-reload` - Enable hot-reload for profiles (re-read directory on every request)

  #### `mcp`

Start the MCP (Model Context Protocol) server

Start an MCP server for AI assistant integration.

Per ADR-006: stdio and HTTP transports are mutually exclusive.
Exactly one transport must be selected per invocation.

Requires the 'mcp' feature flag.

**Usage:**

```bash
pdftract mcp
```

**Options:**

- `--stdio` - Use stdio transport (for Claude Desktop, Claude Code, Continue, Cursor)
- `-b, --bind` <ADDR> - Bind address for the MCP server (enables HTTP+SSE transport)
- `--auth-token-file` <PATH> - Path to a file containing the bearer token (RECOMMENDED)
- `--auth-token` <TOKEN> - Bearer token for authentication (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_TOKEN=1)
- `--max-upload-mb` <MB> - Maximum request body size in MB (default: 256) (default: `256`)
- `--root` <DIR> - Root directory for local filesystem access (enforces path-traversal protection)
- `--audit-log` <FILE> - Write per-request audit log to FILE (NDJSON; use "-" for stdout)

  #### `cache`

Manage the extraction cache

Manage the content-addressed extraction cache.
Cache entries are stored by PDF hash and version constraint.
Requires the 'cache' feature flag.

**Usage:**

```bash
pdftract cache
```

    #### `stats`

Show cache statistics

**Usage:**

```bash
pdftract stats
```

**Arguments:**

- `<dir>` - Path to the cache directory (required)

**Options:**

- `--json` - Output in JSON format

    #### `clear`

Clear all cache entries

Clear all cache entries (preserves index.json and sentinel)

**Usage:**

```bash
pdftract clear
```

**Arguments:**

- `<dir>` - Path to the cache directory (required)

**Options:**

- `-y, --yes` - Skip confirmation prompt

    #### `purge`

Purge old cache entries

**Usage:**

```bash
pdftract purge
```

**Arguments:**

- `<dir>` - Path to the cache directory (required)

**Options:**

- `--older-than` <DURATION> - Delete entries older than this duration (e.g., "30d", "7d", "1h")
- `--version` <CONSTRAINT> - Delete entries matching this version constraint (e.g., "<1.0.0")

  #### `profiles`

Manage document type profiles

Manage document type profiles for classification and extraction tuning.
Requires the 'profiles' feature flag.

**Usage:**

```bash
pdftract profiles
```

    #### `list`

List all available profiles

**Usage:**

```bash
pdftract list
```

    #### `show`

Show a profile's YAML content

**Usage:**

```bash
pdftract show
```

**Arguments:**

- `<name_or_path>` - Profile name or path to YAML file (required)

    #### `export`

Export a built-in profile to stdout

**Usage:**

```bash
pdftract export
```

**Arguments:**

- `<name>` - Name of the built-in profile to export (required)

    #### `install`

Install a profile to the user config directory

**Usage:**

```bash
pdftract install
```

**Arguments:**

- `<path>` - Path to the profile YAML file to install (required)

    #### `validate`

Validate a profile file

**Usage:**

```bash
pdftract validate
```

**Arguments:**

- `<path>` - Path to the profile YAML file to validate (required)

  #### `doctor`

Check environment health and dependencies

Run environment health checks for pdftract dependencies and configuration.

Exit code policy:
- Exits 0 if no checks FAIL (WARN does not affect exit code)
- Exits 1 if any check FAILs
- Exits 2 on argument parse errors

**Usage:**

```bash
pdftract doctor
```

**Options:**

- `--features` - Print compiled features and exit
- `--json` - Output results as JSON
- `--no-color` - Disable colored output
- `--exit-on-fail` - Explicit form of the default policy (exit 1 if any check FAILs)
- `--profile-dir` <DIR> - Verify the profile search path includes DIR
- `--cache-dir` <DIR> - Verify DIR is writable and has sufficient space
- `--lang` <LANGS> - Requested OCR languages (default: eng)

  #### `hash`

Compute the PDF structural fingerprint

Compute a structural hash/fingerprint of a PDF file.
This hash is based on the PDF's structure (xref, trailers, object
locations) rather than content, making it useful for identifying
identical documents with different metadata.

**Usage:**

```bash
pdftract hash
```

**Arguments:**

- `<input>` - Path to the PDF file or URL (required)

**Options:**

- `--password` <PASSWORD> - PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)
- `--header` <HEADER:VALUE> - Custom HTTP headers for remote sources (repeatable; format: HEADER:VALUE)

  #### `verify-receipt`

Verify a receipt against a PDF file

Verify a visual citation receipt against the original PDF.
Checks that quoted text appears at the expected locations.
Requires the 'receipts' feature flag.

**Usage:**

```bash
pdftract verify-receipt
```

**Arguments:**

- `<receipt>` - Path to the receipt JSON file (required)

**Options:**

- `--pdf` <PATH> - Path to the original PDF file
- `--tolerance` <PIXELS> - Tolerance for bounding box matching in pixels (default: `10`)
- `--json` - Output results as JSON

  #### `conformance`

Run SDK conformance test suite

**Usage:**

```bash
pdftract conformance
```

**Options:**

- `-s, --suite` <PATH> - Path to the conformance suite JSON (default: `tests/sdk-conformance/cases.json`)
- `-k, --sdk` <NAME> - SDK name (default: `pdftract`)
- `-v, --version` <VERSION> - SDK version (default: `0.1.0`)
- `-o, --output` <PATH> - Output report path (default: `conformance-report.json`)

  #### `compare`

Compare actual results against expected values

Compare actual extraction results against expected values with tolerances.
Used for conformance testing and validation.

**Usage:**

```bash
pdftract compare
```

**Arguments:**

- `<actual>` - Path to the actual results JSON (required)
- `<expected>` - Path to the expected results JSON (required)

**Options:**

- `-t, --tolerances` <PATH> - Path to the tolerances JSON (optional)
- `-f, --format` <FORMAT> - Output format (text, json) (default: `text`)

  #### `sdk`

SDK code generation commands

**Usage:**

```bash
pdftract sdk
```

    #### `codegen`

Generate SDK skeleton from templates

**Usage:**

```bash
pdftract codegen
```

**Options:**

- `-l, --lang` <LANG> - Target language
- `-o, --out` <DIR> - Output directory
- `-v, --version` <VERSION> - Version string (defaults to current pdftract version) (default: `0.1.0`)

    #### `validate`

Validate existing SDK against current generator output

**Usage:**

```bash
pdftract validate
```

**Options:**

- `-l, --lang` <LANG> - Target language
- `-d, --sdk-dir` <DIR> - Path to existing SDK directory

  #### `list-diagnostics`

List all diagnostic codes with their metadata

List all diagnostic codes emitted during PDF parsing and extraction.
Each diagnostic includes severity, recoverable flag, phase origin,
and suggested action.

**Usage:**

```bash
pdftract list-diagnostics
```

  #### `explain-diagnostic`

Explain a specific diagnostic code in detail

**Usage:**

```bash
pdftract explain-diagnostic
```

**Arguments:**

- `<code>` - Diagnostic code to explain (e.g., STRUCT_MISSING_KEY, STREAM_BOMB) (required)

<!-- AUTOGEN END -->

## Hand-Curated Content

> **Note:** Any content added after this marker will be preserved
> when the CLI reference is regenerated. This section is for
> additional context that doesn't fit in the auto-generated sections.

### Common Patterns

#### Basic Extraction

```bash
pdftract extract document.pdf
```

#### JSON Output

```bash
pdftract extract --json output.json document.pdf
```

#### Markdown with Anchors

```bash
pdftract extract --md-anchors --md output.md document.pdf
```

### Exit Codes

- `0`: Success
- `1`: General error (extraction failed, file not found, etc.)
- `2`: Usage error (invalid arguments, conflicting flags)
- `3`: Decryption error (wrong or missing password)
