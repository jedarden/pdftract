# Parallel Extraction Architecture, Thread Safety, and Shared Cache Design

## Overview

PDF text extraction is an embarrassingly parallel problem at the page level, but the parallelism opportunities are unevenly distributed. Exploiting multi-core hardware correctly requires careful separation of read-only shared state from mutable per-page state, a coherent font cache strategy, and explicit handling of components—like Tesseract OCR—that are not thread-safe by default. This document describes the architecture pdftract must implement to achieve correct, bounded-memory parallel extraction.

## Parallelism Opportunities

PDF pages are structurally independent once the cross-reference table (xref) has been parsed. The xref maps object numbers to byte offsets within the file, and object resolution (reading the raw bytes for a given object number) requires only a shared reference to the file buffer and the xref index—both read-only after parsing. This means page-level parallelism is safe as soon as the document header, xref, and document catalog have been loaded.

Within a single page, additional parallelism exists but is narrower. Font resolution involves looking up font dictionaries by resource name, decoding ToUnicode CMaps, and loading advance-width arrays. If a page references ten distinct fonts and none has been cached yet, those ten font initializations can in principle proceed in parallel. In practice, the overhead of spawning subtasks per font initialization is only worthwhile for fonts with expensive CMap decoding; simpler fonts are faster to initialize sequentially than to dispatch.

Image decoding for OCR is fully parallel at the image level. Each inline or XObject image on a page is an independent stream; decompression and rasterization of one image does not touch any state needed by another. However, as discussed below, Tesseract instances introduce a thread-safety constraint that moves OCR work out of the inline page-worker path entirely.

What cannot be parallelized: cross-page operations. The document outline (bookmarks), page label sequences, article threads, form field trees, and named destinations all reference multiple pages by page index and must be assembled after all per-page extraction is complete. These are extracted from the document catalog as a single sequential pass once the parallel page workers have finished.

## Shared Read-Only State

The xref index, raw file buffer (or memory-mapped file view), decoded font data, embedded font binaries, and ICC profiles are logically read-only once the document is open. pdftract wraps these in `Arc<T>` and clones the Arc into each page worker. No locking is needed for reads because `Arc<T>` guarantees that the pointed-to data is immutable from the perspective of any thread holding a reference.

The document's cross-reference table is represented as `Arc<XrefTable>`, where `XrefTable` holds either a flat `Vec<XrefEntry>` for traditional xref sections or a compressed xref stream decoded at open time. Object bytes are resolved by indexing into the xref and reading the corresponding byte range from the shared file buffer, which is held as `Arc<Mmap>` (a read-only memory mapping). Because the OS page cache handles the actual I/O, concurrent reads to different byte ranges are safe and efficient.

ICC profile data and embedded font binaries (CFF, TrueType, Type1) are similarly wrapped in `Arc<Vec<u8>>` after first decode and shared across all page workers without additional synchronization.

## Mutable Per-Page State

Each page worker owns its own mutable state and never shares it. The PDF graphics state machine maintains a current transformation matrix, a graphics state stack (pushed and popped by `q`/`Q` operators), a text matrix (`Tm`), a text line matrix (`Tlm`), and the active text state parameters (`Tf`, `Tc`, `Tw`, `Th`, `Tl`, `Tmode`). None of these can be shared across pages because different pages may leave the graphics state in different configurations; more importantly, the state evolves as operators are interpreted, so sharing it would require locking on every operator.

The MCID-to-glyph map—which maps marked content identifiers to the sequence of glyphs extracted within each marked content span—is built fresh per page and discarded after the page result is serialized. Similarly, the list of extracted text runs, bounding boxes, and font references is per-page and allocated on the page worker's stack or heap without contention.

## Rayon-Based Page Parallelism

pdftract uses Rayon's work-stealing thread pool for page parallelism. The extraction pipeline calls `(0..page_count).into_par_iter()` and maps each index to a `PageResult`:

```rust
let page_results: Vec<PageResult> = (0..page_count)
    .into_par_iter()
    .map(|page_index| extract_page(page_index, &shared))
    .collect();
```

`shared` is an `Arc<DocumentShared>` that bundles the xref table, file mapping, font cache, and objstm cache. Rayon collects results in-order because `collect()` on a parallel iterator preserves index order; page workers may finish out of order internally, but the output `Vec<PageResult>` is always index-ordered. After `collect()` returns, the main thread extracts document-level structure from the catalog and assembles the final `ExtractionResult`.

## Font Cache Design

Fonts in PDF are referenced by resource name within a page's resource dictionary, which resolves to an indirect object reference—effectively a `u32` object number. The font cache is keyed by object number and shared across all page workers:

```rust
struct FontCache {
    entries: RwLock<HashMap<u32, Arc<FontData>>>,
}
```

When a page worker encounters a font reference, it first acquires a read lock and checks the map. On a hit, it clones the `Arc<FontData>` and releases the read lock immediately—no write contention for the common case. On a miss, it drops the read lock, decodes the font (ToUnicode CMap, advance-width array, font metrics, embedding type), then upgrades to a write lock to insert the result. Because two workers might race to initialize the same font, the inserting worker must check again under the write lock before inserting (check-then-act pattern).

`FontData` contains the pre-decoded Unicode mapping table, a compact `Vec<u16>` of advance widths indexed by glyph ID, and the font's bounding box and ascent/descent metrics. All fields are read-only once constructed, so the `Arc<FontData>` can be shared without further locking.

## OCR Task Queue

Tesseract 5.x creates one `TessBaseAPI` instance per logical document operation. Instances are not safe to share across threads. Rather than attempting to serialize Tesseract calls within the parallel page iterator, pdftract separates OCR into a second pass. During the first Rayon pass, pages identified as raster-only (no extractable text stream) produce a `PendingOcr` entry containing the rasterized image buffer. These entries are collected into a `Vec<PendingOcr>` after the first pass completes.

The OCR pass processes pending entries through a bounded thread pool where each worker thread owns exactly one `TessBaseAPI` instance stored in a `thread_local!` cell. The pool size is configurable (default: number of physical cores, not logical, since Tesseract is CPU-bound and hyperthreading yields diminishing returns for it). Each worker picks a `PendingOcr` from the queue, feeds the image to its thread-local Tesseract instance, and writes the resulting `OcrPageResult` to an output channel. This design avoids both contention on a shared Tesseract instance and the overhead of constructing and destroying an API handle per page.

## Object Stream Caching

PDF 1.5 and later allow objects to be packed into compressed object streams (`ObjStm`). A single `ObjStm` may contain objects referenced by dozens of pages. Decompressing and parsing an `ObjStm` is expensive—it involves flate decompression of potentially hundreds of kilobytes followed by PDF tokenization of the embedded objects.

pdftract caches decoded `ObjStm` content using `OnceCell<Arc<ObjStream>>` within the xref table entry for each object stream object. The first page worker to resolve an object whose generation entry points to an `ObjStm` decompresses the stream, parses all embedded objects into a `HashMap<u32, ParsedObject>`, and stores the result in the `OnceCell`. Subsequent workers call `get_or_try_init()`, which either returns the already-initialized value or blocks until the initializing thread completes. Because `OnceCell` from the `once_cell` crate is thread-safe and initializes exactly once regardless of concurrent callers, no additional locking is needed around `ObjStm` access.

## Memory Budget Across Parallel Pages

With N pages in flight simultaneously, peak memory is proportional to N times the per-page working set. For dense pages with large embedded images, the per-page working set can reach tens of megabytes. Unbounded parallelism on a large PDF would exhaust available memory before any results are emitted.

pdftract bounds this with a `tokio::sync::Semaphore` (or equivalent counting semaphore) sized to `max_parallel_pages`. Each page worker acquires one permit before allocating its working buffers and releases the permit when its `PageResult` has been produced. The default value for `max_parallel_pages` is derived at startup by dividing a configurable `memory_budget_mb` by an estimated per-page cost (measured empirically and stored as a build constant, adjustable via environment variable). Users processing memory-constrained environments can set `PDFTRACT_MAX_PARALLEL_PAGES=4` to override. The semaphore interacts naturally with Rayon's work-stealing: workers that cannot acquire a permit block and yield to Rayon, which picks up other ready work.

## Streaming Output with Parallel Extraction

NDJSON streaming mode emits one JSON object per page as soon as that page's result is available, but pages must be emitted in document order. Rayon's `collect()` waits for all pages before returning, which is incompatible with streaming.

For streaming mode, pdftract uses an ordered slot channel. Before launching the parallel iterator, an array of `N` `Option<PageResult>` slots is allocated (wrapped in a `Mutex<Vec<Slot>>`). Each page worker writes its result to `slots[page_index]`. A dedicated ordering thread runs concurrently, advancing a cursor from 0 upward: when `slots[cursor]` is `Some`, it takes the result, serializes it to stdout, and increments the cursor. This produces in-order output with minimal buffering—at most a few pages ahead of the cursor are held in memory at any time. The ordering thread blocks on a condvar when the next slot is not yet ready, and page workers notify the condvar after writing their slot.

## Error Isolation

A panic or extraction failure in one page worker must not abort other in-flight pages. pdftract wraps the body of each page worker in `std::panic::catch_unwind`:

```rust
let result = std::panic::catch_unwind(|| extract_page(page_index, &shared));
match result {
    Ok(page_result) => page_result,
    Err(_) => PageResult::error(page_index, "internal extraction panic"),
}
```

Non-panic errors (malformed content streams, unsupported filter types) are propagated as `Result` within `extract_page` and converted to `PageResult::Error` entries before returning. Failed pages emit a JSON error object in the output stream at their correct position—downstream consumers can detect and log them without breaking the overall parse. The document-level `extraction_quality` field in the final output reflects the fraction of pages that completed successfully, giving callers a machine-readable signal when partial extraction has occurred.
