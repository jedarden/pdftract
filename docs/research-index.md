# pdftract Research Index

**82 documents** covering PDF internals, font encoding, OCR, layout reconstruction, specialized document types, security, output schema, and multilingual extraction.

This is a navigable reading guide, not a summary. Use it to find the right document when implementing a specific feature. All links are relative to this file's location (`docs/`).

---

## Start Here

Read these six documents first, in order, before touching any other research:

1. **[research/extraction-pipeline-overview.md](research/extraction-pipeline-overview.md)** — End-to-end architectural blueprint: all 9 pipeline stages, decision points, and data transformations. The canonical integration reference that ties every other document together.
2. **[research/pdf-specification.md](research/pdf-specification.md)** — ISO 32000-1/2 implementation reference: file structure, xref tables, object model, content streams, and encoding. The foundation for Phase 1.
3. **[research/pdf-fonts-and-encoding.md](research/pdf-fonts-and-encoding.md)** — Every font type in PDF, character code → Unicode resolution, and the four-level fallback chain. Essential before any font work.
4. **[research/content-stream-operators.md](research/content-stream-operators.md)** — Complete operator reference for text extraction: text state operators, positioning, rendering modes, and their effect on glyph output.
5. **[research/extraction-output-schema.md](research/extraction-output-schema.md)** — Stable v1.0 JSON schema: document, page, span, block, annotation, and form field structures. Read this before writing any serialization code.
6. **[plan/implementation-plan.md](plan/implementation-plan.md)** — Seven-phase build plan with crate dependencies, critical tests, and milestone targets. The execution roadmap.

---

## Reading Path for Implementation

### Phase 1: Core PDF Parser

Build the lexer, object parser, xref resolution, document model, and stream decoder.

| Document | Why |
|---|---|
| [research/pdf-specification.md](research/pdf-specification.md) | File structure, xref tables, object streams, linearized layout |
| [research/pdf-object-model-and-data-types.md](research/pdf-object-model-and-data-types.md) | The eight PDF object types, reference semantics, generation numbers |
| [research/xref-table-parsing-and-object-lookup.md](research/xref-table-parsing-and-object-lookup.md) | Traditional xref, xref streams, hybrid files, incremental update chains |
| [research/malformed-pdf-repair-and-recovery.md](research/malformed-pdf-repair-and-recovery.md) | Forward scan fallback, truncated file recovery, error taxonomy |
| [research/document-catalog-and-structure.md](research/document-catalog-and-structure.md) | Catalog keys, page tree traversal, inherited attributes |
| [research/page-geometry-and-document-structure.md](research/page-geometry-and-document-structure.md) | MediaBox/CropBox/BleedBox, rotation, coordinate systems |
| [research/image-compression-and-filter-decoding.md](research/image-compression-and-filter-decoding.md) | FlateDecode, LZW, ASCII85, RunLength, DCT, JBIG2, JPX filter chain |
| [research/pdf-encryption-and-security.md](research/pdf-encryption-and-security.md) | Standard handler, RC4/AES decryption, password attempt sequence |
| [research/error-handling-and-robustness.md](research/error-handling-and-robustness.md) | Recoverable error model, diagnostic codes, graceful degradation |
| [research/adversarial-inputs-and-parser-security.md](research/adversarial-inputs-and-parser-security.md) | Decompression bombs, circular references, resource exhaustion limits |
| [research/linearized-pdf-and-streaming.md](research/linearized-pdf-and-streaming.md) | Two-xref-table layout, hint streams, fast-web-view parsing |
| [research/incremental-updates-and-versioning.md](research/incremental-updates-and-versioning.md) | Append-only update model, revision history, redaction detection |

### Phase 2: Font and Encoding Pipeline

Map every character code to a Unicode scalar value with a confidence score.

| Document | Why |
|---|---|
| [research/pdf-fonts-and-encoding.md](research/pdf-fonts-and-encoding.md) | All font types, encoding vectors, AGL lookup, four-level fallback |
| [research/cmap-format-and-cid-encoding.md](research/cmap-format-and-cid-encoding.md) | ToUnicode CMap syntax: bfchar, bfrange, usecmap, UTF-16BE sequences |
| [research/font-descriptor-and-metrics.md](research/font-descriptor-and-metrics.md) | FontDescriptor keys, width arrays, hmtx metrics, descriptor flags |
| [research/font-subsetting-and-extraction.md](research/font-subsetting-and-extraction.md) | Subset naming convention, glyph table gaps, metric inference |
| [research/glyph-recognition-and-unicode-recovery.md](research/glyph-recognition-and-unicode-recovery.md) | Shape-hash database, perceptual hashing, Level 4 fallback |
| [research/type3-font-extraction.md](research/type3-font-extraction.md) | CharProcs content streams, per-glyph rasterization, shape recognition |
| [research/cjk-and-asian-script-encoding.md](research/cjk-and-asian-script-encoding.md) | CIDFont encoding, predefined CMaps, Shift-JIS/GB18030/Big5/EUC-KR |
| [research/resource-dictionary-and-inheritance.md](research/resource-dictionary-and-inheritance.md) | Font namespace resolution, inherited resource merging, XObject refs |

### Phase 3: Content Stream Processing

Execute content stream operators to produce a raw glyph list with positions.

| Document | Why |
|---|---|
| [research/content-stream-operators.md](research/content-stream-operators.md) | Full operator reference: Tj, TJ, Td, Tm, Tf, Tr, BT/ET, and all text operators |
| [research/graphics-state-tracking.md](research/graphics-state-tracking.md) | CTM, text matrix, q/Q stack, Tr rendering mode, color state |
| [research/text-positioning-and-font-metrics.md](research/text-positioning-and-font-metrics.md) | Text matrix accumulation, leading, Td/TD/T* semantics, rise |
| [research/content-stream-concatenation.md](research/content-stream-concatenation.md) | Multi-stream pages, /Length mismatch handling, resource namespace scoping |
| [research/optional-content-groups.md](research/optional-content-groups.md) | OCG layer state, BDC/EMC marked content sequences, visibility filtering |
| [research/invisible-and-hidden-text.md](research/invisible-and-hidden-text.md) | Tr=3 invisible text, OCR layer patterns, include_invisible_text flag |
| [research/stroke-and-outlined-text.md](research/stroke-and-outlined-text.md) | Rendering modes 1–7, stroke-only glyphs, clipping path text |
| [research/shading-pattern-and-text-visibility.md](research/shading-pattern-and-text-visibility.md) | Luminance estimation for text-on-background visibility filtering |
| [research/word-boundary-reconstruction.md](research/word-boundary-reconstruction.md) | Inter-glyph gap thresholds, TJ kern detection, synthetic space insertion |

### Phase 4: Text Assembly and Layout

Transform raw glyph lists into structured blocks in reading order.

| Document | Why |
|---|---|
| [research/span-merging-and-text-run-assembly.md](research/span-merging-and-text-run-assembly.md) | Span boundary rules, run assembly, line merging, block formation |
| [research/complex-layout-reading-order.md](research/complex-layout-reading-order.md) | XY-cut algorithm, multi-column detection, sidebar/caption disambiguation |
| [research/tagged-pdf-structure-and-reading-order.md](research/tagged-pdf-structure-and-reading-order.md) | Structure tree walk, MCID-to-span mapping, ActualText override |
| [research/document-classification-and-zone-labeling.md](research/document-classification-and-zone-labeling.md) | Zone classification: body/header/footer/caption/margin heuristics |
| [research/watermark-and-background-separation.md](research/watermark-and-background-separation.md) | Watermark detection, repeated-pattern suppression, z-order analysis |
| [research/semantic-text-reconstruction.md](research/semantic-text-reconstruction.md) | Hyphen removal, soft-hyphen handling, ligature expansion, line joining |
| [research/post-extraction-normalization.md](research/post-extraction-normalization.md) | NFC normalization, whitespace cleanup, paragraph boundary detection |
| [research/confidence-scoring-and-aggregation.md](research/confidence-scoring-and-aggregation.md) | Per-glyph confidence, span aggregation, readability score computation |
| [research/text-readability-validation.md](research/text-readability-validation.md) | Printable-character ratio, dictionary word rate, readability threshold |
| [research/page-labels-and-outline-extraction.md](research/page-labels-and-outline-extraction.md) | PageLabels number tree, outline bookmark walk, named destinations |

### Phase 5: OCR Integration

Extract text from scanned pages; improve broken-vector pages via Tesseract.

| Document | Why |
|---|---|
| [research/scanned-vs-vector-page-classification.md](research/scanned-vs-vector-page-classification.md) | Classification signals, PageClass enum, confidence scoring per signal |
| [research/raster-ocr-pipeline.md](research/raster-ocr-pipeline.md) | 300 DPI render, Sauvola binarization, deskew, Tesseract HOCR integration |
| [research/post-ocr-text-correction.md](research/post-ocr-text-correction.md) | Systematic OCR errors, dictionary-based correction, confidence re-scoring |
| [research/image-and-figure-extraction.md](research/image-and-figure-extraction.md) | Image XObject identification, inline images, figure region detection |
| [research/historical-and-degraded-document-extraction.md](research/historical-and-degraded-document-extraction.md) | Microfilm, low-quality scan preprocessing, multi-pass OCR strategies |

### Phase 6: Output and API

Full JSON schema, PyO3 bindings, HTTP serve mode, NDJSON streaming.

| Document | Why |
|---|---|
| [research/extraction-output-schema.md](research/extraction-output-schema.md) | Complete v1.0 schema: all fields, types, and serialization constraints |
| [research/performance-and-streaming-architecture.md](research/performance-and-streaming-architecture.md) | mmap I/O, rayon page parallelism, BufWriter NDJSON, LRU object cache |
| [research/hyperlinks-and-named-destinations.md](research/hyperlinks-and-named-destinations.md) | URI annotations, GoTo actions, cross-document links, named dest resolution |
| [research/xmp-and-document-metadata.md](research/xmp-and-document-metadata.md) | XMP RDF/XML parsing, /Info dict fallback, Dublin Core fields, conflict resolution |
| [research/chunking-for-llm-consumption.md](research/chunking-for-llm-consumption.md) | Chunk size strategy, block-boundary splitting, overlap, token estimation |
| [research/benchmark-and-test-methodology.md](research/benchmark-and-test-methodology.md) | Test corpus design, accuracy metrics, performance benchmarks, regression suite |

### Phase 7: Advanced Features

StructTree exploitation, table detection, AcroForm/XFA, attachments, signatures.

| Document | Why |
|---|---|
| [research/accessibility-and-tagged-pdf-deep-dive.md](research/accessibility-and-tagged-pdf-deep-dive.md) | StructTree walk, role mapping, artifact suppression, ActualText semantics |
| [research/tagged-pdf-structure-and-reading-order.md](research/tagged-pdf-structure-and-reading-order.md) | ParentTree, MCID resolution, Suspects flag fallback |
| [research/table-structure-reconstruction.md](research/table-structure-reconstruction.md) | Ruling-line detection, borderless heuristics, cell assignment, merged cells |
| [research/form-fields-and-annotations.md](research/form-fields-and-annotations.md) | AcroForm field walk, XFA extraction, annotation types and values |
| [research/digital-signatures-and-certification.md](research/digital-signatures-and-certification.md) | Sig field metadata, ByteRange, SubFilter, validation_status reporting |
| [research/embedded-files-and-portfolios.md](research/embedded-files-and-portfolios.md) | EmbeddedFiles name tree, portfolio navigation, attachment metadata |
| [research/pdf-portfolio-and-attachments.md](research/pdf-portfolio-and-attachments.md) | PDF Portfolio collections, navigator schema, attachment content access |
| [research/article-threads-and-reading-order.md](research/article-threads-and-reading-order.md) | /Threads bead chains, multi-page article flow, reading order override |

---

## Document Categories

### PDF File Format and Parsing

Core specification, file structure, object model, and xref resolution.

- **[research/pdf-specification.md](research/pdf-specification.md)** — ISO 32000-1/2 implementation reference covering file structure, xref tables, object streams, and linearized layout
  - File header, body, xref, trailer structure
  - Xref streams (PDF 1.5+), object streams, incremental updates
  - Content stream grammar and operator categorization

- **[research/pdf-object-model-and-data-types.md](research/pdf-object-model-and-data-types.md)** — The eight fundamental PDF object types with serialization details and reference semantics
  - Boolean, integer, real, string, name, array, dictionary, stream
  - Indirect object references, generation numbers, object identity

- **[research/xref-table-parsing-and-object-lookup.md](research/xref-table-parsing-and-object-lookup.md)** — Cross-reference parsing strategies: traditional table, xref streams, hybrid files, and incremental chains
  - Traditional 20-byte xref entry format and subsection merging
  - Type-0/1/2 xref stream entries, `/W` field widths
  - `/Prev` chain traversal for incremental updates

- **[research/document-catalog-and-structure.md](research/document-catalog-and-structure.md)** — Document catalog keys, page tree traversal, and inherited attribute resolution
  - `/Root`, `/Pages`, `/Outlines`, `/AcroForm`, `/MarkInfo`, `/OCProperties`
  - Page tree flattening, per-key inheritance walk, attribute override rules

- **[research/page-geometry-and-document-structure.md](research/page-geometry-and-document-structure.md)** — Page boxes, coordinate systems, rotation, and media/crop/bleed/trim/art box semantics
  - MediaBox, CropBox, BleedBox, TrimBox, ArtBox inheritance
  - User space to device space transformation, rotation matrix application

- **[research/linearized-pdf-and-streaming.md](research/linearized-pdf-and-streaming.md)** — Linearized ("fast web view") PDF layout: two-xref structure, hint streams, and first-page parsing
  - Linearization dictionary keys and their parsing implications
  - Hint table decoding, page offset table, shared object table

- **[research/incremental-updates-and-versioning.md](research/incremental-updates-and-versioning.md)** — Non-destructive append-only PDF modification: revision history, object shadowing, and redaction forensics
  - Incremental save mechanics, object number reuse across revisions
  - Detecting "soft redaction" where content is hidden but not deleted

- **[research/malformed-pdf-repair-and-recovery.md](research/malformed-pdf-repair-and-recovery.md)** — Forward scan fallback, truncation recovery, and the full taxonomy of real-world PDF corruption
  - Forward object scan from `obj`/`endobj` markers when xref fails
  - Per-corruption diagnostic codes and recovery strategies

- **[research/pdf-generator-quirks.md](research/pdf-generator-quirks.md)** — Per-generator fingerprinting and known deviations from spec for Word, LibreOffice, Chrome, LaTeX, and others
  - Generator detection via `/Producer` field patterns
  - Quirk-specific workarounds keyed to generator fingerprint

- **[research/resource-dictionary-and-inheritance.md](research/resource-dictionary-and-inheritance.md)** — Font, XObject, ExtGState, and pattern namespace resolution with inheritance from ancestor page nodes
  - Multi-level resource merging, last-write-wins semantics at page level
  - Namespace isolation between content streams and Form XObjects

- **[research/image-compression-and-filter-decoding.md](research/image-compression-and-filter-decoding.md)** — All PDF stream filters: FlateDecode predictors, LZW, ASCII85, RunLength, DCT, JBIG2, JPX, CCITT
  - Filter pipeline chaining, `/DecodeParms` alignment
  - Partial decode recovery on zlib truncation errors

---

### Font and Encoding

Everything needed to map a character code to a Unicode codepoint.

- **[research/pdf-fonts-and-encoding.md](research/pdf-fonts-and-encoding.md)** — Complete font type reference and the four-level Unicode resolution fallback chain
  - Type1, TrueType, Type0/CID, Type3, OpenType font loading strategies
  - ToUnicode → AGL → fingerprint → shape recognition fallback

- **[research/cmap-format-and-cid-encoding.md](research/cmap-format-and-cid-encoding.md)** — ToUnicode CMap program syntax and CID encoding for composite fonts
  - `beginbfchar`/`beginbfrange`, `usecmap`, UTF-16BE ligature expansion
  - Predefined CMap names for CJK scripts, CID-to-GID mapping

- **[research/font-descriptor-and-metrics.md](research/font-descriptor-and-metrics.md)** — FontDescriptor dictionary keys, width arrays, hmtx table access, and descriptor flags
  - `/Widths`, `/FirstChar`, `/LastChar`, `/MissingWidth` for Type1/TrueType
  - `/DW`, `/W` sparse width encoding for CIDFonts

- **[research/font-subsetting-and-extraction.md](research/font-subsetting-and-extraction.md)** — Subset font naming convention, glyph table gaps, and metrics inference for missing glyphs
  - Six-uppercase-letter prefix stripping for Standard 14 lookup
  - Identifying and handling subsetting-induced ToUnicode gaps

- **[research/glyph-recognition-and-unicode-recovery.md](research/glyph-recognition-and-unicode-recovery.md)** — Shape-hash database construction and perceptual hashing for Level 4 Unicode recovery
  - 32×32 bitmap rendering, perceptual hash lookup database
  - Shape-match confidence scoring and known-font fingerprint cache

- **[research/type3-font-extraction.md](research/type3-font-extraction.md)** — Type 3 fonts: glyph shapes as content stream fragments requiring per-glyph rasterization
  - `/CharProcs` dictionary parsing, glyph content stream execution
  - Color/grayscale Type 3 glyph rendering for shape recognition

- **[research/cjk-and-asian-script-encoding.md](research/cjk-and-asian-script-encoding.md)** — CJK CIDFont encoding, predefined CMaps, and multi-byte character code parsing
  - Shift-JIS, GB18030, Big5, EUC-KR codepage decoding via `encoding_rs`
  - Vertical writing mode detection and glyph substitution

- **[research/opentype-math-and-formula-extraction.md](research/opentype-math-and-formula-extraction.md)** — OpenType MATH table layout and text-extraction strategy for mathematical formulas
  - MathVariants, MathKern, script/superscript glyph positioning
  - Linearized formula extraction vs. MathML reconstruction tradeoffs

---

### Content Stream Processing

Executing the PDF painting model to produce glyphs with positions.

- **[research/content-stream-operators.md](research/content-stream-operators.md)** — Full PDF operator reference for text extraction: every text, positioning, and state operator
  - Tj, TJ, ', " operators; Td, TD, Tm, T* positioning
  - BT/ET block semantics, Tf font selection, Tr rendering mode

- **[research/graphics-state-tracking.md](research/graphics-state-tracking.md)** — Complete graphics state machine: CTM, text matrix stack, color state, and rendering mode
  - q/Q push/pop, CTM concatenation via `cm` operator
  - Tr modes 0–7 and their visibility implications for extraction

- **[research/text-positioning-and-font-metrics.md](research/text-positioning-and-font-metrics.md)** — Text state scalar accumulation, leading, Td/TD/T* semantics, and rise
  - Text matrix vs. text line matrix distinction
  - Character spacing (Tc), word spacing (Tw), horizontal scaling (Tz)

- **[research/content-stream-concatenation.md](research/content-stream-concatenation.md)** — Multi-stream page assembly, `/Length` mismatch handling, and resource namespace scoping
  - Pages with array `/Contents`, stream boundary handling
  - Form XObject execution and resource inheritance within sub-streams

- **[research/optional-content-groups.md](research/optional-content-groups.md)** — OCG layer state tracking, BDC/EMC marked content, and visibility-based glyph suppression
  - `/OCProperties` catalog entry, OCG on/off state resolution
  - OCMD (optional content membership dictionary) logic operators

- **[research/invisible-and-hidden-text.md](research/invisible-and-hidden-text.md)** — Tr=3 invisible text, PDF/A OCR layer patterns, and the `include_invisible_text` flag
  - Scanned-PDF OCR text layer architecture (visible image + hidden text)
  - White-on-white and zero-font-size hidden text detection

- **[research/stroke-and-outlined-text.md](research/stroke-and-outlined-text.md)** — Text rendering modes 1–7, stroke-only glyphs, and clipping path text handling
  - Mode 1 (stroke), mode 2 (fill+stroke), mode 4 (invisible clip)
  - Outlined text in logos and headings where fill is absent

- **[research/word-boundary-reconstruction.md](research/word-boundary-reconstruction.md)** — Inter-glyph gap thresholds, TJ kern detection, and synthetic space character insertion
  - TeX/LaTeX character-per-glyph patterns, missing inter-word spaces
  - Horizontal gap normalized by font size as word-boundary signal

- **[research/shading-pattern-and-text-visibility.md](research/shading-pattern-and-text-visibility.md)** — Color space luminance estimation for text-on-background visibility and watermark filtering
  - ICC profile color space normalization to approximate luminance
  - Pattern and shading fills as background suppression candidates

---

### Text Assembly and Layout Reconstruction

Transforming raw glyph lists into ordered, structured text.

- **[research/span-merging-and-text-run-assembly.md](research/span-merging-and-text-run-assembly.md)** — Span boundary detection, text run assembly, line merging, and block formation pipeline
  - Font/size/color/mode change triggers for span splits
  - Ascending y-position sort, baseline alignment for line grouping

- **[research/complex-layout-reading-order.md](research/complex-layout-reading-order.md)** — XY-cut recursive page partitioning for multi-column, sidebar, and mixed-layout documents
  - Whitespace gap detection as column separator, minimum gap thresholds
  - Caption-to-figure association, margin note classification

- **[research/tagged-pdf-structure-and-reading-order.md](research/tagged-pdf-structure-and-reading-order.md)** — Structure tree as authoritative reading order source for tagged documents
  - StructElem type mapping to block kinds, ParentTree MCID lookup
  - Suspects flag validation and XY-cut fallback conditions

- **[research/document-classification-and-zone-labeling.md](research/document-classification-and-zone-labeling.md)** — Page zone classification: body text, header, footer, caption, margin, and running head
  - Spatial heuristics: y-position thresholds, font size ratios, repetition
  - Zone label influence on reading order and block kind assignment

- **[research/watermark-and-background-separation.md](research/watermark-and-background-separation.md)** — Watermark detection, repeated-pattern suppression, and z-order layering analysis
  - Low-opacity text, diagonal text, large-font centered text as watermark signals
  - Suppression vs. flagging strategies, `is_artifact` span flag

- **[research/semantic-text-reconstruction.md](research/semantic-text-reconstruction.md)** — Hyphen removal, soft-hyphen handling, ligature expansion, and line-end join heuristics
  - End-of-line hyphen detection and word reconstitution
  - Ligature Unicode expansion (fi, fl, ffi, ffl, st)

- **[research/post-extraction-normalization.md](research/post-extraction-normalization.md)** — NFC normalization, whitespace cleanup, and paragraph boundary detection
  - Unicode combining character normalization pipeline
  - Trailing/leading whitespace removal, control character stripping

- **[research/confidence-scoring-and-aggregation.md](research/confidence-scoring-and-aggregation.md)** — Per-glyph confidence scoring, span aggregation, and page-level readability score computation
  - Source-weighted confidence: to_unicode=1.0, agl=0.9, fingerprint=0.85, shape_match=0.7
  - Span minimum confidence, page aggregate, extraction_quality rollup

- **[research/text-readability-validation.md](research/text-readability-validation.md)** — Printable-character ratio, dictionary word rate, and readability threshold for OCR fallback triggering
  - Character validity checks: printable Unicode, non-sentinel codepoints
  - Readability score thresholds for `ocr_fallback_threshold` comparison

- **[research/page-labels-and-outline-extraction.md](research/page-labels-and-outline-extraction.md)** — PageLabels number tree parsing, outline bookmark walk, and named destination resolution
  - Roman/Arabic/alphabetic label prefix and style encoding
  - `/Outlines` recursive walk, `/Dests` name tree lookup

- **[research/table-structure-reconstruction.md](research/table-structure-reconstruction.md)** — Ruling-line grid detection, borderless column alignment, cell assignment, and merged cell inference
  - Path segment clustering, intersection point detection, grid construction
  - Colspan/rowspan inference from missing interior edges

---

### OCR and Image Processing

Handling scanned and raster pages.

- **[research/scanned-vs-vector-page-classification.md](research/scanned-vs-vector-page-classification.md)** — Classification signals and the PageClass enum (Vector/Scanned/Hybrid/BrokenVector)
  - Image coverage fraction threshold, character validity rate signals
  - 8×8 grid cell per-region classification for Hybrid detection

- **[research/raster-ocr-pipeline.md](research/raster-ocr-pipeline.md)** — Full OCR pipeline: 300 DPI rendering, Sauvola binarization, Hough deskew, Tesseract HOCR
  - leptonica-plumbing preprocessing, HOCR confidence parsing
  - BrokenVector assisted-OCR mode: bounding box seeding from vector positions

- **[research/post-ocr-text-correction.md](research/post-ocr-text-correction.md)** — Systematic OCR error patterns, dictionary-based correction, and post-OCR confidence re-scoring
  - Common substitution errors (l/1/I, O/0, rn/m), context correction
  - Language model probability scoring for candidate correction ranking

- **[research/image-and-figure-extraction.md](research/image-and-figure-extraction.md)** — Image XObject identification, inline image parsing, and figure region demarcation
  - XObject type detection, image placement matrix, DPI computation
  - Figure caption association and alt-text extraction from StructTree

- **[research/historical-and-degraded-document-extraction.md](research/historical-and-degraded-document-extraction.md)** — Preprocessing for microfilm, low-quality photocopies, and physically degraded originals
  - Multi-pass binarization strategies, fold/crease artifact suppression
  - Tesseract PSM mode selection for degraded layout recognition

- **[research/color-management-and-icc-profiles.md](research/color-management-and-icc-profiles.md)** — ICC profile color spaces and luminance estimation for text visibility determination
  - CMYK/Lab/ICCBased color space normalization to approximate RGB
  - Spot color handling, DeviceGray/DeviceRGB identity conversion

---

### Specialized Document Types

Documents with structural patterns that require targeted handling.

- **[research/latex-and-scientific-pdf-patterns.md](research/latex-and-scientific-pdf-patterns.md)** — LaTeX toolchain patterns: pdflatex, XeLaTeX, LuaLaTeX, and their encoding behaviors
  - Type1/OTF font stacks, microtype spacing, missing ToUnicode patterns
  - Figure/table float placement, bibliography link detection

- **[research/medical-and-scientific-pdf-patterns.md](research/medical-and-scientific-pdf-patterns.md)** — Dense mixed-content scientific documents: figures, tables, equations, citations, footnotes
  - Multi-column layout with equation regions, journal template patterns
  - Citation/reference block detection and DOI link extraction

- **[research/mathematical-expression-handling.md](research/mathematical-expression-handling.md)** — Mathematical notation extraction strategies across encoding schemes
  - Symbol font mapping (Symbol, STIX, XITS, Computer Modern math)
  - Subscript/superscript detection, operator precedence linearization

- **[research/legal-and-financial-pdf-patterns.md](research/legal-and-financial-pdf-patterns.md)** — Legal briefs, contracts, financial filings: line numbers, Bates stamps, footnote styles
  - Court filing format patterns (federal, state), header/footer extraction
  - Financial table dense-number extraction, currency symbol handling

- **[research/government-form-pdf-patterns.md](research/government-form-pdf-patterns.md)** — Government form PDFs: IRS, regulatory filings, mixed AcroForm/print-field layouts
  - Form field label-to-value association across non-AcroForm "flat" forms
  - Instruction text vs. fillable field disambiguation

- **[research/book-and-publishing-pdf-patterns.md](research/book-and-publishing-pdf-patterns.md)** — Book PDF structural complexity: running headers, footnotes, sidebars, indices, TOC
  - Chapter/section boundary detection, page number extraction
  - Index entry reconstruction, cross-reference link resolution

- **[research/engineering-document-extraction.md](research/engineering-document-extraction.md)** — PDF/E-1 engineering documents: CAD-exported PDFs, technical drawing annotation extraction
  - Dimension annotation text, title block field extraction
  - Revision table parsing, BOM (bill of materials) table detection

- **[research/presentation-and-spreadsheet-pdfs.md](research/presentation-and-spreadsheet-pdfs.md)** — PowerPoint and Excel PDFs: slide structure, speaker notes, sheet grids, frozen headers
  - Slide bounding box as implicit zone boundary, note text association
  - Spreadsheet cell grid reconstruction from absolute-positioned text

- **[research/pdfa-compliance-and-extraction.md](research/pdfa-compliance-and-extraction.md)** — PDF/A conformance levels and their extraction guarantees and fast-path optimizations
  - PDF/A-1a/1b, 2a/2b/2u, 3a/3b/3u conformance constraints
  - ToUnicode guarantee in PDF/A-1a, mandatory tagging in PDF/A-2a

- **[research/pdfa-archival-extraction-guarantees.md](research/pdfa-archival-extraction-guarantees.md)** — Specific extraction guarantees derivable from PDF/A conformance, enabling fast-path skips
  - Level-specific ToUnicode presence guarantees, XMP metadata mandates
  - Conformance-driven fallback skipping to improve throughput

- **[research/pdfx-prepress-extraction.md](research/pdfx-prepress-extraction.md)** — PDF/X print production formats: spot colors, bleed marks, output intent profiles
  - PDF/X-1a, X-3, X-4, X-6 conformance constraints
  - OutputIntent ICC profile, TrimBox/BleedBox as canonical page boundaries

- **[research/pdfua2-and-accessibility-standards.md](research/pdfua2-and-accessibility-standards.md)** — PDF/UA-2 (ISO 14289-2) built on PDF 2.0: updated structure requirements and WCAG alignment
  - Namespace-qualified structure types, artifact classification changes
  - Associated file attachment for MathML, pronunciation dictionaries

- **[research/pdfvt-variable-transactional-printing.md](research/pdfvt-variable-transactional-printing.md)** — PDF/VT variable and transactional printing: DPart tree, record boundary, reusable content
  - DPart metadata extraction, record-per-recipient text variation
  - Reusable content stream (RCS) handling, page piece dictionary

---

### Security and Robustness

Handling adversarial inputs, encryption, redaction, and JavaScript.

- **[research/adversarial-inputs-and-parser-security.md](research/adversarial-inputs-and-parser-security.md)** — Concrete attack classes and defensive techniques for production PDF parsing
  - Decompression bombs: stream size limits, inflation ratio caps
  - Circular reference guards, stack depth limits, object count caps

- **[research/pdf-encryption-and-security.md](research/pdf-encryption-and-security.md)** — Standard security handler, RC4 and AES decryption, certificate handlers, and password resolution
  - `/V`, `/R`, `/KeyLength`, `/CF`/`/StmF`/`/StrF` handler fields
  - Empty-password-first attempt sequence, unsupported handler error path

- **[research/error-handling-and-robustness.md](research/error-handling-and-robustness.md)** — Recoverable error model, diagnostic code taxonomy, and graceful degradation across all stages
  - No-panic guarantee in library code, per-error diagnostic entries
  - Stage-level error isolation: one page failure does not abort others

- **[research/redaction-detection-and-recovery.md](research/redaction-detection-and-recovery.md)** — Distinguishing true redaction from soft redaction; detecting content beneath covering rectangles
  - Black rectangle over text detection, opacity-0 text identification
  - /Redact annotation type, incremental update soft-redaction forensics

- **[research/javascript-and-interactive-pdf-extraction.md](research/javascript-and-interactive-pdf-extraction.md)** — JavaScript detection, dynamic content identification, and extraction strategy for interactive PDFs
  - `/JS` action detection, `contains_javascript` metadata flag
  - XFA dynamic form extraction vs. static snapshot fallback

- **[research/digital-signatures-and-certification.md](research/digital-signatures-and-certification.md)** — Digital signature field metadata extraction and ByteRange coverage reporting
  - Sig field walk, `/ByteRange`, `/SubFilter` format identification
  - Certification vs. approval signature distinction, validation_status field

---

### Output, API, and Metadata

Schema, serialization, and document-level metadata extraction.

- **[research/extraction-output-schema.md](research/extraction-output-schema.md)** — Stable v1.0 JSON schema: full field inventory for document, page, span, block, form, and annotation output
  - Document-level metadata, outline, page array, extraction_quality
  - Span and block structs, confidence sources, block kind enum

- **[research/xmp-and-document-metadata.md](research/xmp-and-document-metadata.md)** — XMP RDF/XML parsing, /Info dict fallback, Dublin Core fields, and XMP-vs-Info conflict resolution
  - `pdfaid:conformance`, `dc:title`, `pdf:Producer` namespace fields
  - XMP priority over /Info in PDF 1.4+ documents

- **[research/hyperlinks-and-named-destinations.md](research/hyperlinks-and-named-destinations.md)** — URI annotations, GoTo actions, named destination resolution, and internal navigation link extraction
  - `/Annots` Link annotation type, `/A` action dictionary
  - `/Dests` name tree, `/Names` catalog entry, cross-document GoToR

- **[research/page-labels-and-outline-extraction.md](research/page-labels-and-outline-extraction.md)** — PageLabels number tree, outline bookmark traversal, destination types
  - `/S` label style (D/r/R/A/a), `/P` prefix, `/St` start value
  - `/Outlines` `/First`/`/Next`/`/Last` linked list walk

- **[research/form-fields-and-annotations.md](research/form-fields-and-annotations.md)** — AcroForm field hierarchy, XFA extraction, and annotation text (highlights, stamps, notes)
  - `/Fields` array walk, field type detection (Tx/Btn/Ch/Sig)
  - Annotation subtypes: Highlight, StrikeOut, FreeText, Stamp, Link

- **[research/embedded-files-and-portfolios.md](research/embedded-files-and-portfolios.md)** — EmbeddedFiles name tree navigation, attachment metadata, and portfolio structure
  - `/EmbeddedFiles` name tree, `/EF` dictionary, file stream access
  - Portfolio `/Collection` schema, navigator sort order

- **[research/pdf-portfolio-and-attachments.md](research/pdf-portfolio-and-attachments.md)** — PDF Portfolio collections: navigator schema, attachment content access, and sub-document extraction
  - `/Collection` fields array, sort key extraction
  - Recursive extraction of PDF attachments within portfolios

- **[research/performance-and-streaming-architecture.md](research/performance-and-streaming-architecture.md)** — Memory-mapped I/O, rayon page parallelism, NDJSON streaming, and LRU object cache design
  - mmap + `madvise(MADV_SEQUENTIAL)` on content streams
  - `BufWriter<Stdout>` NDJSON, page-level rayon scatter/gather

- **[research/chunking-for-llm-consumption.md](research/chunking-for-llm-consumption.md)** — Block-boundary chunk splitting, overlap strategy, and token count estimation for RAG ingestion
  - Heading-aware chunk boundaries, table/figure keep-together rules
  - Overlap window sizing, chunk metadata (page_index, block_ids)

- **[research/benchmark-and-test-methodology.md](research/benchmark-and-test-methodology.md)** — Test corpus design, extraction accuracy metrics, performance benchmarks, and regression suite
  - Ground-truth corpus construction, character error rate (CER) metric
  - Performance targets: throughput pages/sec, memory ceiling per process

---

### Languages, Scripts, and Multilingual Documents

Non-Latin script handling, bidirectional text, and language detection.

- **[research/multilingual-document-extraction.md](research/multilingual-document-extraction.md)** — Mixed-script documents combining Latin with Arabic, Hebrew, CJK, and other scripts
  - Per-span language detection, BCP-47 tag assignment
  - Bidi paragraph detection and RTL reading order handling

- **[research/language-detection-and-script-handling.md](research/language-detection-and-script-handling.md)** — Unicode script identification, `whichlang` integration, and language tag propagation
  - Script block ranges for Latin/Arabic/Hebrew/CJK/Devanagari/Thai
  - Language tag inheritance from StructTree `/Lang` attribute

- **[research/cjk-and-asian-script-encoding.md](research/cjk-and-asian-script-encoding.md)** — CJK font encoding, multi-byte character code parsing, and vertical writing mode
  - Shift-JIS, GB18030 (GBK), Big5, EUC-KR code page decoding
  - Vertical glyph substitution, column-major reading order

- **[research/indic-script-extraction.md](research/indic-script-extraction.md)** — Devanagari, Tamil, Telugu, Bengali, and related abugida script extraction
  - Akhand/matra glyph cluster reconstruction, halant handling
  - Visual-order to logical-order reordering for Indic scripts

- **[research/southeast-asian-script-extraction.md](research/southeast-asian-script-extraction.md)** — Thai, Lao, Khmer, Burmese: scripts without inter-word spaces requiring segmentation
  - Dictionary-based word segmentation for Thai/Lao/Khmer
  - Stacked consonant cluster handling in Burmese/Khmer

- **[research/ruby-text-and-east-asian-typography.md](research/ruby-text-and-east-asian-typography.md)** — Japanese ruby (furigana) annotation extraction and East Asian typography conventions
  - Ruby base/annotation text pair reconstruction
  - Tate-chu-yoko (horizontal-in-vertical) mixed direction handling

- **[research/unicode-normalization-and-text-cleanup.md](research/unicode-normalization-and-text-cleanup.md)** — NFC normalization pipeline, combining character handling, and post-extraction cleanup
  - Canonical decomposition + canonical composition (NFC) via `unicode-normalization`
  - Zero-width joiner/non-joiner, byte order mark stripping

---

### Accessibility and Tagged PDF

Structure tree exploitation and accessibility standard compliance.

- **[research/accessibility-and-tagged-pdf-deep-dive.md](research/accessibility-and-tagged-pdf-deep-dive.md)** — PDF/UA-1 deep dive: structure tree contract, reading order derivation, and artifact suppression
  - StructTreeRoot walk, RoleMap normalization, ActualText semantics
  - Artifact classification (/Pagination, /Layout, /Background)

- **[research/tagged-pdf-structure-and-reading-order.md](research/tagged-pdf-structure-and-reading-order.md)** — Tagged PDF structure tree as authoritative reading order with MCID-to-span mapping
  - ParentTree reverse lookup, StructElem type-to-block-kind mapping
  - Suspects flag: when to fall back to XY-cut for coverage gaps

- **[research/pdfua2-and-accessibility-standards.md](research/pdfua2-and-accessibility-standards.md)** — PDF/UA-2 standard built on PDF 2.0 with updated structure requirements and WCAG alignment
  - New artifact classification rules, associated file for MathML
  - Namespace-qualified structure element types in PDF 2.0

- **[research/article-threads-and-reading-order.md](research/article-threads-and-reading-order.md)** — PDF article thread bead chains as multi-page reading order override for magazine layouts
  - `/Threads` array, bead rect chains across non-contiguous pages
  - Priority relative to structure tree and XY-cut ordering

---

## Full Document List (Alphabetical)

| Document | Category | One-Line Description |
|---|---|---|
| [accessibility-and-tagged-pdf-deep-dive.md](research/accessibility-and-tagged-pdf-deep-dive.md) | Accessibility | PDF/UA-1 structure tree contract, artifact suppression, ActualText semantics |
| [adversarial-inputs-and-parser-security.md](research/adversarial-inputs-and-parser-security.md) | Security | Decompression bombs, circular refs, resource exhaustion defense |
| [article-threads-and-reading-order.md](research/article-threads-and-reading-order.md) | Accessibility | Article thread bead chains as multi-page reading order override |
| [benchmark-and-test-methodology.md](research/benchmark-and-test-methodology.md) | Output & API | Test corpus design, CER metric, performance targets |
| [book-and-publishing-pdf-patterns.md](research/book-and-publishing-pdf-patterns.md) | Specialized Docs | Book PDFs: running headers, footnotes, sidebars, indices, TOC |
| [chunking-for-llm-consumption.md](research/chunking-for-llm-consumption.md) | Output & API | Block-boundary chunk splitting and overlap for RAG ingestion |
| [cjk-and-asian-script-encoding.md](research/cjk-and-asian-script-encoding.md) | Languages | CJK CIDFont encoding, multi-byte codes, vertical writing mode |
| [cmap-format-and-cid-encoding.md](research/cmap-format-and-cid-encoding.md) | Font & Encoding | ToUnicode CMap syntax: bfchar, bfrange, usecmap, ligature expansion |
| [color-management-and-icc-profiles.md](research/color-management-and-icc-profiles.md) | OCR & Image | ICC profile normalization for text visibility luminance estimation |
| [complex-layout-reading-order.md](research/complex-layout-reading-order.md) | Text Assembly | XY-cut algorithm for multi-column and mixed-layout documents |
| [confidence-scoring-and-aggregation.md](research/confidence-scoring-and-aggregation.md) | Text Assembly | Per-glyph confidence, span aggregation, readability score |
| [content-stream-concatenation.md](research/content-stream-concatenation.md) | Content Stream | Multi-stream pages, /Length mismatches, Form XObject sub-streams |
| [content-stream-operators.md](research/content-stream-operators.md) | Content Stream | Complete text operator reference: Tj, TJ, Td, Tm, Tf, Tr, BT/ET |
| [digital-signatures-and-certification.md](research/digital-signatures-and-certification.md) | Security | Sig field metadata, ByteRange, SubFilter, validation_status |
| [document-catalog-and-structure.md](research/document-catalog-and-structure.md) | File Format | Catalog keys, page tree traversal, inherited attribute resolution |
| [document-classification-and-zone-labeling.md](research/document-classification-and-zone-labeling.md) | Text Assembly | Body/header/footer/caption/margin zone heuristics |
| [embedded-files-and-portfolios.md](research/embedded-files-and-portfolios.md) | Output & API | EmbeddedFiles name tree, attachment metadata, portfolio structure |
| [engineering-document-extraction.md](research/engineering-document-extraction.md) | Specialized Docs | PDF/E-1 CAD exports: dimension annotations, title blocks, BOM tables |
| [error-handling-and-robustness.md](research/error-handling-and-robustness.md) | Security | Recoverable error model, diagnostic taxonomy, stage isolation |
| [extraction-output-schema.md](research/extraction-output-schema.md) | Output & API | Stable v1.0 JSON schema for all output fields |
| [extraction-pipeline-overview.md](research/extraction-pipeline-overview.md) | Start Here | End-to-end 9-stage architectural blueprint |
| [font-descriptor-and-metrics.md](research/font-descriptor-and-metrics.md) | Font & Encoding | FontDescriptor keys, Widths arrays, hmtx metrics |
| [font-subsetting-and-extraction.md](research/font-subsetting-and-extraction.md) | Font & Encoding | Subset naming, glyph table gaps, Standard 14 prefix stripping |
| [form-fields-and-annotations.md](research/form-fields-and-annotations.md) | Output & API | AcroForm field walk, XFA, annotation text types |
| [glyph-recognition-and-unicode-recovery.md](research/glyph-recognition-and-unicode-recovery.md) | Font & Encoding | Shape-hash Level 4 fallback, perceptual hash database |
| [government-form-pdf-patterns.md](research/government-form-pdf-patterns.md) | Specialized Docs | IRS/regulatory forms: flat print fields vs. AcroForm disambiguation |
| [graphics-state-tracking.md](research/graphics-state-tracking.md) | Content Stream | CTM, text matrix, q/Q stack, rendering mode state machine |
| [historical-and-degraded-document-extraction.md](research/historical-and-degraded-document-extraction.md) | OCR & Image | Microfilm/photocopy preprocessing, multi-pass OCR strategies |
| [hyperlinks-and-named-destinations.md](research/hyperlinks-and-named-destinations.md) | Output & API | URI annotations, GoTo actions, named destination resolution |
| [image-and-figure-extraction.md](research/image-and-figure-extraction.md) | OCR & Image | Image XObject identification, inline images, figure regions |
| [image-compression-and-filter-decoding.md](research/image-compression-and-filter-decoding.md) | File Format | All PDF filters: FlateDecode, LZW, ASCII85, DCT, JBIG2, JPX |
| [incremental-updates-and-versioning.md](research/incremental-updates-and-versioning.md) | File Format | Non-destructive PDF modification, revision history, soft redaction |
| [indic-script-extraction.md](research/indic-script-extraction.md) | Languages | Devanagari/Tamil/Bengali: cluster reconstruction, logical reordering |
| [invisible-and-hidden-text.md](research/invisible-and-hidden-text.md) | Content Stream | Tr=3 text, OCR layer patterns, include_invisible_text flag |
| [javascript-and-interactive-pdf-extraction.md](research/javascript-and-interactive-pdf-extraction.md) | Security | JavaScript detection, XFA dynamic content, contains_javascript flag |
| [language-detection-and-script-handling.md](research/language-detection-and-script-handling.md) | Languages | Unicode script identification, whichlang, BCP-47 tag assignment |
| [latex-and-scientific-pdf-patterns.md](research/latex-and-scientific-pdf-patterns.md) | Specialized Docs | LaTeX toolchain patterns: font stacks, microtype, missing ToUnicode |
| [legal-and-financial-pdf-patterns.md](research/legal-and-financial-pdf-patterns.md) | Specialized Docs | Legal briefs/contracts/filings: line numbers, Bates stamps, footnotes |
| [linearized-pdf-and-streaming.md](research/linearized-pdf-and-streaming.md) | File Format | Fast-web-view layout, two-xref structure, hint stream decoding |
| [malformed-pdf-repair-and-recovery.md](research/malformed-pdf-repair-and-recovery.md) | File Format | Forward scan fallback, truncation recovery, corruption taxonomy |
| [mathematical-expression-handling.md](research/mathematical-expression-handling.md) | Specialized Docs | Symbol font mapping, subscript/superscript, formula linearization |
| [medical-and-scientific-pdf-patterns.md](research/medical-and-scientific-pdf-patterns.md) | Specialized Docs | Dense scientific docs: equations, citations, footnotes, journal layouts |
| [multilingual-document-extraction.md](research/multilingual-document-extraction.md) | Languages | Mixed-script documents, per-span language, RTL reading order |
| [opentype-math-and-formula-extraction.md](research/opentype-math-and-formula-extraction.md) | Font & Encoding | OpenType MATH table, MathVariants, script glyph positioning |
| [optional-content-groups.md](research/optional-content-groups.md) | Content Stream | OCG layer state, BDC/EMC marked content, visibility filtering |
| [page-geometry-and-document-structure.md](research/page-geometry-and-document-structure.md) | File Format | Page boxes, coordinate systems, rotation matrix |
| [page-labels-and-outline-extraction.md](research/page-labels-and-outline-extraction.md) | Text Assembly | PageLabels number tree, outline walk, destination resolution |
| [pdfa-archival-extraction-guarantees.md](research/pdfa-archival-extraction-guarantees.md) | Specialized Docs | PDF/A conformance-derived fast-path guarantees and skips |
| [pdfa-compliance-and-extraction.md](research/pdfa-compliance-and-extraction.md) | Specialized Docs | PDF/A-1/2/3 conformance levels and their extraction implications |
| [pdf-encryption-and-security.md](research/pdf-encryption-and-security.md) | Security | Standard handler, RC4/AES decryption, password attempt sequence |
| [pdf-fonts-and-encoding.md](research/pdf-fonts-and-encoding.md) | Font & Encoding | All font types, encoding vectors, AGL, four-level fallback chain |
| [pdf-generator-quirks.md](research/pdf-generator-quirks.md) | File Format | Per-generator fingerprinting and spec-deviation workarounds |
| [pdf-object-model-and-data-types.md](research/pdf-object-model-and-data-types.md) | File Format | Eight PDF object types, reference semantics, generation numbers |
| [pdf-portfolio-and-attachments.md](research/pdf-portfolio-and-attachments.md) | Output & API | Portfolio collection schema, navigator sort, sub-document extraction |
| [pdf-specification.md](research/pdf-specification.md) | File Format | ISO 32000-1/2 file structure, xref, object streams, linearization |
| [pdfua2-and-accessibility-standards.md](research/pdfua2-and-accessibility-standards.md) | Accessibility | PDF/UA-2 on PDF 2.0: updated structure rules, WCAG alignment |
| [pdfvt-variable-transactional-printing.md](research/pdfvt-variable-transactional-printing.md) | Specialized Docs | PDF/VT DPart tree, record boundaries, reusable content streams |
| [pdfx-prepress-extraction.md](research/pdfx-prepress-extraction.md) | Specialized Docs | PDF/X print formats: spot colors, OutputIntent, TrimBox boundaries |
| [performance-and-streaming-architecture.md](research/performance-and-streaming-architecture.md) | Output & API | mmap I/O, rayon parallelism, NDJSON streaming, LRU object cache |
| [post-extraction-normalization.md](research/post-extraction-normalization.md) | Text Assembly | NFC normalization, whitespace cleanup, paragraph boundaries |
| [post-ocr-text-correction.md](research/post-ocr-text-correction.md) | OCR & Image | Systematic OCR error correction, dictionary validation, re-scoring |
| [presentation-and-spreadsheet-pdfs.md](research/presentation-and-spreadsheet-pdfs.md) | Specialized Docs | PowerPoint/Excel PDFs: slide structure, speaker notes, cell grids |
| [raster-ocr-pipeline.md](research/raster-ocr-pipeline.md) | OCR & Image | 300 DPI render, Sauvola, deskew, Tesseract HOCR integration |
| [redaction-detection-and-recovery.md](research/redaction-detection-and-recovery.md) | Security | True vs. soft redaction detection, black rectangle over text |
| [resource-dictionary-and-inheritance.md](research/resource-dictionary-and-inheritance.md) | Font & Encoding | Font/XObject namespace resolution, multi-level resource merging |
| [ruby-text-and-east-asian-typography.md](research/ruby-text-and-east-asian-typography.md) | Languages | Japanese ruby/furigana extraction, tate-chu-yoko mixed direction |
| [scanned-vs-vector-page-classification.md](research/scanned-vs-vector-page-classification.md) | OCR & Image | PageClass signals: image coverage, validity rate, Hybrid detection |
| [semantic-text-reconstruction.md](research/semantic-text-reconstruction.md) | Text Assembly | Hyphen removal, ligature expansion, line-end join heuristics |
| [shading-pattern-and-text-visibility.md](research/shading-pattern-and-text-visibility.md) | Content Stream | Luminance estimation for text-on-background visibility |
| [southeast-asian-script-extraction.md](research/southeast-asian-script-extraction.md) | Languages | Thai/Lao/Khmer/Burmese: word segmentation, stacked consonants |
| [span-merging-and-text-run-assembly.md](research/span-merging-and-text-run-assembly.md) | Text Assembly | Span boundary triggers, text run assembly, line merging pipeline |
| [stroke-and-outlined-text.md](research/stroke-and-outlined-text.md) | Content Stream | Rendering modes 1–7, stroke-only glyphs, clip path text |
| [table-structure-reconstruction.md](research/table-structure-reconstruction.md) | Text Assembly | Ruling-line grid, borderless alignment, cell assignment, merged cells |
| [tagged-pdf-structure-and-reading-order.md](research/tagged-pdf-structure-and-reading-order.md) | Accessibility | StructTree reading order, MCID mapping, Suspects fallback |
| [text-positioning-and-font-metrics.md](research/text-positioning-and-font-metrics.md) | Content Stream | Text state scalars: Tc, Tw, Tz, TL, Ts, and matrix accumulation |
| [text-readability-validation.md](research/text-readability-validation.md) | Text Assembly | Printable-char ratio, dictionary word rate, OCR fallback threshold |
| [type3-font-extraction.md](research/type3-font-extraction.md) | Font & Encoding | Type 3 CharProcs streams, per-glyph rasterization, shape recognition |
| [unicode-normalization-and-text-cleanup.md](research/unicode-normalization-and-text-cleanup.md) | Languages | NFC normalization, combining characters, ZWJ/ZWNJ, BOM stripping |
| [watermark-and-background-separation.md](research/watermark-and-background-separation.md) | Text Assembly | Watermark detection, repeated-pattern suppression, is_artifact flag |
| [word-boundary-reconstruction.md](research/word-boundary-reconstruction.md) | Content Stream | Inter-glyph gap thresholds, TJ kern detection, space insertion |
| [xmp-and-document-metadata.md](research/xmp-and-document-metadata.md) | Output & API | XMP RDF/XML parsing, /Info fallback, Dublin Core, conflict resolution |
| [xref-table-parsing-and-object-lookup.md](research/xref-table-parsing-and-object-lookup.md) | File Format | Traditional xref, xref streams, hybrid files, /Prev chain traversal |
