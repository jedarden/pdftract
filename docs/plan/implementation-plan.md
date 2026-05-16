# pdftract Implementation Plan

**Version:** 1.0  
**Status:** Active  
**Repo:** jedarden/pdftract  
**Last updated:** 2026-05-16

---

## Overview

pdftract is a Rust PDF text extraction library with a CLI (`pdftract extract`), an HTTP server mode (`pdftract serve`), and a PyO3 Python binding. It extracts Unicode text from PDF files — including scanned pages via OCR — and produces structured JSON, NDJSON, or plain text output. The output schema is defined in `docs/research/extraction-output-schema.md` and is stable at schema version 1.0.

The implementation is organized into seven phases. Phases 1–4 deliver a working vector-extraction CLI. Phase 5 adds OCR. Phase 6 adds the full API surface (PyO3, HTTP). Phase 7 adds advanced features that require the Phase 1–4 foundation.

### Key architectural decisions (baked in from the start)

- **File I/O:** `memmap2` for zero-copy random access; `madvise(MADV_SEQUENTIAL)` on content streams.
- **Object cache:** LRU with 4096-entry capacity (`lru` crate); object streams decompressed once and cached as `Arc<[u8]>`.
- **Parallelism:** `rayon` for page-level parallelism; per-page work is embarrassingly parallel after Stage 1–2 complete.
- **Serialization:** `serde` + `serde_json`; `BufWriter` wrapping `io::Stdout` for NDJSON streaming.
- **Error model:** All parse errors are recoverable and produce diagnostic entries in the `errors` array; no `panic!` in library code.
- **Crate layout:** `pdftract-core` (lib), `pdftract-cli` (binary), `pdftract-py` (PyO3, optional feature).

---

## Dependency Matrix

| Crate | Version | Purpose |
|---|---|---|
| `memmap2` | 0.9 | Memory-mapped file access |
| `flate2` | 1 | FlateDecode / zlib decompression |
| `lzw` | 0.10 | LZWDecode |
| `jpeg-decoder` | 0.3 | DCTDecode passthrough validation |
| `ttf-parser` | 0.21 | TrueType/OpenType glyph metrics and cmap lookup |
| `owned_ttf_parser` | 0.21 | Arc-safe wrapper for ttf-parser |
| `lru` | 0.12 | Object cache eviction |
| `rayon` | 1 | Page-level parallelism |
| `serde` | 1 | Serialization derive macros |
| `serde_json` | 1 | JSON output |
| `indexmap` | 2 | Ordered dictionaries (PDF dict key order matters for some CMap parsing) |
| `bytes` | 1 | Zero-copy byte slice sharing for object streams |
| `unicode-normalization` | 0.1 | NFC normalization in Stage 7 |
| `encoding_rs` | 0.8 | CJK encoding decoding (Shift-JIS, GB18030, Big5, EUC-KR) |
| `whichlang` | 0.1 | Language detection |
| `tesseract` | 0.14 | Tesseract OCR FFI bindings |
| `leptonica-plumbing` | 0.4 | Leptonica image preprocessing (Sauvola, deskew) |
| `image` | 0.25 | Raster image decoding and DPI-scaled rendering |
| `pyo3` | 0.21 | Python bindings (optional feature `python`) |
| `maturin` | build | PyO3 wheel packaging |
| `axum` | 0.7 | HTTP serve mode |
| `tokio` | 1 | Async runtime for axum |
| `clap` | 4 | CLI argument parsing |
| `thiserror` | 1 | Error type derivation |
| `log` + `env_logger` | 0.4 | Structured logging |

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

**Crates:** `indexmap` (dict), `bytes` (object stream caching)

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
- **Page tree** (`/Pages` subtree): flatten into a `Vec<PageDict>` with inherited attributes resolved (MediaBox, CropBox, BleedBox, TrimBox, ArtBox, Resources, Rotate). Inheritance walk: page dict overrides parent dict; root `/Pages` is the ultimate fallback.
- **Resource dictionary inheritance:** each page gets a fully resolved `ResourceDict` merging all ancestor `/Resources` dicts (font, XObject, ExtGState, ColorSpace, Shading, Pattern, Properties namespaces). Per-key last-write-wins at the page level.
- **Encryption dictionary** detection: if `/Encrypt` present in trailer, identify handler (`/Standard` vs. custom), extract `/V`, `/R`, `/KeyLength`, `/CF`/`/StmF`/`/StrF` entries. RC4 and AES-128/256 decryption. Password attempt: empty string first, then user-supplied. On failure: emit `ENCRYPTION_UNSUPPORTED` and abort.

**Crates:** none beyond the parser layer

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
| `JBIG2Decode` | passthrough | Pass raw JBIG2 bytes; log global stream reference |
| `JPXDecode` | passthrough | Pass raw JPEG 2000 bytes; for OCR path, decode via `image` crate |
| `CCITTFaxDecode` | passthrough | Pass raw CCITT bytes; for OCR path, decode via `image` crate |
| `Crypt` | identity only | `/Name /Identity` handled; custom crypt filters emit `ENCRYPTION_UNSUPPORTED` |

**Filter pipeline:** `/Filter` is a name or array; `/DecodeParms` is aligned or absent. Apply decoders in order. Mismatched lengths: apply defaults, log diagnostic.

**Error recovery:** zlib decompression error mid-stream: return bytes decoded so far, emit `STREAM_DECODE_ERROR` diagnostic. Never abort the page.

**Crates:** `flate2`, `lzw`, `jpeg-decoder` (JPEG validation only), `image` (JPX/CCITT raster decode for OCR path)

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
| `CIDFontType2` | `/FontFile2` or `/FontFile3` (`/OpenType`) | `/DW`, `/W` |
| `Type3` | `/CharProcs` content streams | `/Widths` |
| OpenType (CFF) | `/FontFile3` (`/OpenType`) | `hhea`/`hmtx` via `ttf-parser` |

**Font subset detection:** Many embedded fonts are subsets with name prefix like `ABCDEF+Helvetica`. Strip the six-uppercase-letter prefix before looking up Standard 14 or glyph name tables.

**Crates:** `ttf-parser`, `owned_ttf_parser`

**Critical tests:**
- Standard 14 font (no embedding): correct metrics returned without font file
- Subset font `ABCDEF+Times-Roman`: stripped to `Times-Roman`, standard metrics used
- CIDFontType2 with `/CIDToGIDMap /Identity`: GID == CID for all lookups
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

Hash the embedded font program (SHA-256 of the font stream bytes). Look up in a bundled database of known font checksums → per-glyph Unicode mapping tables. Initially populated with the most common 200 commercial fonts.

Set `unicode_source = "fingerprint"`, `confidence = 0.85`.

**Level 4: Glyph shape recognition**

Render the glyph to a 32×32 grayscale bitmap using the font program. Hash the bitmap with a perceptual hash. Look up in a bundled shape→Unicode database (see Phase 2.3).

Set `unicode_source = "shape_match"`, `confidence = 0.7`.

**Failure:** Emit U+FFFD, `unicode_source = "unknown"`, `confidence = 0.0`, log `GLYPH_UNMAPPED` diagnostic.

**Crates:** `ttf-parser` (glyph rendering for shape hash), `phf` (compile-time AGL hash map)

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
    flags: EnumSet<SpanFlag>, // bold, italic, smallcaps, subscript, superscript
}
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

**Crates:** None (clustering is a simple sort + gap scan)

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
- `header`/`footer`: block y0 in top/bottom 7% of page height AND appears on 3+ consecutive pages with identical or near-identical text
- `paragraph`: default
- `figure`: bbox contains only image XObjects, no text glyphs
- `list`: line starts with bullet/numbered pattern (regex: `^\s*[•‣◦\-\*]\s` or `^\s*\d+[\.\)]\s`)
- `caption`: small font, follows a `figure` block within 2 lines

**Critical tests:**
- Indented first line of paragraph: not split into two blocks
- Header text appearing on pages 1–10: classified `header` and deduplicated
- Bullet list with mixed font sizes: all items in same `list` block

### 4.5 Reading Order

Determine the reading order of blocks within the page.

**Fast path (tagged PDF):** If `is_tagged = true`, defer to Phase 7 StructTree traversal. Set `reading_order_algorithm = "struct_tree"`.

**XY-cut algorithm (untagged, rectilinear layouts):**
1. Find the widest vertical whitespace gap dividing the page's text bbox into left and right halves → split into two regions
2. For each region, find the widest horizontal gap → split into top and bottom sub-regions
3. Recurse until regions contain a single column of text
4. Reading order: left region before right; top before bottom within each region

**Docstrum fallback (when XY-cut produces > 10 regions with < 3 blocks each):** Compute nearest-neighbor pairs between text blocks. Build a graph of adjacency edges weighted by distance and angle. Traverse the connected components in estimated reading order (sort root nodes by page position, follow edges within each component).

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
- Watermark blocks excluded (Phase 6 watermark detection)

**Critical tests:**
- 10-page document: 9 form-feed characters in output
- Header block: excluded from `--text` output by default
- Invisible text span: excluded from `--text` output

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

**Hybrid detection:** Compute per-region classification: divide page into 8×8 grid cells. Cells with text operators and high validity → vector; cells with image coverage and no text → scanned. If both types present in significant fractions → `Hybrid`.

**Critical tests:**
- Pure text PDF: all pages `Vector` with confidence > 0.95
- Scanned single-page PDF (image only): `Scanned`
- PDF/A with invisible text layer over scanned image: `BrokenVector`
- Hybrid page with text header and scanned body: `Hybrid`, correct region split

### 5.2 Image Extraction for Raster Pages

For `Scanned` and `Hybrid` pages, produce a raster for Tesseract.

**Rendering approach:** Use a PDF rendering backend to rasterize the page. Prefer `pdfium-render` (Chromium's PDFium, FOSS binary available) for rendering fidelity. Fall back to compositing the image XObjects directly using their decoded pixel data and the XObject's placement matrix when a full renderer is not available.

**DPI selection:**
- Standard body text (font_size > 8pt equivalent): 300 DPI
- Fine print or small text: 400 DPI
- Line art / JBIG2 pages: 200 DPI (already binary; higher DPI doesn't help)

**Output:** Grayscale `image::GrayImage` for each page region needing OCR.

**Crates:** `pdfium-render` (optional feature), `image`

### 5.3 Image Preprocessing

Apply the preprocessing pipeline before Tesseract invocation.

**Pipeline (in order):**
1. **Deskew:** Hough line transform on binarized image; compute dominant angle; rotate by negative angle. Skip if detected angle < 0.3° (no meaningful skew).
2. **Binarization:** Sauvola local adaptive thresholding for physical scans; Otsu global for digital-origin scans. Detect origin via image XObject filter: DCTDecode → Sauvola; JBIG2Decode → already binary, skip.
3. **Denoising:** 3×3 median filter for salt-and-pepper noise. Skip for JBIG2 (already clean binary).
4. **Contrast normalization:** Histogram stretch to [0, 255] after binarization.
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

For `BrokenVector` pages, use vector glyph positions as hints to improve Tesseract segmentation.

**Pipeline:**
1. Run Phase 3 content stream processing in position-hint mode: collect glyph bboxes but discard Unicode values (treat all as U+FFFD)
2. Convert glyph bboxes to HOCR-format `word` hint blocks and pass to Tesseract via `SetVariable("applybox_debug", "0")` and Tesseract's box-file input mode
3. Tesseract uses the hint boxes to seed its segmentation, improving word boundary detection
4. Parse HOCR output as in Phase 5.4
5. If OCR confidence > 0.7 for a region: use OCR text; if OCR confidence < 0.3: re-attempt without hints

**Critical tests:**
- PDF/A with invisible text layer at correct positions: OCR output better than blind OCR (validate WER delta)
- PDF/A with incorrect text layer positions (misaligned): hints discarded when Tesseract confidence drops; fallback to unaided OCR

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
- `metadata`: title, author, subject, keywords, creator, producer, creation_date, modification_date, page_count, pdf_version, is_tagged, is_encrypted, conformance, contains_javascript, contains_xfa, generator
- `outline`: recursive bookmark tree with title, destination, level
- `threads`: article thread chains (Phase 7 feature; empty array in Phase 6)
- `attachments`: from `/EmbeddedFiles` name tree (Phase 7; empty array in Phase 6)
- `signatures`: digital signature metadata (Phase 7; empty array in Phase 6)
- `form_fields`: AcroForm fields with values (Phase 7; empty array in Phase 6)
- `links`: document-scoped URI and internal destination links
- `extraction_quality`: aggregate across all pages
- `errors`: all diagnostics emitted during extraction

**Page-level fields (full schema):**
- `page_index`, `page_label`, `width`, `height`, `rotation`, `page_type`
- `spans`: full Span array per schema
- `blocks`: full Block array per schema
- `annotations`: highlights, stamps, notes, links from `/Annots`
- `tables`: parallel table structure objects for `kind: table` blocks (Phase 7)

**Crates:** `serde`, `serde_json`

**Critical tests:**
- Schema validator: produce output from a known-good PDF, validate against a JSON Schema definition of the output schema
- Page with no text: `spans: []`, `blocks: []`, `page_type: "blank"` or `"figure_only"`
- Error entries: each emitted diagnostic has stable `code`, `severity`, and `page_index`

### 6.2 NDJSON Streaming Mode

Implement `--stream` / `ExtractionOptions.streaming = true`.

**Frame sequence:**
1. Header frame: `{"frame":"header","schema_version":"1.0","metadata":{...},"outline":[...],"total_pages":N}`
2. Per-page frames (emitted as each page completes via rayon): `{"frame":"page","page_index":N,...}`  
   Note: rayon may complete pages out of order; buffer completed pages and emit in page_index order with a window of 8 pages maximum.
3. Footer frame: `{"frame":"footer","extraction_quality":{...},"errors":[...],"threads":[],"attachments":[],"signatures":[],"form_fields":[],"links":[]}`

**BufWriter:** Wrap `io::Stdout` in `BufWriter<io::Stdout>` with 128 KB buffer; flush after each frame.

**Critical tests:**
- 100-page document in streaming mode: frame 0 is header, frames 1–100 are pages in order, frame 101 is footer
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

# Options (keyword arguments mapped to ExtractionOptions):
# ocr=False, ocr_language=["eng"], include_invisible=False,
# extract_forms=False, extract_attachments=False, readability_threshold=0.5

# Exceptions
class PdftractError(Exception): ...       # extraction failed
class EncryptionError(PdftractError): ... # encrypted, no password
```

**Python GIL handling:** Release the GIL during extraction (`py.allow_threads(|| ...)`) so Python threads can continue while a page is being processed.

**Build:** `maturin build --features python` produces a `.whl` for the current platform. CI cross-compiles for all five target triples (see `docs/notes/sdk-architecture.md`).

**Crates:** `pyo3` (feature `extension-module`), `maturin` (build tool)

**Critical tests:**
- `pdftract.extract("test.pdf")` returns a dict with correct `metadata.page_count`
- `pdftract.extract_text("test.pdf")` returns a plain-text string
- `pdftract.extract("nonexistent.pdf")` raises `PdftractError`
- `pdftract.extract("encrypted.pdf")` raises `EncryptionError`
- Python threading: 4 threads each extracting different PDFs simultaneously; no deadlock

### 6.4 HTTP Serve Mode

Implement `pdftract serve --port PORT`.

**Endpoints:**

| Method | Path | Request | Response |
|---|---|---|---|
| POST | `/extract` | multipart/form-data `file=<pdf>` + optional form fields for options | JSON extraction result |
| POST | `/extract/text` | same | `text/plain` body |
| POST | `/extract/stream` | same | NDJSON stream (Content-Type: application/x-ndjson) |
| GET | `/health` | none | `{"status":"ok","version":"x.y.z"}` |

**Options via form fields:** `ocr=true`, `ocr_language=eng,fra`, `readability_threshold=0.5`

**Error responses:** HTTP 400 for bad request (no file field, unsupported content type); HTTP 422 for extraction error (encrypted file, corrupt file); HTTP 500 for internal error. Response body is `{"error":"code","message":"..."}`.

**Concurrency:** axum handles concurrent requests; rayon thread pool is shared across all requests. No per-request thread spawning.

**Request size limit:** Default 256 MB; configurable via `--max-upload-mb`.

**Crates:** `axum`, `tokio`, `tower-http` (for `RequestBodyLimit`, `TraceLayer`), `multer` (multipart parsing)

**Critical tests:**
- `curl -F file=@test.pdf http://localhost:8080/extract`: valid JSON response
- File exceeding size limit: HTTP 413 response
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
- Performance: extraction of a 100-page PDF completes in < 5 seconds on a 4-core CI machine

**Fixture categories:**
- `tests/fixtures/vector/`: clean LaTeX, Word, InDesign outputs
- `tests/fixtures/scanned/`: physical scans at various DPIs and skew angles
- `tests/fixtures/cjk/`: Chinese, Japanese, Korean documents
- `tests/fixtures/malformed/`: truncated, corrupt xref, circular references
- `tests/fixtures/encrypted/`: AES-128, AES-256, RC4 encrypted
- `tests/fixtures/forms/`: AcroForm and XFA documents
- `tests/fixtures/tagged/`: PDF/UA and PDF/A-a tagged documents

### Tier 3: Regression Corpus (CI only)

A private corpus of 500 real-world PDFs from diverse sources runs on every PR. Output is compared against a golden snapshot using a character-level diff. Any regression > 0.5% character error rate blocks the PR.

---

## Phase Dependencies and Sequencing

```
Phase 1 (Core Parser)
  └─► Phase 2 (Font Pipeline)
        └─► Phase 3 (Content Stream)
              └─► Phase 4 (Text Assembly)   ← Plain text output works here
                    └─► Phase 5 (OCR)       ← Scanned PDFs work here
                          └─► Phase 6 (API) ← PyO3, HTTP, full JSON schema
                                └─► Phase 7 (Advanced)
                                      ├─ 7.1 StructTree (independent)
                                      ├─ 7.2 Tables (independent)
                                      ├─ 7.3 Signatures (independent)
                                      ├─ 7.4 Forms (independent)
                                      └─ 7.5 Attachments (independent)
```

Phase 7 sub-tasks are independent of each other and can be assigned to separate developers once Phase 6 is complete.

---

## Release Milestones

| Milestone | Phases Complete | Capability |
|---|---|---|
| v0.1.0 (Alpha) | 1–4 | Vector PDF extraction; plain text and JSON output; CLI only |
| v0.2.0 (Beta) | 1–5 | + Scanned PDF OCR; all page classes handled |
| v0.3.0 (RC) | 1–6 | + PyO3 bindings; HTTP serve; full JSON schema; NDJSON streaming |
| v1.0.0 (Stable) | 1–7 | + StructTree; tables; forms; signatures; attachments |

Binary releases for all five target triples are published to GitHub Releases on every milestone tag. The PyO3 wheel is published to PyPI. The CLI binary is the sole dependency for the subprocess-based SDKs documented in `docs/notes/sdk-invocation.md`.
