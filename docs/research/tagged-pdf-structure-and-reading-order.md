# Tagged PDF Structure and Reading Order

## 1. Tagged PDF Overview

A "tagged" PDF is one that carries a logical structure tree alongside the visual content stream. The structure tree expresses the document's semantic organization — headings, paragraphs, lists, tables — independently of the glyph positions on the page. Tagging is declared in the document catalog via the **MarkInfo** dictionary (ISO 32000-2 §14.7.2):

```
MarkInfo << /Marked true >>
```

`/Marked true` asserts that every piece of real content is covered by a marked-content sequence and that a StructTreeRoot exists. Two companion keys are `/UserProperties` (Boolean, whether structure attributes carry user-defined properties) and `/Suspects` (Boolean; `true` warns that the tagging may be unreliable).

### Standards that mandate tagging

- **PDF/UA-1 (ISO 14289-1:2014)** — full tagging mandatory; every real content item must be a tagged marked-content sequence or an artifact; reading order must be encoded in the structure tree; ActualText, Alt, and Lang must be present where applicable.
- **PDF/UA-2 (ISO 14289-2, based on PDF 2.0)** — same accessibility intent, tightened to the PDF 2.0 object model.
- **PDF/A-1a (ISO 19005-1)** — Level A conformance requires tagging; Level B does not.
- **PDF/A-2a / PDF/A-3a** — Level A conformance of their respective parts likewise requires tagging.

### Authoring tool quality

| Tool | Tagging quality |
|---|---|
| Adobe InDesign (Articles panel configured) | High — structure order matches Articles panel order; artifact marking reliable |
| Microsoft Word (Save As PDF, modern versions) | Moderate — headings and paragraphs tagged; tables usually correct; images sometimes lack Alt; complex layouts may produce wrong reading order |
| LibreOffice Writer (Export as PDF/UA) | Moderate-to-good since 7.x; improving but table tagging sometimes produces extra empty cells |
| Adobe Acrobat Accessibility Checker / Make Accessible | Variable — uses heuristics on existing layout; common source of misordered structure trees |
| LaTeX (tagpdf package + pdflatex/lualatex) | Improving; math tagging still maturing |
| Programmatic (iText, Apache PDFBox, reportlab) | Depends on developer; often minimal tagging |

---

## 2. Structure Tree

The structure tree is rooted at the **StructTreeRoot** object, referenced from the document catalog as `/StructTreeRoot`. Key entries on StructTreeRoot (ISO 32000-2 §14.7.4):

| Key | Type | Meaning |
|---|---|---|
| `K` | array or dict | Immediate children (StructElem objects) |
| `IDTree` | name tree | Maps element IDs to StructElem objects |
| `ParentTree` | number tree | Maps MCID integers to the StructElem that contains them (see §3) |
| `ParentTreeNextKey` | integer | Next available key for ParentTree |
| `RoleMap` | dictionary | Maps non-standard structure types to standard ones |
| `ClassMap` | dictionary | Named attribute class definitions |

### StructElem dictionary

Each logical element is a StructElem dictionary (§14.7.5):

| Key | Type | Meaning |
|---|---|---|
| `S` | name | Structure type (required) |
| `P` | indirect ref | Parent StructElem or StructTreeRoot (required) |
| `K` | various | Kids: a StructElem, MCID integer, marked-content ref dict, object ref dict, or array of these |
| `ID` | byte string | Optional unique identifier |
| `Pg` | indirect ref | Page object where the element's content lives (can be inherited) |
| `A` | dict or array | Attributes |
| `C` | name or array | Attribute class names |
| `R` | integer | Revision number |
| `T` | text string | Title |
| `Lang` | text string | BCP 47 language tag (overrides document-level Lang) |
| `Alt` | text string | Alternative text (figures, formulas) |
| `ActualText` | text string | Overrides extracted glyphs for this element |
| `E` | text string | Expansion of an abbreviation |

### Standard structure types (ISO 32000-2 §14.8.4)

**Grouping elements** (contain other elements, no direct content):
`Document`, `DocumentFragment`, `Part`, `Sect`, `Div`, `Aside`, `NonStruct`, `Private`, `TOC`, `TOCI`

**Block-level elements**:
`P` (paragraph), `H` (generic heading), `H1`–`H6` (leveled headings), `Title`, `FENote`, `Sub`

**List elements**:
`L` (list), `LI` (list item), `Lbl` (label/bullet), `LBody` (body of item)

**Table elements**:
`Table`, `TR` (row), `TH` (header cell), `TD` (data cell), `THead`, `TBody`, `TFoot`

**Inline elements**:
`Span`, `Em`, `Strong`, `Link`, `Annot`, `Form`, `Ruby`, `RB`, `RT`, `RP`, `Warichu`, `WT`, `WP`

**Illustration/media**:
`Figure`, `Formula`, `Caption`

The `RoleMap` on StructTreeRoot maps non-standard type names to the nearest standard type, allowing extraction code to normalize custom types from authoring tools without special-casing each tool.

---

## 3. Marked Content and MCIDs

In a content stream, a **marked-content sequence** wraps real content between `BMC`/`BDC` and `EMC` operators. Tagged content uses `BDC` with a property list that includes `/MCID`:

```
/P <</MCID 7>> BDC
  BT ... (text operators) ... ET
EMC
```

The MCID is a non-negative integer unique within the page's content stream (and XObject content streams referenced from that page). MCIDs are the bridge from visual content back to the structure tree.

### Parent tree

The **ParentTree** (a number tree on StructTreeRoot) maps:
- For standard tagged content: `MCID → StructElem` that directly contains it
- For XObject content streams: the key is the XObject's structural parent key

To walk from a rendered text run to its logical element:

1. Identify the page object.
2. Find the MCID from the `BDC` property list in the content stream.
3. Look up the page's **StructParents** integer in the page dictionary.
4. Index into ParentTree at `StructParents` to find the array of parent StructElems for that page. The array index is the MCID itself, giving the direct parent StructElem.
5. Walk up via `P` links to the root to reconstruct ancestry.

A StructElem's `K` array can contain:
- An integer (MCID on the same page as the element's `Pg`)
- A **marked-content reference dictionary** `<</Type /MCR /Pg <pageref> /MCID n>>` — used when content is on a different page than the element's own `Pg` (multi-page elements such as a paragraph split across pages)
- An **object reference dictionary** `<</Type /OBJR /Obj <indirectref>>>` — links to an annotation or XObject

Multi-page StructElems (e.g., a `Table` spanning two pages) use MCR dicts with explicit `/Pg` entries for each page, so extraction must collect MCIDs page-by-page and union them under the single logical element.

---

## 4. Reading Order from Structure

The order of children in a StructElem's `K` array encodes **logical reading order** independent of glyph x/y coordinates. A conforming extractor should:

1. Walk the structure tree depth-first in `K` array order.
2. At each leaf MCID, retrieve the text from the corresponding marked-content sequence.
3. Concatenate text in tree traversal order, inserting whitespace at element boundaries per the element type (block elements → newline; inline elements → preserve or single space as context demands).

### Common misordering problems

Auto-taggers (Acrobat's "Make Accessible", Google Docs export) frequently produce structure trees whose element order mirrors the **content stream order** rather than true reading order. In multi-column PDFs, this can interleave column 1 and column 2 text at the paragraph level.

Detection heuristics:
- Compare bounding boxes of consecutive sibling `P` elements: if the x-origin of element n+1 is dramatically less than that of element n while y-coordinates have not advanced, the two elements are probably in separate columns and the tree order is suspect.
- Check `/Suspects true` in MarkInfo — this is the authoring tool's own admission of uncertainty.
- Count `P` elements whose bounding boxes overlap horizontally but are separated vertically by more than one line-height; a high count signals column-interleaved tagging.

When misordering is detected, fall back to spatial reading-order reconstruction (§5) while still using the structure tree for semantic type labeling (headings, lists, tables).

---

## 5. Reading Order Without Structure

When `/Marked` is absent or `false`, or when `/Suspects true` is set and validation fails, reading order must be inferred from glyph geometry.

### Spatial preprocessing

Collect all text objects from the content stream with their bounding boxes (derived from the text matrix, font metrics, and glyph widths). Group glyphs into **text runs** sharing the same font, size, and baseline, then cluster runs into **lines** by overlapping y-ranges within a vertical tolerance (~0.5× line height).

### Column detection: x-gap analysis

Project all text bounding-box x-extents onto the x-axis. Find gaps (ranges of x with no text) that span at least ~90% of the page height. Each gap boundary is a candidate **column gutter**. Sort columns left-to-right; sort lines within each column top-to-bottom.

This works for simple two- or three-column layouts but fails for mixed layouts (one-column intro, then two columns below).

### Recursive XY-cut (Ha et al. 1995, adapted for PDF)

1. Given a rectangular region containing a set of text bounding boxes, project onto x and y axes.
2. Find the widest gap on the **dominant axis** (try y first — a full-width horizontal gap separating header from body scores higher than a narrow x gap).
3. Split the region at the gap into two sub-regions; recurse.
4. Base case: no gap exceeds a minimum threshold (e.g., one em-width on x, one line-height on y).
5. The recursion tree defines a binary partition; perform an in-order traversal to recover reading order.

XY-cut handles mixed-column pages well. Key parameters: minimum gap width (x-cut: typically 1–2 em; y-cut: typically 0.5–1 line-height), and a maximum depth to prevent over-segmentation of slightly misaligned text.

### Sidebars, footnotes, headers/footers

Heuristic classification before running XY-cut:

- **Headers/footers**: text regions whose y-centroid is within 10% of page height from the top or bottom edge, containing a small number of runs (< 3 lines). Suppress from main flow; emit separately.
- **Footnotes**: text at the bottom of the page body (above footer zone), with smaller font size than body text and often preceded by a superscript numeral. Detected by size delta > 20% relative to modal body font size.
- **Sidebars / pull quotes**: isolated text regions (large whitespace moat on all sides) with x-range contained within the column rather than spanning it. XY-cut naturally isolates these as leaf nodes; they can be reclassified by checking if any fragment overlaps the main text column's x-range.

### Overlapping text spans

Overlapping bounding boxes occur with drop caps, watermarks, or decorative text placed over body text. Resolution:

- If one span has a fill color with near-zero opacity or is rendered in `Tr 3` (invisible), discard it.
- If z-order (content stream sequence) places one span significantly before another and they share x/y overlap, keep the later one (compositing intent is that it replaces the earlier).
- Otherwise keep both and emit the later content-stream span first.

---

## 6. Artifacts

PDF distinguishes **real content** from **artifacts**: pagination decorations that should not be extracted as document text. In tagged PDFs, artifacts are marked with `/Artifact` instead of a structure type (ISO 32000-2 §14.8.2.2):

```
/Artifact <</Type /Pagination /Subtype /Header>> BMC
  BT ... ET
EMC
```

Artifact types: `/Pagination` (headers, footers, page numbers, folios), `/Layout` (column rules, background decorations, gutters), `/Page` (cut marks, color bars in press PDFs), `/Background`.

Pagination subtypes: `/Header`, `/Footer`, `/Watermark`.

An extractor must skip all marked-content sequences whose outermost `BDC` tag is `/Artifact`. In untagged PDFs, artifact detection is heuristic:

- **Page numbers**: a lone integer or "Page N of M" string in the header/footer zone with no semantic relationship to surrounding text.
- **Running headers/footers**: identical or near-identical text appearing at the same y-position across multiple pages. Compare across ≥3 consecutive pages; if the text edit-distance is < 20% (allowing page-number substitution), classify as artifact.
- **Decorative rules and backgrounds**: non-text content (paths, images) in header/footer zones — always suppress.
- **Watermarks**: large text at low opacity or with a `Tr` value ≥ 4, centered on the page.

---

## 7. PDF/UA Attributes Relevant to Extraction

PDF/UA-1 (ISO 14289-1) mandates several StructElem attributes that directly alter what text an extractor should produce.

### ActualText (§14.9.4 of ISO 32000-2)

Present on any StructElem or marked-content property list. When set, it **completely replaces** the visual glyph sequence for extraction purposes. Use cases: ligatures rendered as single glyphs but representing multiple characters (e.g., the `ﬁ` glyph should yield "fi"); decorative fonts where glyph names are unreliable; redacted text replaced with a placeholder; mathematical operators.

Extraction rule: if ActualText is present on a StructElem, output ActualText for the entire element subtree and do not recurse into child MCIDs. If ActualText is on an MCR/BDC property list, it overrides only that marked-content sequence.

### Alt (§14.9.3)

The alternative text attribute on `Figure`, `Formula`, and other non-text elements. An extractor producing plain text should emit Alt as a bracketed description or inline alt-text marker. PDF/UA-1 clause 7.3 requires Alt on every Figure that conveys information. An absent Alt on a Figure is a conformance violation; the extractor should emit a warning and produce a placeholder (e.g., `[Figure]`).

### E (expansion, §14.9.5)

Present on elements with structure type `Span` (or any inline element) to expand abbreviations. When E is present, the extractor should substitute the expansion for the visible abbreviation in the output text stream. Example: a `Span` with visible text "PDF/UA" and `E = "Portable Document Format / Universal Accessibility"`.

### Lang (§14.9.2)

BCP 47 language tag, inheritable from parent elements and ultimately from the document catalog's `/Lang` entry. Lang does not alter the extracted text but is essential metadata for downstream NLP (tokenization, stemming, OCR post-correction). Extraction should propagate Lang from the nearest ancestor that declares it and expose it per-element or per-run in structured output formats.

### Attribute inheritance and the attribute object

Attributes may be stored inline on the StructElem (`/A` key) or via named classes (`/C` key referencing ClassMap). When multiple attributes apply (inline + class), inline values take precedence over class values; class values are applied in array order. ActualText and Alt are not inheritable (they apply to exactly the element on which they appear, not descendants), while Lang is inheritable.

---

## Summary: Extraction Decision Tree

```
Has StructTreeRoot?
├─ Yes → /Marked true and /Suspects false?
│   ├─ Yes → Walk structure tree in K-array order; apply ActualText, Alt, E, Lang;
│   │         skip /Artifact sequences.
│   └─ No  → Validate structure order (spatial consistency check);
│             if order is correct → use structure tree;
│             if disordered → use spatial algorithm, annotate with structure types.
└─ No  → Full spatial pipeline: heuristic artifact suppression, XY-cut column
          detection, line clustering, reading order by column-then-top-to-bottom.
```

The structure tree, when trustworthy, yields semantically richer and more reliably ordered output than any spatial algorithm can. Spatial methods are the fallback for legacy, scanned, or poorly-tagged documents, but they remain essential because a significant fraction of PDFs in the wild are untagged or carry unreliable tags.
