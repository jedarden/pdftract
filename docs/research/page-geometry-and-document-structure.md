# Page Geometry and Document Structure

## Scope

This document covers the structural and geometric elements of the PDF specification that a Rust text extraction library must correctly model: the page tree, box hierarchy, coordinate system, rotation handling, page labels, outlines, named destinations, the document catalog, the resources dictionary, and viewer preferences. Correct handling of these elements is a prerequisite for placing extracted glyphs in meaningful, reading-order coordinates.

---

## 1. Page Tree

The PDF page tree is rooted at the `Pages` object referenced from the document catalog. The tree is composed of two node types:

- **Intermediate nodes** (`/Type /Pages`): have a `Kids` array of indirect references to child nodes, and a `Count` integer giving the total number of leaf pages in the subtree. They may carry inheritable attributes.
- **Leaf nodes** (`/Type /Page`): represent individual pages and hold or inherit all page attributes required for rendering.

The `Count` at each intermediate node enables **O(log n) random-access lookup** without traversing every leaf. To locate page index *k*, inspect `Count` at each child in `Kids` and descend into the subtree whose cumulative count covers *k*. Implementing sequential traversal is simpler but produces O(n) cost per lookup, which is unacceptable for large documents.

**Inherited attributes** propagate from ancestor intermediate nodes to descendant pages unless a descendant overrides them. The inheritable keys are:

| Key | Default if absent |
|---|---|
| `MediaBox` | Required on the root `Pages` node; no PDF default |
| `CropBox` | Equals `MediaBox` |
| `Rotate` | `0` |
| `Resources` | Empty dictionary |
| `UserUnit` | `1` (PDF 1.6+) |

Inheritance resolution: when building the page object for extraction, walk from the leaf upward through each `Parent` reference, collecting values for keys not yet set on the leaf. Stop at the root `Pages` node. Do this once per page and cache the result; never re-traverse the parent chain for each attribute access.

---

## 2. Page Boxes

All boxes are arrays of four numbers `[x0 y0 x1 y1]` in default user space units (points at 1/72 inch per unit unless `UserUnit` is set). The values represent the lower-left and upper-right corners; the specification does not require `x0 < x1` or `y0 < y1`, so normalize to `(min, max)` when reading.

| Box | Key | Defaults to |
|---|---|---|
| **MediaBox** | `MediaBox` | Required |
| **CropBox** | `CropBox` | MediaBox |
| **BleedBox** | `BleedBox` | CropBox |
| **TrimBox** | `TrimBox` | CropBox |
| **ArtBox** | `ArtBox` | CropBox |

For text extraction, `CropBox` is the correct extraction boundary. Content outside the CropBox is not visible to the user and should be clipped before including glyphs in output. BleedBox, TrimBox, and ArtBox carry print-production semantics and are generally irrelevant to text extraction, but should be exposed in the library's page metadata API for callers that need them.

**`UserUnit`** (PDF 1.6, optional): a positive number specifying the size of one default user-space unit in units of 1/72 inch. Default is `1`. Multiply all box coordinates and glyph positions by `UserUnit` to convert to points before any further geometry work. Most documents set `UserUnit` to `1`; documents generated for large-format printing may set it to values like `4` or `72` (the latter making 1 unit = 1 inch).

---

## 3. Page Rotation

The `Rotate` key is an integer, one of `{0, 90, 180, 270}`, specifying **clockwise rotation** applied during rendering. A page with `MediaBox [0 0 612 792]` and `Rotate 90` is rendered as a landscape page 792 units wide and 612 units tall, with the origin at the bottom-left of the rotated view.

Rotation does not change any coordinates stored in the content stream. The coordinate system in the stream is always in the page's unrotated space. When extraction is complete and glyph positions are in unrotated page space, apply the inverse transform to produce display-space coordinates:

- **0°**: no transform.
- **90° CW** (Rotate=90): display point `(x', y') = (y, W - x)` where `W` is the unrotated MediaBox width.
- **180°** (Rotate=180): `(x', y') = (W - x, H - y)`.
- **270° CW** (Rotate=270): `(x', y') = (H - y, x)`.

The effective page width and height in display space also swap for 90° and 270°:

```
if rotate in {90, 270}:
    display_width  = media_height
    display_height = media_width
else:
    display_width  = media_width
    display_height = media_height
```

Apply the rotation transform after inverting the y-axis (see Section 4), not before. The correct order is: extract glyphs in content-stream coordinates → invert y for reading order → apply rotation to map to display space.

---

## 4. Coordinate System Origin

PDF default user space has the **origin at the bottom-left corner of the page**, with x increasing rightward and y increasing upward. Human reading order is top-to-bottom. To convert a glyph's PDF y-coordinate to reading-order y:

```
reading_y = page_height - pdf_y
```

where `page_height` is the height of the CropBox (or MediaBox if CropBox equals MediaBox). Apply this inversion to every bounding box edge: a box `[x0, y0, x1, y1]` in PDF space becomes `[x0, page_height - y1, x1, page_height - y0]` in reading-order space (the vertical extents swap because the top of the original box is at `y1`, which maps to the smaller reading_y).

For **rotated pages**, the effective page height used in the inversion is the height of the display-space page, not the unrotated MediaBox height. Concretely: after computing the display_width/height swap from Section 3, use `display_height` as `page_height` in the inversion formula. Implement the full pipeline as: (1) apply CTM and text matrix to obtain unrotated page coordinates, (2) invert y to get reading-order coordinates in unrotated space, (3) apply the rotation matrix to get display-space reading-order coordinates.

---

## 5. Page Labels

The `PageLabels` entry in the document catalog is a number tree mapping page indices (zero-based) to label range dictionaries. Each entry marks the start of a new labeling range. A range entry may contain:

| Key | Description |
|---|---|
| `S` | Numbering style: `D` (decimal), `r` (lowercase roman), `R` (uppercase roman), `a` (lowercase alpha), `A` (uppercase alpha) |
| `P` | Prefix string (any PDF string) |
| `St` | Starting value (integer ≥ 1, default `1`) |

To compute the human-readable label for physical page index *i*:

1. Find the greatest key in the number tree that is ≤ *i*. That key is the range start *r*.
2. Offset within the range: `offset = i - r`.
3. Numeric value: `n = St + offset` (default `St = 1`).
4. Format *n* according to `S`; prepend `P` if present.

If no `S` key is present, the page has only the prefix (or is unlabeled). Documents with front matter commonly use lowercase roman numerals for the first several pages and decimal for the body; the labeled numbers therefore do not match the physical page order. The library must expose both the zero-based physical index and the string label independently.

---

## 6. Document Outline (Bookmarks)

The `/Outlines` entry in the catalog references the root outline dictionary. Each outline item dictionary contains:

- `Title`: a PDF string in either PDFDocEncoding or UTF-16BE (detected by the BOM `0xFE 0xFF`).
- `First` / `Last`: references to the first and last child items.
- `Next` / `Prev`: sibling links for items at the same level.
- `Count`: if present and positive, the item is open with that many visible descendants; negative means closed.
- `Dest` or `A`: a destination array/string or an action dictionary.

Traverse the tree with a recursive descent: for each item, process `Title` and destination, then recurse into `First` child, then follow `Next` siblings. When `A` is present and its `S` key is `/GoTo`, the `D` entry within `A` is the destination. When `Dest` is a string, resolve it via named destinations (Section 7). When `Dest` is an array, parse it directly.

Expose the outline as a flat or nested table-of-contents structure, each entry carrying the title string (decoded to Rust `String`), nesting depth, and resolved zero-based page index.

---

## 7. Named Destinations

Named destinations are stored in the document catalog under `Names` → `Dests` (a name tree) or, in older documents, directly as a `Dests` dictionary under the catalog. In either case, a name maps to a destination array.

Destination array formats and their semantics:

| Format | Meaning |
|---|---|
| `[page /XYZ left top zoom]` | Specific position on page |
| `[page /Fit]` | Fit entire page in viewport |
| `[page /FitH top]` | Fit page width, scroll to `top` |
| `[page /FitV left]` | Fit page height, scroll to `left` |
| `[page /FitR l b r t]` | Fit rectangle |
| `[page /FitB*]` | Variants of bounding-box fit |

In all cases, the first element is an indirect reference to a `/Page` object. Resolve this reference to a page index by walking the page tree to find the matching object number. Cache the object-number-to-index mapping after the first full tree traversal.

---

## 8. Document Catalog

The document catalog is reached via `trailer → Root`. Its entries relevant to text extraction:

| Key | Purpose |
|---|---|
| `Pages` | Root of the page tree |
| `Outlines` | Root of the outline tree |
| `Names` | Name trees including `Dests`, `EmbeddedFiles`, etc. |
| `PageLabels` | Number tree for page labeling |
| `AcroForm` | Interactive form fields |
| `Metadata` | Stream containing XMP metadata |
| `MarkInfo` | Indicates tagged PDF; `Marked: true` signals reading order is in StructTree |
| `StructTreeRoot` | Root of the logical structure tree |
| `Lang` | BCP 47 language tag for the document |
| `OCProperties` | Optional content (layers) configuration |

`Lang` should be used as the fallback language when no glyph-level or span-level language is specified. `MarkInfo` determines whether to prefer structure-tree reading order over geometric order (covered in the tagged-PDF research document). `OCProperties` affects which content streams are active; for extraction, treat all optional content as visible unless the caller specifies otherwise.

---

## 9. Resources Dictionary

Resources provide the named objects (fonts, images, graphics states, etc.) referenced in a content stream. A `Resources` dictionary has sub-dictionaries keyed by resource category:

| Key | Contents |
|---|---|
| `Font` | Map of resource name → font dictionary reference |
| `XObject` | Map of resource name → XObject stream reference |
| `ExtGState` | Map of resource name → graphics state parameter dictionary |
| `ColorSpace` | Map of resource name → color space definition |
| `Pattern` / `Shading` | Pattern and shading resources |
| `ProcSet` | Legacy array, ignore for extraction |

Resource names in content streams (e.g., `/F1` in `Tf`) are resolved against the active `Resources` dictionary. For a page's main content stream, use the page-level `Resources`; if absent, use the inherited resources resolved per Section 1. For Form XObjects and Type 3 fonts, each has its own `Resources` dictionary that takes precedence within its content stream.

Resolution is always strictly local: a resource name in a Form XObject is looked up in that XObject's own `Resources`, not the parent page's. Implement resource resolution as a stack that pushes the current stream's dictionary on entry and pops on exit.

---

## 10. Viewer Preferences and Page Layout

`ViewerPreferences` (in the catalog) and `PageLayout` affect multi-page presentation but not individual page content. Relevant keys:

| Key | Values | Extraction relevance |
|---|---|---|
| `PageLayout` | `SinglePage`, `OneColumn`, `TwoColumnLeft`, `TwoColumnRight`, `TwoPageLeft`, `TwoPageRight` | Two-column/two-page layouts imply pages are displayed as spreads; expose to caller for spread-aware output |
| `Direction` (in `ViewerPreferences`) | `L2R` (default), `R2L` | R2L affects which page is the left page in a spread; relevant for logical page ordering in output |
| `DisplayDocTitle` | boolean | Whether the viewer shows the document title from `Info` or the filename; informational only |

For extraction, `Direction: R2L` means that in a two-page spread, the higher-numbered page is on the left. A library consumer assembling pages into a multi-column layout should expose this flag and let the caller decide how to reorder output. At the single-page extraction level, `Direction` and `PageLayout` have no effect on glyph coordinates.

---

## Implementation Notes

- Build the object-number-to-page-index map eagerly on document open; it is used by destination resolution, outline traversal, and link annotation handling.
- Normalize all box arrays to `(x_min, y_min, x_max, y_max)` at parse time.
- Resolve inherited attributes into a flat `PageAttributes` struct at page-open time; do not re-traverse the parent chain during glyph extraction.
- Apply `UserUnit` scaling before any geometry comparison or coordinate inversion.
- Store the raw `Rotate` value from the resolved page dictionary; apply the transform matrix as the last step after all content-stream coordinate math is complete.
- Decode `Title` strings in outline items by checking for the UTF-16BE BOM; fall back to PDFDocEncoding (ISO Latin-1 with PDF-specific replacements for the 0x80–0x9F range) if the BOM is absent.
