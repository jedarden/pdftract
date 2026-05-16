# Performance and Streaming Architecture

## Overview

Handling large PDFs (100 MB+, 1000+ pages) efficiently requires deliberate architectural decisions at every layer: file I/O, object parsing, content stream processing, output serialization, and concurrency. This document specifies the performance-critical patterns for `pdftract` and the rationale behind each choice.

---

## 1. Memory-Mapped File Access

Use `memmap2::Mmap` rather than `std::fs::read()` or `BufReader`. Reading the entire file into a `Vec<u8>` allocates contiguous heap memory proportional to file size, which is unacceptable for 500 MB+ inputs. With `mmap`, the kernel maps the file's pages into the virtual address space; physical RAM is allocated only when pages are accessed, and unused pages are evicted under memory pressure without any application code involvement.

The critical advantage for PDF parsing is **random access without sequential read cost**. The cross-reference table at the end of the file maps object numbers to byte offsets throughout the file. With `mmap`, seeking to object offset `0x1A3F00` is a pointer addition — `&mmap[offset..]` — with no syscall. The OS page fault mechanism fetches only the 4 KB page containing that offset.

On 64-bit Linux, the virtual address space is 128 TB; mapping a 1 GB PDF consumes one entry in the process's VMA table and a trivial amount of page table space until pages are touched. The 32-bit limitation (4 GB VA space) is not a concern for any modern deployment target.

**Sequential vs. random access tradeoff:** For a sequential single-pass parse (linearized PDFs, reading content streams in order), `BufReader` with a 64–128 KB buffer can match or exceed `mmap` throughput because the kernel's readahead prefetches pages ahead of the cursor. For the dominant PDF use case — random access to objects scattered across the file — `mmap` is superior. A practical hybrid: open with `mmap`, and call `madvise(MADV_SEQUENTIAL)` on regions known to be read linearly (e.g., large content streams).

```rust
use memmap2::MmapOptions;
use std::fs::File;

let file = File::open(path)?;
let mmap = unsafe { MmapOptions::new().map(&file)? };
// Treat &mmap[..] as &[u8] for all subsequent parsing
```

---

## 2. Lazy Object Loading

A PDF's xref table (or xref stream in PDF 1.5+) provides a complete map from `(object_number, generation)` to byte offset. Parse this table eagerly at open time — it is compact relative to the file — but defer parsing all objects until first access.

Maintain an object cache as `HashMap<ObjRef, PdfObject>`. For documents with thousands of objects, bound the cache with an LRU eviction policy (`lru` crate). A capacity of 4096 entries handles the working set of any realistic page range without unbounded growth.

**Object streams (`/ObjStm`)**: PDF 1.5 compresses groups of objects into a single stream (FlateDecode, typically). When any object from a given `/ObjStm` is requested, decompress the entire stream once, parse all contained objects, and insert them all into the cache. The decompressed bytes can be stored in a `Bytes` handle (from the `bytes` crate) to allow zero-copy slicing across multiple parsed objects from the same stream.

```rust
struct ObjectCache {
    xref: HashMap<ObjRef, XrefEntry>,
    parsed: LruCache<ObjRef, Arc<PdfObject>>,
    objstm_cache: HashMap<ObjRef, Arc<[u8]>>, // decompressed stream bytes
}
```

---

## 3. Streaming Page Output

Accumulating extraction results for a 1000-page document into a single `Vec<PageResult>` before serializing is prohibitive in memory. Instead, emit NDJSON (newline-delimited JSON): one JSON object per line, flushed to the output `io::Write` as each page is processed.

`serde_json`'s streaming API via `serde_json::Serializer::new(writer)` writes directly to any `io::Write` without an intermediate `String` allocation. Wrap the output in a `BufWriter` to amortize `write` syscalls.

**Tradeoff**: Streaming output is incompatible with features requiring a full document pass:
- **Outline (bookmark) building**: PDF outlines reference destination pages; the full outline tree must be resolved before any page is emitted if outline data is included per-page.
- **Page label resolution**: `/PageLabels` is a document-level number tree; it can be parsed once before streaming begins.
- **Cross-page table detection**: Table cells spanning page breaks require buffering multiple pages. This feature must be opt-in and implies non-streaming mode.

Default to streaming mode; expose `--no-stream` for use cases requiring full-document analysis.

---

## 4. Parallel Page Processing

Each page's content stream is self-contained: it references resources (fonts, XObjects) by name within its resource dictionary, resolves them via the document's shared object graph, but produces output independent of other pages. This makes page processing embarrassingly parallel.

Use `rayon::par_iter()` over a range of page indices. Shared mutable state must be wrapped in `Arc<RwLock<...>>`:
- `Arc<RwLock<ObjectCache>>` — read locks dominate on cache hits; contention is low if the cache is warm.
- `Arc<RwLock<FontCache>>` — keyed by font object reference; write locks occur only on first use of each font.
- Image XObject cache — keyed by XObject reference, same pattern.

**Avoiding lock contention on the hot path**: Do not hold a `RwLock` read guard across the content stream parse loop. The pattern is: acquire the lock, clone the `Arc<FontData>` for the needed font, release the lock immediately, then use the unguarded `Arc` for the duration of parsing. Font data and CMap tables should be `Arc`-wrapped immutable structs — once written, never mutated.

**Output ordering**: `rayon` does not guarantee ordering. Collect `(page_index, PageResult)` pairs, sort by index, then stream in order. For memory efficiency with large documents, process in chunks (e.g., 64 pages at a time) and stream each chunk's sorted output before beginning the next.

---

## 5. Content Stream Parsing Performance

PDF content streams are sequences of operands followed by operator names (e.g., `(Hello) Tj`, `10 0 0 10 72 720 cm`). Parsing is dominated by the tokenizer.

A hand-rolled byte-level tokenizer over `&[u8]` outperforms regex-based approaches by 5–10x for this workload: there is no regex engine overhead, no capture group allocation, and no UTF-8 validation on the raw stream. Validate to UTF-8 only when constructing text output from string operands.

Operator names are short ASCII strings. Match them against a static lookup table (a `phf::Map<&[u8], Operator>` built at compile time) to avoid heap allocation for operator dispatch. For the ~70 PDF operators, a perfect hash or simple match on a `&[u8]` slice is O(1).

**Parser combinator crates**: `winnow` (the successor to `nom`) offers a clean combinator API with competitive performance and good error recovery. It operates on `&[u8]` natively. For content streams, a hand-rolled state machine may still win on throughput because content stream tokens are regular enough that the overhead of combinator composition is visible in profiles. Use `winnow` for the structural parser (cross-reference streams, object syntax) where correctness matters more than raw throughput, and a hand-rolled tokenizer for content streams.

---

## 6. Font and Glyph Caching

Font objects (Type1, TrueType, CIDFont) are referenced by resource name within each page but backed by document-level indirect objects. The same font object is typically used across hundreds of pages. Cache at the object reference level, not the resource name level.

Per font entry, cache:
- The decoded `ToUnicode` CMap as a `HashMap<u16, char>` (or `Vec<(u16, char)>` sorted for binary search when the map is dense and ordered).
- The encoding vector (256-entry `[Option<char>; 256]` for simple fonts).
- The glyph width table as `Vec<u32>` indexed by character code, used for text position tracking.

CMap parsing — especially for CIDFont CMaps with `beginbfrange` sections covering thousands of code points — is the most expensive per-font operation. Wrap the parsed result in `Arc<CMapData>` and store in the font cache. Worker threads clone the `Arc`, not the data.

Glyph-to-Unicode lookup must be O(1) on the hot path. Use `HashMap<u32, char>` for sparse CMaps (CID fonts with sparse mappings) and a direct-index `Vec<char>` for dense simple-font encodings.

---

## 7. Image Decoding Performance

PDF image XObjects use several compression filters:

- **FlateDecode**: `flate2` with the `miniz_oxide` backend. Fast, pure Rust, no FFI overhead. Suitable for in-process decoding.
- **DCTDecode (JPEG)**: Prefer `zune-jpeg` over `jpeg-decoder` — benchmarks show 20–40% higher throughput for typical PDF-embedded JPEGs. Both are pure Rust.
- **JPEG2000 (JPXDecode)**: No mature pure-Rust decoder exists. Use OpenJPEG via FFI (`openjpeg-sys`) or defer to a subprocess. This is a correctness requirement for scanned PDFs from certain scanners.
- **JBIG2**: Used in scanned document PDFs. The only production-grade decoder is `jbig2dec` (C). Invoke via `jbig2dec-sys` FFI bindings or a subprocess. Do not block the rayon thread pool on subprocess I/O — use a dedicated blocking thread pool (`tokio::task::spawn_blocking` or `std::thread::spawn`).
- **CCITTFaxDecode**: Pure Rust implementation is feasible; a reference exists in the `pdf` crate ecosystem.

Cache decoded image data in an `Arc<ImageData>` keyed by XObject reference. A page that places the same image 50 times (e.g., a watermark) should decode once.

---

## 8. Benchmarking Methodology

Measure at multiple granularities:

- **Throughput**: pages/second and MB of PDF input/second, end-to-end.
- **Memory**: peak RSS via `/proc/self/status` snapshots, and heap allocations via `dhat` (compile with `dhat` feature, profile with `dhat-viewer`).
- **Latency distribution**: tail latency (p99) matters for the HTTP server mode.

Representative corpus categories:
- Academic papers (LaTeX-generated, many Type1/TrueType fonts, dense text).
- Financial filings (SEC EDGAR PDFs: forms, tables, mixed fonts).
- Scanned documents (rasterized pages, JBIG2/JPEG images, minimal text layer).
- Technical manuals (large page counts, complex layouts, embedded vector graphics).
- PDF forms (AcroForm, interactive fields — primarily object graph stress test).

Use `criterion` for microbenchmarks of hot functions (tokenizer, CMap lookup, FlateDecode). For end-to-end benchmarks, drive with `hyperfine` against a fixed corpus. Profile with `cargo flamegraph` (wraps `perf record` + `inferno`) to identify throughput bottlenecks. Use `dhat` specifically for allocation hotspots — it attributes each allocation to its call stack, which is essential for finding unnecessary `String` clones in the parse path.

---

## 9. Binary Size and Startup Time

Full feature compilation (font handling, JBIG2 FFI, JPEG2000, Tesseract OCR) produces a binary well over 50 MB. Mitigate with Cargo feature flags:

```toml
[features]
default = ["flate", "jpeg"]
jbig2 = ["jbig2dec-sys"]
jpeg2000 = ["openjpeg-sys"]
ocr = ["tesseract-sys"]
```

Apply LTO and size optimization for release builds:

```toml
[profile.release]
lto = "thin"       # "fat" for maximum but slow; "thin" is a good default
opt-level = "z"    # minimize binary size; switch to "3" if throughput is more important
codegen-units = 1
```

Use `cargo-bloat` (`cargo bloat --release --crates`) to identify which crates dominate binary size. Common offenders: `regex`, `unicode-data` tables, and statically linked C libraries. Link Tesseract dynamically (`tesseract-sys` supports this) to keep the binary distributable without embedding the full OCR runtime.

Avoid `lazy_static!` or `once_cell::sync::Lazy` initializations on the startup critical path for the CLI. Prefer computing lookup tables (`phf::Map`) at compile time.

---

## 10. HTTP Server Mode Performance

Use `axum` for the `pdftract serve` endpoint: ergonomic handler composition, `tower` middleware ecosystem, and `tokio` integration. Key performance considerations:

**Request-level memory bounding**: A naive implementation that buffers the full multipart body before parsing can OOM under concurrent large-PDF submissions. Stream the multipart body into a temporary file (via `axum::extract::Multipart` + `tokio::fs::File`), then open the temp file with `mmap` for parsing. This limits in-flight memory per request to roughly the working set of one PDF parse.

**Concurrency control**: Bound concurrent extraction jobs to `num_cpus::get()` with a `tokio::sync::Semaphore`. Requests beyond this limit queue with a configurable timeout. Without this, four simultaneous 500 MB PDFs can saturate RAM before any job completes.

**Connection keep-alive**: Enable HTTP/1.1 keep-alive (axum default) and consider HTTP/2 for high-throughput callers. HTTP/2 multiplexing allows the client to pipeline multiple extraction requests on one connection without head-of-line blocking.

**Response streaming**: Use `axum::response::Body::from_stream()` with a `tokio_stream::wrappers::ReceiverStream` to stream NDJSON output as pages complete, rather than buffering the full extraction result before sending the first byte. This reduces time-to-first-byte significantly for large documents.
