# Hyperlinks, Named Destinations, and Internal Navigation Structure

## Overview

PDF documents support a rich hyperlink model built on top of the annotation and action systems. Extracting hyperlinks faithfully requires understanding three distinct but interlocking subsystems: the Link annotation that marks a clickable region on the page, the action or destination dictionary that specifies the link target, and the document catalog's named destination index that resolves symbolic names to physical page locations. pdftract must implement all three layers to produce accurate link records for every annotation type used for navigation in real-world PDF files.

## Link Annotations and Spatial Extraction of Anchor Text

A PDF hyperlink begins with a `/Subtype /Link` annotation object in the page's `/Annots` array. The annotation carries a `/Rect` entry — an array of four numbers in page user space `[llx lly urx ury]` — that defines the bounding box of the clickable region. The coordinate system is the standard PDF one: origin at lower-left, y increasing upward, units in points.

To recover anchor text, pdftract intersects the annotation's `/Rect` with the text spans extracted from the page's content stream. A text span intersects if its bounding box overlaps the annotation rectangle with sufficient coverage — in practice a threshold of roughly 50% overlap by area is reliable for single-line links, though more precise matching uses the span's individual glyph positions. The concatenated text of all intersecting spans, in reading order, forms the anchor text reported in the output.

Additional annotation keys shape how the annotation presents visually but are not required for text extraction. `/Border` (a three-element array: horizontal corner radius, vertical corner radius, line width) and the newer `/BS` (border style dictionary with `/W` width and `/S` style) control the visible border; pdftract records border presence as metadata. `/H` is the highlight mode — `/I` (invert), `/O` (outline), `/P` (push), or `/N` (none) — which determines the visual response to a click and is otherwise ignored during extraction.

## URI Actions

When the annotation's `/A` dictionary has `/S /URI`, the link points to an external URL. The `/URI` key holds a byte string containing the URL, encoded as ASCII or UTF-8 depending on the producer. pdftract decodes it as UTF-8 with a fallback to Latin-1 for legacy files. The optional `/IsMap` boolean, when true, signals that the annotation is part of a server-side image map; the coordinates of the click are appended to the URL as a query string by the viewer. pdftract records this flag in the output but does not modify the extracted URL, since the coordinate-appending behavior is a viewer responsibility. For URI actions, the output `link_type` is `uri` and the `url` field carries the decoded string; anchor text comes from the spatial intersection described above.

## GoTo Actions and Internal Link Resolution

When `/A` carries `/S /GoTo`, or when the annotation has a direct `/Dest` key instead of an action dictionary, the link is internal — it targets a page within the same document. The destination is identified either by an explicit destination array or by a named destination string or name object that must be resolved through the catalog.

An explicit destination array has the form `[page_ref /XYZ left top zoom]`, `[page_ref /Fit]`, `[page_ref /FitH top]`, `[page_ref /FitV left]`, `[page_ref /FitB]`, `[page_ref /FitBH top]`, or `[page_ref /FitBV left]`. The first element is always an indirect reference to the target page object. pdftract resolves this reference against the cross-reference table, identifies the page's zero-based index from the page tree, and emits `target_page` as that index. The destination type keyword governs how the viewport fits to the target but is not needed for link extraction beyond noting the page number.

For GoTo actions, `link_type` is `internal` in the output schema, `target_page` holds the zero-indexed page number, and `url` is null.

## GoToR Actions and Cross-Document Links

The `/S /GoToR` action type points to a page in a different PDF file. The `/F` key is a file specification — either a plain string path or a file specification dictionary with `/F` (DOS/Windows path), `/UF` (Unicode path), and `/FS` (file system type). pdftract extracts the most specific path available, preferring `/UF` over `/F`. The `/Dest` key follows the same syntax as a GoTo destination — either a named string or an explicit array — but pdftract cannot resolve the page number without reading the remote file, so it extracts the destination as a raw label string and marks `link_type` as `external`. The `url` field is populated with the file path, and a separate `destination_label` field carries the raw destination value for downstream consumers that can open the referenced file.

## Named Destination Resolution

Named destinations decouple the link reference from the physical page number, allowing documents to be reorganized without updating every annotation. A named destination in a GoTo or GoToR action is expressed either as a PDF name object (`/SomeName`) or as a PDF string (`(SomeName)` or a hex string). Both forms must be supported — name objects are limited to printable ASCII without spaces, while string-valued names can carry arbitrary characters.

Resolution follows a two-path strategy. The modern mechanism is the `/Names` dictionary in the document catalog, which contains a `/Dests` entry pointing to a name tree. The legacy mechanism is a `/Dests` dictionary directly under the catalog — a flat mapping of name to destination. pdftract checks the name tree first, then falls back to the flat catalog dictionary.

## The /Names→/Dests Name Tree

A PDF name tree is a balanced B-tree structure stored as a graph of dictionary objects. Interior nodes carry a `/Kids` array of indirect references to child nodes, and a `/Limits` array of two strings giving the lexicographically smallest and largest names in the subtree. Leaf nodes omit `/Kids` and instead carry a `/Names` array of alternating name/destination pairs: `[(name1) dest1_array (name2) dest2_array ...]`.

pdftract's traversal algorithm starts at the root node referenced by `/Names→/Dests`. If the root has `/Names`, it is already a leaf; iterate the array and build a hash map of name to destination. If the root has `/Kids`, examine each child's `/Limits` to prune branches that cannot contain the target name, then recurse into matching children. Because names in each node are lexicographically sorted, a binary search over the `/Names` array finds the target in O(log n) time per leaf. For bulk extraction — needed to annotate all links in a document in one pass — pdftract performs a full tree walk once and caches the resulting map, avoiding repeated traversals per annotation.

After resolving a named destination to an explicit destination array, the page reference in the array is resolved to a zero-based page index using the page tree, exactly as for direct GoTo actions. The `target_page_label` field in the output is populated from the page labels subsystem (the `/PageLabels` number tree in the catalog), giving the human-readable label such as "iv" or "A-3" alongside the numeric index.

## Annotation /QuadPoints for Non-Rectangular Links

When a hyperlink spans multiple lines — for example, a two-line heading that serves as a table-of-contents entry — the rectangular `/Rect` is an enclosing bounding box that may include significant whitespace between lines. The `/QuadPoints` array provides a per-quadrilateral clickable region that precisely tracks the actual text geometry. Each group of eight numbers defines one quadrilateral as four corner points in the order: lower-left, lower-right, upper-right, upper-left (or the alternate order used by some producers — both must be handled).

When `/QuadPoints` is present, pdftract uses those quadrilaterals for anchor text extraction rather than the gross `/Rect`. For each quadrilateral, compute its axis-aligned bounding box, intersect with text spans, and collect matching glyphs. The union of text from all quadrilaterals, de-duplicated and sorted by reading order, is the anchor text. This produces significantly cleaner results for multi-line links in structured documents such as academic papers and reference manuals.

## Output Schema for Links

Each extracted link is represented as an object in the top-level `links` array of the pdftract JSON output, keyed by source page:

```json
{
  "anchor_text": "Section 4.2 — Data Types",
  "url": null,
  "target_page": 17,
  "target_page_label": "18",
  "link_type": "internal",
  "source_page": 2,
  "source_rect": [72.0, 611.5, 310.25, 624.0]
}
```

For URI links, `url` carries the decoded URL string, `target_page` and `target_page_label` are null, and `link_type` is `uri`. For GoToR external links, `url` holds the target file path, `link_type` is `external`, and a `destination_label` field carries the raw named or array destination. The `source_rect` is always reported in the same coordinate space as the page's `/MediaBox` origin, with the y-axis unflipped (PDF default lower-left origin), so consumers can map the rect back to extracted text spans using the same coordinate frame pdftract uses internally.

## Link Density as a Document Signal

Link density — the ratio of `/Link` annotation count to total text span count on a page — is a lightweight signal for classifying page function. Pages whose link density exceeds a threshold (empirically around 0.3 links per span works well for most document types) are likely tables of contents, indices, or navigation pages, not body text. pdftract annotates such pages with a `page_type: navigation` hint in the per-page metadata block. This hint is advisory: downstream consumers can use it to skip navigation pages in full-text extraction pipelines, route them to outline reconstruction logic, or flag them for special handling. The threshold is configurable via the extraction profile so that documents with dense inline citation links — common in legal briefs and academic papers — are not incorrectly classified.

## Implementation Priorities

Handling the full annotation and action type matrix requires disciplined fallback logic. Not all annotations carry an `/A` action dictionary — some use the annotation-level `/Dest` key directly, which must be checked when `/A` is absent. Not all named destinations live in the name tree — the flat catalog `/Dests` dictionary is widely used in documents produced by older toolchains and must be consulted as a fallback. Not all link annotations have `/QuadPoints` — the implementation must detect presence before attempting quadrilateral-based text extraction and silently fall back to `/Rect` intersection when the key is absent. With these fallbacks in place, pdftract covers the complete range of link annotation patterns found in production PDF files across publishing, legal, academic, government, and technical documentation domains.
