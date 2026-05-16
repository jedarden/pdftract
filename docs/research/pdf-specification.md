# PDF Specification — Text Extraction Reference

**Scope:** ISO 32000-1:2008 (PDF 1.7) and ISO 32000-2:2020 (PDF 2.0), with version deltas noted.
This document is implementation-focused reference material for building a PDF text extraction parser in Rust.

---

## 1. File Structure

### 1.1 Body, Cross-Reference Table, and Trailer

A PDF file is divided into four logical sections: header, body, cross-reference table (xref), and trailer. The header contains `%PDF-x.y` on line 1; if the file contains binary data, a comment with four bytes ≥ 128 (e.g., `%âãÏÓ`) follows on line 2 to signal binary content to transport agents.

The **body** is a sequence of indirect objects. Each object is identified by an object number and generation number: `obj_num gen_num obj ... endobj`. The generation number is 0 for all non-reused objects; it increments to 1 when the slot is freed and reallocated.

The **traditional xref table** (PDF 1.0+) begins with the keyword `xref`, followed by one or more subsections. Each subsection starts with `start_obj_num count`, then `count` 20-byte entries: `nnnnnnnnnn ggggg n\r\n` (offset, generation, `n`=in-use / `f`=free). The final entry in a free list is always generation 65535. Offsets are byte offsets from the start of the file.

The **trailer dictionary** immediately follows the xref table (`trailer` keyword, then the dict). Mandatory keys:

- `/Size` — total number of indirect object slots (one more than the highest object number)
- `/Root` — indirect reference to the document catalog
- `/Prev` — byte offset of the previous xref table (for incremental updates)
- `/Encrypt` — encryption dictionary (if encrypted)
- `/Info` — metadata dictionary (deprecated in PDF 2.0 in favor of XMP)
- `/ID` — two-element array of 16-byte MD5 strings; required if `/Encrypt` is present

The file ends with `%%EOF`, preceded by `startxref` and the byte offset of the most recent xref table (or xref stream).

### 1.2 Xref Streams (PDF 1.5+)

In PDF 1.5+, the xref table may be replaced by an **xref stream**, which is an ordinary indirect stream object whose dictionary combines the xref subsection metadata with the trailer dictionary. Key fields:

- `/Type /XRef`
- `/W [w1 w2 w3]` — widths in bytes of the three fields per entry (type, field2, field3)
- `/Index [start count ...]` — subsection descriptors (default: `[0 /Size]`)
- `/Size`, `/Root`, `/Prev`, `/Encrypt`, `/ID` — same semantics as traditional trailer

Entry types (first field of each row):
- `0` — free object (field2 = next free obj num, field3 = generation)
- `1` — in-use uncompressed object (field2 = byte offset, field3 = generation)
- `2` — compressed object in object stream (field2 = object stream obj num, field3 = index within the stream)

All numeric fields are big-endian unsigned integers. `/W` entries of 0 mean the field is implicitly 0 (useful for stripping the type byte when all entries are type 1).

### 1.3 Object Streams (PDF 1.5+)

An **object stream** (`/Type /ObjStm`) packs multiple indirect objects into a single compressed stream. The stream dictionary includes:

- `/N` — number of compressed objects
- `/First` — byte offset within the decoded stream where the first object body begins
- `/Extends` — reference to an earlier object stream this one augments

The first `/First` bytes of the decoded stream are pairs `obj_num offset`, giving the position of each object body relative to the `/First` offset. Object streams may not contain stream objects or objects with generation numbers other than 0. They are referenced via type-2 xref entries.

### 1.4 Linearized PDFs

Linearized ("fast web view") PDFs reorganize the byte layout so that the first page can be rendered before the full file is downloaded. The linearization dictionary appears as the first object: `/Linearized 1.0`, `/L` (file length), `/H` (hint stream offsets), `/O` (first-page object number), `/E` (end of first-page section offset), `/N` (page count), `/T` (offset of main xref table). Parsers must handle the two-xref-table layout (one at the beginning for the first page, one at the end for the rest of the document).

### 1.5 Incremental Updates

PDF supports appending updates without rewriting the original body. Each update appends: new/modified object definitions, a new xref table or xref stream (with `/Prev` pointing to the previous one), and a new `startxref`/`%%EOF`. Parsers must start from the last `startxref`, build the authoritative object table by walking `/Prev` chains, and let later definitions override earlier ones for the same object number.

---

## 2. Page Content Streams

### 2.1 Content Stream Mechanics

Each page dictionary (`/Type /Page`) contains a `/Contents` key that references either a single stream or an array of streams. Multiple streams are concatenated with a single space between them before parsing; they share a single graphics state. The stream body is a sequence of operands followed by an operator keyword — postfix notation.

Streams are filtered via `/Filter` (e.g., `/FlateDecode`, `/LZWDecode`, `/ASCII85Decode`). Multiple filters are applied in array order. The unfiltered stream body must be parsed as PDF syntax.

### 2.2 Graphics State

The graphics state is a stack-based structure. `q` pushes a copy; `Q` pops. Relevant entries for text extraction:

- **CTM** — Current Transformation Matrix (see Section 3)
- **Clipping path** — not needed for text extraction but must be tracked for completeness
- Text state (see Section 2.3) is part of the graphics state and is reset to defaults at `q`/`Q` boundaries

### 2.3 Text State Parameters

| Parameter | Set by | Default | Description |
|-----------|--------|---------|-------------|
| `/Font` + size | `Tf` | (none, required) | Current font and size |
| `Tc` | `Tc` | 0 | Character spacing (unscaled text units) |
| `Tw` | `Tw` | 0 | Word spacing (unscaled text units) |
| `Th` (Tz) | `Tz` | 100 | Horizontal scaling (%) |
| `Tl` | `TL` | 0 | Text leading |
| `Tmode` | `Tr` | 0 | Text rendering mode |
| `Trise` | `Ts` | 0 | Text rise |
| `Tm` | `Tm`, `Td`, `TD`, `T*` | identity/reset per BT | Text matrix |
| `Tlm` | `Td`, `TD`, `T*` | identity | Text line matrix |

Text state parameters persist across `q`/`Q` only for the _non-matrix_ parameters in PDF < 1.7; in ISO 32000-1 §9.3.1, text state is part of the graphics state and is saved/restored with `q`/`Q`. In practice, parsers should save the entire text state on `q`.

### 2.4 Text Object Operators

Text objects are delimited by `BT` (Begin Text) and `ET` (End Text). `BT` resets `Tm` and `Tlm` to the identity matrix; it does not reset other text state parameters. `ET` terminates the text object.

**Text positioning operators:**

- `tx ty Td` — move to start of next line: `Tlm = Tlm × [[1,0,0],[0,1,0],[tx,ty,1]]`; also sets `Tm = Tlm`
- `tx ty TD` — equivalent to `-ty TL; tx ty Td` (sets leading and moves)
- `T*` — equivalent to `0 -Tl Td`
- `a b c d e f Tm` — set `Tm` and `Tlm` directly to the matrix `[[a,b,0],[c,d,0],[e,f,1]]`; does not concatenate with CTM

**Text showing operators:**

- `(string) Tj` — show string; advance `Tm` by glyph widths
- `[(string|num)...] TJ` — show array; numeric elements adjust horizontal position by `-n/1000 × Tfs × Th/100` text units (negative values = kern tighter)
- `(string) '` — equivalent to `T*; (string) Tj`
- `aw ac (string) "` — equivalent to `aw Tw; ac Tc; (string) '`

---

## 3. Coordinate Systems and Text Position

### 3.1 Spaces

- **Device space** — physical output device pixels
- **User space** — default is 1 unit = 1/72 inch at 72 DPI
- **Text space** — defined by `Tm` concatenated with CTM: `Tspace_to_device = Tm × CTM`
- **Glyph space** — defined by the font; for Type 1 fonts, 1000 units = 1 unit in text space scaled by `Tfs`

### 3.2 CTM

The CTM maps user space to device space. It is initialized from the page's `/MediaBox` and any `/Rotate` entry. `cm` concatenates a new matrix: `CTM_new = matrix × CTM_old`. The CTM is a 6-element affine transform `[a b c d e f]` representing:

```
| a  b  0 |
| c  d  0 |
| e  f  1 |
```

Point transformation: `[x' y' 1] = [x y 1] × M`.

### 3.3 Text Matrix Update per Glyph

After rendering each glyph with width `w` (in glyph units, normalized by dividing by 1000), the text matrix advances:

```
tx = (w/1000 × Tfs + Tc + (is_space ? Tw : 0)) × Th/100
Tm = [[1,0,0],[0,1,0],[tx,0,1]] × Tm
```

For vertical writing mode, the advance is in the y direction with analogous computation using `/DW2` and `/W2`. The text matrix `Tm` is a local variable within the text object; it is not preserved across `BT`/`ET`.

---

## 4. Character Spacing, Word Spacing, and Horizontal Scaling

- **`Tc` (character spacing):** Added to the advance width of every glyph, including space. Units are unscaled text space units (before Tz scaling). Applied after glyph advance, before the next glyph.
- **`Tw` (word spacing):** Added to the advance width only for single-byte character code 0x20 (ASCII space). In multi-byte encodings and Type0/CIDFont fonts, word spacing applies only if the character code is exactly the single byte `0x20`; it does not apply to multi-byte space characters. Units are unscaled text space units.
- **`Tz` (horizontal scaling, operator `Tz`):** Scales the horizontal component of all glyph displacements and character/word spacing. Value is a percentage (100 = no scaling). Formally: `tx_scaled = tx × Tz/100`. This is applied before converting to user space.

The combined advance formula per glyph (horizontal writing):

```
advance = (w0/1000 × Tfs + Tc + Tw_if_space) × Tz/100
```

where `w0` is the glyph's horizontal width from the font (possibly overridden by `/Widths` array in the font dict) and `Tfs` is the font size from `Tf`.

---

## 5. Marked Content

### 5.1 Operators

Marked content allows semantic annotation of content stream regions:

- `tag BMC` — Begin Marked Content with tag name only
- `tag props BDC` — Begin Marked Content with property dictionary (inline dict or name referencing `/Properties` in the page/resource dict)
- `EMC` — End Marked Content (matching pop)
- `tag MP` — Marked Content Point (no extent)
- `tag props DP` — Marked Content Point with properties

Operators nest: each `BMC`/`BDC` must be matched by an `EMC`. They can span across content streams in a page's `/Contents` array only if the array forms a single logical stream (per ISO 32000-1 §14.6, marked content sequences must not span stream boundaries).

### 5.2 Tagged PDF (PDF 1.3+)

The document catalog's `/MarkInfo` dictionary signals tagged PDF: `/Marked true`. Tagged PDFs have a **structure tree** rooted at `/StructTreeRoot` in the catalog. Structure elements (SEs) are dictionaries with:

- `/S` — structure type (e.g., `/P`, `/H1`, `/Span`, `/Table`, `/TR`, `/TD`)
- `/K` — children: structure elements or **marked content references** (MCRs)
- `/Pg` — page reference
- `/P` — parent SE

An MCR references content stream marked content via `/Type /MCR`, `/MCID` (integer), and optionally `/Pg`. The `/MCID` matches the integer in `BDC` property dicts (`/MCID n`). The `/ParentTree` in `/StructTreeRoot` is a number tree mapping MCIDs to their parent structure elements, enabling reverse lookup from content stream to structure tree.

For text extraction preserving logical reading order, parse the structure tree top-down and use MCID mappings to retrieve text spans from the content stream — rather than extracting text in paint order from the stream directly. This is essential for multi-column layouts and reflow-capable documents.

### 5.3 ActualText and Alt

Structure elements and marked content property dicts may carry `/ActualText` (a UTF-16BE or UTF-8 string providing the Unicode text that the content renders, overriding glyph-level decoding) and `/Alt` (alternate description, for accessibility). For extraction, `/ActualText` in a BDC property dict or on a structure element takes precedence over decoded glyph text within the marked region.

---

## 6. PDF Version Deltas Relevant to Text Extraction

### PDF 1.2 (Acrobat 3)
Introduced CMaps for CIDFont encoding and ToUnicode CMaps. The `/ToUnicode` stream in a font dictionary maps character codes to Unicode codepoints (using `beginbfchar`/`endbfchar`/`beginbfrange`/`endbfrange` operators in the CMap syntax). Without `/ToUnicode`, extraction must fall back to font encoding and glyph name heuristics.

### PDF 1.3 (Acrobat 4)
Introduced **Tagged PDF**: `/MarkInfo`, `/StructTreeRoot`, marked content operators (`BMC`/`BDC`/`EMC`), and the `/ActualText` attribute. Also added `/TT` (TrueType) and Type0 composite fonts as first-class. Digital signatures added.

### PDF 1.4 (Acrobat 5)
Transparency model (`/Group` with `/S /Transparency`), soft masks. Does not directly affect text extraction mechanics, but transparency groups introduce nested content streams (the group's `/Contents` stream) that must be recursed.

### PDF 1.5 (Acrobat 6)
Object streams (`/ObjStm`) and cross-reference streams (`/XRef`). **Critical for parsing**: the traditional xref table may be absent entirely. Also introduced JBIG2 and JPEG2000 image filters. Optional content groups (OCGs, `/OCProperties`) can mark content as conditionally visible — text inside an invisible OCG should typically be excluded from extraction or flagged.

### PDF 1.6 (Acrobat 7)
AES encryption (RC4 still supported). No structural changes to text representation.

### PDF 1.7 (Acrobat 8) / ISO 32000-1:2008
Codified as an ISO standard. Added `/Extensions` dictionary in the catalog for third-party extensions. Formalized the full specification of all features up to this version. `/ToUnicode` CMaps are now the authoritative mechanism for character-to-Unicode mapping.

### PDF 2.0 / ISO 32000-2:2020
- `/Info` metadata dictionary deprecated (XMP stream in `/Metadata` is now the sole authoritative metadata source)
- `/EncryptMetadata` behavior clarified
- New encryption algorithms (AES-256 with revised key derivation)
- Removed several deprecated features (Type1C embedded font variants as standalone, LZWDecode with early-change=0 for new files)
- `/ActualText` semantics clarified: if present, the full Unicode string it provides replaces all glyph-level text decoding for that span
- Structure namespace concept added (structure types are now namespace-qualified)
- Unambiguous specification that `/ToUnicode` CMap entries for ligatures (e.g., `fi` ligature → `fi`) use `beginbfchar` with a multi-character destination string

---

## 7. Font Encoding and ToUnicode

Font dicts (`/Type /Font`) encode characters via:

1. **`/Encoding`** — simple fonts only; either a name (`/WinAnsiEncoding`, `/MacRomanEncoding`, `/StandardEncoding`, `/PDFDocEncoding`) or a diff array (`/Differences`)
2. **`/ToUnicode`** — CMap stream present in all well-formed tagged PDFs; maps `<hex>` character codes to `<hex>` Unicode codepoints in UTF-16BE
3. **Type0 (composite) fonts** — use `/Encoding /Identity-H` or a CMap name; character codes are 1 or 2 bytes; glyph IDs via the `/DescendantFonts` array → CIDFont → `/CIDToGIDMap`
4. **Fallback** — glyph name → Adobe Glyph List (AGL) lookup; last resort for fonts lacking `/ToUnicode`

The CMap stream grammar relevant to `/ToUnicode`:
- `begincodespacerange` / `endcodespacerange` — defines valid code ranges
- `beginbfchar n` / `endbfchar` — n mappings of `<src_code> <dst_unicode>`
- `beginbfrange n` / `endbfrange` — range mappings: `<start> <end> <base>` (sequential) or `<start> <end> [str0 str1 ...]` (array form)

Destination strings are UTF-16BE byte sequences. A single src code mapping to a multi-char destination (e.g., ligature) is valid and must be handled.

---

## 8. Key Dictionary Keys Summary

| Dict | Key | Type | Notes |
|------|-----|------|-------|
| Page | `/Contents` | stream or array | Content streams |
| Page | `/Resources` | dict | Fonts, XObjects, Properties, ColorSpaces |
| Resources | `/Font` | dict | Font name → font dict |
| Resources | `/Properties` | dict | Tag name → property dict (for BDC) |
| Font | `/Type /Font` | name | Always `/Font` |
| Font | `/Subtype` | name | `/Type1`, `/TrueType`, `/Type0`, `/Type3`, `/CIDFontType0`, `/CIDFontType2` |
| Font | `/BaseFont` | name | PostScript font name |
| Font | `/Encoding` | name or dict | Character encoding |
| Font | `/ToUnicode` | stream | CMap stream for Unicode mapping |
| Font | `/Widths` | array | Glyph widths for simple fonts |
| Font | `/FirstChar`, `/LastChar` | integer | Range for `/Widths` |
| Type0 Font | `/DescendantFonts` | array | One-element array of CIDFont dict |
| CIDFont | `/DW` | integer | Default width (default: 1000) |
| CIDFont | `/W` | array | Individual/range width overrides |
| Catalog | `/MarkInfo` | dict | `/Marked true` for tagged PDF |
| Catalog | `/StructTreeRoot` | dict | Structure tree root |
| StructTreeRoot | `/ParentTree` | number tree | MCID → structure element |
| ObjStm | `/N` | integer | Number of compressed objects |
| ObjStm | `/First` | integer | Offset of first object body |
