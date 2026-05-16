# Cross-Reference Table Parsing and Indirect Object Resolution

## Overview

Every PDF file is a collection of numbered, versioned objects — dictionaries, streams, arrays, and scalars — that are connected by indirect references of the form `N G R`, where N is the object number and G is the generation number. Reliably resolving those references to byte positions in the file is the foundation of any PDF parser. The cross-reference table (or its stream equivalent) is the mechanism that provides this map. pdftract must handle every variant of this mechanism, including corrupted files where no valid table exists at all.

---

## 1. Traditional Cross-Reference Tables

A traditional xref table begins with the keyword `xref` on its own line, followed by one or more subsections. Each subsection opens with a pair of integers on a single line — the first object number in the subsection and the count of consecutive entries — then a sequence of exactly that many 20-byte fixed-width entries.

Each entry has the form:

```
nnnnnnnnnn ggggg n \r\n
```

The ten-digit field is the byte offset of the object body from the beginning of the file (for in-use entries) or the object number of the next free object in the free list (for free entries). The five-digit field is the generation number. The single character flag is either `n` (in-use) or `f` (free). The two-byte end-of-line sequence is either `\r\n` or ` \n` (space + newline) — both are valid per the specification, and pdftract must accept either.

Object 0 is always the head of the free list and carries generation 65535. Its offset field holds the object number of the next free object, or 0 if the list is empty.

The xref table is followed immediately by a trailer dictionary, introduced by the `trailer` keyword. The trailer dictionary carries several mandatory or important keys:

- `/Size` — one greater than the highest object number present; defines the minimum size of the cross-reference table in memory.
- `/Root` — an indirect reference to the document catalog; mandatory.
- `/Info` — an indirect reference to the document information dictionary; optional but common.
- `/Encrypt` — an indirect reference to the encryption dictionary; present only in encrypted files.
- `/ID` — a two-element array of byte strings used for encryption and document identity; required when `/Encrypt` is present.
- `/Prev` — byte offset of the previous xref table or stream, used in incremental updates.

After the trailer dictionary comes the `startxref` keyword followed by a byte offset, then `%%EOF`.

---

## 2. Cross-Reference Streams (PDF 1.5+)

PDF 1.5 introduced cross-reference streams as an alternative to the traditional table-plus-trailer structure. A cross-reference stream is a regular PDF stream object whose dictionary doubles as the trailer dictionary and whose compressed body encodes the xref entries in binary form.

The stream dictionary must contain `/Type /XRef`. It also carries the same keys as a trailer dictionary (`/Size`, `/Root`, `/Info`, `/Prev`, etc.). Three additional keys control how the binary body is parsed:

- `/W` — an array of three integers specifying the byte widths of the three fields in each entry: field 1 (type), field 2 (offset or object stream object number), field 3 (generation number or index within object stream). A width of 0 means the field is absent and its default value applies.
- `/Index` — an array of integer pairs `[first_obj count ...]` analogous to the subsection headers in a traditional table. If omitted, defaults to `[0 /Size]`.
- `/Filter` — almost always `FlateDecode`; the stream body must be decompressed before parsing.

Each entry is a concatenation of the three fixed-width binary fields. The three entry types are:

- **Type 0**: free object. Field 2 is the object number of the next free object; field 3 is the generation number to use if the object is reused.
- **Type 1**: uncompressed object at a direct byte offset. Field 2 is the byte offset from the beginning of the file; field 3 is the generation number.
- **Type 2**: object compressed inside an object stream. Field 2 is the object number of the containing object stream; field 3 is the zero-based index of this object within that stream. Generation number is implicitly 0.

When `/W[0]` is 0, type 1 is the default — a common optimization in writers that emit only uncompressed objects.

---

## 3. Hybrid Reference Files

Some PDF writers produce documents that contain both a traditional xref table and a cross-reference stream in the same revision. In a hybrid file, the traditional table's trailer dictionary contains an `/XRefStm` key pointing to the byte offset of a cross-reference stream. The cross-reference stream, in turn, covers object entries not listed in the traditional table (typically compressed objects).

The resolution rule is straightforward: when both structures are present, the traditional xref table takes precedence for any object number it explicitly covers. The cross-reference stream fills in entries for object numbers not present in the table. pdftract should merge the two maps, giving priority to the traditional entries, so that a hybrid file is fully resolved without requiring the parser to choose one structure over the other.

---

## 4. Locating the xref via startxref

Parsing begins by scanning backward from the end of the file. The specification allows up to 1,024 bytes of garbage or comment padding after `%%EOF`; pdftract should scan the last 1,024 bytes for the final occurrence of `startxref`. The integer on the next non-whitespace line is the byte offset of the root xref structure (table or stream) for the most recent revision.

In incremental update chains, each revision's xref structure carries a `/Prev` key pointing to the previous revision's xref. pdftract must follow this chain to the end in order to build a complete object map, applying each revision on top of the previous one so that newer definitions shadow older ones.

The declared PDF version in the `%PDF-N.N` header is a lower bound. If the document catalog dictionary contains a `/Version` key, that key takes precedence and pdftract should use it to decide which features (such as object streams) are valid in this file.

---

## 5. Object Addressing Modes

Once the xref map is built, every object number resolves to one of two addressing modes.

**Type 1 — direct byte offset.** The parser seeks to the recorded offset, skips any leading whitespace, then reads the object header: `N G obj`. The content between `obj` and `endobj` is the object's value.

**Type 2 — inside an object stream.** The parser first resolves the containing object stream (itself a type 1 object), then locates the target object within that stream using the per-stream offset table described in the next section.

---

## 6. Object Streams (ObjStm)

An object stream is a compressed stream whose dictionary carries `/Type /ObjStm`, `/N` (the count of stored objects), and `/First` (byte offset from the start of the decoded stream body to the first stored object's data). The decoded stream body begins with an ASCII header consisting of `/N` pairs of integers: `objnum offset`, where `offset` is relative to the byte indicated by `/First`. After reading these `/N` pairs, pdftract has a local offset table. To retrieve the i-th object, it seeks within the decoded stream to `/First` + `offset[i]` and parses from that position. Object streams cannot contain stream objects, only non-stream objects such as dictionaries, arrays, and scalars.

---

## 7. Indirect Object Resolution

When the parser encounters an indirect reference `N G R`, it consults the xref map for object number N. It then follows the addressing mode to the byte offset or object stream location, reads the `N G obj` header, and verifies that the declared object number and generation number match the requested values. A mismatch — where the bytes at the recorded offset contain a different object number — usually indicates a corrupted or patched file. pdftract should log the mismatch and, for generation-number mismatches where the object number matches, return the object anyway, since generation-number cycling is rare in practice and writers sometimes emit inconsistent values.

---

## 8. Generation Numbers in Practice

Generation numbers exist to handle object reuse: when an object is freed and its slot is reused for a new object, the new object receives a generation number one higher than the freed object. In practice, the vast majority of PDF files never free and reuse any object slot, so every in-use object has generation number 0. pdftract should track the generation number recorded in the xref map and validate it during object lookup, but should never refuse to parse an otherwise valid object solely because of a generation mismatch. For extraction purposes, the correct behavior is to return the highest-generation object for any given object number.

---

## 9. Null Objects and the Free List

A reference to a free-list entry must resolve to the null object rather than triggering a parse error. The canonical null reference is `0 0 R`, which by specification always resolves to null. Any indirect reference whose xref entry is marked free (type 0) or whose object number exceeds `/Size` also resolves to null. The generation number 65535 marks a permanently freed slot that may never be reused. pdftract must handle all of these cases without panicking: the resolution function returns `Option<PdfObject>`, yielding `None` (interpreted as the null object) for any free, out-of-range, or missing entry.

---

## 10. Error Recovery: Linear Scan Fallback

When the xref structure is missing, truncated, or internally inconsistent — as can happen with corrupted incremental updates or files produced by non-conforming writers — pdftract falls back to a linear scan of the entire file body.

The scanner searches for the byte sequence `obj` preceded on the same line by two integers matching the pattern `N G`. For each candidate, it attempts to parse the object body, then records the byte offset and object-number/generation-number pair. When the same object number appears multiple times (a common artifact of corrupted incremental updates where the xref chain is broken), the definition appearing latest in the file takes precedence, since incremental updates append to the end of the file and later definitions supersede earlier ones.

After the scan completes, the reconstructed object table is used for all subsequent indirect reference resolution. The scan is significantly slower than xref-guided lookup, so pdftract should attempt xref parsing first and fall back only after confirming that the recorded `startxref` offset points to invalid data or that the decoded xref entries fail internal consistency checks (e.g., a type 1 entry's offset points to bytes that do not begin an `obj` header).

---

## Summary

Reliable object resolution in pdftract requires a layered strategy: locate `startxref` by scanning backward from `%%EOF`, parse the root xref structure (traditional table, cross-reference stream, or hybrid combination), follow `/Prev` chains to assemble the complete object map across all incremental revisions, and resolve each indirect reference through either a direct byte offset or an object stream lookup. Generation numbers and free-list entries must be handled gracefully rather than treated as hard errors. When the xref mechanism fails entirely, a linear scan of the file provides a workable fallback that makes pdftract robust against the full range of malformed files encountered in production.
