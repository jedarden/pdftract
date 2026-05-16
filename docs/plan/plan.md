# pdftract Implementation Plan

**Version:** 1.0  
**Status:** Active  
**Repo:** jedarden/pdftract  
**Last updated:** 2026-05-16

---

## Primary Objectives

pdftract must be the **most accurate, fastest, and lightest-weight** PDF text extraction tool available. These are not aspirational — they are acceptance criteria. Every architectural and dependency decision is evaluated against all three in priority order.

### Accuracy targets (acceptance criteria — CI-gated)

| Metric | Target | Measurement |
|---|---|---|
| Character error rate, clean vector PDFs | < 0.5% | Against ground-truth corpus, `tests/fixtures/vector/` |
| Word error rate, clean OCR (300 DPI scans) | < 3% | Against ground-truth corpus, `tests/fixtures/scanned/` |
| Reading order correctness, multi-column | > 95% | Left column entirely before right column in all fixtures |
| Unicode recovery rate (no ToUnicode) | > 90% | Font fingerprint + AGL levels 2–4 on `tests/fixtures/encoding/` |
| Regression gate, real-world corpus | < 0.5% CER delta vs. golden | 500-PDF private corpus on every PR |
| Text readability score | > 0.85 | Proprietary composite of printable ratio, dict word ratio, ligature repair |

### Speed targets (acceptance criteria — CI-gated)

| Metric | Target | Measurement |
|---|---|---|
| 100-page vector PDF, 4-core CI | < 3 seconds | `cargo bench`, `tests/fixtures/perf/` |
| 10-page scanned PDF (OCR path), 4-core CI | < 30 seconds | includes Tesseract |
| Single-page extraction latency (serve mode) | < 150 ms p99 | wrk benchmark against `/extract` |
| Throughput vs. pdfminer.six (Python) | ≥ 10× faster | Benchmarked on identical hardware |
| Throughput vs. pypdf (Python) | ≥ 5× faster | Same benchmark suite |

### Weight targets (acceptance criteria)

| Metric | Target |
|---|---|
| Binary size, default features (no OCR, no serve) | < 4 MB stripped |
| Binary size, `--features ocr,serve` | < 12 MB stripped |
| Default dependency count (`cargo tree -d`) | < 30 unique crates (direct, verified against `cargo tree --depth 1 -e normal --features default`). Transitive dependency count is not gated — only direct crates are tracked. The < 30 direct crate limit is verified as a CI check on the first passing build. |
| Shared library dependencies (ldd) | Zero beyond libc + libm |
| Docker image, CLI only | < 20 MB (distroless base) |
| Docker image, with OCR (`tesseract-ocr` system pkg) | < 120 MB |

Decisions that violate any target require explicit justification and a waiver comment in the relevant section below.

---

## Overview

pdftract is a Rust PDF text extraction library with a CLI (`pdftract extract`), an HTTP server mode (`pdftract serve`), and a PyO3 Python binding. It extracts Unicode text from PDF files — including scanned pages via OCR — and produces structured JSON, NDJSON, or plain text output. The output schema is defined in `docs/research/extraction-output-schema.md` and is stable at schema version 1.0.

The implementation is organized into eight phases. Phase 0 establishes CI infrastructure (prerequisite). Phases 1–4 deliver a working vector-extraction CLI. Phase 5 adds OCR. Phase 6 adds the full API surface (PyO3, HTTP). Phase 7 adds advanced features that require the Phase 1–4 foundation.

### Key architectural decisions (baked in from the start)

- **File I/O:** `memmap2` for zero-copy random access; `madvise(MADV_SEQUENTIAL)` on content streams.
- **Object cache:** LRU with 4096-entry capacity (`lru` crate); object streams decompressed once and cached as `Arc<[u8]>`.
- **Parallelism:** `rayon` for page-level parallelism; per-page work is embarrassingly parallel after Phases 1–2 (parser and font pipeline) complete.
- **Serialization:** `serde` + `serde_json`; `BufWriter` wrapping `io::Stdout` for NDJSON streaming.
- **Error model:** All parse errors are recoverable and produce diagnostic entries in the `errors` array; no `panic!` in library code.
- **Crate layout:** `pdftract-core` (lib), `pdftract-cli` (binary), `pdftract-py` (PyO3, optional feature).

---

## Dependency Matrix

Feature flags control the binary footprint. The default build (`cargo build`) includes only the core extraction path. Heavy optional capabilities are behind named features.

**Feature flags:**
- `default` = `["cli", "decrypt"]` — strips to core + CLI + encryption; no OCR, no HTTP, no Python
- `decrypt` — RC4 and AES-128/256 decryption (RustCrypto crates; part of the default feature set because encryption handling is core, not optional)
- `ocr` — adds Tesseract + Leptonica (system libraries required)
- `serve` — adds axum + tokio (HTTP server)
- `python` — adds PyO3 (maturin build)
- `full-render` — adds pdfium-render (large native binary; improves scanned-page rasterization)
- `full` = `["ocr", "serve", "python"]`
- `wordlist-bloom` — replaces the default phf::Set English word list with a Bloom filter; enable if the binary-size CI check (`cargo bloat`) reports the word list exceeds 250 KB.

| Crate | Version | Feature | Purpose |
|---|---|---|---|
| `memmap2` | 0.9 | default | Memory-mapped file access |
| `flate2` | 1 | default | FlateDecode / zlib decompression |
| `lzw` | 0.10 | default | LZWDecode |
| `ttf-parser` | 0.21 | default | TrueType/OpenType glyph metrics and cmap lookup |
| `owned_ttf_parser` | 0.21 | default | Arc-safe wrapper for ttf-parser |
| `fontdue` | 0.9 | default | TrueType/OpenType glyph rasterization for shape-based Unicode recognition (Level 4). Estimated binary contribution ~60 KB. |
| `lru` | 0.12 | default | Object cache eviction |
| `rayon` | 1 | default | Page-level parallelism |
| `serde` | 1 | default | Serialization derive macros |
| `serde_json` | 1 | default | JSON output |
| `indexmap` | 2 | default | Ordered dictionaries (PDF dict key order matters for CMap parsing) |
| `unicode-normalization` | 0.1 | default | NFC normalization |
| `sha2` | 0.10 | default | SHA-256 hashing for font program fingerprinting (Level 3 Unicode recovery) |
| `encoding_rs` | 0.8 | default | CJK encoding decoding (Shift-JIS, GB18030, Big5, EUC-KR) |
| `phf` | 0.11 | default | Compile-time AGL hash map (zero runtime allocation) |
| `clap` | 4 | cli | CLI argument parsing |
| `thiserror` | 1 | default | Error type derivation |
| `log` + `env_logger` | 0.4 | default | Structured logging |
| `image` | 0.25 | ocr | Raster image decoding and DPI-scaled rendering (TIFF/CCITT support requires system libtiff; documented trade-off) |
| `tesseract` | 0.14 | ocr | Tesseract OCR FFI bindings |
| `leptonica-plumbing` | 0.4 | ocr | Leptonica image preprocessing (Sauvola, deskew) |
| `quick-xml` | 0.36 | ocr | HOCR and XFA XML parsing |
| `pdfium-render` | 0.8 | full-render | High-fidelity rasterization via PDFium (large native binary — ~20 MB) |
| `pyo3` | 0.21 | python | Python bindings |
| `maturin` | build | python | PyO3 wheel packaging |
| `axum` | 0.7 | serve | HTTP serve mode |
| `tokio` | 1 | serve | Async runtime for axum |
| `tower-http` | 0.5 | serve | Request size limiting and tracing |
| `multer` | 3 | serve | Multipart form parsing |
| `bytes` | 1 | serve | Zero-copy byte sharing in HTTP path |
| `aes` | 0.8 | decrypt | AES-128 and AES-256 decryption (RustCrypto, ~50 KB) |
| `rc4` | 0.1 | decrypt | RC4 decryption (RustCrypto, ~10 KB) |
| `bloomfilter` | 0.2 | wordlist-bloom (optional) | An alternative to the default phf::Set word list. Enable with `--features wordlist-bloom` to replace the phf word list with a Bloom filter if the binary-size CI check fails. Not a default dep — it is a manual authoring decision. ~25 KB for 20k words at 0.1% false-positive rate |
| `unicode-bidi` | 0.3 | default | Unicode bidi character category lookup for RTL line detection |
| `strsim` | 0.11 | default | String similarity metrics (Levenshtein) for header/footer cross-page deduplication |

**Build dependencies (Cargo.toml `[build-dependencies]`):**

| Crate | Version | Purpose |
|---|---|---|
| `phf_codegen` | 0.11 | Generates compile-time phf maps (AGL, word list, font fingerprints, glyph shapes) from `build.rs` |
| `serde_json` | 1 | Parses `build/font-fingerprints.json` and `build/glyph-shapes.json` in `build.rs` |

**Removed vs. first draft:** `jpeg-decoder` dropped — DCTDecode is passthrough; SOI/EOI marker validation is a 4-byte check with no external dependency. `whichlang` dropped — language detection is not on the critical accuracy path; BCP-47 lang tags come from PDF `/Lang` attributes and StructTree `/Lang`, not inference.

---

## Phase 0: CI Infrastructure (Prerequisite)

**Goal:** Establish the Argo Workflows CI pipeline required by all subsequent phases. Binary releases and Python wheel builds are automated from day one; no milestone can ship without this.
**Complexity:** Medium
**Estimate:** 3–5 days
**Delivers:** `pdftract-ci` and `pdftract-py-ci` WorkflowTemplates active in `iad-ci`; milestone tags trigger automated releases to GitHub Releases and PyPI.

Create Argo WorkflowTemplate `pdftract-ci` in `jedarden/declarative-config → k8s/iad-ci/argo-workflows/`. The template must:

1. Build the Rust binary for five targets using `cross` (Docker-based cross-compilation):
   - `x86_64-unknown-linux-musl`
   - `aarch64-unknown-linux-musl`
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`
   - `x86_64-pc-windows-gnu`
2. Run `cargo test --all-features` on `x86_64-unknown-linux-musl`.
3. Publish binaries to GitHub Releases on milestone tags via `gh release upload`.
4. Build the PyO3 wheel via the `pdftract-py-ci` template (separate template, uses a `ghcr.io/rust-cross/manylinux` base image for Linux wheels; `osxcross` toolchain for macOS targets; `cross` with `x86_64-pc-windows-gnu` for the Windows `.whl`). All five triples ship to PyPI on milestone tags.

The `pdftract-py-ci` WorkflowTemplate YAML is created in Phase 0 as a stub with placeholder steps (exit 0) to establish the CI infrastructure. Actual wheel-build logic is filled in during Phase 6.3 implementation.

**Phase 0 must be complete before Phase 1 code review begins.**

---

## Phase 1: Core PDF Parser (Foundation)

**Goal:** Parse any PDF object, resolve xref tables, decode streams. No text extraction yet.  
**Complexity:** Complex  
**Estimate:** 3–4 weeks  
**Delivers:** `pdftract-core::parser` module usable in unit tests.

### 1.1 Lexer

Tokenize the raw byte slice into PDF tokens. This is the lowest layer; all higher-level parsers call into it.

**Tokens to produce:**
- Boolean (`true`, `false`)
- Integer (`123`, `-7`)
- Real (`3.14`, `-.5`)
- String literals: literal strings `(...)` with all escape sequences (`\n`, `\r`, `\t`, `\\`, `\(`, `\)`, `\ddd` octal, line-continuation `\<newline>`), and hex strings `<...>` (odd-length padded with trailing zero nibble)
- Name objects: `/Name`, with `#XX` hex escape expansion, NUL-byte rejection, and length limit (127 bytes per spec)
- Array delimiters: `[`, `]`
- Dictionary delimiters: `<<`, `>>`
- Stream keyword: `stream` (validated against following `\n` or `\r\n`)
- End-stream keyword: `endstream`
- Indirect object markers: `obj`, `endobj`, `R`
- Comments: `%` to end of line (discarded)
- Whitespace: consumed between tokens (0x00, 0x09, 0x0A, 0x0C, 0x0D, 0x20)

**Crates:** none (hand-written; `nom` is an option but PDF's grammar is simple enough to avoid the dependency)

**Critical tests:**
- String with nested balanced parentheses: `(foo (bar) baz)` → `foo (bar) baz`
- String with octal escape at end of string: `(abc\101)` → `abcA`
- Hex string with odd length: `<4>` → `\x40`
- Name with `#20` → space character
- Name with `#00` → rejected (NUL in name is invalid per spec; emit diagnostic)
- Name object length limit: 127 bytes, applied to the raw byte count in the file before `#XX` hex escape expansion, matching PDF spec section 7.3.5; if exceeded, truncate the name at 127 bytes and emit `STRUCT_INVALID_NAME` diagnostic
- Whitespace-only file → empty token stream, no panic

### 1.2 Object Parser

Parse the token stream into the PDF object model.

**Types:**
- `PdfNull`
- `PdfBool(bool)`
- `PdfInt(i64)`
- `PdfReal(f64)`
- `PdfString(Vec<u8>)` — raw bytes before any encoding interpretation
- `PdfName(Arc<str>)`
- `PdfArray(Vec<PdfObject>)`
- `PdfDict(IndexMap<Arc<str>, PdfObject>)` — preserves insertion order
- `PdfRef(u32, u16)` — object number, generation number
- `PdfStream { dict: PdfDict, offset: u64 }` — offset into mmap; data decoded lazily
- `PdfIndirect { id: ObjRef, obj: Box<PdfObject> }`

**Key behaviors:**
- Indirect object parsing: `N G obj ... endobj` wrapper
- Object streams (`/ObjStm`): decompress once, parse all embedded objects, cache them under their object numbers
- Circular reference guard: track in-resolution set per thread; emit `STRUCT_CIRCULAR_REF` diagnostic and return `PdfNull` on cycle

**Crates:** `indexmap` (dict), std `Arc<[u8]>` (object stream caching — no external crate needed)

**Critical tests:**
- Nested dict: `<< /A << /B 1 >> >>` — correct inner dict
- Array of mixed types: `[1 true (str) /Name null]`
- Object stream: decompress, parse all N objects, verify all ObjRefs resolve
- Self-referencing object (circular): returns PdfNull with diagnostic, no stack overflow

### 1.3 Cross-Reference Resolution

Build the complete object → byte-offset map from the file's xref structure.

**Strategies (attempted in order on failure):**
1. **Traditional xref table:** parse from `startxref` offset; 20-byte fixed-width entries; handle `\r\n` and ` \n` line endings; merge multi-subsection tables
2. **Xref streams (PDF 1.5+):** parse `/W` field widths; decompress body with FlateDecode; parse `/Index` subsections; handle type-0/1/2 entries
3. **Hybrid files:** merge traditional table (priority) with xref stream (`/XRefStm` pointer); type-2 entries from stream fill gaps not covered by traditional table
4. **Forward scan fallback:** sequential scan for `N G obj` patterns; slower but handles severely truncated or overwritten files; emit `XREF_REPAIRED` diagnostic

**Incremental updates:** When `/Prev` is present in a trailer, recursively load the previous xref revision; later revisions override earlier entries for the same object number. This handles incremental saves, linearized files, and comment-editing workflows.

**Linearized PDF detection:** Check for a `/Linearized` dictionary in the first object of the file (object at byte offset 0 or nearby). If found: (1) parse the partial xref at the beginning of the file (the 'first-page xref'), (2) parse the complete xref at the end of the file (the 'full xref'), (3) merge them with the full xref taking precedence for any object number present in both. The hint stream (`/H` entry in the Linearized dict) is parsed for page offset hints to accelerate random-access page loading but is not required for correctness. The forward scan fallback is disabled for linearized files (it would find the partial leading xref and stop).

**Crates:** `flate2` (xref stream decompression)

**Critical tests:**
- PDF with `/Prev` chain of 3 revisions: latest value of each object number wins
- Type-2 xref entry: object resolved through `/ObjStm` correctly
- Hybrid file: traditional entries override stream entries for same object numbers
- File truncated after xref: forward scan finds all objects before truncation point
- `startxref` offset off by one (common real-world corruption): forward scan triggered, `XREF_REPAIRED` diagnostic emitted

### 1.4 Document Model

Build the in-memory document model over the xref-resolved object graph.

**Structures to build:**
- **Document catalog** from `/Root`: record `/Pages`, `/Outlines`, `/MarkInfo`, `/StructTreeRoot`, `/AcroForm`, `/Names`, `/Metadata`, `/PageLabels`, `/OCProperties`
- **Page tree** (`/Pages` subtree): flatten into a `Vec<PageDict>` with inherited attributes resolved (MediaBox, CropBox, BleedBox, TrimBox, ArtBox, Resources, Rotate). Inheritance walk: page dict overrides parent dict; root `/Pages` is the ultimate fallback. If a page's `/Contents` is an array of stream references, all streams are decoded and concatenated in order before Phase 3 content stream processing begins. Graphics state is NOT reset between concatenated streams — they are treated as a single logical stream.
- **Resource dictionary inheritance:** each page gets a fully resolved `ResourceDict` merging all ancestor `/Resources` dicts (font, XObject, ExtGState, ColorSpace, Shading, Pattern, Properties namespaces). Per-key last-write-wins at the page level.
- **Encryption dictionary** detection: if `/Encrypt` present in trailer, identify handler (`/Standard` vs. custom), extract `/V`, `/R`, `/KeyLength`, `/CF`/`/StmF`/`/StrF` entries. RC4 and AES-128/256 decryption implemented via the `aes` and `rc4` crates (RustCrypto; both gated behind the `decrypt` feature, which is on by default — see Dependency Matrix). Password attempt: empty string first, then user-supplied via `ExtractionOptions.password: Option<String>` (CLI: `--password <PASSWORD>`; Python keyword arg: `password=None`; HTTP form field: `password`). On failure: emit `ENCRYPTION_UNSUPPORTED` and abort.

**Optional Content Groups (OCGs):** If `/OCProperties` is present in the catalog, read default visibility from `/OCProperties /D /BaseState` (name value `ON` or `OFF`; defaults to `ON` if absent). Each individual OCG's membership in the default ON or OFF list is given by the arrays `/OCProperties /D /ON` (array of OCG object refs that are ON by default) and `/D /OFF` (OFF by default). An OCG present in neither array inherits `BaseState`. During content stream processing (Phase 3), track the `OC` marked content tag: if a `BDC` block carries `/OC /OCGRef`, check the referenced OCG's default state. If `OFF`, suppress all glyphs within the marked content block (they are not extracted). If `ON` or no OCG present, extract normally. Emit `ocg_present: true` in document metadata. Full OCG toggle support (programmatic state changes) is deferred to Phase 7.

**JavaScript detection:** Record `contains_javascript = true` if any of the following are present: (1) `/OpenAction` value is a JavaScript action dict (`/S /JavaScript`), (2) `/AA` (Additional Actions) at document or page level contains a JavaScript action, (3) any AcroForm field's `/AA` dict contains a JavaScript action, (4) any annotation's `/A` or `/AA` dict contains a JavaScript action. JavaScript is never executed — only its presence is flagged. This check runs during document model construction and costs one dict key scan per object.

**`conformance` detection:** Parse the `/Metadata` stream (if present) as XMP XML using `quick-xml`. Extract the `pdfaid:part` and `pdfaid:conformance` elements to construct values like `PDF/A-1b`, `PDF/A-2u`. If no XMP metadata or no `pdfaid:` namespace tags are present, `conformance = null`. **`quick-xml` feature gate:** Move `quick-xml` from the `ocr` feature to `default` since conformance detection runs for all documents. **`contains_xfa` detection:** Check for the presence of `/AcroForm /XFA` key during document model construction; if present and non-null, `contains_xfa = true`.

**Crates:** `aes`, `rc4` (both via `decrypt` feature), `quick-xml` (moved to `default` feature for conformance detection)

**Critical tests:**
- Page inheriting MediaBox from grandparent `/Pages` node
- Page overriding `/Resources /Font` partially (merged, not replaced)
- `PageLabels` number tree: pages with roman-numeral labels followed by arabic labels
- Encrypted file with empty owner password: decrypts successfully
- Encrypted file with unknown handler: `ENCRYPTION_UNSUPPORTED` error, no crash

### 1.5 Stream Decoder

Decode stream data through its filter pipeline. Called lazily when stream content is first accessed.

**Filters to implement (in priority order):**

| Filter | Implementation | Notes |
|---|---|---|
| `FlateDecode` | `flate2::read::ZlibDecoder` | Apply predictor post-inflate: TIFF predictor 2, PNG predictors 10–15 (per-row byte selects predictor for value 15) |
| `LZWDecode` | `lzw` crate | `/EarlyChange` parameter: 1 = early (default), 0 = late; same predictor support as FlateDecode |
| `ASCII85Decode` | hand-written | `z` shortcut, partial final group, `~>` terminator, embedded whitespace ignored |
| `ASCIIHexDecode` | hand-written | Digit pairs, whitespace ignored, `>` terminator |
| `RunLengthDecode` | hand-written | Length byte: 0–127 = copy next N+1 bytes literally; 129–255 = repeat next byte 257-N times; 128 = EOD |
| `DCTDecode` | passthrough | Pass raw JPEG bytes to consumer; validate SOI/EOI markers; log `/ColorTransform` for consumer |
| `JBIG2Decode` | passthrough | Pass raw JBIG2 bytes; log global stream reference. For OCR path: requires `full-render` feature (pdfium-render decodes JBIG2 internally). Without `full-render`, emit `OCR_JBIG2_UNSUPPORTED` diagnostic and skip those image regions; JBIG2 is rare in modern PDFs. |
| `JPXDecode` | passthrough | Pass raw JPEG 2000 bytes. For OCR path: requires `full-render` feature (pdfium-render decodes JPEG 2000 internally) or system `libopenjp2`. Without either, emit `OCR_JPX_UNSUPPORTED` diagnostic and skip the page. |
| `CCITTFaxDecode` | passthrough | Pass raw CCITT bytes. For OCR path: `image` with `tiff` feature decodes Group 3/4 CCITT; this requires `libtiff` system library. Alternatively, require `full-render` feature. Emit `OCR_CCITT_UNSUPPORTED` if neither is available. |
| `Crypt` | identity only | `/Name /Identity` handled; custom crypt filters emit `ENCRYPTION_UNSUPPORTED` |

**Filter pipeline:** `/Filter` is a name or array; `/DecodeParms` is aligned or absent. Apply decoders in order. Mismatched lengths: apply defaults, log diagnostic.

**Error recovery:** zlib decompression error mid-stream: return bytes decoded so far, emit `STREAM_DECODE_ERROR` diagnostic. Never abort the page.

**Crates:** `flate2`, `lzw`, `image` (JPX/CCITT raster decode for OCR path) — DCTDecode SOI/EOI marker validation is a 4-byte inline check; no external crate needed

**Critical tests:**
- FlateDecode with PNG predictor 15 (per-row): all six predictor types appear in one stream, all decoded correctly
- LZWDecode with EarlyChange=0: verify against known reference output
- ASCII85 with `z` shortcut and odd final group
- Filter array `[/ASCII85Decode /FlateDecode]`: decoded in order
- FlateDecode with truncated zlib stream: partial output returned, diagnostic emitted
- DCTDecode: raw bytes passed through unchanged; SOI marker present

### 1.6 Error Recovery

Cross-cutting concerns for malformed files.

**Strategies:**
- **Truncated file at EOF:** forward xref scan; any `endobj` before truncation point is valid
- **Corrupt xref entry (bad offset):** attempt to parse at listed offset; if first bytes are not `N G obj`, skip entry with diagnostic; do not remove from xref map (other objects may be valid)
- **Missing required dict key:** return `PdfNull`, emit `STRUCT_MISSING_KEY` diagnostic with object number; caller must handle null gracefully
- **Integer overflow in object dimensions:** clamp to `i32::MAX` and log; do not panic
- **Circular object reference:** detected via per-thread resolution stack; return `PdfNull` with diagnostic

**Critical tests:**
- File where 30% of xref entries point to wrong offsets: remaining 70% extracted correctly
- Missing `/MediaBox` on every page: default letter size (612×792) used, diagnostic emitted per page
- Object with `endobj` missing: parser reads to next `N G obj` pattern and continues

---

## Phase 2: Font and Encoding Pipeline

**Goal:** For any character code from a content stream, resolve a Unicode scalar value and a confidence score.  
**Complexity:** Complex  
**Estimate:** 3–4 weeks  
**Depends on:** Phase 1 complete  
**Delivers:** `pdftract-core::font` module

### 2.1 Font Type Detection

Load and classify the font from the resource dictionary.

**Font types and loading strategy:**

| Subtype | Font Program Location | Metric Source |
|---|---|---|
| `Type1` | `/FontFile` in FontDescriptor | `/Widths` array |
| `Type1` (Standard 14) | No font program; synthesized | Known metrics table (hardcoded) |
| `TrueType` | `/FontFile2` | `/Widths` array; `hmtx` for verification |
| `Type0` (composite) | Descendant CIDFont | `/DW`, `/W` array in CIDFont dict |
| `CIDFontType0` | `/FontFile3` (`/CIDFontType0C`) | `/DW`, `/W` |
| `CIDFontType2` | `/FontFile2` or `/FontFile3` (`/OpenType`) | `/DW`, `/W` — `/CIDToGIDMap` may be the name `/Identity` (GID==CID) or a stream (decoded as 2-byte big-endian GID array) |
| `Type3` | `/CharProcs` content streams | `/Widths` |
| OpenType (CFF) | `/FontFile3` (`/OpenType`) | `hhea`/`hmtx` via `ttf-parser` |

**Font subset detection:** Many embedded fonts are subsets with name prefix like `ABCDEF+Helvetica`. Strip the six-uppercase-letter prefix before looking up Standard 14 or glyph name tables.

**Crates:** `ttf-parser`, `owned_ttf_parser`

**Critical tests:**
- Standard 14 font (no embedding): correct metrics returned without font file
- Subset font `ABCDEF+Times-Roman`: stripped to `Times-Roman`, standard metrics used
- CIDFontType2 with `/CIDToGIDMap /Identity`: GID == CID for all lookups
- CIDFontType2 with `/CIDToGIDMap` as a stream: decode the stream (FlateDecode), interpret as a flat array of 2-byte big-endian GID values indexed by CID (`CIDToGIDMap[CID*2 .. CID*2+2]` → GID); array length is 2 × (max CID + 1)
- OpenType CFF font: metrics via `ttf-parser`'s CFF support

### 2.2 Encoding Resolution

Map character codes → Unicode. Four-level fallback chain with `unicode_source` tag on each result.

**Level 1: ToUnicode CMap**

Parse the `/ToUnicode` stream as a CMap program. CMap syntax to implement:
- `beginbfchar` / `endbfchar`: `<srcCode> <dstHex>` pairs; `<dstHex>` may be a UTF-16BE multi-codepoint sequence for ligature expansion
- `beginbfrange` / `endbfrange`: `<lo> <hi> <dst>` (contiguous single-codepoint range) or `<lo> <hi> [<d0> <d1> ...]` (explicit array for non-contiguous targets)
- `usecmap` directive: inherit from named CMap (e.g., `Adobe-Japan1-UCS2`)
- Comment lines (`%`) stripped

Successful lookup: set `unicode_source = "to_unicode"`, `confidence = 1.0`.  
Result is U+FFFD or empty: fall through to Level 2.

**Level 2: Encoding vector + AGL**

Map character code → glyph name via the font's `/Encoding`:
- Named encodings: `WinAnsiEncoding`, `MacRomanEncoding`, `MacExpertEncoding`, `StandardEncoding`, `SymbolEncoding`, `ZapfDingbatsEncoding` — hardcoded tables
- `/Differences` array: sparse overlay on top of base encoding; format `[n /GlyphName1 /GlyphName2 ...]` (n is starting code)

Map glyph name → Unicode via Adobe Glyph List (AGL 1.4, ~4400 entries, compiled in as a static phf::Map). Also support AGLFN (friendly names).

Set `unicode_source = "agl"`, `confidence = 0.9`.

**Level 3: Font fingerprint cache**

Hash the embedded font program (SHA-256 of the raw font program stream bytes, computed via the `sha2` crate). Look up in a bundled database of known font checksums → per-glyph Unicode mapping tables. Initially populated with the most common 200 commercial fonts.

**Database spec:** The database is a compile-time `phf::Map<[u8; 32], &'static [(u16, char)]>` where the key is the 32-byte SHA-256 digest of the raw font program stream (the bytes of the `/FontFile`, `/FontFile2`, or `/FontFile3` stream after filter decoding, before any interpretation) and the value is a slice of `(glyph_id, unicode_char)` pairs covering every mapped glyph in that font. The map is generated at build time from a JSON source file (`build/font-fingerprints.json`) by a `build.rs` script that emits the `phf_codegen` output. **Estimated binary footprint:** ~500 KB added to the stripped binary, within the 4 MB default-feature budget (documented here as an approved allocation). **Source:** Initially curated from open-source font metric data — Adobe's publicly available font databases and Google Fonts `cmap` metric exports. The JSON source file is the authoritative artifact; PRs that add new fonts add entries to `build/font-fingerprints.json`. The database is not user-extensible at runtime.

If the font has no embedded program (Standard-14 fonts or fonts with no `/FontFile`, `/FontFile2`, or `/FontFile3`), skip Level 3 and proceed directly to Level 4. Standard-14 fonts are guaranteed to have AGL-compatible glyph names, so Level 3 is normally unreachable for them; this is a defensive guard.

Set `unicode_source = "fingerprint"`, `confidence = 0.85`.

**Level 4: Glyph shape recognition**

Render the glyph to a 32×32 grayscale bitmap rendered via `fontdue`'s rasterizer (for TrueType/OpenType glyphs) or the Type 3 content stream renderer (for Type 3 glyphs). Hash the bitmap with a perceptual hash. Look up in a bundled shape→Unicode database (see `docs/research/glyph-recognition-and-unicode-recovery.md` and Phase 2.5).

Set `unicode_source = "shape_match"`, `confidence = 0.7`.

**Failure:** Emit U+FFFD, `unicode_source = "unknown"`, `confidence = 0.0`, log `GLYPH_UNMAPPED` diagnostic.

**Crates:** `fontdue` (glyph rasterization for shape hash), `phf` (compile-time AGL hash map)

**Critical tests:**
- `ToUnicode` with multi-codepoint bfchar (`fi` ligature → `fi`): expanded to two characters
- `beginbfrange` with explicit array: non-contiguous targets resolved correctly
- `WinAnsiEncoding` code 0x92: maps to U+2019 RIGHT SINGLE QUOTATION MARK (not U+0092)
- MacRoman code 0xD2 / 0xD3: left/right double quotation marks
- Unknown glyph name not in AGL: falls through to Level 3 or 4
- Type1 font with no `/Encoding` and no `/ToUnicode`: Level 3/4 fallback triggered

### 2.3 CJK Encoding

Handle multi-byte CJK character sets for Type 0 composite fonts.

**Predefined CMaps to implement (or reference via bundled data):**
- `Identity-H` / `Identity-V`: CID == character code (passthrough)
- `UniJIS-UTF16-H`, `UniJIS-UTF16-V`: Japanese JIS → Unicode
- `UniGB-UTF16-H`, `UniGB-UTF16-V`: GB2312 → Unicode
- `UniCNS-UTF16-H`, `UniCNS-UTF16-V`: Big5/CNS → Unicode
- `UniKS-UTF16-H`, `UniKS-UTF16-V`: KS → Unicode

**Encoding decoding for raw byte sequences:**
- Shift-JIS: `encoding_rs::SHIFT_JIS`
- GB18030: `encoding_rs::GB18030`
- Big5: `encoding_rs::BIG5`
- EUC-KR: `encoding_rs::EUC_KR`

**Multi-byte code parsing:** Type 0 font's `/Encoding` CMap defines the codespace ranges (`begincodespacerange`/`endcodespacerange`). Parse the CMap to determine 1- vs. 2-byte code boundaries, then tokenize the content stream byte sequence accordingly.

**Crates:** `encoding_rs`

**Critical tests:**
- Identity-H Type 0 font with ToUnicode: CID passthrough, Unicode from ToUnicode
- Embedded Shift-JIS ToUnicode CMap: all 6879 JIS X 0208 characters resolve correctly
- Two-byte code boundary in codespace: first byte in 0x81–0xFE range triggers two-byte read; 0x00–0x7F is single-byte
- Mixed single/double-byte codes in same TJ string: all boundaries parsed correctly

### 2.4 Type 3 Font Handling

Type 3 fonts define each glyph as a content stream in `/CharProcs`. No standard Unicode mapping exists unless `/ToUnicode` is provided.

**Pipeline:**
1. Check `/ToUnicode` first (same Level 1 logic as above)
2. If absent, attempt `/Encoding` glyph name lookup (Level 2)
3. If glyph name is non-standard (arbitrary user name), rasterize the content stream to a 32×32 bitmap and apply shape recognition (Level 4)
4. Track the content stream rendering state: Type 3 glyphs can invoke other PDF operators including form XObjects; apply the same graphics state machine as Phase 3

**Metrics:** Use `/Widths`, `/FirstChar`, `/LastChar`, `/FontMatrix` to compute advance widths. `/FontMatrix` default is `[1 0 0 1 0 0]` for Type 3 (glyph units == text units); apply it to convert glyph-space advance to text space.

**Critical tests:**
- Type 3 font with meaningful `/ToUnicode`: resolved correctly
- Type 3 font with arbitrary glyph names and no ToUnicode: shape recognition fallback, `confidence = 0.7`
- Type 3 glyph stream that invokes a form XObject: recursive processing without stack overflow
- `/FontMatrix [0.001 0 0 0.001 0 0]`: advances scaled to 1/1000 of text units (matches Type 1)

### 2.5 Glyph Shape Database

The glyph shape database backs Level 4 shape recognition in Phase 2.2 and the Type 3 shape fallback in Phase 2.4. Full methodology is documented in `docs/research/glyph-recognition-and-unicode-recovery.md`.

**Perceptual hash algorithm:** Each glyph outline is rasterized to a 32×32 grayscale bitmap using `fontdue`'s rasterizer (for TrueType/OpenType glyphs) or the Type 3 content stream renderer (for Type 3 glyphs). The bitmap is then hashed using pHash (perceptual hash): apply a 32×32 DCT, retain the top-left 8×8 AC coefficients (64 values), threshold against the median of those 64 values to produce a 64-bit integer. This yields a scale-invariant hash robust to minor rendering differences.

**Database format:** A compile-time `&'static [(u64, char)]` — a sorted slice of `(pHash, char)` pairs sorted by pHash ascending. Generated at build time from a JSON source file (`build/glyph-shapes.json`) via `build.rs` (emitted as a `static` array, no `phf_codegen` needed for this structure). An exact `phf::Map<u64, char>` cannot be used here because the collision-handling requirement needs a nearest-neighbor scan over Hamming distance, not exact key lookup.

**Query algorithm:** Linear scan over all ~5,000 entries computing `(query_hash XOR entry_hash).count_ones()` for each entry. Collect all entries with Hamming distance ≤ 8; select the entry with the smallest distance. Ties broken by the Unicode frequency rank stored in the source JSON's `frequency` field (precomputed into a companion `&'static [(u64, u32)]` frequency table sorted by pHash, queried in the same pass). **Performance:** 5,000 entries × ~8 ns per XOR+popcount ≈ 40 µs worst-case scan — well within the per-page time budget. The winning character is returned with `confidence = 0.7`; if no entry falls within the 8-bit Hamming threshold, fall through to failure (U+FFFD).

**Estimated binary footprint:** ~300 KB for approximately 5,000 common glyphs (covering Latin, Greek, Cyrillic, common symbols, and extended Latin). Within the 4 MB default-feature budget.

**Source:** Glyph bitmaps are rendered from open-source fonts (Google Fonts corpus, SIL Open Font License fonts) and hashed offline. The JSON source file is the authoritative artifact; new glyphs are added by re-running the offline hash pipeline and updating `build/glyph-shapes.json`.

---

## Phase 3: Content Stream Processing

**Goal:** Execute PDF content stream operators to produce a raw glyph list with positions.  
**Complexity:** Complex  
**Estimate:** 3–4 weeks  
**Depends on:** Phase 2 complete  
**Delivers:** `pdftract-core::content` module; raw `Vec<Glyph>` per page

### 3.1 Graphics State Machine

Maintain the full graphics state stack as the content stream is executed.

**State struct fields:**
```
ctm: Matrix3x3           -- current transformation matrix
text_matrix: Matrix3x3   -- Tm (set by Tm/Td/TD/T*)
text_line_matrix: Matrix3x3  -- Tlm (reset by Td/TD/T*)
font: Option<Arc<Font>>
font_size: f64
char_spacing: f64        -- Tc
word_spacing: f64        -- Tw
horiz_scaling: f64       -- Tz (percentage, default 100)
leading: f64             -- TL
text_rise: f64           -- Ts
text_rendering_mode: u8  -- Tr (0–7)
fill_color: Color
stroke_color: Color
```

**`Color` type definition:** The `fill_color` and `stroke_color` fields above use the following enum, which covers all PDF color spaces relevant to text extraction:

```rust
enum Color {
    DeviceGray(f32),           // 0.0–1.0
    DeviceRGB([f32; 3]),       // 0.0–1.0 each
    DeviceCMYK([f32; 4]),      // 0.0–1.0 each
    Spot(Arc<str>, f32),       // (colorant name, tint 0.0–1.0)
    Other,                     // CalRGB, ICCBased, Pattern — treated as transparent
}
```

CSS hex conversion rule for the `color` field in the Span output: `DeviceRGB → #rrggbb`; `DeviceGray(v) → DeviceRGB([v,v,v]) → #rrggbb`; `DeviceCMYK([c,m,y,k]) → approximate RGB via standard formula → #rrggbb`; `Spot` and `Other → null` in the JSON output (not serialized as a color string).

**Stack operators:** `q` pushes a clone of the current state; `Q` pops. Stack depth limit: 64 (per spec); deeper push emits `GSTATE_STACK_OVERFLOW` diagnostic and discards the push (safe failure).

**Text state operators:**

| Operator | Effect |
|---|---|
| `BT` | Reset `text_matrix = identity`, `text_line_matrix = identity` |
| `ET` | End text object; discard current text matrix |
| `Tc n` | `char_spacing = n` |
| `Tw n` | `word_spacing = n` |
| `Tz n` | `horiz_scaling = n` |
| `TL n` | `leading = n` |
| `Tf name size` | Load font by resource name, set `font_size` |
| `Tr n` | `text_rendering_mode = n` |
| `Ts n` | `text_rise = n` |
| `Td tx ty` | `text_line_matrix = translate(tx, ty) * text_line_matrix`; copy to `text_matrix` |
| `TD tx ty` | Same as `Td`; also `leading = -ty` |
| `Tm a b c d e f` | Set both matrices directly |
| `T*` | Equivalent to `Td 0 -leading` |

**CTM operators:** `cm a b c d e f` — multiply CTM by the given matrix.

**Page rotation:** After all glyph bboxes for a page are computed, if the page's `/Rotate` entry is 90, 180, or 270, apply the corresponding inverse rotation matrix to all glyph bboxes so that downstream phases (baseline clustering, column detection, reading order) always operate in an un-rotated coordinate system. The page `width` and `height` in the output schema reflect the rotated page dimensions (as the viewer sees them).

**Crates:** none (hand-written matrix arithmetic; 3x3 f64 matrices, no external linear algebra dependency needed)

**Critical tests:**
- `q`/`Q` nesting 64 levels deep: succeeds; level 65 emits diagnostic
- `Td` chain: verify accumulated text_line_matrix matches manual calculation
- `Tm` followed by `Td`: Td is relative to previous text_line_matrix, not Tm
- `Tr 3` (invisible): glyph produced with `rendering_mode = 3`
- Color operators `rg`, `RG`, `k`, `K`, `cs`, `scn`: fill/stroke color tracked correctly

### 3.2 Text Operator Processing

Parse text-showing operators and produce `Glyph` structs.

**Text-showing operators:**

| Operator | Argument | Behavior |
|---|---|---|
| `Tj` | `(string)` | Show string; advance text position |
| `TJ` | `[...]` array | Alternate strings and numeric kerning adjustments |
| `'` | `(string)` | `T*` then `Tj` |
| `"` | `aw ac (string)` | Set word_spacing=aw, char_spacing=ac, then `'` |

**Per-glyph processing:**
1. Decode character code(s) from the string bytes using the current font's codespace
2. Resolve Unicode via Phase 2 font pipeline
3. Compute glyph advance width from font metrics (accounting for Tc, Tw if space glyph, Tz)
4. Compute device-space bounding box: apply text_matrix * CTM to the glyph bbox
5. Detect word boundary: if actual next-glyph x-position > expected by more than threshold → inject synthetic space
6. Advance text_matrix by advance width

**Word boundary threshold (adaptive):** Initial threshold = 0.25 * font_size. After processing 20 glyphs, compute the median actual inter-glyph gap and adjust the threshold to 1.5× that median. This adapts to per-document spacing norms. See `docs/research/word-boundary-reconstruction.md` for full formula including Tc, Tw, Tz corrections.

Three implementation requirements:
- **(a) Comparison space:** The threshold comparison is performed in **text space** (before applying the CTM). Use the glyph's advance width and gap as computed from the text matrix only; do not transform to device space before comparing.
- **(b) Recalibration window scope:** The 20-glyph recalibration window is **reset on every font switch** (`Tf` operator). Each new font starts fresh with zero samples and the fixed initial threshold.
- **(c) Bootstrap behavior:** For the **first 20 glyphs** after a font switch (or at stream start), use the fixed initial threshold of `0.25 × font_size` with no recalibration. Recalibration begins only after the 21st glyph in the current font has been processed.

**TJ kerning:** Numeric elements in a TJ array adjust the text position by `-n/1000 * font_size * Tz/100` (negative n = kern closer, positive = move apart). Large positive values (> 0.2 * font_size) produce word boundaries.

**Glyph struct:**
```rust
struct Glyph {
    codepoint: char,         // resolved Unicode or U+FFFD
    unicode_source: UnicodeSource,
    confidence: f32,
    bbox: [f32; 4],          // [x0, y0, x1, y1] in PDF user space (lower-left origin)
    font_name: Arc<str>,
    font_size: f32,
    rendering_mode: u8,
    fill_color: Color,
    is_word_boundary: bool,  // synthetic space injected before this glyph
    mcid: Option<u32>,       // MCID of innermost enclosing marked content sequence; populated during Phase 3.4 marked content tracking
}
```

**Critical tests:**
- TeX-generated PDF with no space characters: word boundaries injected at correct positions
- TJ array with large positive kerning value (word gap): space injected
- Negative TJ kern (kern tighter): no space injected
- Glyph at Tr=3: present in output with rendering_mode=3
- Font size 0 (degenerate): glyph bbox degenerates to point; no panic

### 3.3 Resource Context and Form XObject Recursion

Handle nested resource scopes introduced by form XObjects (Do operator).

**ResourceStack:** Each page starts with its resolved resource dictionary (from Phase 1.4). When a form XObject is invoked via `Do`, push a new resource scope merging the form's own `/Resources` with the current scope (form resources shadow parent resources). Pop on return.

**Form XObject execution:** Retrieve the form XObject stream, decode it, and execute it as a nested content stream. The form's `/Matrix` entry is applied to the CTM before execution; the form's `/BBox` is applied as a clipping boundary. After execution, restore the pre-form CTM.

**Cycle detection:** Track the set of form XObject object numbers currently in the execution stack. If the same object number appears twice, emit `STRUCT_XOBJECT_CYCLE` diagnostic and return without executing. Stack depth limit: 20 levels.

**Critical tests:**
- Form XObject with its own `/Resources /Font`: inner font resolved from form resources, not page resources
- Form XObject with `/Matrix [2 0 0 2 0 0]`: all glyph bboxes in form space scaled by 2
- Form XObject cycle (A invokes B invokes A): cycle detected at second A; diagnostic emitted; extraction continues
- Form XObject with empty content stream: no crash, no glyphs produced

### 3.4 Marked Content Tracking

Track BDC/BMC/EMC marked content sequences for MCID association (used in Phase 7 StructTree exploitation).

**Operators:**
- `BMC /Tag` and `BDC /Tag << props >>` or `BDC /Tag /PropName`: push tag frame with tag name and optional MCID from properties dict (`/MCID` key)
- `EMC`: pop tag frame

**Output:** Each `Glyph` carries an optional `mcid: Option<u32>` — the MCID of the innermost marked content sequence enclosing it, if any.

**Critical tests:**
- Nested BDC: innermost MCID wins for enclosed glyphs
- EMC without matching BMC (malformed): ignored; no stack underflow panic
- MCID 0: valid (zero is a legal MCID)

### 3.5 Inline Images

Detect and skip inline image data (BI/ID/EI operator sequence) without confusing the parser.

**Parsing:** `BI` signals start of inline image dict; consume key-value pairs until `ID`; then scan raw bytes for the `EI` terminator (two-byte sequence `\nEI` where the preceding byte is not a continuation of image data — the spec requires the EI to be preceded by whitespace). Extract image bytes for passthrough.

**Critical tests:**
- Inline image immediately followed by text operators: text operators parsed correctly after EI
- Inline image data containing the byte sequence `EI` in the middle: not treated as terminator (must be preceded by whitespace)

---

## Phase 4: Text Assembly and Layout

**Goal:** Transform raw `Vec<Glyph>` → structured blocks in reading order.  
**Complexity:** Complex  
**Estimate:** 3–4 weeks  
**Depends on:** Phase 3 complete  
**Delivers:** Per-page `Vec<Block>` with `Vec<Span>` in reading order; plain text output mode works

### 4.1 Glyph → Span Merging

Group consecutive glyphs into spans. A new span begins when any of the following change:
- `font_name`
- `font_size` (delta > 0.5pt)
- `rendering_mode`
- `fill_color` (normalized to RGB; spot colors treated as distinct)
- `is_word_boundary` (inject a synthetic space span or embed space in current span text)

**Span struct:**
```rust
struct Span {
    text: String,
    bbox: [f32; 4],          // union of member glyph bboxes
    font: Arc<str>,
    size: f32,
    color: Option<CssHexColor>,
    rendering_mode: u8,
    confidence: f32,         // minimum glyph confidence
    confidence_source: ConfidenceSource,
    lang: Option<Arc<str>>,  // filled in Phase 7 normalization
    flags: u8,               // SpanFlags bitmask: bit 0=bold, 1=italic, 2=smallcaps, 3=subscript, 4=superscript
}
```

**`ConfidenceSource` enum → output schema string mapping:**
```
ConfidenceSource enum → schema string:
  unicode_source "to_unicode" | "agl"          → confidence_source = "native"
  unicode_source "fingerprint"                  → confidence_source = "native"
  unicode_source "shape_match"                  → confidence_source = "heuristic"
  unicode_source "unknown" (U+FFFD)             → confidence_source = "heuristic"
  OCR path (Phase 5.4 HOCR)                    → confidence_source = "ocr"
  Phase 4.7 correction applied                  → confidence_source = "heuristic"
```

**Flag detection:**
- Bold: font name contains "Bold" or FontDescriptor `/Flags` bit 18 set or `/StemV` > 120
- Italic: font name contains "Italic"/"Oblique" or `/ItalicAngle` != 0
- Smallcaps: font name contains "SC"/"SmallCaps" or `/Flags` bit 3 set
- Subscript: `text_rise` < -0.1 * font_size
- Superscript: `text_rise` > 0.1 * font_size

**Critical tests:**
- Mixed bold/regular in one text object: span break at font change
- Word boundary between two same-font glyphs: either space appended to previous span or new space span created (implementation choice; must round-trip to correct plain text)
- Subscript with `Ts -3`: SuperScript flag NOT set, Subscript flag set

### 4.2 Line Formation

Group spans into lines by baseline proximity.

**Algorithm:**
1. Compute baseline y-coordinate for each span: `y0 + (bbox_height * 0.2)` (approximation; exact value requires font descender metrics)
2. Cluster spans with baseline within `0.5 * median_font_size` of each other → same line
3. Within a line, sort spans by x0 (left-to-right for LTR scripts)
4. **RTL detection:** If the majority of characters in a line have Unicode bidi category R or AL (right-to-left), sort spans by x1 descending and set `direction = "rtl"` on the resulting line struct

**Crates:** `unicode-bidi` (bidi character category lookup for RTL detection); clustering is otherwise a simple sort + gap scan

**Critical tests:**
- Two-column layout: columns not merged into one line (column gap exceeds threshold)
- Superscript span at higher y than baseline text: not treated as a separate line
- Arabic text: bidi R characters detected, spans sorted right-to-left

### 4.3 Column Detection

Identify column boundaries in multi-column layouts.

**Algorithm:** Collect the x0 and x1 coordinates of all spans on the page. Compute a histogram of x0 values at 1pt resolution. Gaps wider than `0.03 * page_width` with zero span coverage are column boundary candidates. Require at least 3 lines to start in each candidate column before promoting it to a confirmed column.

Apply column labels to each span. This gates the XY-cut reading order algorithm in Phase 4.5.

**Critical tests:**
- Three-column academic paper: three distinct columns detected
- Full-width heading above two-column body: heading spans all columns; body spans within columns
- Single-column page: no false column splits

### 4.4 Block Formation

Group lines into blocks (paragraphs, headings, etc.).

**Heuristics (applied in order):**
1. **Vertical gap:** gap between consecutive lines > `1.5 * line_height` → new block
2. **Indent change:** first line x0 differs from subsequent lines by > `0.03 * column_width` → paragraph indent signal; may indicate block boundary above
3. **Font size change:** median font size of next line differs from current block by > 1pt → new block
4. **Rendering mode change:** invisible (Tr=3) text separated from visible text
5. **Column boundary:** span in different column from previous span → mandatory block break

**Block kind assignment (heuristic):**
- `heading`: font size > 1.2× body median AND line count == 1 (or short)
- `header`/`footer`: block y0 in top/bottom 7% of page height AND appears on 3+ consecutive pages with identical or near-identical text. **Sequencing note:** Header/footer detection is a sequential post-processing pass executed after all pages are assembled by rayon. The pass iterates over the sorted page list, maintaining a sliding window of the last 4 pages. Blocks in the top/bottom 7% of the page that appear in ≥ 3 consecutive pages with Levenshtein distance ≤ 5% of the text length are classified `header` or `footer`. This pass runs in O(pages × blocks_per_page) and is negligible compared to per-page extraction time. Crate: `strsim` (`strsim::levenshtein` applied at the Unicode char level, not byte level).
- `paragraph`: default
- `figure`: bbox contains only image XObjects, no text glyphs
- `list`: line starts with bullet/numbered pattern (regex: `^\s*[•‣◦\-\*]\s` or `^\s*\d+[\.\)]\s`)
- `caption`: small font, follows a `figure` block within 2 lines
- `code`: all spans in the block use a monospace font (font name contains 'Mono', 'Courier', 'Code', 'Fixed', or `FontDescriptor /Flags` bit 0 set) AND the block is indented ≥ 2em relative to the surrounding body text baseline. Deferred to Phase 7 for full detection; Phase 4 emits `paragraph` for code blocks and upgrades to `code` in a post-processing pass if the monospace heuristic fires.
- `formula`: detected in Phase 7 via OpenType Math table presence (see `docs/research/opentype-math-and-formula-extraction.md`). Phase 4 emits `paragraph` for formula blocks.

**Critical tests:**
- Indented first line of paragraph: not split into two blocks
- Header text appearing on pages 1–10: classified `header` and deduplicated
- Bullet list with mixed font sizes: all items in same `list` block

### 4.5 Reading Order

Determine the reading order of blocks within the page.

**Fast path (tagged PDF):** If `is_tagged = true`, defer to Phase 7 StructTree traversal. Set `reading_order_algorithm = "struct_tree"`. Until Phase 7 is implemented (v0.1.0–v0.3.0), `is_tagged = true` pages fall through to XY-cut; `reading_order_algorithm` is set to `'xy_cut'` and a `TAGGED_PDF_STRUCT_TREE_DEFERRED` informational diagnostic is emitted. Phase 7.1 replaces this path.

**XY-cut algorithm (untagged, rectilinear layouts):**
1. Find the widest vertical whitespace gap dividing the page's text bbox into left and right halves → split into two regions
2. For each region, find the widest horizontal gap → split into top and bottom sub-regions
3. Recurse until regions contain a single column of text
4. Reading order: left region before right; top before bottom within each region

**Docstrum fallback (when XY-cut produces > 10 regions with < 3 blocks each):** Compute nearest-neighbor pairs between text blocks. Build a graph of adjacency edges weighted by distance and angle. Traverse the connected components in estimated reading order (sort root nodes by page position, follow edges within each component).

**Parameters:** k=5 nearest neighbors per block (standard Docstrum value); distance metric: Euclidean center-to-center in PDF user space; within-line adjacency angle: ±30° from horizontal; between-line adjacency angle: ±30° from vertical (blocks not meeting either constraint are not connected). **Root node definition:** A block with no incoming edges from blocks whose center-y is greater than this block's center-y (i.e., no block above it in the page is connected to it). Root nodes are sorted by (x_column_index, y descending) to establish the traversal start order.

Set `reading_order_algorithm = "xy_cut"` or `"docstrum"` in page output.

**Crates:** None (graph is a simple `Vec<Edge>`)

**Critical tests:**
- Two-column academic paper: all left-column blocks before all right-column blocks
- Magazine layout with sidebar: main text flow separated from sidebar
- Single-column text: XY-cut produces single region, no spurious splits
- Rotated page (Rotate=90): coordinate system rotated before applying algorithm

### 4.6 Output Serialization (Plain Text Mode)

Implement `--text` output as a projection of the block list.

**Rules:**
- Blocks serialized in reading order
- Paragraphs separated by `\n\n`
- Page breaks: `\f` (form feed, 0x0C)
- Headers and footers excluded by default; `--include-headers-footers` flag re-enables
- Invisible text (Tr=3) excluded unless `--include-invisible-text` flag set
- Watermark blocks excluded (Phase 7 watermark detection — see `docs/research/watermark-and-background-separation.md`). Prior to Phase 7, watermarks are not excluded from `--text` output; `kind: 'watermark'` blocks are not emitted.

**Critical tests:**
- 10-page document: 9 form-feed characters in output
- Header block: excluded from `--text` output by default
- Invisible text span: excluded from `--text` output

### 4.7 Text Readability Validation and Correction

**This phase is a primary accuracy differentiator.** Existing extractors emit raw glyph sequences regardless of whether the output text is human-readable. pdftract validates every span and repairs or discards unreadable output, ensuring extracted text can be used directly without downstream cleanup.

**Readability scoring (per-span):**

| Signal | Weight | Threshold |
|---|---|---|
| Printable Unicode fraction (non-U+FFFD, non-control) | 0.35 | > 0.95 → good |
| Dictionary word coverage (English; fast trie lookup) | 0.30 | > 0.60 → good |
| Whitespace distribution (not all one word, not all spaces) | 0.15 | ratio in [0.05, 0.40] → good |
| Ligature integrity (no split ligatures: fi, fl, ffi, ffl) | 0.10 | 0 split ligatures → good |
| Glyph confidence floor (from Phase 2) | 0.10 | min confidence > 0.6 → good |

Composite score [0.0, 1.0]. Spans below `readability_threshold` (default 0.5, configurable) are flagged `readability: "low"`.

**Correction pipeline (applied before flagging):**

1. **Ligature repair:** If `fi`, `fl`, `ffi`, `ffl`, `ff` appear as adjacent U+FFFD + glyph (Phase 2 glyph level missed the ligature but position data shows adjacency < 0.1pt gap), reconstruct the ligature string from shape-matched component glyphs.
2. **Hyphenation repair:** End-of-line hyphen (`-\n` at right edge of column) joined with start of next line's first word. Strip the hyphen; concatenate. Applies only within the same block; do not join across block boundaries.
3. **Mojibake detection:** If the span contains sequences characteristic of Latin-1 interpreted as UTF-8 (e.g., `Ã©` for `é`), attempt re-decoding via `encoding_rs` and accept if readability score improves.
4. **Soft-hyphen removal:** U+00AD (soft hyphen) stripped from output text; it is a formatting hint, not content.
5. **Word-break normalization:** U+200B (zero-width space), U+FEFF (BOM mid-stream), U+200C/200D (non-joiner/joiner used incorrectly) stripped unless the script requires them (Arabic, Indic).

**Per-page readability score:** Median of span scores, weighted by span character count. Stored in `page.extraction_quality.readability`. If page score < 0.5 and page is `Vector` class, escalate to `BrokenVector` and re-route to assisted OCR path (Phase 5.5). Prior to Phase 5 availability (v0.1.0 builds compiled without the `ocr` feature), pages escalated to BrokenVector are emitted with `page_type: 'broken_vector'`, `extraction_quality.readability` set to the computed score, and a `BROKENVECTOR_OCR_UNAVAILABLE` diagnostic. No re-extraction is attempted. The OCR escalation path is compiled conditionally via `#[cfg(feature = 'ocr')]`.

**Crates:** `unicode-normalization` (already in default deps)

**Word list:** Embed a minimal 20,000-word English frequency list as a compile-time `phf::Set` (adds ~200 KB to binary; acceptable). Binary size is verified by a CI check: `cargo bloat --release --crates | grep pdftract_wordlist` must report ≤ 250 KB. If the actual size exceeds this, replace the phf::Set with a Bloom filter (`bloomfilter` crate, ~25 KB for 20k words at 0.1% false-positive rate) and accept that ~0.1% of non-words will score as words — negligible impact on readability scoring accuracy. Non-English documents: score only on printable fraction, whitespace distribution, and glyph confidence (skip dict lookup if `lang` attribute indicates non-English). The `lang` used here is the document-level language from the catalog `/Lang` entry (available from Phase 1.4), not the per-span `lang` field (which is populated in Phase 7). If `/Lang` is absent or non-English (not matching `en*`), the dictionary word signal is set to 1.0 (disabled) for all spans in the document.

**Critical tests:**
- Span with split ligature `U+FFFD U+0069` adjacent to `f`: repaired to `fi`
- Hyphenated word spanning line break: joined correctly, hyphen stripped
- Latin-1 mojibake `Ã©` → corrected to `é` when re-decode raises readability score
- Page readability < 0.5 on vector page: page re-classified to BrokenVector, OCR invoked
- Non-English page (Chinese): dict-word signal disabled; score driven by printable fraction + confidence
- 20,000-word phf::Set lookup: < 100 ns per word (benchmark assertion)

---

## Phase 5: OCR Integration

**Goal:** Extract text from scanned pages and improve broken-vector pages via Tesseract.  
**Complexity:** Complex  
**Estimate:** 3–4 weeks  
**Depends on:** Phase 4 complete (OCR output feeds back into Phase 4 assembly)  
**Delivers:** Full extraction for scanned PDFs; `pdftract extract --ocr` flag active

### 5.1 Page Classification

Classify each page to select the extraction path before any expensive work.

**Signals (computed in order, short-circuit when confident):**

| Signal | Vector | Scanned | BrokenVector |
|---|---|---|---|
| No text operators in content stream | — | Strong | — |
| All text Tr=3 + full-page image | — | — | Definitive |
| Image coverage fraction > 0.85 | — | Strong | — |
| Character validity rate < 0.4 | — | — | Strong |
| Character validity rate > 0.85 | Strong | — | — |
| Character density ratio < 0.03 | — | Moderate | — |

**PageClass output:** `Vector | Scanned | Hybrid | BrokenVector` with `confidence: f32`.

**Hybrid detection:** Compute per-region classification: divide page into 8×8 grid cells. Cells with text operators and high validity → vector; cells with image coverage and no text → scanned. If both types present in significant fractions — defined as ≥ 15% each (≥ 10 of 64 grid cells classified as vector AND ≥ 10 classified as scanned) — → `Hybrid`.

**Critical tests:**
- Pure text PDF: all pages `Vector` with confidence > 0.95
- Scanned single-page PDF (image only): `Scanned`
- PDF/A with invisible text layer over scanned image: `BrokenVector`
- Hybrid page with text header and scanned body: `Hybrid`, correct region split

### 5.2 Image Extraction for Raster Pages

For `Scanned` and `Hybrid` pages, produce a raster for Tesseract.

**Rendering approach — two-tier:**

**Default (no `full-render` feature):** Direct image compositing. Collect all image XObjects on the page, decode each (Phase 1.5 stream decoder), and composite them onto a blank canvas using each XObject's placement matrix (CTM from `cm` and `Do` operators). This path has zero additional binary cost and handles > 90% of scanned PDFs correctly (those where the scan is a single full-page image).

**`full-render` feature:** `pdfium-render` (wraps Chromium's PDFium). Use when the page has complex rendering geometry — multiple overlapping images, image masks, soft masks — where compositing gets the wrong result. Binary cost: ~20 MB native library (tracked against the weight target; document in PR if this feature is enabled in the default Docker image). Enable with `--features full-render` at compile time or set `ExtractionOptions.full_render = true` at runtime (feature must be compiled in).

**Release Docker images:** The standard `pdftract:latest` and `pdftract:ocr` images are built with `--features ocr,serve` only (no `full-render`). A separate `pdftract:full` image tag is built with `--features ocr,serve,full-render` and has a higher size budget (~140 MB). The weight target table's 120 MB limit applies to `pdftract:ocr` only; `pdftract:full` is documented as a heavyweight variant.

**DPI selection:**
- Standard body text (font_size > 8pt equivalent): 300 DPI
- Fine print or small text: 400 DPI
- Line art / JBIG2 pages: 200 DPI (already binary; higher DPI doesn't help) (JBIG2 decoding for OCR requires `full-render` feature; see Phase 1.5 filter notes)

**Hybrid page handling:** For Hybrid pages, Phase 3 content stream extraction runs first on the entire page to capture vector text. OCR runs only on the grid cells with image coverage fraction > 0.80 (identified during Phase 5.1 classification). Results are merged by bounding box: where a vector span's bbox overlaps an OCR span's bbox by > 50%, the vector span is used (higher confidence); non-overlapping regions use whichever source produced text in that area.

**Output:** Grayscale `image::GrayImage` for each page region needing OCR.

**Crates:** `image` (default `ocr` feature), `pdfium-render` (`full-render` feature only)

### 5.3 Image Preprocessing

Apply the preprocessing pipeline before Tesseract invocation.

**Pipeline (in order):**
1. **Deskew:** Hough line transform on grayscale input via `leptonica-plumbing`'s `pixDeskew`; no pre-binarization required for skew detection. Compute dominant angle; rotate by negative angle. Skip if detected angle < 0.3° (no meaningful skew).
2. **Contrast normalization:** Histogram stretch to [0, 255]. Applied before binarization to improve threshold quality on unevenly-lit scans. Skip for JBIG2 (already binary).
3. **Binarization:** Sauvola local adaptive thresholding for physical scans; Otsu global for digital-origin scans. Detect origin via image XObject filter: DCTDecode → Sauvola; JBIG2Decode → already binary, skip.
4. **Denoising:** 3×3 median filter for salt-and-pepper noise. Skip for JBIG2 (already clean binary).
5. **Border padding:** Add 10px white border on all sides (Tesseract accuracy improves with padding).

**Crates:** `leptonica-plumbing` (Sauvola, deskew via `pixDeskew`), `image` (Otsu, median filter)

**Critical tests:**
- 2° skewed scan: deskewed to within 0.1° before OCR
- Page with uneven lighting (shadow from binding): Sauvola thresholding produces clean binary
- Already-binary JBIG2 image: binarization step skipped, no quality degradation

### 5.4 Tesseract Integration

Invoke Tesseract on preprocessed raster images and parse HOCR output.

**Configuration:**
- Language: from `ExtractionOptions.ocr_language` (default `["eng"]`)
- Page segmentation mode: `PSM_AUTO` (Tesseract decides)
- Output format: HOCR XML (provides per-word bounding boxes and confidence scores)
- Tesseract init: one `TessBaseAPI` per thread (stored in `thread_local!`); avoid re-initialization cost

**HOCR parsing:**
- Parse `ocrx_word` elements: extract `title` attribute for `bbox x0 y0 x1 y1` and `x_wconf NNN` (confidence 0–100 → 0.0–1.0)
- Convert HOCR pixel coordinates to PDF user-space coordinates using the DPI and page geometry
- Each HOCR word → one Span with `confidence_source = "ocr"`

**Crates:** `tesseract` (0.14; wraps `libtesseract` FFI), `quick-xml` (HOCR parsing)

**Critical tests:**
- Clean black-on-white scan of Lorem Ipsum: word error rate < 2%
- Multi-language page (English and French): both language packs loaded; correct characters extracted
- Tesseract confidence < 30 on a region: `confidence = 0.3` in span output
- HOCR bbox coordinates correctly converted to PDF space after DPI scaling

### 5.5 Assisted OCR (BrokenVector Path)

For `BrokenVector` pages, use vector glyph position data to validate Tesseract output rather than as segmentation pre-seeds.

**Pipeline:**
1. Run Phase 3 content stream processing in position-hint mode: collect glyph bboxes but discard Unicode values (treat all as U+FFFD)
2. Run Tesseract in `PSM_SPARSE_TEXT` mode (page segmentation mode 11), which allows Tesseract to find text in arbitrary positions without requiring a dominant text block — appropriate for BrokenVector pages where the visible text layer may be fragmented or partially occluded
3. After OCR completes, validate each Tesseract word result against the nearest vector glyph bbox: if the Tesseract word's center falls within 5pt of a vector glyph bbox center, the word is accepted with its OCR confidence; otherwise it is flagged low-confidence (confidence capped at 0.4)
4. Parse HOCR output as in Phase 5.4, applying per-word confidence adjustments from step 3
5. If OCR confidence > 0.7 for a region: use OCR text; if OCR confidence < 0.3: re-attempt without the validation filter (pure OCR fallback)

**Critical tests:**
- PDF/A with invisible text layer at correct positions: OCR output better than blind OCR (validate WER delta)
- PDF/A with incorrect text layer positions (misaligned): validation filter rejects misaligned words; fallback to unaided OCR confidence scores

---

## Phase 6: Output and API

**Goal:** Deliver the full output schema, PyO3 bindings, and HTTP serve mode.  
**Complexity:** Medium  
**Estimate:** 3–4 weeks  
**Depends on:** Phase 5 complete  
**Delivers:** Shippable CLI, Python package, HTTP service

### 6.1 JSON Output (Full Schema)

Implement the complete output schema from `docs/research/extraction-output-schema.md`.

**Document-level fields:**
- `schema_version: "1.0"`
- `metadata`: title, author, subject, keywords, creator, producer, creation_date, modification_date, page_count, pdf_version, is_tagged, is_encrypted, conformance, contains_javascript, contains_xfa, ocg_present, generator
- `outline`: recursive bookmark tree with title, destination, level
- `threads`: article thread chains (Phase 7 feature; empty array in Phase 6)
- `attachments`: from `/EmbeddedFiles` name tree (Phase 7; empty array in Phase 6)
- `signatures`: digital signature metadata (Phase 7; empty array in Phase 6)
- `form_fields`: AcroForm fields with values (Phase 7; empty array in Phase 6)
- `links`: document-scoped URI and internal destination links (Phase 7 feature; empty array in Phase 6)
- `extraction_quality`: aggregate across all pages
- `errors`: all diagnostics emitted during extraction

**Page-level fields (full schema):**
- `page_index` (0-based integer, canonical for programmatic use), `page_number` (1-based integer, human-facing; always equals `page_index + 1`), `page_label` (string from PDF `/PageLabels` number tree, e.g. `"iv"` or `"A-3"`; absent if the PDF defines no page labels), `width`, `height`, `rotation`, `page_type`

  > **Naming convention:** `page_index` is the stable, zero-based identifier used in all internal references (e.g., error diagnostics, NDJSON frame ordering). `page_number` is emitted alongside it as a convenience for human-facing display. Both fields are always present. SDK code and downstream tools MUST key on `page_index` for programmatic access; `page_number` is informational only.
- `spans`: full Span array per schema
- `blocks`: full Block array per schema
- `annotations`: highlights, stamps, notes, links from `/Annots` (Phase 7 feature; empty array in Phase 6)
- `tables`: parallel table structure objects for `kind: table` blocks (Phase 7)

**Crates:** `serde`, `serde_json`

**JSON Schema deliverable:** A machine-readable JSON Schema is generated from the extraction output schema and stored at `docs/schema/v1.0/pdftract.schema.json`. This file is generated once and checked into the repo. The Phase 6.1 critical test uses `jsonschema` (Python) or `jsonschema-valid` (Rust) to validate test output against this file. Creating this JSON Schema is a Phase 6.1 deliverable alongside the Rust implementation.

**Critical tests:**
- Schema validator: produce output from a known-good PDF, validate against `docs/schema/v1.0/pdftract.schema.json`
- Page with no text: `spans: []`, `blocks: []`, `page_type: "blank"` or `"figure_only"`
- Error entries: each emitted diagnostic has stable `code`, `severity`, and `page_index`

### 6.2 NDJSON Streaming Mode

Implement `--stream` / `ExtractionOptions.streaming = true`.

**Frame sequence:**
1. Header frame: `{"frame":"header","schema_version":"1.0","metadata":{...},"outline":[...],"total_pages":N}`
2. Per-page frames (emitted as each page completes via rayon): `{"frame":"page","page_index":N,...}`  
   Note: rayon may complete pages out of order; buffer completed pages and emit in page_index order with a window of 8 pages maximum. When the out-of-order buffer holds 8 completed pages and the next in-order page has not yet completed, the output thread blocks on a `Condvar` until that page's rayon task signals completion. The window size of 8 is chosen to be larger than the typical rayon thread pool size (4–8 threads), ensuring the output thread is never the bottleneck on balanced workloads. For pathological cases (one very slow page surrounded by fast pages), the window is effectively a backpressure signal to the downstream consumer.
3. Footer frame: `{"frame":"footer","extraction_quality":{...},"errors":[...],"threads":[],"attachments":[],"signatures":[],"form_fields":[],"links":[]}`

**Header/footer detection in streaming mode:** The cross-page header/footer deduplication pass (Phase 4.4) cannot run before individual page frames are emitted. In streaming mode, header and footer blocks are emitted as `kind: 'header'` / `kind: 'footer'` only if they can be identified from the trailing window of up to 4 already-emitted pages. For the first 3 pages, header/footer detection is deferred: those blocks are emitted as `kind: 'paragraph'` and NOT retroactively corrected. Consumers relying on exact `kind` values for headers/footers should use the non-streaming mode.

**BufWriter:** Wrap `io::Stdout` in `BufWriter<io::Stdout>` with 128 KB buffer; flush after each frame.

**Critical tests:**
- 100-page document in streaming mode: output contains exactly 102 newline-delimited JSON objects: 1 header object (first), 100 page objects (in page_index=0 to page_index=99 order), 1 footer object (last). Each object is complete and valid JSON.
- Out-of-order page completion: pages buffered and emitted in correct index order
- Consumer reads frame-by-frame with `newline` delimiter: each frame is valid JSON

### 6.3 PyO3 Python Bindings

Build a Python extension module exposing the extraction API.

**Module:** `pdftract` (import as `import pdftract`)

**API surface:**
```python
# Synchronous extraction
result: dict = pdftract.extract(path: str, **options) -> dict
text: str = pdftract.extract_text(path: str, **options) -> str

# Streaming (returns an iterator of page dicts)
pages: Iterator[dict] = pdftract.extract_stream(path: str, **options)
# Yields only page dicts (frame: 'page' equivalent). Metadata and errors are not yielded — call extract() for the full document result including metadata.

# Options (keyword arguments mapped to ExtractionOptions):
# ocr=False, ocr_language=["eng"], include_invisible=False,
# extract_forms=False, extract_attachments=False, readability_threshold=0.5,
# password=None

# Exceptions
class PdftractError(Exception): ...       # extraction failed
class EncryptionError(PdftractError): ... # encrypted, no password
```

**Python GIL handling:** Release the GIL during extraction (`py.allow_threads(|| ...)`) so Python threads can continue while a page is being processed.

**Build:** `maturin build --features python` produces a `.whl` for the current platform. CI cross-compiles for all five target triples (see `docs/notes/sdk-architecture.md`).

**CI note:** PyO3 wheel cross-compilation for macOS and Windows from a Linux runner is handled using `maturin build --target <triple>` with the `cross` tool (Docker-based cross-compilation). The Argo WorkflowTemplate `pdftract-py-ci` (to be created in `jedarden/declarative-config → k8s/iad-ci/argo-workflows/`) will use a `ghcr.io/rust-cross/manylinux` base image for Linux wheel builds and `osxcross` toolchain for macOS targets. Windows `.whl` is built using `cross` with `x86_64-pc-windows-gnu`. All five triples ship to PyPI on milestone tags via the same workflow.

**Crates:** `pyo3` (feature `extension-module`), `maturin` (build tool)

**Critical tests:**
- `pdftract.extract("test.pdf")` returns a dict with correct `metadata.page_count`
- `pdftract.extract_text("test.pdf")` returns a plain-text string
- `pdftract.extract("nonexistent.pdf")` raises `PdftractError`
- `pdftract.extract("encrypted.pdf")` raises `EncryptionError`
- Python threading: 4 threads each extracting different PDFs simultaneously; no deadlock

### 6.4 HTTP Serve Mode

Implement `pdftract serve --port PORT`. Requires `--features serve` at compile time (`axum` + `tokio` are not in the default build — they add ~2 MB to the binary). The pre-built release binaries for the `serve` Docker image are compiled with `--features ocr,serve`.

**Endpoints:**

| Method | Path | Request | Response |
|---|---|---|---|
| POST | `/extract` | multipart/form-data `file=<pdf>` + optional form fields for options | JSON extraction result |
| POST | `/extract/text` | same | `text/plain` body |
| POST | `/extract/stream` | same | NDJSON stream (Content-Type: application/x-ndjson) |
| GET | `/health` | none | `{"status":"ok","version":"x.y.z"}` |

**Optional form fields (all endpoints):**

| Field | Type | Default | Maps to |
|---|---|---|---|
| `ocr` | boolean | `false` | `ExtractionOptions.ocr` |
| `ocr_language` | string (comma-separated) | `eng` | `ExtractionOptions.ocr_language` |
| `readability_threshold` | float | `0.5` | `ExtractionOptions.readability_threshold` |
| `include_invisible` | boolean | `false` | `ExtractionOptions.include_invisible` |
| `extract_forms` | boolean | `false` | `ExtractionOptions.extract_forms` |
| `extract_attachments` | boolean | `false` | `ExtractionOptions.extract_attachments` |
| `password` | string | `""` | `ExtractionOptions.password` |

**Error responses:**

| Status | Condition |
|---|---|
| 400 | Bad request (no file field, unsupported content type) |
| 413 | Request exceeds `--max-upload-mb` limit |
| 422 | Extraction error (encrypted file, corrupt file) |
| 500 | Internal error |

Response body for all error statuses is `{"error":"code","message":"..."}`. A custom `RequestBodyLimit` rejection handler is implemented to convert tower-http's default plain-text 413 response to the standard JSON error body `{"error":"REQUEST_TOO_LARGE","message":"Request body exceeds the configured limit"}`.

**Concurrency:** axum handles concurrent requests; rayon thread pool is shared across all requests. No per-request thread spawning. Each POST handler bridges async and sync via `tokio::task::spawn_blocking(|| extraction_call())`, which runs the synchronous rayon work on tokio's blocking thread pool (separate from the async executor). Rayon provides within-document page-level parallelism; tokio's blocking pool handles per-request concurrency. Rayon's default pool sizing (equivalent to the logical CPU count) is used; no explicit pool configuration is required.

**Request size limit:** Default 256 MB; configurable via `--max-upload-mb`.

**Security constraints:**
- **Decompression limit:** The stream decoder (Phase 1.5) enforces a `max_decompressed_bytes` limit (default: 2 GB per document, configurable via `--max-decompress-gb`). Any stream that exceeds this limit emits a `STREAM_BOMB` diagnostic and returns the bytes decoded so far.
- **Authentication:** No auth is built in. Deploy behind a reverse proxy (nginx, Traefik) with authentication. The serve mode is not safe to expose directly on a public port without a proxy.
- **Path parameters:** No file-path parameters are accepted in serve mode — the PDF is always received as a multipart upload. This eliminates path traversal risk.

**Crates:** `axum`, `tokio`, `tower-http` (for `RequestBodyLimit`, `TraceLayer`), `multer` (multipart parsing)

**Critical tests:**
- `curl -F file=@test.pdf http://localhost:8080/extract`: valid JSON response
- File exceeding size limit: HTTP 413 response with JSON body `{"error":"REQUEST_TOO_LARGE","message":"Request body exceeds the configured limit"}` (not tower-http's default plain-text response)
- Concurrent requests with 8 simultaneous PDFs: all complete correctly
- `/health` endpoint: 200 OK, even while extractions are in progress

---

## Phase 7: Advanced Features

**Goal:** StructTree exploitation, table detection, AcroForm/XFA, attachments, signatures.  
**Complexity:** Medium–Complex per feature  
**Estimate:** 4–5 weeks (features developed independently; can be parallelized across developers)  
**Depends on:** Phase 6 complete

### 7.1 StructTree Exploitation (Tagged PDF)

Use the PDF structure tree as the authoritative reading order for tagged documents.

**Implementation:**
1. From document catalog `/StructTreeRoot`, load the root `StructElem`
2. Walk the structure tree depth-first; at each `StructElem`, record the element type (mapped via `/RoleMap` if non-standard), the `/ActualText` attribute (overrides extracted text if present), the `/Alt` attribute (alternative text for figures), and the `/Lang` attribute (BCP-47 language tag)
3. For each `StructElem`, collect its MCID references: each marked content sequence (identified by its MCID from Phase 3.4) is assigned to its owning `StructElem` via the `ParentTree`
4. Build the block list by traversing the structure tree in document order; each `StructElem` maps to one block; its constituent MCIDs provide the spans in reading order
5. Map structure element types to block kinds: `P` → paragraph, `H`/`H1`–`H6` → heading with level, `Table` → table, `L`/`LI` → list, `Figure` → figure, `Artifact` → suppressed (not emitted in output)

**Validation:** If `MarkInfo /Suspects true`, fall back to XY-cut for any page where the structure tree coverage is less than 80% of extracted glyphs.

**`reading_order_algorithm`:** Set to `"struct_tree"` when used.

**Crates:** None beyond Phase 1 parser

**Critical tests:**
- Word-generated tagged PDF: heading levels correctly extracted (H1/H2 map to level 1/2)
- Tagged PDF with `/ActualText` on a ligature: ActualText value used, not glyph-decoded text
- Tagged PDF with `/Artifact` marked content: artifact glyphs excluded from output
- PDF with `Suspects true`: falls back to XY-cut, `reading_order_algorithm = "xy_cut"`

### 7.2 Table Detection and Structure Reconstruction

Detect tables and reconstruct cell structure.

**Detection pipeline:**
1. **Line-based detection:** Collect all horizontal and vertical path segments from the content stream (operators `m`/`l`/`S`, `re`/`S`, `re`/`f`). Cluster collinear segments. Find intersection points. Build grid from intersections. See `docs/research/table-structure-reconstruction.md` for the full grid reconstruction algorithm.
2. **Borderless table detection:** If no ruling lines found, examine span alignment: if 3+ lines share identical x0 positions for multiple groups, treat as candidate columns. Require 3+ rows to confirm.
3. **Cell content assignment:** For each cell bbox, collect all spans whose centroid falls within the bbox. Assign to the cell.
4. **Header row detection:** First row is header if all cells have bold font or if StructTree marks the row as `TH` type.
5. **Merged cell detection:** Missing interior edge between two cells → colspan or rowspan; infer from geometry.

**Output:** Block with `kind: "table"` and a parallel `table` object in the page output with rows/cells as per the schema.

**Crates:** None (geometry is pure arithmetic)

**Critical tests:**
- 5×3 bordered table: all 15 cells extracted with correct text
- Merged header cell spanning 3 columns: colspan=3 in output
- Borderless two-column table: detected via alignment heuristic
- Table spanning two pages: detected and flagged (full reconstruction deferred to non-streaming mode)

### 7.3 Digital Signature Metadata

Extract digital signature field metadata.

**Implementation:** Walk AcroForm `/Fields` array looking for Sig-type fields (`/FT /Sig`). For each signature field, extract: `/T` (field name), `/V` (signature dict) → `/Name` (signer name), `/M` (signing date, ISO 8601), `/Reason`, `/Location`, `/ByteRange` (byte ranges signed, for coverage analysis), `/SubFilter` (signature format: `adbe.pkcs7.detached`, `adbe.x509.rsa.sha1`, etc.).

**Validation:** pdftract does NOT perform cryptographic validation (that requires the full certificate chain and OCSP/CRL infrastructure). Instead, report `validation_status: "not_checked"`. A future version may integrate `ring` or `openssl` for validation.

**Output:** `signatures` array at document level per the output schema.

**Crates:** None beyond Phase 1 parser

**Critical tests:**
- PDF with two signature fields: both extracted with correct signer names and dates
- Signature field with no `/V` (unsigned): extracted with `value: null`
- `/ByteRange` coverage: correctly computed as fraction of file bytes signed

### 7.4 AcroForm and XFA Field Extraction

Extract interactive form field definitions and current values.

**AcroForm:**
- Walk `/Fields` recursively (fields may be nested in `/Kids`)
- For each field: `/T` (partial name), `/FT` (type: Tx/Btn/Ch/Sig), `/V` (current value), `/DV` (default value), `/Ff` (flags: required, read-only, multi-line), `/Rect` (bbox)
- Tx fields: `/V` is a string
- Btn fields: `/V` is a name (the selected appearance state); compute is_checked
- Ch fields: `/V` is selected option; `/Opt` array lists all options
- Construct full field names by joining partial names with `.`

**XFA:**
- If `/AcroForm /XFA` is present, parse the XFA XML stream(s) (either single stream or array of named streams concatenated as XML)
- Walk the XFA data model to extract field values from `<field>` elements; use the XFA field name as the key
- If both AcroForm and XFA are present, prefer XFA values for overlapping fields

**Crates:** `quick-xml` (XFA parsing)

**Critical tests:**
- PDF with text field, checkbox, and dropdown: all three types extracted with correct values
- Nested field hierarchy: full dot-separated name constructed correctly
- XFA-only form: all field values extracted from XFA XML
- Hybrid XFA+AcroForm: XFA values preferred

### 7.5 Portfolio and Attachment Extraction

Extract embedded files from PDF portfolios and `/EmbeddedFiles` name trees.

**Implementation:**
- Locate the `/EmbeddedFiles` name tree in the catalog `/Names` dictionary
- Walk the name tree leaves, each yielding a `Filespec` dictionary
- From each `Filespec`: `/F` or `/UF` (filename), `/Desc` (description), `/Type /Filespec`, `/EF` dict → `/F` stream (the embedded file data)
- From the EF stream dictionary: `/Subtype` (MIME type hint), `/Params` dict → `/Size`, `/CreationDate`, `/ModDate`, `/CheckSum`
- Decode the stream (applying its filters)

**Size limit:** If attachment stream decoded size > 50 MB, include metadata only and set `data: null` with a `truncated: true` flag.

**Portfolio navigator:** Check for `/Collection` entry in catalog; if present, extract portfolio schema and sort fields for richer metadata.

**Output:** `attachments` array at document level.

**Crates:** None beyond Phase 1 parser and stream decoder

**Critical tests:**
- PDF with 3 embedded files of different MIME types: all three extracted with correct filenames and sizes
- Attachment with no `/Desc`: description is null (not empty string)
- Attachment exceeding size limit: metadata present, `data: null`, `truncated: true`

### 7.6 Hyperlink and Annotation Extraction

Extract URI hyperlinks and page annotation objects.

**Implementation:**
- For each page, walk the `/Annots` array in the page dictionary
- Collect Link annotations (`/Subtype /Link`):
  - Extract `/A` action dict: if `/S /URI`, read the `/URI` string as the target URL
  - Extract `/Dest`: if present (named or explicit destination), record as an internal link
  - Both URI and internal links are appended to the document-level `links` array with `page_index`, `rect` (the annotation bbox), and `uri` or `dest` as appropriate
- Collect other annotation subtypes (Highlight, Stamp, FreeText, Note, Squiggly, StrikeOut, Underline):
  - Extract `/Subtype`, `/Rect`, `/Contents` (comment text), `/T` (author), `/M` (modification date), `/C` (color array)
  - Append to the page-level `annotations` array

**Output:** Document-level `links` array (URI and internal destination links from all pages); page-level `annotations` array (all non-link annotations on each page).

**Crates:** None beyond Phase 1 parser

**Critical tests:**
- PDF with 5 URI hyperlinks: all 5 appear in document-level `links` with correct URLs
- Link annotation with named destination (`/Dest /SectionTwo`): emitted as internal link with `dest: "SectionTwo"`
- Page with Highlight and Note annotations: both appear in page-level `annotations` with correct subtypes
- Annotation with no `/Contents`: `contents` field is null (not empty string)

### 7.7 Article Thread Chains

Reconstruct PDF article thread chains for multi-column and multi-page reading flows.

**Implementation:**
- Read the `/Threads` array from the document catalog; each entry is an article thread dict
- Each thread dict has `/F` (first bead object reference) and `/I` (thread info dict with `/Title`, `/Author`, `/Subject`, `/Keywords`)
- Walk the bead chain by following `/N` (next bead) links from the first bead; detect the chain end when `/N` loops back to the first bead (circular list)
- Each bead dict has `/R` (page object reference, resolves to the page containing the bead) and `/V` (bbox rect of the bead region on the page)
- Reconstruct the ordered list of beads for each thread: `[{ page_index, rect }, ...]`

**Output:** Document-level `threads` array; each entry has `title` (from thread info `/Title`, or null), `author`, `subject`, and `beads` (ordered list of `{ page_index, rect }` objects).

**Crates:** None beyond Phase 1 parser

**Critical tests:**
- PDF with two article threads: both reconstructed with correct bead order and page references
- Thread with no `/I` info dict: `title`, `author`, `subject` all null; bead chain still reconstructed
- Bead `/V` rect correctly converted to PDF user-space coordinates for the referenced page
- Circular bead chain termination: chain walk stops after visiting all beads without infinite loop

---

## Cross-Cutting: Test Infrastructure

Tests are organized into three tiers:

### Tier 1: Unit Tests (in-crate `#[test]`)

Each module has unit tests covering the critical test cases listed per phase above. These run with `cargo test` and have no external dependencies.

**Target:** 100% of public function surfaces; all error paths exercised.

### Tier 2: Integration Tests (`tests/` directory)

Integration tests use a corpus of reference PDFs stored in `tests/fixtures/`. Each fixture has a corresponding expected-output JSON file. Tests verify:
- Exact text content match (for clean vector PDFs)
- Schema validity (all output against JSON Schema)
- Performance: extraction of a 100-page vector PDF completes in **< 3 seconds** on a 4-core CI machine (failure = CI block)

**Fixture categories:**
- `tests/fixtures/vector/`: clean LaTeX, Word, InDesign outputs
- `tests/fixtures/scanned/`: physical scans at various DPIs and skew angles
- `tests/fixtures/cjk/`: Chinese, Japanese, Korean documents
- `tests/fixtures/malformed/`: truncated, corrupt xref, circular references
- `tests/fixtures/encrypted/`: AES-128, AES-256, RC4 encrypted
- `tests/fixtures/forms/`: AcroForm and XFA documents
- `tests/fixtures/tagged/`: PDF/UA and PDF/A-a tagged documents
- `tests/fixtures/encoding/`: fonts with no ToUnicode CMap; verifies Levels 2–4 Unicode recovery; matched against known-good Unicode output
- `tests/fixtures/perf/`: one or more large (≥100 page) vector PDFs for speed benchmarking; output is validated for correctness but the primary metric is wall-clock time

`tests/fixtures/bench/` (Tier 4) uses the same PDFs as `tests/fixtures/perf/` plus competitor-run results; no separate corpus needed.

### Tier 3: Regression Corpus (CI only)

A private corpus of 500 real-world PDFs from diverse sources runs on every PR. Output is compared against a golden snapshot using a character-level diff. Any regression > 0.5% character error rate blocks the PR.

### Tier 4: Competitive Benchmarks (CI, tracked over time)

Benchmark suite runs `pdftract`, `pdfminer.six`, `pypdf`, and `pdfplumber` against identical fixture PDFs on the same CI machine. Results are stored as a JSON artifact per commit so regressions are detectable.

**Benchmark runner infrastructure:** A dedicated step in the `pdftract-ci` WorkflowTemplate uses a `python:3.11-slim` container. A `benches/competitors/requirements.txt` file (checked into repo) pins: `pdfminer.six==20231228`, `pypdf==4.2.0`, `pdfplumber==0.11.0`. A `benches/competitors/run_all.py` script drives competitor runs and emits results as `benches/results/<commit-sha>.json`. Results are stored as Argo Workflow artifacts. The pdftract binary time is measured with `hyperfine --warmup 2 --runs 5`.

**Metrics tracked per tool per fixture:**
- Wall-clock extraction time (mean of 5 runs)
- Peak RSS (resident set size)
- Character error rate vs. ground truth
- Reading order correctness score

**Minimum passing bar (blocks PR if missed):**
- pdftract must be ≥ 10× faster than `pdfminer.six` on vector PDFs
- pdftract CER must be ≤ `pdfminer.six` CER on all fixture categories
- pdftract binary (default features) must be ≤ 4 MB stripped

**Benchmark fixtures** (`tests/fixtures/bench/`):
- `vector-10.pdf`, `vector-100.pdf`: clean LaTeX output
- `cjk-20.pdf`: mixed CJK
- `two-column-academic.pdf`: multi-column reading order
- `scanned-5.pdf`: physical scan (OCR path only in pdftract)

---

## Phase Dependencies and Sequencing

```
Phase 0 (CI Infrastructure) ← must complete before Phase 1 code review
  └─► Phase 1 (Core Parser)
        └─► Phase 2 (Font Pipeline)
              └─► Phase 3 (Content Stream)
                    └─► Phase 4 (Text Assembly)
                          ├─ 4.7 Readability Validation ← feeds back into 5.1 page classification
                          └─► Phase 5 (OCR)       ← Scanned PDFs work here; 4.7 escalates broken-vector pages here
                                └─► Phase 6 (API) ← PyO3, HTTP, full JSON schema
                                      └─► Phase 7 (Advanced)
                                      ├─ 7.1 StructTree (independent)
                                      ├─ 7.2 Tables (independent)
                                      ├─ 7.3 Signatures (independent)
                                      ├─ 7.4 Forms (independent)
                                      ├─ 7.5 Attachments (independent)
                                      ├─ 7.6 Hyperlinks & Annotations (independent)
                                      └─ 7.7 Article Threads (independent)
```

Phase 0 is a prerequisite for all subsequent phases — no milestone release can ship without active CI. Phase 7 sub-tasks are independent of each other and can be assigned to separate developers once Phase 6 is complete.

---

## Release Milestones

| Milestone | Phases Complete | Capability |
|---|---|---|
| v0.1.0 (Alpha) | 0, 1–4 (incl. 4.7) | CI infrastructure active; vector PDF extraction with readability validation; plain text and JSON output; CLI only; all three primary objective targets must pass |
| v0.2.0 (Beta) | 0, 1–5 | + Scanned PDF OCR; all page classes handled; competitive benchmark suite green |
| v0.3.0 (RC) | 0, 1–6 | + PyO3 bindings; HTTP serve; full JSON schema; NDJSON streaming |
| v1.0.0 (Stable) | 0, 1–7 | + StructTree; tables; forms; signatures; attachments; hyperlinks; article threads |

Binary releases for all five target triples are published to GitHub Releases on every milestone tag. The PyO3 wheel is published to PyPI. The CLI binary is the sole dependency for the subprocess-based SDKs documented in `docs/notes/sdk-invocation.md`.
