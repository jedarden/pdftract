# pdftract

[![MSRV](https://img.shields.io/badge/MSRV-1.78-orange)](https://github.com/rust-lang/rust/releases/tag/1.78.0)

A PDF text extraction library that gets the hard parts right.

## What it does

- **Correct reading order** — layout regions are segmented and sequenced before text is emitted, handling multi-column pages, sidebars, footnotes, and mixed-layout documents without relying on PDF operator order
- **Font encoding recovery** — when `ToUnicode` CMaps are absent, wrong, or incomplete, pdftract works through a layered recovery pipeline: glyph name lookup via the Adobe Glyph List, font fingerprinting against known metrics and embedded checksums, and glyph outline shape matching
- **Structure tree extraction** — PDF/UA and PDF/A documents encode their logical structure (headings, paragraphs, lists, tables, reading order) in a `StructTree`; pdftract reads this directly when present, producing accurate semantic output at no extra cost
- **Per-page hybrid routing** — each page is independently classified and routed to the appropriate pipeline: vector text extraction, full OCR, or assisted OCR where vector hints improve raster accuracy
- **Structured output with provenance** — the primary output is JSON carrying per-span bounding boxes, font name, size, and confidence score alongside the extracted text, not a flat string dump

## Output

```json
{
  "pages": [
    {
      "page": 1,
      "blocks": [
        { "kind": "heading", "text": "Introduction", "bbox": [72, 680, 400, 700] },
        { "kind": "paragraph", "text": "...", "bbox": [72, 640, 540, 670] }
      ],
      "spans": [
        { "text": "Introduction", "bbox": [72, 680, 400, 700], "font": "Times-Bold", "size": 14.0, "confidence": 0.99 }
      ]
    }
  ],
  "metadata": { "title": "...", "author": "...", "page_count": 10 }
}
```

## Usage

```
pdftract extract invoice.pdf            # structured JSON to stdout
pdftract extract invoice.pdf --text     # plain text to stdout
pdftract extract invoice.pdf --output out.json
pdftract serve --port 8080              # HTTP service: POST /extract
```

## Architecture

Rust core with PyO3 Python bindings and a CLI binary. The same binary runs as a command-line tool or as an HTTP microservice — the container deployment is just `pdftract serve`.

See `docs/research/` for technical deep-dives into the PDF specification, font encoding, glyph Unicode recovery, and tagged PDF structure. See `docs/notes/` for SDK invocation examples in Python, Node.js, Go, Ruby, Java, Rust, and Bash.

## Status

Early development. See `docs/plan/` for the implementation roadmap.
