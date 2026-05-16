# PDF Document Catalog, Page Tree, and Document-Level Structure

## Overview

Before pdftract can extract a single glyph, it must correctly parse the document catalog and traverse the page tree. These structures are the authoritative source of truth for every resource, every page coordinate system, and every inherited property that governs text extraction. A failure at this stage propagates silently into incorrect bounding boxes, missing pages, or misinterpreted coordinate spaces. This document describes what the PDF specification defines at the catalog and page-tree level, and what pdftract must correctly handle in each area.

---

## The Document Catalog

The document catalog is the root object of the PDF document's logical structure. The cross-reference table's trailer dictionary contains a `/Root` key whose value is an indirect reference to the catalog object. The catalog is always of type `/Catalog`.

The only strictly required keys in the catalog are `/Type` (which must equal `/Catalog`) and `/Pages` (an indirect reference to the root of the page tree). Every other key is optional, but several are extraction-relevant.

**`/Outlines`** is a reference to the document outline (bookmarks). pdftract uses this to reconstruct hierarchical section structure when named destinations within the outline correspond to page ranges, which assists in logical document segmentation.

**`/PageLabels`** is a number tree mapping page indices to label dictionaries. Each label can specify a numbering style (Arabic, Roman, alphabetic), a label prefix, and a starting value. pdftract records these to produce human-readable page identifiers in its output that match the labels visible in a viewer, rather than raw zero-based indices.

**`/Names`** is a dictionary of named trees covering multiple categories, detailed separately below. It is one of the most complex optional structures in the catalog.

**`/Dests`** is a legacy dictionary of named destinations that maps name strings directly to destination arrays. This was superseded by `/Names /Dests` in PDF 1.2, but pdftract must handle both forms since many documents were produced by older generators. Named destinations that target specific pages are informational for extraction and are recorded as metadata.

**`/AcroForm`** is a reference to the interactive form dictionary. For text extraction purposes, pdftract processes form field values, particularly text field appearances, as potential content streams that supplement the page's `/Contents`.

**`/Metadata`** is a reference to a metadata stream in XMP format. pdftract reads this stream to populate its document-level metadata output (title, author, creation date, language) before any page content is processed.

**`/StructTreeRoot`** is a reference to the document's structure tree. pdftract only attempts to use this when `/MarkInfo` indicates the document is properly tagged, as detailed below.

**`/Lang`** specifies the natural language of the document as a BCP 47 language tag. pdftract propagates this as the default language for text spans that do not override it at the structure element or marked-content level.

**`/OutputIntents`** is an array of output intent dictionaries used in PDF/X and PDF/A documents. While primarily a color management concern, pdftract records the conformance level (e.g., PDF/A-2b) in its output metadata because it constrains what content structures are legal in the file.

---

## The /MarkInfo Dictionary

The `/MarkInfo` dictionary in the catalog signals whether the document has been authored as a tagged PDF. Its three keys are `/Marked`, `/UserProperties`, and `/Suspects`.

**`/Marked`** is the critical flag. When `true`, the document author asserts that all content is associated with marked-content sequences and that a complete structure tree exists in `/StructTreeRoot`. pdftract uses this flag as the gate for structure-tree extraction: if `/Marked` is `false` or absent, pdftract falls back to heuristic reading-order analysis rather than traversing the structure tree, because an incomplete structure tree is worse than no tree at all.

**`/Suspects`**, when `true`, means the document author acknowledges that the structure tree may be incorrect or incomplete, even though `/Marked` is `true`. pdftract treats a document with both `/Marked true` and `/Suspects true` as untrustworthy for structure-guided extraction, applying the same heuristic fallback used for untagged documents.

**`/UserProperties`**, when `true`, indicates that structure elements may carry user-defined property lists in their `/A` attribute dictionaries. pdftract records these as opaque metadata on the corresponding output spans.

---

## The Page Tree

The `/Pages` entry in the catalog references the root of a balanced tree of page objects. This tree has two kinds of nodes. Intermediate nodes have `/Type /Pages` and contain a `/Kids` array of references to child nodes (which may themselves be intermediate or leaf nodes) and a `/Count` integer giving the total number of leaf page nodes in the subtree. Leaf nodes have `/Type /Page` and contain the actual page attributes.

Traversal is a straightforward depth-first walk: start at the root node, and for each node encountered, if its `/Type` is `/Pages`, recurse into each entry in `/Kids` in order. If its `/Type` is `/Page`, emit it as the next page in the enumeration sequence. The `/Count` value on intermediate nodes is used by pdftract to pre-allocate output structures and to validate that the traversal found the expected number of leaves, but is never used to skip subtrees. pdftract always performs a full traversal; it does not trust `/Count` to be accurate in malformed documents.

---

## Page Dictionary Keys

Each leaf page node carries a set of keys governing its geometry and content.

**`/MediaBox`** is the only required geometry key on a page (or an ancestor node via inheritance). It defines the full physical extent of the page in default user space units. All content coordinates are interpreted relative to this space.

**`/CropBox`**, **`/BleedBox`**, **`/TrimBox`**, and **`/ArtBox`** define progressively tighter regions within the media box. For text extraction, pdftract clips output text positions to the `/CropBox` if present, discarding glyphs whose positions fall outside it, since those glyphs are not visible in normal viewing.

**`/Rotate`** is an integer multiple of 90, specifying a clockwise rotation applied to the page before display. pdftract applies this rotation when computing the final bounding boxes of extracted text runs so that reported coordinates are in the oriented (viewer-facing) coordinate space.

**`/Resources`** is a dictionary of resource dictionaries (fonts, XObjects, color spaces, and so on) available to the page's content streams. This is the entry point for font resolution during text extraction.

**`/Contents`** is either a single indirect reference to a content stream or an array of such references. See the dedicated section below.

**`/Annots`** is an array of annotation dictionaries. For extraction, pdftract processes widget annotations (form fields) and link annotations (to record destination metadata), but ignores display-only annotation types.

**`/StructParents`** is an integer key that pdftract uses when walking the structure tree in reverse: it identifies which entry in the parent tree corresponds to this page, allowing structure elements to be matched to their page.

---

## /UserUnit

Introduced in PDF 1.6, `/UserUnit` is a positive real number on a page dictionary that scales the default user space. The default user space unit is 1/72 of an inch. A `/UserUnit` value of `2.0` means each user space unit represents 2/72 of an inch. When `/UserUnit` is absent, its value is exactly `1.0`.

pdftract must apply this scale factor to every coordinate derived from the page, including text positions extracted from content streams, glyph bounding boxes computed from font metrics, and the geometry boxes (`/MediaBox`, `/CropBox`, etc.). Failure to apply `/UserUnit` produces coordinates that are numerically correct in PDF user space but physically incorrect when converted to physical units. The scale is applied after coordinate extraction, as a final multiplication before output.

---

## /Contents: Single Stream vs. Array

A page's content is described by one or more PDF streams referenced from `/Contents`. When the value is a single indirect reference, pdftract decodes and processes that stream. When the value is an array, each element is an indirect reference to a separate stream. The specification requires that these streams be treated as if concatenated into a single stream before parsing, meaning the graphics state is continuous across stream boundaries — an operator in one stream can depend on state established in a preceding stream.

pdftract decodes each stream separately (to handle per-stream compression filters independently) but feeds them to a single graphics state machine in order. Inserting an implicit whitespace character between streams is required because some generators split streams mid-token, relying on the concatenation to reconstruct valid syntax. pdftract inserts a single ASCII space between decoded stream buffers before tokenizing.

---

## The /Names Dictionary

The `/Names` entry in the catalog is a dictionary of named trees organized by category. Each value is a name tree (a balanced B-tree of key-value pairs with string keys). The categories relevant to pdftract are:

**`/Dests`** maps destination names to destination arrays or dictionaries. pdftract resolves these when it encounters named-destination references in annotations or outlines.

**`/EmbeddedFiles`** maps names to file specification dictionaries. pdftract records these as attachments in its document metadata output and, when configured, can extract embedded file content alongside the main text.

**`/JavaScript`** is informational only; pdftract records the presence of JavaScript but does not execute it.

---

## /ViewerPreferences and /OpenAction

The `/ViewerPreferences` dictionary controls how a viewer renders the document. Most of its keys (toolbar visibility, window fitting) are irrelevant to extraction. The one key pdftract reads is **`/Direction`**: when set to `/R2L`, it signals that the document's page progression and text base direction are right-to-left. pdftract uses this as a document-level hint for bidirectional text processing.

The `/OpenAction` specifies an action or destination to activate when the document is opened. For extraction, this is purely informational: pdftract records the action type and any associated destination as document metadata (for example, to note that the document opens at a specific named destination), but it does not execute the action.

---

## Page Inheritance

The PDF page tree supports property inheritance: the keys `/MediaBox`, `/CropBox`, `/Resources`, and `/Rotate` may be defined on any intermediate `/Pages` node and are inherited by all descendant page nodes that do not define the key themselves.

When pdftract processes a leaf page, it resolves each inheritable key by walking up the parent chain — using the `/Parent` reference present on every non-root node — until it finds the nearest ancestor that defines the key. The walk stops at the catalog root if no ancestor defines the key (in which case no value exists for `/CropBox`, and the key is absent, or `/Rotate` defaults to 0). The resolved values are cached per page during the initial tree traversal so the parent walk is not repeated during content stream processing.

This inheritance mechanism means pdftract cannot resolve a page's complete attribute set from the leaf node alone. The full tree traversal that enumerates pages must simultaneously accumulate inherited values, threading them down through intermediate nodes and recording the resolved set at each leaf. pdftract performs this accumulation in a single top-down pass over the page tree, storing the resolved page descriptor before any content decoding begins.
