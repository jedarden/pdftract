# PDF Fonts and Encoding: Technical Reference for Text Extraction

This document describes every font type found in PDF files, how character codes are decoded to Unicode, and the data structures a Rust extraction engine must interpret. References are to the PDF 1.7 specification (ISO 32000-1:2008) and Adobe technical notes where applicable.

---

## 1. Font Types

### 1.1 Type 1 (Simple Font)

Type 1 fonts originate from the Adobe Type 1 format, stored as PFB (binary) or PFA (ASCII) font programs. In a PDF the font dictionary has `/Subtype /Type1`.

**Glyph storage.** The font program is a PostScript charstring program. When embedded, it appears under `/FontDescriptor` as the stream value of `/FontFile` (Type 1 binary). The charstrings are keyed by glyph name, not by a numeric glyph ID.

**Character code interpretation.** A one-byte character code from the content stream is mapped through the font's `/Encoding` to a glyph name, then the glyph name is looked up in the charstring dictionary. See §3 for encoding details.

**Widths.** The `/Widths` array (required) contains `LastChar - FirstChar + 1` entries, each giving the horizontal advance width in text-space units (1/1000 em). `/FirstChar` and `/LastChar` define the range. Codes outside this range use `/MissingWidth` from the font descriptor.

**Standard 14 fonts.** PDF readers must implement the 14 standard Type 1 fonts (Helvetica, Times-Roman, Courier, Symbol, ZapfDingbats, and their variants) without an embedded font program. These are never embedded; the reader synthesizes metrics.

### 1.2 Type 3 (Simple Font)

`/Subtype /Type3`. Glyphs are defined as PDF content streams directly in the font dictionary under `/CharProcs`, a dictionary from glyph name to stream. There is no external font program.

**Character code interpretation.** One-byte code → glyph name via `/Encoding` → content stream in `/CharProcs`. Because glyph names are arbitrary (user-defined), there is often no reliable path to Unicode without a `/ToUnicode` CMap. If `/ToUnicode` is absent, extraction must fall back to glyph name heuristics or report the text as unresolvable.

**Widths.** `/Widths`, `/FirstChar`, `/LastChar` as in Type 1. Additionally, `/FontMatrix` transforms glyph-space coordinates; the default for Type 1 is `[0.001 0 0 0.001 0 0]`, but Type 3 fonts frequently use `[1 0 0 1 0 0]` with glyph streams drawn at full size.

### 1.3 TrueType (Simple Font)

`/Subtype /TrueType`. The embedded program is a TrueType font binary under `/FontFile2` in the font descriptor.

**Glyph storage.** Glyphs are stored by integer glyph ID (GID) inside the `glyf` table. The `cmap` table maps Unicode codepoints (or platform-specific codes) to GIDs.

**Character code interpretation.** One-byte code → glyph name via `/Encoding` → GID via the font's `cmap`. When the encoding is a standard PDF encoding (WinAnsiEncoding, MacRomanEncoding, etc.), the implementation maps code → Unicode codepoint → GID using `cmap` platform/encoding subtable (platform 3, encoding 1: Windows Unicode BMP). If the font's `cmap` contains only platform 1 (Macintosh), platform-specific code mappings apply. This is a common source of extraction errors.

**Widths.** Same `/Widths` array mechanism as Type 1. The `hmtx` TrueType table provides the authoritative advance widths; the PDF `/Widths` array should match but may differ in broken documents.

### 1.4 Type 0 (Composite Font)

`/Subtype /Type0`. This is the container for multi-byte (CJK and other large character set) text. The font dictionary has:

- `/Encoding` — a CMap name (e.g., `Identity-H`) or a stream containing a CMap program.
- `/DescendantFonts` — a one-element array holding a CIDFont dictionary.

**Character code interpretation.** The multi-byte content stream codes are fed through the CMap named in `/Encoding`, which maps character codes to CIDs. The CIDFont then maps CIDs to GIDs. See §4.

**Widths.** Widths are specified in the CIDFont descendant, not in the Type 0 dictionary itself.

### 1.5 CIDFont Type 0 (CFF-Based)

`/Subtype /CIDFontType0` inside a `/DescendantFonts` array. The font program is a CFF (Compact Font Format, also called Type 2 charstrings) font embedded under `/FontFile3` with `/Subtype /CIDFontType0C` or `/Subtype /OpenType`.

**Glyph storage.** CFF stores charstrings keyed by GID (integer index). GIDs map directly to charstrings; glyph names may or may not be present depending on the CFF variant.

**Widths.** The CIDFont dictionary uses `/DW` (default width, default 1000) and `/W` (array of per-CID widths). The `/W` syntax is: an array whose elements alternate between `c [w1 w2 ...]` (individual CIDs) and `c1 c2 w` (range with uniform width).

### 1.6 CIDFont Type 2 (TrueType-Based)

`/Subtype /CIDFontType2`. The embedded program is a TrueType or OpenType/TT font under `/FontFile2` (TrueType) or `/FontFile3` with `/Subtype /OpenType`.

**CID-to-GID mapping.** The `/CIDToGIDMap` entry in the CIDFont dictionary is critical:
- If the value is the name `/Identity`, CID equals GID directly (CID = GID).
- Otherwise it is a stream of 2×65536 bytes: the GID for CID `n` is the 16-bit big-endian value at byte offset `2n`.

**Widths.** Same `/DW` and `/W` mechanism as CIDFont Type 0.

### 1.7 OpenType in PDF

OpenType fonts are embedded as `/FontFile3` streams with `/Subtype /OpenType`. An OpenType font may contain either CFF outlines (`CFF` table present → CIDFont Type 0) or TrueType outlines (`glyf` table present → CIDFont Type 2). The handling follows the respective CIDFont rules. The PDF spec does not treat OpenType as a separate subtype; it is identified by the stream subtype.

---

## 2. Encoding Mechanisms

### 2.1 Predefined Encodings

The PDF spec defines four named encodings for simple fonts (§D.1–D.4, PDF 1.7):

| Name | Character set | Typical use |
|------|--------------|-------------|
| `StandardEncoding` | 229 glyphs from the Adobe standard | Default for Type 1 fonts that omit `/Encoding` |
| `MacRomanEncoding` | Mac OS Roman 256 code points | Older Mac-generated PDFs |
| `WinAnsiEncoding` | Windows-1252 (cp1252) | Windows-generated PDFs; most common |
| `MacExpertEncoding` | Expert font character set (fractions, small caps) | Rare; expert-set fonts |

`PDFDocEncoding` is a PDF-internal encoding used for text strings in the document catalog (info dictionary, annotations) but **not** for font encoding; it must not be confused with font encodings. It extends Latin-1 by filling 0x18–0x1F and 0x80–0x9F with additional characters.

`Symbol` and `ZapfDingbats` fonts use built-in symbol encodings defined in the respective AFM files. They do **not** use the standard named encodings; their code-to-glyph mapping is private and must be looked up against the font-specific tables provided in PDF Annex D.

### 2.2 The `/Encoding` Dictionary and `/Differences` Array

When a font's `/Encoding` value is a dictionary rather than a name, the dictionary may contain:

- `/Type /Encoding` (optional)
- `/BaseEncoding` — a name (`StandardEncoding`, `MacRomanEncoding`, `WinAnsiEncoding`) designating the starting table. If absent, the base depends on font type (Type 1 defaults to built-in; others to StandardEncoding).
- `/Differences` — an array of the form `[code name code name ...]` or `[code name name name ...]`. Starting from the numeric code, each following name overrides successive slots. Example: `[32 /space /exclam /quotedbl]` overrides slots 32, 33, 34.

Encoding resolution algorithm for simple fonts:
1. Start from the BaseEncoding table.
2. Apply each `/Differences` entry, replacing the glyph name at the given code position.
3. Resolve each resulting glyph name to Unicode via the Adobe Glyph List (§5).

### 2.3 Symbol and ZapfDingbats

These two standard fonts carry the `Symbolic` flag (bit 3 of `/Flags` in the font descriptor). Their encoding is defined entirely by the glyph names in the font program; the predefined named encodings do not apply. Extraction must use the AGL or the font's own encoding vector. ZapfDingbats glyph names are documented in the PDF spec Annex D.6.

---

## 3. ToUnicode CMaps

### 3.1 CMap Stream Format

A ToUnicode CMap is a PostScript-inspired stream embedded directly in the PDF. The structure (PDF §9.10.3):

```
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo 3 dict dup begin
  /Registry (Adobe) def
  /Ordering (UCS) def
  /Supplement 0 def
end def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
4 beginbfchar
<0041> <0041>          % code 0x41 → U+0041 (A)
<00A0> <00A0>
<F001> <FB01>          % code 0xF001 → U+FB01 (fi ligature)
<F002> <FB02>          % code 0xF002 → U+FB02 (fl ligature)
endbfchar
1 beginbfrange
<0061> <007A> <0061>   % codes 0x61–0x7A → U+0061–U+007A (a–z)
endbfrange
endcmap
CMapName currentdict /CMap defineresource pop
end
end
```

**`beginbfchar` / `endbfchar`:** Each entry is a pair `<src-code> <dst-unicode>`. The destination is UTF-16BE hex bytes; a surrogate pair encodes a codepoint above U+FFFF.

**`beginbfrange` / `endbfrange`:** Range `<lo> <hi> <start-unicode>` maps a contiguous code range to a contiguous Unicode range. Alternatively, `<lo> <hi> [<u1> <u2> ...]` maps each code in the range to the corresponding Unicode string in the array.

**`begincidrange` / `endcidrange`:** Used in Type 0 CMaps (not ToUnicode) to map codes to CID ranges; see §4.

### 3.2 Embedding in PDF

The ToUnicode CMap appears as the value of the `/ToUnicode` key in the font dictionary (both simple and composite fonts). It is a stream object, usually with `/Filter /FlateDecode`.

### 3.3 When ToUnicode is Absent or Wrong

**Absent:** Extraction must fall back to encoding → glyph name → AGL lookup (simple fonts) or CID-to-Unicode tables derived from the predefined CMap ordering (composite fonts). Many PDFs produced by older tools (TeX-based pipelines, some CAD exporters) omit `/ToUnicode`; the AGL fallback is the only reliable option.

**Wrong or incomplete:** Some generators emit a `/ToUnicode` CMap with missing entries or incorrect mappings. A bfchar entry with destination `<0000>` or `<FFFD>` signals an intentionally unmapped glyph. An implementation should not blindly trust all mappings; NUL and replacement-character destinations should be treated as absent.

**Implications for extraction:** Without a `/ToUnicode` map, ligature glyphs (`fi`, `fl`, `ffi`, etc.) will be decoded as their AGL expansions (multi-character strings), which is usually correct. Private Use Area (PUA) codepoints require a `/ToUnicode` map to resolve; without one the extracted text should preserve the PUA codepoint but flag it as unresolved.

---

## 4. CID-to-GID Mapping (Composite Fonts)

### 4.1 Decoding Path

For a Type 0 composite font, the decoding pipeline is:

```
content-stream bytes
    → CMap (named in /Encoding)
    → CID
    → GID (via CIDToGIDMap or CFF index)
    → glyph outline
```

The `/Encoding` CMap converts multi-byte character codes (1–4 bytes) to CIDs. The CMap may be:
- A name referring to a predefined CMap (see §4.2).
- A stream object containing a CMap program.

### 4.2 Predefined CMaps

Adobe distributes predefined CMaps for CJK encodings (PDF Annex M). Key examples:

| Name | Script | Code space | Notes |
|------|--------|-----------|-------|
| `Identity-H` | any (horizontal) | 2-byte | CID = code (identity) |
| `Identity-V` | any (vertical) | 2-byte | CID = code, vertical writing |
| `90ms-RKSJ-H` | Japanese | Shift-JIS | Maps SJIS codes → Adobe-Japan1 CIDs |
| `GBK-EUC-H` | Simplified Chinese | GBK/EUC | Maps GBK → Adobe-GB1 CIDs |
| `UniGB-UTF16-H` | Simplified Chinese | UTF-16BE | Unicode input → Adobe-GB1 CIDs |
| `UniJIS-UTF16-H` | Japanese | UTF-16BE | Unicode input → Adobe-Japan1 CIDs |

For `Identity-H`/`Identity-V`, the CID equals the raw 2-byte code value, and if `/CIDToGIDMap /Identity`, the GID equals the CID. These are the simplest cases for TrueType-based CIDFonts.

### 4.3 CIDSystemInfo

Every CIDFont and its associated CMap must declare `/CIDSystemInfo`, a dictionary with `/Registry` (string), `/Ordering` (string), and `/Supplement` (integer). This identifies the CID character collection, e.g., Adobe-Japan1-6. The CIDFont and its CMap must share the same Registry and Ordering. Implementations should use this to select fallback Unicode tables when `/ToUnicode` is absent (Adobe publishes CID→Unicode mappings for its standard collections).

---

## 5. Glyph Name to Unicode (Adobe Glyph List)

### 5.1 The AGL

The Adobe Glyph List (AGL, `aglfn.txt`, version 1.7) maps glyph names to Unicode scalar values. An implementation should embed the AGL as a static hash table (approximately 4,000 entries).

**Algorithmic fallback** (AGL specification §2): If a glyph name is not in the AGL table:
1. Strip any trailing `.<suffix>` (e.g., `A.sc` → `A`).
2. If the name starts with `uni`, parse the following hex digits as UTF-16BE codepoint(s): `uni0041` → U+0041.
3. If the name starts with `u`, parse the following hex as a Unicode scalar: `u1F600` → U+1F600.
4. If none of the above, the glyph is unmapped.

**Ligatures.** `fi` → U+FB01, `fl` → U+FB02, `ffi` → U+FB03, `ffl` → U+FB04. These are single AGL entries mapping to single Unicode codepoints. Many extraction engines prefer to expand ligatures to their component characters (fi → "fi") for searchability; this is a policy choice, not a spec requirement.

**`.notdef`.** The glyph named `.notdef` is the fallback glyph for unmapped codes. It has no Unicode mapping. Extractors should silently skip or emit U+FFFD for `.notdef`.

**`afii` names.** Legacy glyph names starting with `afii` (e.g., `afii57506`) appear in older Arabic and Hebrew fonts. The AGL maps these to their correct Unicode codepoints; no special handling beyond AGL lookup is needed.

---

## 6. Font Descriptors

The `/FontDescriptor` dictionary (§9.8, PDF 1.7) is referenced by the font dictionary via `/FontDescriptor`. It provides metrics and the embedded font binary.

### 6.1 Key Entries

| Key | Type | Description |
|-----|------|-------------|
| `/FontName` | name | PostScript name of the font |
| `/FontBBox` | rectangle | Glyph bounding box in glyph-space units |
| `/Flags` | integer | Bitfield describing font characteristics |
| `/ItalicAngle` | number | Dominant italic angle in degrees |
| `/Ascent` | number | Maximum ascent above baseline |
| `/Descent` | number | Maximum descent below baseline (negative) |
| `/CapHeight` | number | Height of capital letters |
| `/XHeight` | number | Height of lowercase letters |
| `/StemV` | number | Dominant vertical stem width |
| `/FontFile` | stream | Type 1 PFB data |
| `/FontFile2` | stream | TrueType binary |
| `/FontFile3` | stream | CFF, OpenType, or CIDFontType0C binary (identified by stream `/Subtype`) |

### 6.2 Flags Bitfield

The `/Flags` integer is a 32-bit field; bits are numbered from 1 (LSB). Key bits:

| Bit | Mask | Meaning |
|-----|------|---------|
| 1 | 0x0001 | FixedPitch |
| 2 | 0x0002 | Serif |
| 3 | 0x0004 | Symbolic — font uses a private encoding; standard encodings do not apply |
| 4 | 0x0008 | Script (cursive) |
| 6 | 0x0020 | Nonsymbolic — font uses a standard Latin encoding |
| 7 | 0x0040 | Italic |
| 17 | 0x10000 | AllCap |
| 18 | 0x20000 | SmallCap |
| 19 | 0x40000 | ForceBold |

The `Symbolic` (bit 3) and `Nonsymbolic` (bit 6) flags are mutually exclusive and affect encoding resolution: a symbolic font's encoding is its own built-in table; a nonsymbolic font follows the standard named encoding fallback rules.

### 6.3 Inferring Unicode When CMap Data Is Absent

When both `/ToUnicode` and a useful `/Encoding` are missing, the following heuristics apply, in order:
1. If the embedded font is TrueType (`/FontFile2`) and the `/Flags` `Nonsymbolic` bit is set, use the font's `cmap` table with the `WinAnsiEncoding` assumption (platform 3, encoding 1).
2. If the font is CFF (`/FontFile3` with `/Subtype /CIDFontType0C`), the CFF `charset` table may supply glyph names; apply AGL.
3. If `/FontName` identifies a known standard font (e.g., `Symbol`, `ZapfDingbats`), apply the font-specific encoding table from PDF Annex D.
4. Otherwise, emit PUA codepoints or U+FFFD and flag the text as requiring post-processing.

The font descriptor `/FontBBox` and `/Flags` provide no path to Unicode; they are useful only for layout heuristics (detecting whitespace, line boundaries) when Unicode resolution fails.

---

## Appendix: Key Dictionary Locations

```
/Font dictionary
  /Subtype               → Type1 | Type3 | TrueType | Type0 | CIDFontType0 | CIDFontType2
  /Encoding              → name or dictionary (simple); CMap name or stream (Type0)
  /ToUnicode             → stream (CMap program)
  /FontDescriptor        → dictionary
    /Flags               → integer (bitfield)
    /FontFile            → stream (Type 1)
    /FontFile2           → stream (TrueType)
    /FontFile3           → stream (CFF/OpenType; /Subtype in stream dict)
  /Widths                → array (simple fonts)
  /FirstChar             → integer
  /LastChar              → integer
  /DescendantFonts       → array [ CIDFont dict ] (Type0 only)

CIDFont dictionary (inside /DescendantFonts)
  /Subtype               → CIDFontType0 | CIDFontType2
  /CIDSystemInfo         → dict (/Registry /Ordering /Supplement)
  /DW                    → integer (default advance width)
  /W                     → array (per-CID widths)
  /CIDToGIDMap           → /Identity or stream (CIDFontType2 only)
  /FontDescriptor        → dictionary (as above)
```

---

*Spec references: ISO 32000-1:2008 §9 (Fonts), §D (Character Sets), §M (Predefined CMaps); Adobe Glyph List Specification v1.7; Adobe Type 1 Font Format (Black Book); Adobe CMap and CIDFont Files Specification v1.0.*
