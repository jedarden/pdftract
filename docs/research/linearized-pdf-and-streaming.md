# Linearized PDF and Streaming Extraction

## Overview

Linearized PDFs (also called "web-optimized" PDFs) are files reorganized so that a conforming reader can display the first page after receiving only the first portion of the file. For `pdftract`, this structure provides two distinct opportunities: fast first-page extraction without reading the full file, and demand-driven page-by-page streaming when extracting from a remote URL.

---

## 1. What Linearization Is

A standard PDF places the cross-reference table (xref) at the end of the file. A reader must download the entire file, seek to the `startxref` offset, parse the xref, then locate any object. Linearization (PDF specification §F) reorders the file so that all objects needed to render the first page appear first, enabling a single HTTP request covering only the initial byte range to produce a renderable first page.

The file layout for a linearized PDF is:

1. **Linearization dictionary** — the first object in the file.
2. **First-page xref and trailer** — a small xref covering only first-page objects.
3. **First-page content objects** — page dictionary, content streams, fonts, and resources used on page 1.
4. **Primary hint stream** — page offset and shared object tables.
5. **Remaining pages and shared objects** — in page order.
6. **Main xref and trailer** — covers all objects, at the end of the file.

The linearization dictionary is a regular PDF dictionary object with the key `Linearized` set to the real number `1.0` (or `1` in some implementations). Its required entries are:

| Key | Meaning |
|-----|---------|
| `L` | Total file length in bytes |
| `H` | Array of two or four integers: `[offset, length]` of the primary hint stream, optionally followed by `[offset, length]` of the overflow hint stream |
| `O` | Object number of the first page's page object |
| `E` | Byte offset of the end of the first page section |
| `N` | Number of pages in the document |
| `T` | Byte offset of the main xref table (the one at the end of the file) |

Contrast with a non-linearized file: its only xref is at the end, and objects are stored in creation order with no guarantees about the first page appearing first.

---

## 2. Hint Streams

The primary hint stream (located at the byte range given by `H[0]` and `H[1]`) contains two sub-tables serialized in a compact binary format.

**Page offset hint table.** One entry per page, each containing:

- The byte offset of the first object belonging to that page.
- The total byte length of all objects on that page.

These offsets are relative to the start of the file and are sufficient to compute the exact `Range: bytes=N-M` request needed to fetch any specific page's raw object data without reading the xref for that page.

**Shared object hint table.** Objects shared across multiple pages (a common font embedded once but referenced from every page, a logo image) are listed separately. Each entry contains the object number and file offset of the shared object. When fetching an arbitrary page, the extractor must also fetch any shared objects that page references; the shared object hint table makes this a direct seek rather than an xref lookup.

For files larger than 2 GB, the hint stream offsets may overflow 32-bit integers. The spec accommodates this with an optional overflow hint stream at `H[2]`/`H[3]` that contains corrected 64-bit offsets for any entry that overflowed.

Parsing the hint stream requires reading a bitfield-packed binary structure: the spec defines a table of `nSharedObjects` entries, each encoded with a fixed bit width recorded at the top of the table. This is not length-prefixed text — it requires a bit-level reader that tracks the current bit position within the decompressed stream buffer.

---

## 3. First-Page Xref

Linearized files contain two xref structures:

- **First-page xref**: immediately follows the linearization dictionary. It is a conventional xref table (or cross-reference stream for PDF 1.5+) covering only the objects needed for the first page. Its trailer has a `Size` entry equal to the count of first-page objects.
- **Main xref**: at the end of the file, covering all objects. Its trailer contains the standard `Size`, `Root`, `Info`, and optional `Prev` (for incremental updates) entries.

Parsing strategy for `pdftract`:

1. Read the first 1 KB (or up to `E`, whichever is smaller) to locate the linearization dictionary.
2. Validate the dictionary (see §5).
3. Parse the first-page xref. This xref is sufficient to extract page 1 without any further I/O.
4. Defer parsing the main xref until a non-first-page object is requested.

This lazy strategy means that for a request extracting only the first page (common in preview generation), the main xref — which may be many megabytes into a large file — is never read.

---

## 4. Streaming Extraction for HTTP Delivery

Consider extracting text from a 500-page PDF hosted at a remote URL. Waiting for a full download before beginning extraction is wasteful in both latency and peak memory.

**Protocol.** HTTP/1.1 and HTTP/2 both support `Range: bytes=N-M` requests. A `HEAD` request first confirms `Accept-Ranges: bytes` and retrieves `Content-Length` (needed to validate the `L` key and compute the total file size).

**Fetch sequence for a linearized file:**

1. Fetch bytes `0` through `E` (the end-of-first-page offset from the linearization dict). This yields the linearization dictionary, first-page xref, first-page objects, and the hint stream.
2. Parse and emit page 1 immediately.
3. For each subsequent page `i`, compute the byte range from the page offset hint table: `[page_offset[i], page_offset[i] + page_length[i])`. Fetch that range plus any referenced shared object ranges.
4. Parse and emit page `i`.

Using `reqwest` with range requests:

```rust
let response = client
    .get(&url)
    .header(RANGE, format!("bytes={}-{}", start, end))
    .send()
    .await?;
let bytes = response.bytes().await?;
```

Each range fetch is independent. For pages whose shared object dependencies are already cached from a prior fetch, no additional request is needed. A local `HashMap<ObjNum, Vec<u8>>` cache keyed by object number avoids re-fetching shared fonts and images.

For non-linearized remote files, fall back to: fetch the last 1 KB to read `startxref`, fetch the main xref, then fetch individual pages using xref offsets.

---

## 5. Detecting and Validating Linearization

**Detection.** The linearization dictionary must be the first indirect object in the file and must carry `Linearized 1.0` (or `1`). In practice, many tools emit linearized-looking files that fail validation. Check:

1. The first object in the file is a dictionary with the `Linearized` key.
2. `L` matches `file.metadata()?.len()` exactly. If the file length does not match, the linearization is stale.
3. `H`, `O`, `E`, `N`, and `T` are all present and within file bounds.

**Invalid linearization.** Incremental updates (see §6) are the most common cause. If `L` does not match the actual file size, the hint stream offsets are unreliable. Fall back to standard xref parsing: seek to `startxref` at the end of the file, parse the main xref chain, and process the document normally. Log a structured warning at the `tracing::debug!` level.

**False positives.** Some non-linearized PDFs happen to have their first object numbered 1 and start with a dictionary. Confirm the `Linearized` key is present and the value is a number equal to 1 before treating the file as linearized.

---

## 6. Incremental Update Interaction

When a linearized PDF is updated incrementally — a common operation for annotation tools, form fillers, and digital-signature workflows — the update is appended at the end of the file. This invalidates the `L` key (file is now longer) and renders all hint stream offsets stale for any updated object.

The hint streams still reflect the original layout. For first-page extraction on a file that has not had its first page modified, the hint stream may still be usable. However, this is difficult to determine without reading the full incremental update delta.

**Safe strategy:**

- Use the linearization structure only for detecting that the first-page xref is available; read and extract page 1 from the first-page xref as long as `L` is consistent with the original (pre-update) length.
- For any non-first-page content, or any file where `L` mismatches, follow the full xref chain from the end of the file. The last trailer's `Prev` pointer chains back through all prior xref sections. The last xref in the chain is authoritative for all object locations including updates.
- Never trust hint stream page offsets for updated files.

---

## 7. Memory-Efficient Streaming Output

For large documents, accumulating the full extraction result in memory before writing output is not viable. `pdftract` supports NDJSON (newline-delimited JSON) streaming output: each page's `PageExtraction` is serialized and written to stdout before the next page is fetched or parsed.

```rust
let stdout = std::io::stdout();
let mut writer = BufWriter::new(stdout.lock());

for page_result in extractor.pages() {
    let page = page_result?;
    serde_json::to_writer(&mut writer, &page)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
}
```

`BufWriter` amortizes the flush cost across many small writes. The `flush()` after each page ensures the consumer receives complete objects as they are produced rather than waiting for the buffer to fill.

**Tradeoff.** Streaming output precludes any feature requiring a full-document pass before emitting output: assembling the document outline, resolving cross-page table structures, or applying page labels to page numbers. These features require either a pre-pass (§8) or a second pass over already-extracted data. If neither is acceptable, emit those features at the end of the stream as a final summary object.

---

## 8. Pre-Pass for Document-Level Features

When streaming output is requested, a lightweight pre-pass fetches the document catalog and a small set of document-level structures before per-page streaming begins:

- **Document catalog**: contains `Outlines`, `PageLabels`, `AcroForm`, `Metadata` (XMP), and `MarkInfo` references. The catalog object is listed in the main trailer's `Root` entry — fetch it with a single seek using the main xref.
- **Outline tree**: the `Outlines` dictionary with its full `First`/`Next`/`Last` child chain. For typical documents this is a few dozen objects; fetch them all upfront.
- **Page labels**: a small number tree in `PageLabels`; fetch and resolve once.
- **XMP metadata**: a single stream object referenced from `Metadata`.

For linearized files, the hint stream's shared object table may include the catalog's dependents. For non-linearized files, these objects are clustered near the main xref at the end of the file and can be fetched in a single range request covering the last 64 KB.

After the pre-pass, emit one NDJSON line containing a `DocumentMetadata` object, then begin per-page streaming.

---

## 9. Partial File Extraction

For truncated downloads or interrupted network reads, `pdftract` extracts all pages whose object byte ranges fall within the available bytes.

Detection using the hint stream page offset table is direct: for page `i`, if `page_offset[i] + page_length[i] <= available_bytes`, the page is extractable. Iterate until the condition fails.

Output metadata for partial extractions:

```json
{"type": "metadata", "partial": true, "pages_extracted": 12, "pages_total": 500}
```

For linearized files, page 1 is always available from any file at least `E` bytes long. A file truncated to its first few kilobytes still yields the first page. For non-linearized files, page availability depends entirely on xref accessibility; if `startxref` is missing (file truncated before the end), attempt to reconstruct the xref by scanning for `obj` keywords — but this falls under malformed PDF recovery territory, not linearization handling.

---

## 10. Implementation: Lazy Page Iterator

The Rust API for streaming extraction exposes a lazy iterator:

```rust
pub struct PdfExtractor { /* ... */ }

impl PdfExtractor {
    pub fn pages(&mut self) -> PageIter<'_>;
}

pub struct PageIter<'a> {
    extractor: &'a mut PdfExtractor,
    page_index: usize,
    xref: ParsedXref,
    graphics_state: GraphicsState,
}

impl<'a> Iterator for PageIter<'a> {
    type Item = Result<PageExtraction>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.page_index >= self.extractor.page_count() {
            return None;
        }
        self.graphics_state.reset();
        let result = self.extractor.extract_page(self.page_index, &self.xref, &self.graphics_state);
        self.page_index += 1;
        Some(result)
    }
}
```

Key design points:

- `graphics_state.reset()` at each page boundary discards font state and CTM from the prior page; graphics state does not persist across PDF pages unless explicitly inherited via resource inheritance.
- For linearized files, `extract_page` uses the hint stream to compute the byte range for each page, issuing a range fetch (or a seek on a local file) on demand.
- For standard files, `extract_page` uses xref offsets directly.
- The iterator holds a `&mut PdfExtractor` rather than `Arc<Mutex<...>>` to avoid lock contention in the single-threaded path.

**Parallel extraction.** `rayon`'s `par_bridge()` converts any `Iterator` into a parallel iterator with preserved output order:

```rust
use rayon::iter::{ParallelBridge, ParallelIterator};

extractor.pages()
    .par_bridge()
    .map(|page_result| page_result.map(render_page))
    .collect::<Result<Vec<_>>>()?;
```

`par_bridge()` preserves ordering by numbering tasks internally. For I/O-bound extraction (remote range fetches), parallelism here is limited by HTTP connection reuse; prefer async concurrency with `tokio::join!` over rayon for the HTTP case. For CPU-bound extraction (complex content streams from a local file), rayon's thread pool is appropriate.

---

## Summary

Linearized PDFs expose byte-level structure that enables three extraction optimizations: first-page extraction from the initial byte range alone, demand-driven page fetching via hint stream offsets, and partial-file extraction from truncated downloads. The critical implementation discipline is validating the `L` key before trusting any hint offset, falling back to main xref parsing when linearization is stale, and always treating the last xref in the incremental update chain as authoritative. The lazy `PageIter` API makes these optimizations composable with NDJSON streaming output and optional document-level pre-pass metadata.
