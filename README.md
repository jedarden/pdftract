# pdftract

A PDF text extraction library designed to address the persistent shortcomings of existing tools.

## The problem

Current PDF text extractors — PyMuPDF, pdfplumber, pdfminer, Camelot, Tabula, marker, nougat — cover a lot of ground but share a set of well-known, largely unsolved failures:

- **Reading order is broken** for multi-column layouts, sidebars, footnotes, and mixed-layout pages. Most tools dump text in PDF operator order or naive top-to-bottom order.
- **Font encoding failures** produce silent garbage when PDFs use missing or incorrect `ToUnicode` CMaps, Type3 fonts, or symbol-font abuse for math.
- **Tagged PDFs are ignored.** PDF/UA and PDF/A documents contain a `StructTree` with explicit logical structure — headings, paragraphs, lists, tables, reading order — that almost no extractor reads.
- **No confidence or provenance.** Extracted text carries no signal about reliability, bounding box, or font metadata, making downstream filtering and validation impossible.
- **Hybrid documents are mishandled.** PDFs that mix vector pages and scanned pages are treated as one type throughout, degrading accuracy on both.
- **Flat output.** Nearly every tool returns a string or character stream. RAG pipelines, LLM preprocessing, and document QA need structured output — sections, headings, tables, figures — not a flat dump.

## What pdftract does differently

- Reads `StructTree` when present (PDF/UA, PDF/A) for near-perfect logical structure at zero cost
- Per-page hybrid routing: each page is independently classified and sent to the right pipeline (vector extraction, full OCR, or assisted OCR where vector text hints improve accuracy)
- Font encoding recovery via glyph fingerprinting to reconstruct correct Unicode mappings
- Layout region segmentation for reading order without requiring a full neural OCR pipeline
- Structured JSON output as the primary interface, with per-span bounding box and confidence score

## Architecture

Rust core with PyO3 Python bindings and a CLI binary. The binary can run as a microservice (`pdftract serve`) for container deployments — the container is just the binary in serve mode, not a separate product.

```
pdftract extract invoice.pdf          # stdout JSON
pdftract extract invoice.pdf --text   # plain text
pdftract serve --port 8080            # HTTP: POST /extract
```

## Status

Early development. See `docs/plan/` for the implementation roadmap and `docs/research/` for analysis of existing tools and approaches.
