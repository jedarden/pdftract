# CMap Format and CID Encoding

## Purpose

CMap files are the primary mechanism by which PDF maps character codes to Unicode codepoints. Mishandling them produces garbled output, missing characters, or silent data loss — particularly for CJK text, composite fonts, and any font with a `ToUnicode` stream. This document describes the CMap file format and the parsing requirements an implementation must satisfy.

The authoritative specifications are the Adobe CMap and CIDFont Files Specification (version 1.0, 2012) and ISO 32000-2 (PDF 2.0).

---

## 1. CMap File Structure

A CMap file is a PostScript-like text file, not a binary format. Its structure divides into a header block, a body, and a footer.

**Header block:**

```
%!PS-Adobe-3.0 Resource-CMap
%%DocumentNeededResources: ProcSet (CIDInit)
%%IncludeResource: ProcSet (CIDInit)
%%BeginResource: CMap (Adobe-GB1-UCS2)
%%Title: (Adobe-GB1-UCS2 Adobe GB1 0)
%%Version: 1.000
%%EndComments
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CMapName /Adobe-GB1-UCS2 def
/CMapType 1 def
/CIDSystemInfo
  << /Registry (Adobe)
     /Ordering (GB1)
     /Supplement 0
  >> def
```

**Key header fields:**

- `/CMapName` — the name of this CMap as a PostScript name literal. Used to identify it in `usecmap` references.
- `/CMapType` — integer:
  - `0` = code-to-glyph (maps character codes to CIDs; used by Type 0 composite fonts)
  - `1` = Unicode-to-glyph (maps Unicode values to CIDs; used for ToUnicode reverse lookups)
  - `2` = code-to-Unicode (maps character codes to Unicode; the most common type for text extraction)
- `/CIDSystemInfo` — a dictionary with three required keys: `Registry` (e.g., `Adobe`), `Ordering` (e.g., `GB1`, `Japan1`, `CNS1`, `Korea1`), and `Supplement` (integer revision level). This identifies the glyph collection the CMap targets.
- `/WMode` — `0` for horizontal writing (default), `1` for vertical writing. Horizontal CMaps are sufficient for Unicode extraction; vertical CMaps only affect glyph selection.

**Footer:**

```
endcmap
CMapName usecmap
end
end
%%EndResource
%%EOF
```

The `CMapName usecmap` in the footer installs the just-defined CMap into the PostScript resource dictionary — a no-op for PDF parsers, but syntactically required.

---

## 2. Codespace Ranges

`begincodespacerange` / `endcodespacerange` defines which byte sequences are valid character codes in this CMap. Each entry is a pair of equal-length hex strings:

```
begincodespacerange
<00>   <FF>
<8140> <FEFE>
endcodespacerange
```

The first hex string is the lower bound, the second is the upper bound. A byte sequence is a valid character code if it falls within any range (byte-by-byte comparison, same length). The length of the hex string (in bytes) determines how many bytes constitute one character code for that range.

**Encoding-specific examples:**

- Single-byte: `<00> <FF>` — codes 0x00–0xFF are each one character.
- Shift-JIS (double-byte lead bytes): `<8140> <FEFE>` alongside `<00> <7E>` for the ASCII portion.
- GB18030 and Big5 use mixed-length codespaces: some codes are 1 byte, others are 2, and GB18030 also has 4-byte codes.

**Critical point:** the codespace is not an encoding definition in itself — it is a scan grammar. The parser reads raw bytes from the content stream and uses the codespace ranges to segment them into character codes. This is not UTF-8 and cannot reuse a Unicode decoder.

---

## 3. begincidchar / endcidchar

Maps individual character codes to CID integers (in Type 0 CMaps) or Unicode codepoints (in ToUnicode CMaps):

```
begincidchar
<0041> 65
<4E2D> 20013
endcidchar
```

In a `ToUnicode` CMap, the value is a hex string rather than a decimal integer:

```
begincidchar
<0041> <0041>
endcidchar
```

Here `<0041>` on the right is a UTF-16BE encoded Unicode value. For characters above U+FFFF, the right-hand side is a surrogate pair encoded in UTF-16BE: `<D840DC00>` for U+20000 (CJK Extension B character). A parser must detect 4-byte right-hand hex strings and decode them via the UTF-16BE surrogate-pair formula:

```
high = (value >> 16) & 0xFFFF;
low  = value & 0xFFFF;
codepoint = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
```

---

## 4. begincidrange / endcidrange

Maps a contiguous range of character codes to a contiguous range of CIDs or Unicode values:

```
begincidrange
<0041> <005A> <0041>
endcidrange
```

For each code `c` in `[start, end]`, the mapped value is `start_value + (c - start)`. This is the most compact form and covers the bulk of CJK character mappings.

A range entry may also specify an array on the right side:

```
begincidrange
<xx> <yy> [<val1> <val2> <val3>]
endcidrange
```

The array must have exactly `(end - start + 1)` elements. Each element maps explicitly to the code at that offset. This handles non-contiguous Unicode assignments within a contiguous code range, which occurs in vendor character sets where the standard Unicode mapping is irregular.

---

## 5. beginbfchar / endbfchar and beginbfrange / endbfrange

The `bf` (base-font) variants appear exclusively in `ToUnicode` CMaps. Their right-hand side is always a Unicode string, not a CID integer.

**bfchar** maps a single code to a Unicode string:

```
beginbfchar
<FB01> <00660069>
endbfchar
```

`<00660069>` is the UTF-16BE encoding of the two-character string "fi" (U+0066, U+0069). This means the character code `FB01` is a ligature that expands to two Unicode codepoints. An implementation must produce both codepoints — not just the first — in the output string.

**bfrange** maps a code range to Unicode strings:

```
beginbfrange
<0041> <005A> <0041>
endbfrange
```

When the right side is a single hex string, the Unicode value increments by 1 per code step, just as in `cidrange`. When the right side is an array, each element is a full Unicode string for that code offset.

The distinction between `bfchar`/`bfrange` and `cidchar`/`cidrange` is purely semantic: `bf` variants target Unicode text, `cid` variants target glyph indices. A `ToUnicode` CMap will contain only `bf` variants. A code-to-CID CMap for a composite font will contain only `cid` variants.

---

## 6. usecmap — CMap Inheritance

A CMap may delegate to a base CMap:

```
/UniJIS-UTF16-H usecmap
```

This means: for any character code not mapped in the current CMap, consult the named CMap. Inheritance chains can be several levels deep. The predefined CMaps (embedded in conforming PDF viewers) must be known to `pdftract` without requiring them to be embedded in the PDF.

**Required predefined CMap names for CJK support:**

- `Identity-H`, `Identity-V` — maps each 2-byte code `<XXXX>` directly to CID `XXXX`; codespace `<0000>` to `<FFFF>`.
- Japanese: `90ms-RKSJ-H`, `90ms-RKSJ-V`, `90msp-RKSJ-H`, `UniJIS-UTF16-H`, `UniJIS-UTF16-V`, `UniJIS2004-UTF16-H`, `UniJIS-UCS2-H`, `UniJIS-UCS2-V`, `H`, `V`
- Simplified Chinese: `UniGB-UCS2-H`, `UniGB-UCS2-V`, `UniGB-UTF16-H`, `UniGB-UTF16-V`, `GBK-EUC-H`, `GBK-EUC-V`, `GBKp-EUC-H`, `GBKp-EUC-V`, `GBK2K-H`, `GBK2K-V`, `GB-EUC-H`, `GB-EUC-V`
- Traditional Chinese: `UniCNS-UCS2-H`, `UniCNS-UCS2-V`, `UniCNS-UTF16-H`, `UniCNS-UTF16-V`, `B5pc-H`, `B5pc-V`, `ETen-B5-H`, `ETen-B5-V`, `CNS-EUC-H`, `CNS-EUC-V`
- Korean: `UniKS-UCS2-H`, `UniKS-UCS2-V`, `UniKS-UTF16-H`, `UniKS-UTF16-V`, `KSCms-UHC-H`, `KSCms-UHC-V`, `KSCms-UHC-HW-H`, `KSCms-UHC-HW-V`, `KSCpc-EUC-H`

`Identity-H` is the most common: when a PDF uses `Encoding = Identity-H`, every 2-byte code in the content stream is its own CID, and the `ToUnicode` CMap (if present) provides the code-to-Unicode translation layered on top.

---

## 7. Parsing Mixed-Length Codespace

The character code segmentation algorithm must be implemented as an explicit state machine — it cannot be delegated to `str::from_utf8` or any fixed-width integer read.

**Algorithm:**

```
fn read_code(bytes: &[u8], pos: &mut usize, codespace: &[CodespaceRange]) -> Option<u32> {
    let mut accum: u32 = 0;
    let mut len: usize = 0;
    while len < 4 && *pos < bytes.len() {
        accum = (accum << 8) | bytes[*pos] as u32;
        *pos += 1;
        len += 1;
        for range in codespace.iter().filter(|r| r.byte_len == len) {
            if accum >= range.low && accum <= range.high {
                return Some(accum);
            }
        }
        // check if any range of this length or longer might still match
        if !codespace.iter().any(|r| r.byte_len > len) {
            break;
        }
    }
    None  // error: no codespace range matched; caller advances pos by 1 and retries
}
```

On a `None` return, the outer loop must advance `pos` by 1 (not by `len`) and retry. This is the error recovery mandated by the specification for malformed content streams.

The codespace ranges must be sorted by `byte_len` and stored separately per length to make the inner loop O(ranges\_at\_this\_length) rather than O(all\_ranges).

---

## 8. ToUnicode CMap in Practice

The `ToUnicode` CMap is attached to a font dictionary as a stream:

```
/ToUnicode <stream-object-reference>
```

The stream contains a complete CMap file. The parser must handle the full file syntax, not just extract mapping sections.

**Authoring defects that must be handled as partial-mapping cases:**

- **Empty sections:** `beginbfchar\nendbfchar` with zero entries is legal; do not treat it as a parse error.
- **U+0000 / U+FFFD sentinels:** Some tools map unmapped codes to `<0000>` or `<FFFD>`. Discard these — do not emit NUL or replacement characters into the extracted text.
- **Incomplete coverage:** A ToUnicode CMap may only cover a subset of the codes used in the content stream. Fall through to glyph-name-based Unicode recovery for unmapped codes.
- **Wrong code lengths:** The hex string length of a code in a `bfchar` entry may differ from what the codespace declares. If the mismatch is detectable, prefer the codespace definition for segmentation and use the `bfchar` value for the mapping.

---

## 9. Vertical CMaps

Vertical CMaps (`WMode 1`) map the same character codes as their horizontal equivalents but to different glyph IDs. The glyphs are rotated or have adjusted metrics for vertical typesetting. The Unicode value of a character does not change when transitioning from horizontal to vertical layout — only the rendered glyph differs.

For text extraction purposes: if the content stream is processed under a vertical CMap (detected via `WMode 1` in the CMap header or `/WMode 1` in the font dictionary), apply the corresponding horizontal CMap for Unicode mapping. The vertical CMap (`UniJIS-UTF16-V`) inherits from its horizontal counterpart (`UniJIS-UTF16-H`) via `usecmap`; the inherited horizontal mappings cover Unicode lookup. No special vertical-specific Unicode logic is needed.

---

## 10. Implementation: CMap Parser in Rust

### Tokenizer

The tokenizer must handle PostScript-like syntax:

- **Hex strings:** `<4E2D>` — collect bytes between `<` and `>`, ignoring whitespace. Odd-length hex strings should be right-padded with `0`.
- **Decimal integers:** `65`, `20013` — standard integer parsing.
- **Name literals:** `/CMapName` — the `/` is stripped; the remainder is the name.
- **Keywords:** `begincmap`, `endcmap`, `begincodespacerange`, `begincidchar`, `beginbfchar`, `beginbfrange`, `begincidrange`, `usecmap`, `def`, and their `end*` counterparts.
- **Comments:** `%` to end of line — skip.
- **Arrays:** `[` and `]` delimit value arrays in range entries.

### Core Structs

```rust
pub struct CodespaceRange {
    pub byte_len: usize,
    pub low: u32,
    pub high: u32,
}

pub struct BfRange {
    pub start: u32,
    pub end: u32,
    pub target: BfRangeTarget,
}

pub enum BfRangeTarget {
    StartCode(u32),              // increment from this Unicode value
    Array(Vec<String>),          // explicit per-code Unicode strings
}

pub struct CMap {
    pub name: String,
    pub cmap_type: u8,
    pub wmode: u8,
    pub codespace: Vec<CodespaceRange>,       // sorted by byte_len
    pub bf_char: HashMap<u32, String>,        // code → Unicode string
    pub bf_range: Vec<BfRange>,              // sorted by start for binary search
    pub usecmap: Option<String>,             // inherited CMap name
}
```

### decode Method

```rust
impl CMap {
    pub fn decode(&self, bytes: &[u8], base: Option<&CMap>) -> Vec<(u32, String)> {
        let mut out = Vec::new();
        let mut pos = 0;
        while pos < bytes.len() {
            match read_code(bytes, &mut pos, &self.codespace) {
                Some(code) => {
                    let unicode = self.lookup(code)
                        .or_else(|| base.and_then(|b| b.lookup(code)));
                    if let Some(s) = unicode {
                        out.push((code, s));
                    }
                    // if None: unmapped — caller may attempt glyph-name fallback
                }
                None => { pos += 1; }  // error recovery: skip one byte
            }
        }
        out
    }

    fn lookup(&self, code: u32) -> Option<String> {
        if let Some(s) = self.bf_char.get(&code) {
            return Some(s.clone());
        }
        // binary search bf_range by start
        let idx = self.bf_range.partition_point(|r| r.start <= code);
        if idx > 0 {
            let r = &self.bf_range[idx - 1];
            if code <= r.end {
                return Some(match &r.target {
                    BfRangeTarget::StartCode(base) => {
                        char::from_u32(base + (code - r.start))
                            .map(|c| c.to_string())
                            .unwrap_or_default()
                    }
                    BfRangeTarget::Array(arr) => {
                        arr.get((code - r.start) as usize).cloned().unwrap_or_default()
                    }
                });
            }
        }
        None
    }
}
```

### Inheritance and Predefined CMaps

Resolve `usecmap` by:

1. Checking whether the named CMap is embedded in the PDF (look up the `CMap` resource dictionary in the PDF's document catalog or page resource dictionaries).
2. Falling back to a built-in table of predefined CMap data compiled into the library. `Identity-H` and `Identity-V` must always be available as built-ins since they are extremely common and have trivial definitions.

Guard against circular references by tracking visited names in a `HashSet<String>` during resolution. A chain longer than 8 levels should be treated as an error.

### UTF-16BE String Conversion

When converting a right-hand hex string to a Rust `String`:

1. Interpret bytes as UTF-16BE code units.
2. Collect surrogate pairs: when a high surrogate (`0xD800`–`0xDBFF`) is followed by a low surrogate (`0xDC00`–`0xDFFF`), decode to a single codepoint.
3. Use `char::from_u32` and reject values that are not valid Unicode scalar values.
4. Concatenate all resulting `char` values into the `String`.

This handles both BMP characters and supplementary characters (above U+FFFF) correctly without relying on platform-specific wide-character APIs.
