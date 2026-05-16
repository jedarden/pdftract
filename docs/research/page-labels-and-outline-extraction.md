# Page Labels, Outlines, and Document Navigation Structure

## Overview

PDF documents carry navigation metadata that goes far beyond raw text content. Page labels define how a document's pages are logically numbered — Roman numerals for front matter, alphabetic codes for appendices, decimal for body chapters. The outline tree (commonly called bookmarks) encodes the document's hierarchical structure as a tree of titled entries each pointing to a specific page and position. Named destinations bridge these two systems, providing stable symbolic references that both outline items and in-page hyperlink annotations can target. For pdftract, implementing full extraction of this navigation layer transforms output from a flat stream of text into a structured artifact that reflects the document author's intent.

---

## 1. Page Labels: Logical Numbering via /PageLabels

The PDF specification stores page label definitions in a number tree rooted at `/PageLabels` in the document catalog. A number tree maps integer keys (physical page indices, zero-based) to label range dictionaries. Each dictionary defines how pages are labeled from that index until the next range begins.

Each label range dictionary contains up to three fields:

- `/S` (style): the numbering style applied within this range. Legal values are `/D` (decimal Arabic), `/R` (uppercase Roman), `/r` (lowercase Roman), `/A` (uppercase alphabetic A–Z, AA–ZZ, …), and `/a` (lowercase alphabetic). Omitting `/S` produces pages with no numeric component — the label is the prefix string alone.
- `/P` (prefix): an optional PDF string prepended to every label in the range. A prefix of `"App-"` combined with `/S /D` and a start value of 1 yields labels `App-1`, `App-2`, and so on.
- `/St` (start value): the integer at which counting begins within the range. Defaults to 1 if absent.

A typical scholarly monograph might define three ranges: physical pages 0–7 labeled with `/S /r` (lowercase Roman: i through viii), physical page 8 onward labeled with `/S /D /St 1` (decimal: 1, 2, 3, …), and a final range starting at the first appendix page labeled with `/P "A-" /S /D /St 1` (A-1, A-2, …). Back matter can resume a fresh decimal sequence by introducing another range with the appropriate `/St` offset.

pdftract must parse the `/PageLabels` number tree in full and precompute a mapping from every physical page index (0-based) to its logical label string. This mapping is then available at extraction time so that every output object — text blocks, annotations, outline entries — can carry both `page_index` (the zero-based physical position) and `page_label` (the human-readable string such as `"vi"` or `"A-3"`). Exposing both values is essential: downstream consumers that want to render "Page vi of xii" use the label, while those doing positional math use the index.

---

## 2. The Outline Tree: /Outlines and Its Node Structure

The document catalog's `/Outlines` entry points to an outline dictionary that serves as the root of the bookmark tree. The root itself is not displayed; it acts as a container whose `/First` and `/Last` entries reference the first and last top-level outline items respectively.

Each outline item is a dictionary with the following fields:

- `/Title`: a PDF string (potentially UTF-16BE encoded) that contains the visible label shown to the reader. Extraction requires decoding byte-order-mark-prefixed UTF-16BE correctly, falling back to PDFDocEncoding for byte strings without the BOM.
- `/Parent`: a reference back to the containing node (the root or another outline item).
- `/First` / `/Last`: references to the first and last child items if the entry has children.
- `/Next` / `/Prev`: references to the adjacent siblings within the same parent's child list.
- `/Count`: an integer indicating the number of descendant items visible when this node is open. A negative `/Count` signals that the node is collapsed in the viewer; a positive value signals it is open. The absolute value gives the total descendant count. pdftract should record both the count and whether the node was collapsed.
- `/F` (flags): a bitmask. Bit 1 (value 1) means italic rendering; bit 2 (value 2) means bold. These can be combined.
- `/C` (color): an array of three floats in the DeviceRGB space for the title's display color. Absent means black.

Traversal of the outline tree is a linked-list walk, not an array iteration. Starting at the root's `/First`, pdftract follows `/Next` pointers across siblings and recursively descends into children via `/First` at each node that has them, tracking depth as it goes.

---

## 3. Outline Item Destinations

Each outline item points to a location in the document through either a `/Dest` entry or an `/A` (action) entry.

A `/Dest` value is either an array or a name/string that references a named destination. An explicit destination array has the form `[page_ref /XYZ left top zoom]` where `page_ref` is an indirect reference to the target page object, `/XYZ` is the most common destination type (others include `/Fit`, `/FitB`, `/FitH`, `/FitV`), and `left`, `top`, `zoom` are optional coordinate and zoom parameters that may be `null`. pdftract resolves the page reference against the document's page tree to determine the zero-based physical page index, then looks up the page label from the precomputed mapping.

When `/Dest` is a string or name, it is a named destination reference. Named destinations are stored in one of two places: the `/Dests` dictionary directly under the document catalog (older format, maps name to destination array), or the `/Names` dictionary's `/Dests` name tree (modern format, a balanced tree structure mapping string keys to destination arrays or dictionaries). pdftract must resolve named destinations by checking both locations. The resolution produces the same kind of destination array, from which the page reference is extracted identically.

When the outline item uses `/A` instead of `/Dest`, the value is an action dictionary. The relevant cases are:

- `/S /GoTo` with a `/D` entry: a within-document GoTo action. The `/D` value is a destination, treated identically to a `/Dest` entry — either an explicit array or a named destination string.
- `/S /GoToR` with a `/F` (file spec) entry: a cross-document GoTo action targeting another PDF file. pdftract should record these as unresolvable with a note that the target is external, rather than attempting file system resolution.
- `/S /URI` with a `/URI` entry: a hyperlink to a web address. In outline items this is unusual but valid; pdftract records the URI string.

---

## 4. Structured Outline Output

pdftract should serialize the outline tree as a JSON array of hierarchical node objects. Each node carries:

```json
{
  "title": "Chapter 3: Signal Processing",
  "level": 2,
  "page_index": 47,
  "page_label": "38",
  "open": true,
  "bold": false,
  "italic": false,
  "children": [ ... ]
}
```

`level` is the zero-based depth in the tree (top-level items are level 0). `page_index` and `page_label` are both included. `open` reflects the sign of `/Count`. The `children` array is present and may be empty; it is never omitted, which allows consumers to handle the structure uniformly without null checks. Items whose destinations could not be resolved (named destinations absent from the document, cross-file GoToR actions) include `"page_index": null` and `"page_label": null` with a `"destination_type"` field set to `"external"` or `"unresolved"` as appropriate.

---

## 5. Outline as a Reading-Order and Heading Hint

For structured documents — technical reports, academic books, reference manuals — the outline tree encodes the heading hierarchy that the author intended. Outline items at level 0 typically correspond to chapters or major sections; level 1 items to subsections; level 2 to sub-subsections. The title strings often exactly match the heading text on the target page.

pdftract can exploit this relationship during text extraction. After extracting text blocks from a page, each block's bounding box can be compared against outline entries whose `page_index` matches the current page. When the normalized text of an outline title appears in a text block at or near the top of the region, that block's inferred heading level can be set to the outline item's depth. This cross-reference is a heuristic and should be reported with a confidence field rather than applied silently, since rendered heading text may differ from the outline title through abbreviation, line wrapping, or font substitution. Nevertheless, it substantially improves structure inference for documents that lack tagged PDF or explicit heading role markup.

---

## 6. URI Actions and Hyperlink Annotations

In-page hyperlinks are stored as link annotations (`/Subtype /Link`) in each page's `/Annots` array. Each annotation has a `/Rect` defining its bounding box on the page and either a `/Dest` or `/A` entry for its target.

For external hyperlinks, the action is `/S /URI` with a `/URI` string. pdftract extracts the URL and determines the anchor text by finding the text content within the annotation's `/Rect` on that page — the text spans whose bounding boxes overlap the annotation rectangle constitute the visible link text. This spatial join requires that text extraction has already produced positioned text runs before annotation extraction runs; pdftract's pipeline should process annotations in a second pass after text geometry is established.

For internal links, the action is `/S /GoTo` or the `/Dest` shorthand, resolved to a physical page index and page label using the same machinery as outline destinations. These are serialized as `{"type": "internal", "page_index": 12, "page_label": "5", "anchor_text": "see Figure 3"}` alongside `{"type": "external", "url": "https://example.com", "anchor_text": "specification"}` for URI links.

pdftract should expose per-page annotation arrays in its output, each entry containing `type`, `rect` (normalized to user-space coordinates), `anchor_text`, and the destination or URL. This allows consumers to reconstruct hyperlink graphs, validate internal cross-references, and render interactive overlays without re-parsing the PDF.

---

## Implementation Priorities

Page label extraction is relatively self-contained and should be implemented early since it enriches every other output field. The outline tree walk and destination resolver share infrastructure with the named destination resolver needed for link annotations, so these should be built together. The heading-inference cross-reference between outline titles and text blocks is the most heuristic component and belongs in a post-processing pass that can be toggled independently. Together, this navigation layer gives pdftract output that is immediately useful for document indexing, accessibility tooling, and structured content pipelines.
