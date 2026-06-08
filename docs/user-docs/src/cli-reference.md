# CLI Reference

> This page is auto-generated from the clap command tree.
> Run `cargo run --bin gen-cli-reference` to regenerate.

# Command-Line Help for `pdftract`

This document contains the help content for the `pdftract` command-line program.

**Command Overview:**

* [`pdftract`↴](#pdftract)
* [`pdftract list-diagnostics`↴](#pdftract-list-diagnostics)
* [`pdftract explain-diagnostic`↴](#pdftract-explain-diagnostic)
* [`pdftract compare`↴](#pdftract-compare)
* [`pdftract conformance`↴](#pdftract-conformance)
* [`pdftract sdk`↴](#pdftract-sdk)
* [`pdftract sdk codegen`↴](#pdftract-sdk-codegen)
* [`pdftract sdk validate`↴](#pdftract-sdk-validate)
* [`pdftract extract`↴](#pdftract-extract)
* [`pdftract classify`↴](#pdftract-classify)
* [`pdftract inspect`↴](#pdftract-inspect)
* [`pdftract verify-receipt`↴](#pdftract-verify-receipt)
* [`pdftract hash`↴](#pdftract-hash)
* [`pdftract cache`↴](#pdftract-cache)
* [`pdftract cache stats`↴](#pdftract-cache-stats)
* [`pdftract cache clear`↴](#pdftract-cache-clear)
* [`pdftract cache purge`↴](#pdftract-cache-purge)
* [`pdftract profiles`↴](#pdftract-profiles)
* [`pdftract profiles list`↴](#pdftract-profiles-list)
* [`pdftract profiles show`↴](#pdftract-profiles-show)
* [`pdftract profiles export`↴](#pdftract-profiles-export)
* [`pdftract profiles install`↴](#pdftract-profiles-install)
* [`pdftract profiles validate`↴](#pdftract-profiles-validate)
* [`pdftract serve`↴](#pdftract-serve)
* [`pdftract mcp`↴](#pdftract-mcp)
* [`pdftract validate`↴](#pdftract-validate)
* [`pdftract migrate-schema`↴](#pdftract-migrate-schema)
* [`pdftract doctor`↴](#pdftract-doctor)

## `pdftract`

pdftract CLI - PDF extraction and conformance testing

**Usage:** `pdftract <COMMAND>`

###### **Subcommands:**

* `list-diagnostics` — List all diagnostic codes with their metadata
* `explain-diagnostic` — Explain a specific diagnostic code in detail
* `compare` — Compare actual results against expected values with tolerances (for conformance testing)
* `conformance` — Run SDK conformance test suite
* `sdk` — SDK code generation commands
* `extract` — Extract text and structure from a PDF file
* `classify` — Classify document type (runs metadata + signal extraction, not full text extraction)
* `inspect` — Inspect a PDF file in a local web browser with debugging overlays
* `verify-receipt` — Verify a receipt against a PDF file
* `hash` — Compute the PDF structural fingerprint (hash)
* `cache` — Manage the extraction cache
* `profiles` — Manage document type profiles
* `serve` — Start the HTTP server for extraction
* `mcp` — Start the MCP (Model Context Protocol) server
* `validate` — Validate a JSON file against the pdftract schema
* `migrate-schema` — Migrate JSON output between schema versions
* `doctor` — Check environment health and dependencies



## `pdftract list-diagnostics`

List all diagnostic codes with their metadata

**Usage:** `pdftract list-diagnostics`



## `pdftract explain-diagnostic`

Explain a specific diagnostic code in detail

**Usage:** `pdftract explain-diagnostic <CODE>`

###### **Arguments:**

* `<CODE>` — Diagnostic code to explain (e.g., STRUCT_MISSING_KEY, STREAM_BOMB)



## `pdftract compare`

Compare actual results against expected values with tolerances (for conformance testing)

**Usage:** `pdftract compare [OPTIONS] <ACTUAL> <EXPECTED>`

###### **Arguments:**

* `<ACTUAL>` — Path to the actual results JSON
* `<EXPECTED>` — Path to the expected results JSON

###### **Options:**

* `-t`, `--tolerances <TOLERANCES>` — Path to the tolerances JSON (optional)
* `-f`, `--format <FORMAT>` — Output format (text, json)

  Default value: `text`



## `pdftract conformance`

Run SDK conformance test suite

**Usage:** `pdftract conformance [OPTIONS]`

###### **Options:**

* `-s`, `--suite <SUITE>` — Path to the conformance suite JSON

  Default value: `tests/sdk-conformance/cases.json`
* `-k`, `--sdk <SDK>` — SDK name

  Default value: `pdftract`
* `-v`, `--version <VERSION>` — SDK version

  Default value: `0.1.0`
* `-o`, `--output <OUTPUT>` — Output report path

  Default value: `conformance-report.json`



## `pdftract sdk`

SDK code generation commands

**Usage:** `pdftract sdk <COMMAND>`

###### **Subcommands:**

* `codegen` — Generate SDK skeleton from templates
* `validate` — Validate existing SDK against current generator output



## `pdftract sdk codegen`

Generate SDK skeleton from templates

**Usage:** `pdftract sdk codegen --lang <LANG> --out <OUT>`

###### **Options:**

* `-l`, `--lang <LANG>` — Target language

  Possible values: `python`, `rust`, `node`, `go`, `java`, `dotnet`, `ruby`, `php`, `swift`

* `-o`, `--out <OUT>` — Output directory
* `-v`, `--version <VERSION>` — Version string (defaults to current pdftract version)

  Default value: `0.1.0`



## `pdftract sdk validate`

Validate existing SDK against current generator output

**Usage:** `pdftract sdk validate --lang <LANG> --sdk-dir <SDK_DIR>`

###### **Options:**

* `-l`, `--lang <LANG>` — Target language

  Possible values: `python`, `rust`, `node`, `go`, `java`, `dotnet`, `ruby`, `php`, `swift`

* `-s`, `--sdk-dir <SDK_DIR>` — Path to existing SDK directory



## `pdftract extract`

Extract text and structure from a PDF file

**Usage:** `pdftract extract [OPTIONS] <INPUT>`

###### **Arguments:**

* `<INPUT>` — Path to the PDF file (use '-' for stdin)

###### **Options:**

* `--password-stdin` — Read password from stdin (one line, terminated by newline)
* `--password <PASSWORD>` — PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)
* `--header <HEADER:VALUE>` — Custom HTTP headers for remote sources (repeatable; format: HEADER:VALUE)
* `--pages <RANGE>` — Page range to extract (1-based, comma-separated: 1-5,7,12-)
* `--json <PATH>` — Output JSON to PATH (use '-' for stdout)
* `--md <PATH>` — Output Markdown to PATH (use '-' for stdout)
* `--text <PATH>` — Output plain text to PATH (use '-' for stdout)
* `--ndjson` — Output NDJSON to stdout (mutually exclusive with other formats)
* `--format <FORMATS>` — Output formats (comma-separated: json,markdown,text,ndjson)
* `-o`, `--output <BASE>` — Base path for auto-named outputs (used with --format)
* `--receipts <MODE>` — Receipt mode: off (default), lite, or svg

  Default value: `off`

  Possible values: `off`, `lite`, `svg`

* `--ocr` — Enable OCR for scanned pages (requires 'ocr' feature)
* `--ocr-language <OCR_LANGUAGE>` — OCR language codes (comma-separated, e.g., 'eng,fra,deu')
* `--cache-dir <DIR>` — Enable cache at this directory (creates if absent)
* `--cache-size <SIZE>` — Set cache size limit (default 1 GiB; accepts KiB, MiB, GiB suffixes)

  Default value: `1 GiB`
* `--no-cache` — Disable cache for this extraction (even if --cache-dir is set)
* `--md-anchors` — Emit HTML comment anchors before each block in Markdown output
* `--md-no-page-breaks` — Suppress page-break horizontal rules between pages
* `--auto` — Auto-detect document type and apply appropriate profile
* `--profile <NAME|PATH>` — Force-apply a specific profile (by name or YAML file path)
* `--include-headers` — Include header blocks in output
* `--include-footers` — Include footer blocks in output
* `--include-headers-footers` — Include both header and footer blocks in output
* `--include-invisible-text` — Include invisible text spans in output (rendering_mode == 3)
* `--include-hidden-layers` — Include hidden-layer text spans in output (OCG-controlled)
* `--include-watermarks` — Include watermark blocks in output (no-op until Phase 7)



## `pdftract classify`

Classify document type (runs metadata + signal extraction, not full text extraction)

**Usage:** `pdftract classify [OPTIONS] <INPUT>`

###### **Arguments:**

* `<INPUT>` — Path to the PDF file

###### **Options:**

* `--password-stdin` — Read password from stdin (one line, terminated by newline)
* `--password <PASSWORD>` — PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)
* `--profiles <DIR>` — Directory containing custom profile YAML files
* `--pretty` — Pretty-print JSON output
* `--top-k <TOP_K>` — Number of top reasons to include (default: all)

  Default value: `0`
* `--exit-on-unknown` — Exit with code 1 if document type is unknown



## `pdftract inspect`

Inspect a PDF file in a local web browser with debugging overlays

**Usage:** `pdftract inspect [OPTIONS] <FILE>`

###### **Arguments:**

* `<FILE>` — Path to the PDF file to inspect

###### **Options:**

* `-p`, `--port <PORT>` — Port to bind the inspector server (default: 7676)

  Default value: `7676`
* `-b`, `--bind <BIND>` — Bind address for the inspector server (default: 127.0.0.1)

   Binding to a non-loopback address requires --auth-token for security.

  Default value: `127.0.0.1`
* `--auth-token <AUTH_TOKEN>` — Authentication token for non-loopback binds

   Required when --bind is not a loopback address (127.0.0.1 or ::1).
* `--no-open` — Suppress automatic browser launch

   Useful for CI environments or when you want to manually open the browser.
* `--compare <FILE>` — Optional second PDF file for comparative debugging

   When provided, the inspector shows side-by-side comparison.
* `--audit-log <FILE>` — Write per-request audit log to FILE (NDJSON; use "-" for stdout, "/dev/stderr" for stderr)

   Rotation: pdftract does NOT rotate logs; configure logrotate on the audit-log file. When FILE is "-", rotation is the responsibility of the supervisor (e.g., journald).



## `pdftract verify-receipt`

Verify a receipt against a PDF file

**Usage:** `pdftract verify-receipt [OPTIONS] <FILE.pdf> <RECEIPT.json>`

###### **Arguments:**

* `<FILE.pdf>` — Path to the PDF file to verify against
* `<RECEIPT.json>` — Path to the receipt JSON file, or "-" for stdin

###### **Options:**

* `--stdin` — Read receipt from stdin (alternative to "-")
* `--inline <INLINE>` — Receipt JSON as inline string (alternative to file path)
* `--json` — Output machine-readable JSON result
* `--quiet` — Suppress human-readable output (exit code only)
* `--password <PASSWORD>` — PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)
* `--password-stdin` — Read password from stdin (one line, terminated by newline)



## `pdftract hash`

Compute the PDF structural fingerprint (hash)

**Usage:** `pdftract hash [OPTIONS] <INPUT>`

###### **Arguments:**

* `<INPUT>` — Path to the PDF file or URL

###### **Options:**

* `--password <PASSWORD>` — PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)
* `--header <HEADER:VALUE>` — Custom HTTP headers for remote sources (repeatable; format: HEADER:VALUE)



## `pdftract cache`

Manage the extraction cache

**Usage:** `pdftract cache <COMMAND>`

###### **Subcommands:**

* `stats` — Show cache statistics
* `clear` — Clear all cache entries (preserves index.json and sentinel)
* `purge` — Purge old cache entries



## `pdftract cache stats`

Show cache statistics

**Usage:** `pdftract cache stats [OPTIONS] <DIR>`

###### **Arguments:**

* `<DIR>` — Path to the cache directory

###### **Options:**

* `--json` — Output in JSON format



## `pdftract cache clear`

Clear all cache entries (preserves index.json and sentinel)

**Usage:** `pdftract cache clear [OPTIONS] <DIR>`

###### **Arguments:**

* `<DIR>` — Path to the cache directory

###### **Options:**

* `-y`, `--yes` — Skip confirmation prompt



## `pdftract cache purge`

Purge old cache entries

**Usage:** `pdftract cache purge [OPTIONS] <DIR>`

###### **Arguments:**

* `<DIR>` — Path to the cache directory

###### **Options:**

* `--older-than <DURATION>` — Delete entries older than this duration (e.g., "30d", "7d", "1h")
* `--version <CONSTRAINT>` — Delete entries matching this version constraint (e.g., "<1.0.0")



## `pdftract profiles`

Manage document type profiles

**Usage:** `pdftract profiles <COMMAND>`

###### **Subcommands:**

* `list` — List all available profiles
* `show` — Show a profile's YAML content
* `export` — Export a built-in profile to stdout
* `install` — Install a profile to the user config directory
* `validate` — Validate a profile file



## `pdftract profiles list`

List all available profiles

**Usage:** `pdftract profiles list`



## `pdftract profiles show`

Show a profile's YAML content

**Usage:** `pdftract profiles show <NAME_OR_PATH>`

###### **Arguments:**

* `<NAME_OR_PATH>` — Profile name or path to YAML file



## `pdftract profiles export`

Export a built-in profile to stdout

**Usage:** `pdftract profiles export <NAME>`

###### **Arguments:**

* `<NAME>` — Name of the built-in profile to export



## `pdftract profiles install`

Install a profile to the user config directory

**Usage:** `pdftract profiles install <PATH>`

###### **Arguments:**

* `<PATH>` — Path to the profile YAML file to install



## `pdftract profiles validate`

Validate a profile file

**Usage:** `pdftract profiles validate <PATH>`

###### **Arguments:**

* `<PATH>` — Path to the profile YAML file to validate



## `pdftract serve`

Start the HTTP server for extraction

## Security Model

**pdftract serve has no built-in authentication.** Deploy behind a reverse proxy (nginx, Traefik, Caddy) for production use. The server accepts PDFs via multipart upload only; no endpoint accepts file paths from server filesystem.

## Concurrency

The server uses a two-level concurrency architecture:

- **tokio**: Per-request concurrency via the async executor. Each HTTP request is handled asynchronously on tokio's multi-threaded runtime. - **rayon**: Per-document parallelism within each extraction. PDF pages are processed in parallel using rayon's work-stealing thread pool.

The bridge between async (tokio) and sync (rayon) is `tokio::task::spawn_blocking`. Each POST handler wraps the synchronous extraction call in `spawn_blocking`, which runs the work on tokio's blocking thread pool (separate from the async reactor).

This design ensures: - The async reactor is never blocked by extraction work - Multiple PDFs can be extracted concurrently (one per request) - Within each PDF, pages are processed in parallel (rayon) - Thread pools are sized appropriately (tokio: 512 blocking threads; rayon: num_cpus)

## Endpoints

- `POST /extract` - Extract PDF and return JSON with metadata - `POST /extract/text` - Extract PDF and return plain text - `POST /extract/stream` - Extract PDF and return streaming NDJSON - `GET /health` - Health check (responds within 100ms even during concurrent extractions)

## Cache

Cache is optional. When enabled, extracted results are stored on disk and reused for identical PDFs. Cache status is reported via the `X-Pdftract-Cache` response header.

**Usage:** `pdftract serve [OPTIONS]`

###### **Options:**

* `-b`, `--bind <BIND>` — Bind address (e.g., "127.0.0.1:8080", "[::1]:9000", "0.0.0.0:3000")

  Default value: `127.0.0.1:8080`
* `--cache-dir <DIR>` — Enable cache at this directory
* `--cache-size <SIZE>` — Set cache size limit (default 1 GiB; accepts KiB, MiB, GiB suffixes)

  Default value: `1 GiB`
* `--no-cache` — Disable cache
* `--max-upload-mb <MAX_UPLOAD_MB>` — Maximum request body size in MB (default: 256, max: 4096)

  Default value: `256`
* `--max-decompress-gb <GB>` — Maximum decompression size in GB (default: 1, overrides per-request max_decompress_gb)

  Default value: `1`
* `--audit-log <FILE>` — Write per-request audit log to FILE (NDJSON; use "-" for stdout, "/dev/stderr" for stderr)

   Rotation: pdftract does NOT rotate logs; configure logrotate on the audit-log file. When FILE is "-", rotation is the responsibility of the supervisor (e.g., journald).
* `--trust-forwarded-for` — Trust X-Forwarded-For header for client IP detection (DANGER: enables IP spoofing if not behind a trusted proxy)
* `--profile-dir <DIR>` — Directory containing custom profile YAML files (repeatable)
* `--profile-hot-reload` — Enable hot-reload for profiles (re-read directory on every request)



## `pdftract mcp`

Start the MCP (Model Context Protocol) server

Per ADR-006: stdio and HTTP transports are mutually exclusive because they have opposite stdout discipline (stdio: JSON-RPC sink; HTTP: log channel). Exactly one transport must be selected per invocation.

**Usage:** `pdftract mcp [OPTIONS]`

###### **Options:**

* `--stdio` — Use stdio transport (for Claude Desktop, Claude Code, Continue, Cursor)

   This is the default transport mode if neither --stdio nor --bind is specified.
* `-b`, `--bind <ADDR>` — Bind address for the MCP server (e.g., "127.0.0.1:8080", "[::1]:9000", "0.0.0.0:3000")

   Enables HTTP+SSE transport mode. Mutually exclusive with --stdio.
* `--auth-token-file <AUTH_TOKEN_FILE>` — Path to a file containing the bearer token (RECOMMENDED)
* `--auth-token <AUTH_TOKEN>` — Bearer token for authentication (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_TOKEN=1)
* `--max-upload-mb <MAX_UPLOAD_MB>` — Maximum request body size in MB (default: 256)

  Default value: `256`
* `--root <DIR>` — Root directory for local filesystem access (enforces path-traversal protection)

   When set, all local-path tool arguments are resolved relative to DIR and any path that escapes DIR is rejected with JSON-RPC error code -32602. HTTPS URLs are not affected by this flag. Without --root, the server runs in trust-the-caller mode (no path-check applied).
* `--audit-log <FILE>` — Write per-request audit log to FILE (NDJSON; use "-" for stdout, "/dev/stderr" for stderr)

   Rotation: pdftract does NOT rotate logs; configure logrotate on the audit-log file. When FILE is "-", rotation is the responsibility of the supervisor (e.g., journald).



## `pdftract validate`

Validate a JSON file against the pdftract schema

**Usage:** `pdftract validate [OPTIONS] <FILE>`

###### **Arguments:**

* `<FILE>` — Path to the JSON file to validate (use '-' for stdin)

###### **Options:**

* `-s`, `--schema <PATH>` — Path to a custom schema file (default: bundled v1.0 schema)
* `-q`, `--quiet` — Quiet mode - suppress error output (only exit code matters)



## `pdftract migrate-schema`

Migrate JSON output between schema versions

**Usage:** `pdftract migrate-schema [OPTIONS] --from <FROM> --to <TO> [INPUT]`

###### **Arguments:**

* `<INPUT>` — Input JSON file (use '-' for stdin)

  Default value: `-`

###### **Options:**

* `--from <FROM>` — Source schema version (e.g., "1.0", "1.1")
* `--to <TO>` — Target schema version (e.g., "1.0", "1.1")
* `-o`, `--output <OUTPUT>` — Output JSON file (use '-' for stdout)

  Default value: `-`
* `-p`, `--pretty` — Pretty-print output JSON



## `pdftract doctor`

Check environment health and dependencies

Exit code policy: exits 0 if no checks FAIL (WARN does not affect exit code); exits 1 if any check FAILs; exits 2 on argument parse errors.

**Usage:** `pdftract doctor [OPTIONS]`

###### **Options:**

* `--features` — Print compiled features and exit
* `--json` — Output results as JSON
* `--no-color` — Disable colored output
* `--exit-on-fail` — Explicit form of the default policy (exit 1 if any check FAILs).

   This flag is the default behavior and is provided for CI script readability. WARN does not affect exit code regardless of this flag.
* `--profile-dir <DIR>` — Verify the profile search path includes DIR
* `--cache-dir <DIR>` — Verify DIR is writable and has sufficient space
* `--lang <LANG>` — Requested OCR languages (default: eng)



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>


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
