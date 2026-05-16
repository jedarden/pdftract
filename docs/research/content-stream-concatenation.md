# Content Stream Concatenation and Resource Resolution in PDF

## Overview

PDF text extraction depends on two coupled mechanisms: correct reassembly of content streams and correct resolution of resource names within those streams. Both are more complex than a naïve reading of the spec suggests. This document covers each mechanism in the depth required for a correct Rust implementation.

---

## 1. Single vs. Multiple Content Streams

A page dictionary's `/Contents` entry is either a single indirect reference to a stream object or an array of such references (PDF 1.7 spec §7.8.2). Both forms are valid; generators choose based on workflow — incremental update tools often append a new content stream rather than rewriting the existing one.

```
% Single stream
/Contents 42 0 R

% Array of streams
/Contents [42 0 R  43 0 R  44 0 R]
```

When `/Contents` is an array, the streams are logically concatenated in order to form one content stream. The spec mandates that a single space or newline (0x0A) be inserted between adjacent streams before parsing. The reason is token-boundary safety: a stream may end with the byte sequence `BT` (no trailing whitespace) and the next stream may begin with `q`. The raw concatenation produces `BTq`, which is a single unrecognized operator token. Inserting `0x0A` yields `BT\nq` — two valid tokens. This newline must be injected regardless of whether the adjacent bytes appear to need it; looking ahead to determine whether the boundary is safe is not reliable in the general case.

---

## 2. Content Stream Concatenation Semantics

After concatenation (with injected newlines), the resulting byte sequence is parsed as a **single logical content stream**. The PDF graphics model is stateful, and that state is continuous across the boundary between adjacent streams:

- **Graphics state stack**: A `q` operator in stream N pushes a graphics state entry; the matching `Q` may appear in stream N+1 or any later stream. The stack depth is not reset between streams.
- **Text object**: A `BT` in stream N without a matching `ET` before stream N ends is technically non-conforming, but common. The parser must carry the text object state (current font, text matrix, text line matrix) across the boundary. Treat encountering `BT` without a prior `ET` as an implicit end of the previous text object followed by the start of a new one — but do not reset state that `ET` would not reset (e.g., the graphics state stack depth).
- **Marked-content nesting**: `BMC`/`BDC` and `EMC` markers may straddle stream boundaries (see §10 below). The nesting counter must persist across boundaries.

The implementation consequence: the content stream reader must operate on a logical `ContentStreamReader` abstraction that owns an iterator over `(stream_index, byte_offset, byte)` tuples rather than a flat `&[u8]`. The newline injection happens at the seam between streams, not in the stored data.

---

## 3. Resources Dictionary Resolution

Every named resource referenced in a content stream — fonts, XObjects, color spaces, etc. — is resolved through a `/Resources` dictionary. The lookup path follows the page tree (PDF 1.7 §7.7.3.4):

1. Check the page dictionary's own `/Resources`.
2. If absent or the name is not found there, walk up through parent `/Pages` nodes in order, checking each `/Resources` entry.
3. The **first** definition found wins — page-level overrides parent-level.

Some generators omit per-page `/Resources` entirely and rely on the root `/Pages` node's `/Resources`. This is permitted by the spec and must be handled. A correct implementation resolves the inherited resource dictionary chain at page load time, before processing any content stream, and caches the result as a merged view (do not re-walk the tree on every resource lookup).

The page-level resource dictionary is the outermost scope in the resource name stack (§6 below).

---

## 4. Form XObject Resources

A Form XObject is an XObject stream with `/Subtype /Form`. It has its own `/Resources` dictionary (PDF 1.7 §8.10.2). When the `Do` operator invokes a Form XObject:

- **Resource lookup inside the Form's content stream uses the Form's `/Resources` exclusively.**
- Page resources are **not** visible inside the Form. The Form's content stream is a self-contained resource scope.
- A font named `/F1` in the page's Resources and a font named `/F1` in the Form's Resources are independent entries that may refer to different font objects.

The common extraction failure mode: a `Tf /F1 12` inside a Form XObject resolves against the Form's `/Resources/Font`, not the page's. If the font is not declared in the Form's Resources, the resource lookup fails — even if the page declares the same name. The PDF spec does not permit fallback to the parent scope for Form XObjects.

---

## 5. Type 3 Font Glyph Stream Resources

A Type 3 font defines each glyph as a content stream stored under `/CharProcs`. Each glyph stream is parsed in its own resource scope: the `/Resources` entry in the **Type 3 font dictionary** (PDF 1.7 §9.6.5).

- Glyph streams do not use the page resources.
- Glyph streams do not use the Form XObject resources (even if the Type 3 font was invoked from inside a Form).
- Only the Type 3 font's own `/Resources` applies.

This creates a three-level nesting: page → Form XObject → Type 3 font glyph, each with its own resource scope. The resource name stack (§6) must support all three levels simultaneously.

---

## 6. The Resource Name Stack

At any point during parsing, there is an ordered stack of active resource dictionaries. Lookup proceeds from innermost (top of stack) to outermost (bottom):

```
[page_resources, form_resources?, type3_font_resources?]
               ^--- bottom (outermost)               ^--- top (innermost)
```

In Rust:

```rust
struct ResourceStack<'a> {
    stack: Vec<&'a Resources>,
}

impl<'a> ResourceStack<'a> {
    fn lookup_font(&self, name: &Name) -> Option<&'a FontRef> {
        for resources in self.stack.iter().rev() {
            if let Some(font) = resources.fonts.get(name) {
                return Some(font);
            }
        }
        None
    }

    fn push(&mut self, r: &'a Resources) { self.stack.push(r); }
    fn pop(&mut self) { self.stack.pop(); }
}
```

Push on entry to a Form XObject (`Do`) or Type 3 glyph stream; pop on exit. For Type 3 glyphs, "exit" is the end of the glyph's content stream. For Form XObjects, "exit" is reaching the end of the Form's content stream after the `Do` operator dispatched into it.

The page-level resources are always the bottom of the stack (index 0) and are never popped during page processing.

---

## 7. Operators That Reference Resources

Every operator that references a resource name must resolve that name through the current resource stack:

| Operator | Resource sub-dictionary | Key |
|----------|------------------------|-----|
| `Tf name size` | `/Font` | font name |
| `Do name` | `/XObject` | XObject name |
| `cs name` | `/ColorSpace` | color space name |
| `CS name` | `/ColorSpace` | color space name |
| `gs name` | `/ExtGState` | graphics state name |
| `sh name` | `/Shading` | shading name |
| `scn`/`SCN` (with pattern name argument) | `/Pattern` | pattern name |
| `BDC /OC /Properties name` | `/Properties` | marked-content property |

The `Tf` operator is the most critical for text extraction: it sets the current font, which determines how character codes in subsequent `Tj`/`TJ`/`'`/`"` operators map to Unicode. An unresolved font name must be surfaced as a recoverable error — the parser should log the failure and skip character mapping for the affected text spans rather than aborting page extraction.

---

## 8. Inline Images vs. XObjects in Streams

Inline images are encoded directly in the content stream bytes between `BI`, `ID`, and `EI` operators (PDF 1.7 §8.9.7). The image data between `ID` and `EI` is raw binary and may contain arbitrary bytes, including sequences that look like PDF operators.

The `EI` token is identified as: a `EI` byte pair preceded by whitespace and followed by whitespace or end of stream. Do not scan for a bare `EI` pattern inside the image data.

When the image parameters include `/W` (width), `/H` (height), and `/BPC` (bits per component) and no compression filter is applied, the data length is exactly `ceil(W * BPC / 8) * H` bytes. Parse that many bytes after `ID` and then expect `EI`. With a compression filter (`/F` or `/Filter`), the data ends at the first whitespace-delimited `EI` token after decompression. In practice, treating the search as: skip past any initial whitespace after `ID`, read bytes until a valid whitespace-delimited `EI` is found (with a heuristic maximum scan length), is the safe fallback for malformed streams.

---

## 9. Pages with Many Content Streams

Some generators produce pages with dozens or hundreds of content streams — one per drawing element, update layer, or annotation. This is legal. The page-level concatenation must handle an arbitrary-length `/Contents` array.

Memory management is essential: do not decompress all streams into memory before parsing. Use a lazy `ContentStreamSource` that implements `Iterator<Item = Result<DecompressedStream>>` and decompresses each stream on demand. The parser reads from one stream at a time, injecting the newline separator at each boundary, without buffering more than one stream's bytes in memory.

Complex technical drawings (CAD exports, large engineering PDFs) can produce per-page content totaling hundreds of megabytes after decompression. A pull-based streaming parser is the only viable architecture.

---

## 10. Optional Content in Content Streams

Optional Content Groups (OCGs) control the visibility of content regions. They are activated with `BMC`/`BDC` (begin marked content) and deactivated with `EMC` (end marked content). An OCG region opened in stream N may be closed in stream N+1:

```
% stream 1
/OC /MC0 BDC
  BT /F1 12 Tf (Hello) Tj ET
% stream 1 ends here — no EMC

% stream 2
  BT /F1 12 Tf (World) Tj ET
EMC
```

The OCG nesting depth and the current active OCG stack must persist across stream boundaries, just like the graphics state stack. An `EMC` in stream 2 closes the region opened in stream 1.

The `/Properties` name referenced in a `BDC` inline dictionary or as an argument is resolved through the current resource stack's `/Properties` sub-dictionary, subject to the same innermost-first lookup as all other resource names. An OCG reference inside a Form XObject resolves through the Form's Resources `/Properties`, not the page's.

---

## Summary

Correct content stream processing requires treating the page's `/Contents` array as a single logical stream with injected newline separators at each boundary, maintaining continuous parser state (graphics stack, text object, OCG nesting) across those boundaries, and resolving every resource name through a dynamically managed stack of resource dictionaries whose depth reflects the current nesting level (page → Form XObject → Type 3 glyph). Failures in any of these three areas produce incorrect text extraction silently — wrong glyphs, missing text spans, or incorrect reading order — making them the highest-priority correctness requirements for the parser implementation.
