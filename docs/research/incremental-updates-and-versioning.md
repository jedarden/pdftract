# PDF Incremental Updates, Object Revisions, and Version-Aware Extraction

## Overview

PDF documents are not always written in a single pass. The format was designed to support non-destructive modification: content can be appended to the end of a file without touching the bytes that precede it. This mechanism, known as an incremental update, is how form fills, annotation additions, and digital signatures are layered onto an existing document. A parser that treats the first `%%EOF` as the end of meaningful content will silently discard everything after it — form data, reviewer comments, correction overlays, and signature metadata included. pdftract must parse the complete file as a sequence of revisions and merge all update layers into a coherent object table before any extraction begins.

## Incremental Update Structure

Each PDF revision consists of three parts appended sequentially: a body section of new or modified objects, a cross-reference (xref) section describing their byte offsets, and a trailer dictionary followed by `startxref` and `%%EOF`. When a file is incrementally updated, the original body, xref, and trailer are left intact; the new revision is concatenated at the end. A file may therefore contain multiple `%%EOF` markers, each marking the end of one revision.

Each appended trailer carries a `/Prev` key pointing to the previous revision's xref. This creates a backward-pointing chain from the most recent revision to the original. Walking the chain from the final `startxref` and collecting all object entries — with later definitions superseding earlier ones — yields the complete, current object table.

## Why This Matters for Extraction

Interactive PDF forms store field values as objects. When a user fills out a form in a reader application, the reader does not rewrite the original document; it appends an incremental update containing the modified AcroForm field objects. Annotations — highlights, sticky notes, ink marks — are added in exactly the same way. Revision corrections submitted by document management systems follow the same pattern.

A parser that halts at the first `%%EOF` will see the blank form as it was distributed, with no field values populated. It will see none of the annotations. Any extraction output will be incomplete, and because the document will otherwise appear structurally valid, the failure will be silent. pdftract must locate all `%%EOF` markers in the file, enumerate every appended revision, and resolve the full xref chain before building the object map that drives content extraction.

## Cross-Reference Table Chaining

The traditional cross-reference table is a plain-text section beginning with the keyword `xref`, containing one or more subsections headed by a first-object-number and count, followed by 20-byte entries encoding byte offset, generation number, and an in-use or free flag.

Reconstructing the complete object table requires merging all xref sections in reverse chronological order. The algorithm starts at the final `startxref` offset, reads the xref table there, records its entries, reads the `/Prev` offset from its trailer, and repeats until `/Prev` is absent. Entries from a newer revision are never overwritten. The result is a map from object number to the byte offset of its most recent — and therefore correct — definition.

## Cross-Reference Streams (PDF 1.5 and Later)

Beginning with PDF 1.5, the cross-reference table may be replaced by a cross-reference stream: a stream object with `/Type /XRef`. This compresses well and is used by most modern authoring tools. pdftract must handle both formats and be prepared for a mix within the same file — an original with traditional xref tables may be updated by a tool that appends a compressed xref stream.

Parsing a cross-reference stream requires reading three dictionary fields. The `/W` array contains three integers specifying the byte widths of each field in the binary stream entries: the entry type, the offset or object-stream index, and the generation number or index within an object stream. The `/Index` array, if present, specifies the object number ranges described by the stream entries; its absence implies a single range starting at zero with length equal to the `/Size` value. Entry types are defined as: 0 for a free object (the offset field holds the next free object number), 1 for an uncompressed in-use object (the offset field is the byte offset of the object in the file), and 2 for an object compressed inside an object stream (the offset field is the object number of the containing stream, and the generation field is the object's index within that stream). The `/Prev` chaining mechanism is identical to traditional xref tables.

## Object Streams (ObjStm)

PDF 1.5 also introduced object streams, stream objects of type `/ObjStm`, which pack multiple PDF objects into a single compressed stream. This reduces file size but adds a layer of indirection for any parser that needs to locate an individual object.

An object stream's dictionary contains `/N` (count of packed objects) and `/First` (byte offset within the decoded stream where the first object's content begins). Before `/First` is a plain-text index of N pairs — object number and byte offset relative to `/First`. pdftract decompresses the stream, parses this index, and extracts any individual object on demand. Objects inside an object stream carry no generation number; they inherit generation zero unless the xref entry specifies otherwise.

## Tracking Object Revisions

Because xref entries record both object number and generation number, pdftract can determine not just the current state of an object but the sequence in which it changed. Each time an object is deleted and its number recycled, the generation number increments. By collecting xref entries from all revisions — not only the most recent — pdftract can present a complete revision history: which revision introduced each generation, its byte offset, and when the object was freed.

This per-object revision tracking is particularly useful for form field extraction. A document distributed to a reviewer, filled, annotated by a second reviewer, and corrected by a third will contain at least three incremental updates. Each form field's value object will appear in the xref of the revision that last modified it. By cross-referencing object revision data with update timestamps from the document's XMP metadata or Info dictionary, pdftract can report when each field was filled and distinguish original content from subsequent corrections.

## Deleted Objects and the Free List

When an object is deleted in an incremental update, the xref gains a free-list entry with an incremented generation number. pdftract must tolerate indirect references to deleted objects without crashing — the correct behavior is to return a null object, consistent with the PDF specification's treatment of free-object references. The content model built on the raw object map must handle null gracefully wherever an optional indirect reference may resolve to a deleted object.

## Signature ByteRange and Unsigned Content Regions

A PDF digital signature uses the `/ByteRange` entry in the signature dictionary to specify which bytes of the file the cryptographic digest covers. ByteRange is an array of four integers: offset and length of the range before the signature value, and offset and length of the range after it. The signature value itself — a hex-encoded blob — occupies the gap between those two ranges and is excluded from the digest.

When a signed PDF receives an incremental update, the appended bytes fall entirely outside the original signature's ByteRange. pdftract identifies which byte ranges are covered by each signature and which content was added afterward, allowing callers to distinguish content that was signed at a point in history from content added post-signing — a distinction critical in legal, compliance, and forensic extraction contexts.

## Repair Parsing for Broken Incremental Updates

Incremental updates can be malformed. A partially written update may have a corrupted xref section or a `startxref` pointing to the wrong offset. Network truncation, file system corruption, or authoring tool bugs can produce files where the normal chain-following algorithm fails before reaching all revisions.

pdftract's repair strategy is to fall back to a full-file object scan. The scanner reads linearly, identifying all byte sequences matching the pattern `N G obj`, recording each object's byte offset, and similarly locating all `xref` and `startxref` markers. From this inventory it reconstructs a best-effort object table, applying the same later-definition-wins rule. This recovers content from all update layers even when xref metadata is unreliable, at the cost of a full sequential read.

## Version History Extraction

pdftract can expose document history as a linear sequence of revisions, each containing the objects modified or added in that update. This is useful in forensic extraction — determining what a document said at a specific point in time — and in contract comparison workflows where counterparties exchanged multiple revisions of the same file rather than using a diff-capable format.

The performance tradeoff is real: constructing the complete object table is O(total xref entries across all revisions), which is manageable, but materializing each individual revision requires re-running the page content extraction pipeline once per revision. pdftract exposes revision history as a lazy iterator: callers can request a specific revision's content without materializing all others. For forensic use cases where every revision must be inspected, full linearization is available but documented as an expensive operation on deeply revised files.

## Summary

Correct extraction from incrementally updated PDFs requires following the complete `startxref` chain, merging all xref sections and xref streams in reverse chronological order, locating objects at raw byte offsets and inside compressed object streams, handling free-list entries without errors, and identifying byte regions covered by digital signatures. pdftract implements all of these mechanisms as prerequisites to content extraction, ensuring that form fills, annotations, and corrections appended as incremental updates are never silently omitted from output.
