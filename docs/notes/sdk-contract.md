# pdftract SDK Contract

## Scope

This document is the constitutional specification for all pdftract SDK implementations. Every SDK in every language MUST implement the contract exactly as written — deviations are bugs in the SDK, not in this document. The contract defines the method surface, error mapping, versioning compatibility, option naming conventions, native type requirements, and conformance enforcement that make it possible to maintain ten SDKs with a single maintainer.

## Method surface

All SDKs expose nine methods mirroring CLI subcommands and MCP tools.

| Method | Maps to CLI | Maps to MCP tool |
|---|---|---|
| `extract(source, options) -> Document` | `pdftract extract --json` | `extract` |
| `extract_text(source, options) -> string` | `pdftract extract --text` | `extract_text` |
| `extract_markdown(source, options) -> string` | `pdftract extract --md` | `extract_markdown` |
| `extract_stream(source, options) -> Iterator<Page>` | `pdftract extract --ndjson` | (n/a) |
| `search(source, pattern, options) -> Iterator<Match>` | `pdftract grep` | `search` |
| `get_metadata(source, options) -> Metadata` | `pdftract extract --metadata-only` | `get_metadata` |
| `hash(source, options) -> Fingerprint` | `pdftract hash` | `hash` |
| `classify(source) -> Classification` | `pdftract classify` | `classify` |
| `verify_receipt(path, receipt) -> bool` | `pdftract verify-receipt` | (n/a) |

### Method signatures

**extract**
- Signature: `extract(source: Source, options?: ExtractOptions) -> Document`
- Honored options: `ocr_language`, `ocr_threshold`, `preserve_layout`, `extract_images`, `image_format`, `min_image_size`
- Return: `Document` struct (see Return types)
- Errors: All 8 error mappings from Error mapping section

**extract_text**
- Signature: `extract_text(source: Source, options?: ExtractOptions) -> string`
- Honored options: `ocr_language`, `ocr_threshold`, `preserve_layout`
- Return: Plain text string (all pages concatenated with `\n\n` separators)
- Errors: All 8 error mappings from Error mapping section

**extract_markdown**
- Signature: `extract_markdown(source: Source, options?: ExtractOptions) -> string`
- Honored options: `ocr_language`, `ocr_threshold`, `preserve_layout`
- Return: Markdown formatted string
- Errors: All 8 error mappings from Error mapping section

**extract_stream**
- Signature: `extract_stream(source: Source, options?: ExtractOptions) -> Iterator<Page>`
- Honored options: `ocr_language`, `ocr_threshold`, `preserve_layout`
- Return: Lazy iterator yielding `Page` structs one at a time
- Errors: All 8 error mappings from Error mapping section; may raise during iteration

**search**
- Signature: `search(path: string, pattern: string, options?: SearchOptions) -> Iterator<Match>`
- Honored options: `case_insensitive`, `regex`, `whole_word`
- Return: The PyO3 native entry point returns `{"pattern": string, "matches": Match[]}`;
  the public Python wrapper yields `Match` objects from that list
- Errors: See the PyO3/native search error behavior below

#### PyO3/native search contract

This subsection is authoritative for `crates/pdftract-py/src/lib.rs::search`
and `pdftract_core::sdk::search`. It describes the native binding's actual
boundary, which is narrower than the general `Source` union documented below:

- The Python entry point requires two positional strings:
  `search(path: str, pattern: str, **kwargs)`. `path` is passed to
  `Path::new` and therefore names a local PDF filesystem path; URLs and byte
  buffers are not accepted by this PyO3 function. `pattern` is required and
  is forwarded unchanged as the result dictionary's `pattern` value. There is
  no explicit non-empty-pattern validation.
- The Rust entry point is
  `search(pdf_path: &Path, pattern: &str, case_insensitive: bool,
  use_regex: bool, whole_word: bool) -> anyhow::Result<Vec<SearchMatch>>`.
  It extracts with `ExtractionOptions::default()` and scans every extracted
  page span in page order, then span order.
- The only supported Python matcher options are boolean
  `case_insensitive`, `regex`, and `whole_word`; each defaults to `false`.
  Unknown keyword names are ignored. A missing value, `None`, or value that
  cannot be extracted as a boolean is treated as `false` by the native
  parser.
- Matching uses these rules, in precedence order: `whole_word=true` searches
  for a literal escaped pattern surrounded by `\b` boundaries (and therefore
  takes precedence over `regex=true`); otherwise `regex=true` uses the raw
  pattern as a Rust `regex` expression; otherwise the pattern is escaped and
  searched literally. `case_insensitive=true` adds the `(?i)` flag to the
  selected expression.
- A match is emitted once for each span whose text matches. `text` is the
  complete span text, not the substring matched inside it, and `bbox` is the
  span's full bounding box. There is no `max_results` option and no per-match
  substring geometry.

The PyO3 native result has exactly this shape (and no other match fields):

```json
{
  "pattern": "<the input pattern>",
  "matches": [
    {
      "page_index": 0,
      "span_index": 0,
      "text": "<complete span text>",
      "bbox": [x0, y0, x1, y1]
    }
  ]
}
```

`page_index` and `span_index` are zero-based `usize` values. `page_index`
identifies the extracted page and `span_index` identifies the span within that
page. `text` is a Python `str` copied from the span. `bbox` is a Python list
of exactly four `float` values in PDF points, ordered `[x0, y0, x1, y1]`;
the Rust source type is `[f64; 4]`. The list is ordered by page, then span.
With no matches, the call succeeds and returns `matches: []`.

Core search returns `anyhow::Result`; PDF extraction/open/parse failures are
returned as `Err`. An invalid raw regex is also an `Err` with context
`Invalid regex pattern: <pattern>`. At the PyO3 boundary, errors are converted
by the existing message classifier: encryption/password failures become
`EncryptionError`, corrupt/invalid failures become `CorruptPdfError`, TLS or
certificate failures become `TlsError`, network/interruption failures become
`RemoteFetchInterruptedError`, unreachable/not-found failures become
`SourceUnreachableError`, and all other failures become `PdftractError`.
The converted exception carries `page_index=None`; classified errors also
carry the classifier's diagnostic `code` and catalog `hint`. In particular,
an invalid regex is exposed as `CorruptPdfError` with code
`STRUCT_INVALID_NAME` and a message containing `Invalid regex pattern`.

The deterministic downstream fixture is
`crates/pdftract-py/tests/fixtures/search_regression.pdf`, generated by
`crates/pdftract-py/tests/fixtures/generate_search_regression_pdf.py`. The
known string is `PYTHON_SEARCH_REGRESSION`; its checked-in expectation is
`crates/pdftract-py/tests/fixtures/search_regression.expected.json`:

```json
{
  "page_index": 0,
  "span_index": 0,
  "text": "PYTHON_SEARCH_REGRESSION",
  "bbox": [72.0, 720.0, 244.8000030517578, 732.0]
}
```

The Rust assertion lives in
`crates/pdftract-core/tests/search_contract.rs`; Python binding coverage lives
in `crates/pdftract-py/tests/test_search_integration.py`.

**get_metadata**
- Signature: `get_metadata(source: Source, options?: BaseOptions) -> Metadata`
- Honored options: `timeout`
- Return: `Metadata` struct
- Errors: Exit codes 0, 4, 5, 6 only

**hash**
- Signature: `hash(source: Source, options?: BaseOptions) -> Fingerprint`
- Honored options: `timeout`
- Return: `Fingerprint` struct
- Errors: Exit codes 0, 2, 4, 5, 6

**classify**
- Signature: `classify(source: Source) -> Classification`
- Honored options: None
- Return: `Classification` struct
- Errors: Exit codes 0, 2, 4, 5, 6

**verify_receipt**
- Signature: `verify_receipt(path: string, receipt: string) -> bool`
- Honored options: None
- Return: `true` if receipt valid, `false` otherwise (does NOT raise for exit code 10)
- Errors: Exit codes 0, 2, 4 only

## Source argument types

The `source` parameter for all methods (except `verify_receipt`) accepts three types:

| Type | Format | Example |
|---|---|---|
| Path | Local filesystem path (absolute or relative) | `"/path/to/file.pdf"` or `"./doc.pdf"` |
| URL | Remote URL with `http://` or `https://` scheme | `"https://example.com/doc.pdf"` |
| Bytes | Language-native byte buffer | `bytes` (Python), `Buffer` (Node), `byte[]` (Java/C#), `[]byte` (Go) |

SDKs MAY add language-specific overloads for path-like types (e.g. `java.io.File`, `System.IO.FileInfo`, `pathlib.Path`) without violating the contract. The contract requires the three core types; additional conveniences are permitted.

## Options object

The options object is the union of all CLI flags. Individual methods document which options they honor. All options are optional; default values match the CLI defaults.

### BaseOptions (all methods)

| Option | Type | Default | Description |
|---|---|---|---|
| `timeout` | number | 30 | Maximum seconds to wait for the operation |

### ExtractOptions (extract, extract_text, extract_markdown, extract_stream)

| Option | Type | Default | Description |
|---|---|---|---|
| `ocr_language` | string | `"eng"` | ISO 639-3 language code for OCR |
| `ocr_threshold` | number | 0.7 | Confidence threshold (0-1) for accepting OCR text |
| `preserve_layout` | boolean | false | Preserve original reading order and layout |
| `extract_images` | boolean | false | Extract embedded images |
| `image_format` | string | `"png"` | Format for extracted images: `png`, `jpg`, `webp` |
| `min_image_size` | number | 64 | Minimum dimension (pixels) for image extraction |

### SearchOptions (search)

| Option | Type | Default | Description |
|---|---|---|---|
| `case_insensitive` | boolean | false | Ignore case when matching |
| `regex` | boolean | false | Treat pattern as regular expression |
| `whole_word` | boolean | false | Match only whole words |

## Return types

All SDKs MUST expose language-native types, NOT raw JSON dictionaries. The type definitions below derive from the JSON schema in `docs/schema/v1.0/`. SDKs SHOULD generate these types from the schema to guarantee alignment.

### Document

```typescript
{
  schema_version: string;           // e.g. "1.0"
  pages: Page[];
  metadata: Metadata;
}
```

### Page

```typescript
{
  page: number;                     // 1-indexed page number
  width: number;                    // Points (1/72 inch)
  height: number;                   // Points
  rotation: number;                 // Degrees (0, 90, 180, 270)
  spans: Span[];
  blocks: Block[];
}
```

### Span

```typescript
{
  text: string;
  bbox: [number, number, number, number];  // [x0, y0, x1, y1] in points
  font: string;
  size: number;                     // Font size in points
  confidence: number;               // 0-1, null for non-OCR text
}
```

### Block

```typescript
{
  kind: 'paragraph' | 'heading' | 'table' | 'figure' | 'list';
  text: string;
  bbox: [number, number, number, number];
  level?: number;                   // For headings (1-6) and lists (nested depth)
}
```

### Match

The generic shape below is used by language-level SDKs that add context or
rename `page_index` to `page`. It does not override the exact PyO3/native
dictionary shape specified in the search contract above; that native shape has
`page_index`, `span_index`, `text`, and four-coordinate `bbox` only.

```typescript
{
  text: string;                     // The matched text
  page: number;                     // Page number where match occurred
  bbox: [number, number, number, number];  // Location of the match
  context: {                       // 50 chars before/after
    before: string;
    after: string;
  }
}
```

### Fingerprint

```typescript
{
  hash: string;                     // SHA-256 hex of document content
  page_count: number;
  fast_hash: string;                // BLAKE3 hex of first 10KB
  metadata: Metadata;
}
```

### Classification

```typescript
{
  category: string;                 // Primary category
  confidence: number;               // 0-1
  tags: string[];
  heuristics: Record<string, boolean>;  // Individual feature detections
}
```

### Metadata

```typescript
{
  title?: string;
  author?: string;
  subject?: string;
  keywords?: string[];
  creator?: string;
  producer?: string;
  created?: string;                 // ISO 8601 date
  modified?: string;                // ISO 8601 date
  page_count: number;
}
```

## Error mapping

All SDKs map CLI exit codes to language-native exception classes. The mapping is exhaustive; new error kinds require a contract bump and coordinated SDK release.

| Exit code | Meaning | Native exception |
|---|---|---|
| 0 | Success | (no exception) |
| 2 | Corrupt PDF | `CorruptPdfError` |
| 3 | Encrypted / password missing/wrong | `EncryptionError` |
| 4 | Source unreadable | `SourceUnreachableError` |
| 5 | Network interrupted | `RemoteFetchInterruptedError` |
| 6 | TLS / cert failure | `TlsError` |
| 10 | Receipt verify failed | `ReceiptVerifyError` |
| any other non-zero | Internal | `PdftractError` (base) |

### Per-language base exception types

Every language-specific exception inherits from a single base type following language conventions:

| Language | Base type |
|---|---|
| Python | `class PdftractError(Exception)` |
| Node.js | `class PdftractError extends Error` |
| Java | `class PdftractException extends Exception` |
| C# | `class PdftractException : Exception` |
| Go | Single `error` type with `errors.As`-compatible kind |
| Ruby | `class PdftractError < StandardError` |
| PHP | `class PdftractException extends Exception` |
| Rust | `enum PdftractError` (all variants in one enum) |
| C++ | `class PdftractException : std::runtime_error` |

### Exception properties

All exceptions MUST expose:

| Property | Type | Description |
|---|---|---|
| `exit_code` | number | The CLI exit code |
| `message` | string | Human-readable description |
| `stderr` | string? | Raw stderr output from CLI (if available) |

## Versioning compatibility

SDK version is pinned to binary version via semantic versioning.

### MAJOR version lock

- SDK MAJOR MUST match binary MAJOR exactly
- `@pdftract/sdk@1.x.y` works with `pdftract@1.0.0` through `pdftract@1.x.x`
- `@pdftract/sdk@1.x.y` MUST refuse to invoke `pdftract@2.0.0` with a clear startup error
- The error message MUST indicate version mismatch and suggest installing the correct binary

### MINOR version flexibility

- SDK MINOR MAY add wrappers for new binary features behind feature flags
- Calling a method whose underlying CLI subcommand the binary doesn't recognize raises `UnsupportedOperationError`
- Example: SDK 1.3 calling `hash()` when binary 1.0 lacks the `hash` subcommand raises `UnsupportedOperationError`

### Binary resolution

The SDK constructor or client initialization follows this resolution order:

1. Explicit binary path (if provided via constructor/client config)
2. Probe PATH for `pdftract` executable
3. Download matching binary version into per-user cache (opt-in via `auto_install=true`)

Download URL format:
```
https://github.com/jedarden/pdftract/releases/download/v{VERSION}/pdftract-{TARGET}.tar.gz
```

Where `{TARGET}` is one of:
- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

## Option-naming conventions

CLI flags use `kebab-case`. Each SDK converts to its language's conventional case.

| CLI flag | Python | Node.js | Go | Java | C# | C |
|---|---|---|---|---|---|---|
| `--ocr-language` | `ocr_language` | `ocrLanguage` | `OCRLanguage` | `ocrLanguage` | `OcrLanguage` | `ocr_language` |
| `--ocr-threshold` | `ocr_threshold` | `ocrThreshold` | `OCRThreshold` | `ocrThreshold` | `OcrThreshold` | `ocr_threshold` |
| `--preserve-layout` | `preserve_layout` | `preserveLayout` | `PreserveLayout` | `preserveLayout` | `PreserveLayout` | `preserve_layout` |
| `--extract-images` | `extract_images` | `extractImages` | `ExtractImages` | `extractImages` | `ExtractImages` | `extract_images` |
| `--image-format` | `image_format` | `imageFormat` | `ImageFormat` | `imageFormat` | `ImageFormat` | `image_format` |
| `--min-image-size` | `min_image_size` | `minImageSize` | `MinImageSize` | `minImageSize` | `MinImageSize` | `min_image_size` |
| `--case-insensitive` | `case_insensitive` | `caseInsensitive` | `CaseInsensitive` | `caseInsensitive` | `CaseInsensitive` | `case_insensitive` |
| `--whole-word` | `whole_word` | `wholeWord` | `WholeWord` | `wholeWord` | `WholeWord` | `whole_word` |
| `--max-results` | `max_results` | `maxResults` | `MaxResults` | `maxResults` | `MaxResults` | `max_results` |

C bindings keep `snake_case` for the FFI ABI stability.

## Native type mapping

SDKs MUST expose return types as language-native structs/classes. An SDK that returns `Map<String, Object>` or `Dict[str, Any]` fails the contract even if the data is correct.

### Type generation

The canonical source of truth for type definitions is the JSON schema in `docs/schema/v1.0/`. SDKs SHOULD generate types from this schema using code generation tools:

| Language | Recommended tool |
|---|---|
| Python | `datamodel-code-generator` + Pydantic |
| Node.js/TypeScript | `json-schema-to-typescript` or `quicktype` |
| Java | `jsonschema2pojo` |
| C# | `NJsonSchema` |
| Go | `jsonschema` |
| Rust | `serde_json` + manual structs |
| Ruby | `json_schemer` + dry-struct |

## Async conventions

Each SDK follows its language's idiomatic async pattern.

| Language | Async pattern |
|---|---|
| Node.js | All methods return `Promise<T>` |
| Python | Optional asyncio via `asyncio.to_thread`; sync methods by default |
| Go | `context.Context` passed for cancellation |
| C# | All methods return `Task<T>` |
| Java | Optional `CompletableFuture<T>`; sync methods by default |
| Rust | `async fn` with tokio/async-std runtime |

## Conformance

All SDKs MUST pass the conformance suite before release. The suite lives at `tests/sdk-conformance/cases.json` and defines language-neutral test cases.

### Conformance format

Each test case specifies:

```json
{
  "id": "extract-vector-academic-paper",
  "fixture": "fixtures/vector/academic-paper-2col.pdf",
  "method": "extract",
  "options": { "ocr_language": "eng" },
  "assertions": {
    "page_count": 2,
    "has_title": true,
    "has_blocks": true
  }
}
```

### Gating rule

100% of conformance tests MUST pass before an SDK is published to its package registry. A failing test is a release blocker.

### Running conformance

Each SDK repo includes a test runner that:
1. Parses `cases.json`
2. Executes each case against the SDK
3. Compares assertions to actual results
4. Reports pass/fail per case

## Change policy

The contract is versioned with the `schema_version` field. Changing the contract requires:

1. An Architecture Decision Record (ADR) describing the change
2. A coordinated PR wave against all SDK repos
3. A `schema_version` bump in `docs/schema/v1.0/`
4. Updated conformance tests for new behavior
5. A milestone release of all SDKs before the next binary release

Minor additions (e.g., a new optional field in Metadata) MAY be backward-compatible. Breaking changes (e.g., renaming a field) trigger a MAJOR version bump across all SDKs.
