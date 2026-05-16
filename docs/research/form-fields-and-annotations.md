# Form Fields and Annotations: AcroForm, XFA, and Annotation Text Extraction

## 1. AcroForm Overview

The document catalog (`/Type /Catalog`) may contain an `/AcroForm` dictionary. This dictionary is the root of all interactive form machinery in the document. Its primary entries are:

- **`Fields`** — an array of indirect references to field dictionaries that are direct children of the field hierarchy (the root fields). Fields not referenced here are reachable only through their parent's `Kids` array.
- **`DA`** — a document-level default appearance string, used when a field lacks its own `DA`.
- **`DR`** — a resource dictionary shared across all fields; typically contains the `/Font` sub-dictionary mapping font names used in appearance strings.
- **`NeedAppearances`** — a boolean flag; when `true`, viewers must regenerate appearance streams before rendering. A library performing text extraction should not depend on pre-generated appearances being present.
- **`XFA`** — present only in XFA documents; see section 5.

The field hierarchy is a tree. Non-terminal (intermediate) nodes group related fields and act as inheritance sources; they carry no value themselves. Terminal fields are the leaf nodes — they define a field type and hold a value. The distinction is made by the presence or absence of the `/FT` (field type) entry: a terminal field has `/FT`; a non-terminal node may omit it, inheriting from a parent or leaving it undefined.

**Inherited attributes.** A terminal field that lacks `DA`, `FT`, `Ff` (field flags), or `DV` inherits those values by walking up the `Parent` chain until a value is found or the chain is exhausted. When extracting field data, the implementation must perform this walk for each attribute independently.

## 2. Field Types

### 2.1 Text Fields (`/FT /Tx`)

A text field stores a user-entered string. Key entries:

- **`V`** — the current value, a string object (PDFDocEncoding or UTF-16BE with BOM).
- **`DV`** — the default value, same encoding rules as `V`.
- **`MaxLen`** — maximum number of characters permitted.
- **`DA`** — default appearance string: a content-stream fragment such as `/Helv 12 Tf 0 g`. The font name is resolved against the `DR` dictionary in `/AcroForm`.
- **`Ff` bit 13** (`Multiline`) — when set, the field accepts multiple lines of text. Extraction should preserve embedded newlines in `V`.
- **`Ff` bit 14** (`Password`) — the value should be treated as sensitive; some implementations may redact it.

### 2.2 Button Fields (`/FT /Btn`)

Three subtypes are distinguished by `Ff`:

- **Pushbutton** (`Ff` bit 17 set) — carries no persistent value; its purpose is to trigger actions. No text value to extract.
- **Checkbox** — `V` holds the current appearance state name (e.g., `/Yes` or `/Off`); `DV` holds the default. The `AS` entry in the widget annotation mirrors the checked state and is the authoritative indicator when rendering; extraction should prefer `V` on the field, cross-referencing `AS` to confirm.
- **Radio group** — a non-terminal field node whose `Kids` are individual radio button widgets. Each kid widget has an `AS` entry whose value matches the export value when selected. The parent's `V` holds the export value of the currently selected option. To find the selected label, match `V` against the `AS` values of all kids.

### 2.3 Choice Fields (`/FT /Ch`)

- **`Opt`** — an array of options. Each element is either a string (the export value equals the display value) or a two-element array `[export_value, display_string]`.
- **`V`** — a string (single selection) or array of strings (multi-select when `Ff` bit 22 is set). Contains the export value of the selected option(s).
- **`TI`** — top index; the first visible option in a scrollable listbox.
- **`Ff` bit 18** — when set, the field is a combo box rather than a listbox.

To extract the display text for a selection, locate the entry in `Opt` whose export value matches `V`.

### 2.4 Signature Fields (`/FT /Sig`)

`V` is a signature dictionary, not a string. Text extraction is out of scope for signature fields; record the field name and type, but emit no text value.

## 3. Field Value Extraction

**String decoding.** A string value in a field is encoded in either PDFDocEncoding or UTF-16BE. The BOM `\xFE\xFF` at the start of the byte sequence signals UTF-16BE; otherwise, assume PDFDocEncoding. Implement a lookup table for the 39 PDFDocEncoding code points that differ from Latin-1.

**Text fields.** Read `V` directly. If `V` is null or absent, fall back to `DV`. If both are absent, emit an empty string.

**Checkboxes.** Read `V` from the field; the value is a name object. Any value other than `/Off` (the conventional unchecked state) indicates a checked state. Confirm against `AS` in the widget annotation.

**Radio buttons.** Read `V` from the parent field. Walk `Kids`; the selected kid is the one whose `AS` matches `V`. Emit the matching export value or display string from `Opt` if present.

**Choice fields.** Read `V` (string or array). For each selected export value, find the corresponding display string in `Opt`. If `Opt` contains plain strings, export value equals display string.

## 4. Widget Annotations

Every terminal field is associated with one or more widget annotations. A field may merge with its single widget (the same dictionary object serves both roles) or the field may have a `Kids` array of separate widget dictionaries, each with `/Subtype /Widget`. Widgets carry:

- **`Rect`** — a four-element array `[x1 y1 x2 y2]` in default user space units, giving the bounding box of the field on the page. This is the `bbox` used in output.
- **`P`** — indirect reference to the page object on which the widget appears.
- **`AP`** — appearance dictionary with up to three sub-dictionaries: `N` (normal), `R` (rollover), `D` (down). Each entry is either a Form XObject stream or a sub-dictionary keyed by appearance state names (used for checkboxes and radio buttons).

**Extracting text from appearance streams.** When `V` is absent or when the document sets `NeedAppearances false` and has pre-generated streams, the `N` appearance stream is a Form XObject containing a content stream. This stream can be processed identically to a page content stream: extract text operators (`Tj`, `TJ`, `'`, `"`) using the font resources in the stream's own `/Resources` dictionary. This is the fallback path for fields whose value is encoded only in the rendered appearance.

## 5. XFA Forms

The `/XFA` entry in `/AcroForm` contains the XFA form data. Its value is either a single stream (the complete XFA document as XML) or an array of alternating name/stream pairs representing named XFA packets:

```
[ /xdp:xdp stream /template stream /datasets stream /config stream ... ]
```

XFA versions range from 2.0 through 3.3; the version is declared in the `xdp:xdp` root element's namespace URI.

**Relevant packets:**

- **`template`** — defines the form structure: field names, types, binding expressions, and layout. Field names follow XPath-like dot-notation (`form1.page1.subform1.field1`).
- **`datasets`** — contains the actual data bound to the template. The `xfa:data` element holds a tree of XML elements whose tag names and text content correspond to field values.

**Extraction algorithm for XFA.** Parse the `datasets` XML; walk the element tree depth-first. For each leaf text node, construct its full path by joining ancestor element names with `.`. Emit `(path, text_content)` pairs. For structured arrays, the XFA spec uses sibling elements with the same tag name; track occurrence indices.

**Hybrid XFA documents.** Some PDFs contain both `/XFA` and an AcroForm `Fields` array. The AcroForm fields serve as a compatibility layer for viewers that do not support XFA. When `/XFA` is present, prefer XFA data extraction; the AcroForm values may be stale or absent.

## 6. Annotation Types Relevant to Text Extraction

Annotations are listed in the `Annots` array of a page dictionary. Each annotation dictionary has `/Type /Annot` and a `/Subtype` that determines its semantics.

- **`Text`** (sticky note) — `Contents` holds the annotation text; `T` holds the author name; `RC` may hold rich text.
- **`FreeText`** — text rendered directly on the page surface. `Contents` is the plain text; `DA` and `DS` control styling; `RC` may carry formatted content.
- **Markup annotations** (`Highlight`, `Underline`, `Squiggly`, `StrikeOut`) — these reference existing page text via `QuadPoints`, an array of 8n numbers defining n quadrilaterals over the marked text. `Contents` carries the reviewer's comment.
- **`Link`** — `Contents` may hold descriptive text; the `A` entry holds an action dictionary (`/S /URI` with a `URI` string, or `/S /GoTo` with a destination).
- **`Stamp`** — `Contents` is the stamp text (e.g., "Approved").
- **`Popup`** — associated with a markup annotation via `Parent`; `Contents` mirrors the parent's comment. Skip independently; capture through the parent.

## 7. Rich Text (`RC` Field)

The `RC` entry in both annotation dictionaries and text field dictionaries holds an XHTML-like string defined by PDF spec §12.7.3.4. The markup uses a restricted subset: `<body>`, `<p>`, `<span>` elements with inline style attributes (`font-family`, `font-size`, `font-weight`, `font-style`, `color`, `text-decoration`).

**Plain-text extraction.** Parse the XML, discard all tags, and concatenate text node content. `<p>` boundaries map to newlines. `<br/>` within a paragraph maps to a newline.

**Formatted extraction.** For callers that want span metadata, capture each `<span>` with its computed style. The style attribute follows a semicolon-separated CSS-like syntax; parse it into a key-value map. Relevant keys: `font-weight: bold`, `font-style: italic`, `color: #rrggbb` or `color: rgb(r,g,b)`.

When both `RC` and `Contents` are present, `RC` is the richer source. When `RC` is absent, fall back to `Contents`.

## 8. Extracting Annotation Text

**Iteration.** For each page, read the `Annots` array. Each element is an indirect reference to an annotation dictionary. Resolve each reference and filter by `Subtype`.

**Fields to extract per annotation:**

| Entry | Meaning |
|---|---|
| `Contents` | Primary text content |
| `RC` | Rich text override (parse for plain text) |
| `T` | Author / title |
| `Subtype` | Annotation kind |
| `Rect` | Bounding box on the page |
| `QuadPoints` | Highlighted region (markup annotations only) |

**Spatial ordering.** To interleave annotation text with body text, compute the center of `Rect` (or the centroid of all `QuadPoints` quads) and sort annotations by their vertical position (descending `y`) then horizontal position (ascending `x`), matching the reading-order convention used for body text.

**Markup annotation text recovery.** For `Highlight`, `Underline`, `Squiggly`, and `StrikeOut`, the `QuadPoints` array identifies the page content already extracted by the main text extraction pipeline. A library can optionally resolve these quads against the extracted glyph positions to return the marked span as a first-class excerpt, in addition to the `Contents` comment.

## 9. Output Representation

**Form fields.** Emit a top-level `form_fields` array. Each entry is a struct:

```rust
pub struct FormField {
    pub name: String,          // fully qualified field name (dot-joined)
    pub field_type: FieldType, // Tx | Btn | Ch | Sig
    pub value: Option<String>, // decoded current value
    pub default_value: Option<String>,
    pub page: Option<u32>,     // 0-indexed page number from widget P entry
    pub bbox: Option<[f32; 4]>,// [x1, y1, x2, y2] from widget Rect
}
```

**Annotations.** Emit an `annotations` array per page:

```rust
pub struct Annotation {
    pub kind: AnnotationKind,  // Text | FreeText | Highlight | ... 
    pub contents: Option<String>,
    pub rich_text: Option<String>, // raw RC XML if present
    pub author: Option<String>,
    pub bbox: [f32; 4],
    pub quad_points: Vec<[f32; 8]>, // populated for markup annotations
}
```

**Caller-controlled inclusion.** Expose boolean flags on the extraction configuration:

```rust
pub struct ExtractionOptions {
    pub extract_forms: bool,
    pub extract_annotations: bool,
    pub prefer_xfa: bool, // when XFA present, skip AcroForm field scan
}
```

When `extract_forms` is `false`, skip the AcroForm traversal entirely. When `extract_annotations` is `false`, skip the `Annots` array on each page. Both default to `true`. When `prefer_xfa` is `true` and `/XFA` is present, use XFA dataset extraction and suppress AcroForm field output to avoid duplicates.
