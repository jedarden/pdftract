# Optional Content Groups (Layers) in PDF Extraction

## 1. OCG Overview

Optional Content Groups (OCGs) are named, independently togglable layers defined in ISO 32000-2 §8.11. Each OCG represents a logical grouping of PDF content — text, graphics, images, or annotations — that can be rendered or suppressed as a unit. Layers were introduced to support use cases such as technical drawings (where construction lines appear on a separate layer), multilingual documents (one layer per locale), and print-vs-screen variants.

From an extraction standpoint, OCGs are critical because content on an off layer **must not** be treated as visible text. A PDF viewer filters out off-layer content before rendering; an extraction library that ignores OCG state will silently include invisible text — watermarks, alternate-language duplicates, or suppressed annotations — mixed with the visible body text.

OCGs are registered in the document catalog under `/OCProperties` (ISO 32000-2 §8.11.4). The structure is:

```
/OCProperties <<
  /OCGs [ ref1 ref2 ref3 ]   % all OCGs in the document
  /D << ... >>               % default configuration dictionary
  /Configs [ << ... >> ]     % optional additional named configurations
>>
```

`/OCGs` lists every OCG indirect reference in the document. A conforming reader must process this array to build the initial visibility table before parsing any content stream.

---

## 2. OCG Dictionary

Each OCG is an indirect object of the form:

```
<< /Type /OCG
   /Name (English Text)
   /Intent /View
   /Usage << ... >>
>>
```

- `/Type /OCG` — required; marks the object as an OCG.
- `/Name` — required; a UTF-16BE or PDFDocEncoding string giving a human-readable layer name (e.g., `"Background"`, `"English"`, `"HeaderFooter"`). Used for display in layer panels and, in pdftract, as the `ocg_name` tag on extracted spans.
- `/Intent` — optional; a name or array of names (`/View`, `/Design`, or application-defined). `/View` means the OCG governs visibility for screen rendering and, by convention, for extraction. `/Design` means it governs visibility in design tools. If absent, treat as `/View`.
- `/Usage` — optional dictionary; machine-readable context hints that drive automatic state computation from the `/AS` (auto-state) rules in the default configuration.

OCGs are referenced from content streams via the property list mechanism (see §6), from XObject dictionaries via `/OC`, and from annotation dictionaries via `/OC`.

---

## 3. Usage Dictionary

The `/Usage` dictionary on an OCG supplies structured metadata for automatic visibility determination. The relevant subkeys are:

- **`CreatorInfo`** — `<< /Creator (ApplicationName) /Subtype /Technical >>`. Informational; identifies the originating application and layer purpose.
- **`Language`** — `<< /Lang (fr-CA) /Preferred /ON >>`. The `/Lang` value is a BCP 47 language tag. `/Preferred` specifies `ON` or `OFF` — whether this is the preferred language layer when automatic language selection is active. Critical for multilingual PDFs: an auto-state rule with event `/View` applied to `/Language` will turn on only the preferred-language layers and turn off others.
- **`Export`** — `<< /ExportState /ON >>`. Controls layer state when the document is exported (saved as PDF). Values: `/ON` or `/OFF`.
- **`Zoom`** — `<< /min 0.5 /max 2.0 >>`. The layer is visible only when the zoom factor is within `[min, max]`. For extraction, zoom is conventionally treated as 1.0 unless the caller specifies otherwise.
- **`Print`** — `<< /Subtype /Watermark /PrintState /ON >>`. Governs layer state when printing. `/Subtype` can be `/Watermark` or application-defined. Watermark layers visible only on print should be excluded from extraction by default.
- **`View`** — `<< /ViewState /ON >>`. Explicitly sets layer state for on-screen viewing. This is the primary signal for extraction; a `/ViewState /OFF` layer is invisible on screen and should be excluded.
- **`User`** — `<< /Type /Ind /Name [(Alice)] >>`. User-based visibility; category is `/Ind` (individual) or `/Grp` (group). Rarely relevant for extraction.
- **`PageElement`** — `<< /Subtype /HF >>`. Marks the layer as containing page elements of a specific functional type. `/HF` (Header/Footer) is the defined value. Extraction policy for HF layers is covered in §9.

---

## 4. Optional Content Membership Dictionary (OCMD)

An OCMD expresses a boolean combination of OCG states. It is not an OCG itself but acts as a computed visibility gate:

```
<< /Type /OCMD
   /OCGs [ ref1 ref2 ]
   /P /AnyOn
>>
```

- `/OCGs` — a single OCG reference or an array. If a single reference, the OCMD is equivalent to a direct OCG reference (with policy `/AnyOn`).
- `/P` — the policy applied to the `/OCGs` set:
  - `/AllOn` — visible iff every listed OCG is on.
  - `/AllOff` — visible iff every listed OCG is off.
  - `/AnyOn` — visible iff at least one listed OCG is on.
  - `/AnyOff` — visible iff at least one listed OCG is off.
- `/VE` — optional visibility expression array (PDF 2.0, §8.11.2.3); a recursive boolean expression using `And`, `Or`, `Not` operators over OCG references. Implement `/VE` evaluation as a tree walk; fall back to `/P`+`/OCGs` if `/VE` is absent.

Resolving OCMD state:

```rust
fn resolve_ocmd(ocmd: &Ocmd, states: &HashMap<ObjId, bool>) -> bool {
    let ocg_states: Vec<bool> = ocmd.ocgs.iter()
        .map(|id| *states.get(id).unwrap_or(&true))
        .collect();
    match ocmd.policy {
        Policy::AllOn  => ocg_states.iter().all(|&s| s),
        Policy::AllOff => ocg_states.iter().all(|&s| !s),
        Policy::AnyOn  => ocg_states.iter().any(|&s| s),
        Policy::AnyOff => ocg_states.iter().any(|&s| !s),
    }
}
```

---

## 5. Default Viewing State

The `/D` entry of `/OCProperties` is the default configuration dictionary. It establishes the initial OCG visibility table:

```
/D <<
  /Name (Default)
  /BaseState /OFF
  /ON  [ ref1 ref2 ]
  /OFF [ ref3 ]
  /AS  [ << /Event /View /OCGs [ ref1 ref2 ] /Category [ /View /Zoom ] >> ]
  /Order [ ... ]
  /RBGroups [ ... ]
  /Locked [ ... ]
>>
```

**Computing initial visible set:**

1. Set all OCGs to the `/BaseState` value (`ON`, `OFF`, or `Unchanged`; for the `/D` entry, `Unchanged` is equivalent to `ON`).
2. Apply the `/ON` array: set each listed OCG to on.
3. Apply the `/OFF` array: set each listed OCG to off. `/ON` and `/OFF` take explicit precedence over `/BaseState`.
4. Process `/AS` (auto-state) entries. Each entry specifies an event (e.g., `/View`), a set of OCGs, and usage categories. For each category, read the corresponding key from the OCG's `/Usage` dictionary and apply the state. For extraction, process only entries with `/Event /View`.

`/RBGroups` defines radio-button groups — only one OCG in the group may be on at a time. Honor this constraint when applying `/AS` overrides.

`/Locked` lists OCGs whose state may not be changed by the user; treat locked OCGs as fixed at their computed state.

---

## 6. Content Stream Marking

OCGs gate content in content streams through the **Marked Content** mechanism (ISO 32000-2 §14.6). The operator pair is `BDC` / `EMC`. When an OCG or OCMD governs a content region, the marking takes the form:

```
/OC /Lyr1 BDC
  ... text operators ...
EMC
```

where `/Lyr1` is a name that resolves via the page's `/Resources /Properties` dictionary to an OCG or OCMD indirect reference:

```
/Resources <<
  /Properties <<
    /Lyr1 ref_to_ocg_or_ocmd
  >>
>>
```

Alternatively, the OCG dictionary can be inlined directly in the `BDC` property list:

```
/OC << /Type /OCG /Name (English) >> BDC
```

though inline objects are rare in well-formed PDFs.

**Nesting.** `BDC`/`EMC` pairs can be nested. A content stream may have an outer OCG marking a section and inner OCG markings for subsections. The visibility rule is conjunctive: a span is visible only if **all** enclosing OCG contexts are visible. Implement this as a stack:

```rust
struct OcgStack(Vec<bool>);

impl OcgStack {
    fn push(&mut self, visible: bool) { self.0.push(visible); }
    fn pop(&mut self) { self.0.pop(); }
    fn is_visible(&self) -> bool { self.0.iter().all(|&v| v) }
}
```

On each `BDC` with an `/OC` property, resolve the referenced OCG or OCMD to a boolean and push it. On `EMC`, pop. Text operators encountered while `is_visible()` returns `false` are discarded.

---

## 7. XObject and Annotation OCG References

**Form XObjects** — a Form XObject (stream with `/Subtype /Form`) may carry an `/OC` entry:

```
<< /Type /XObject /Subtype /Form /OC ref_to_ocg ... >>
```

Before descending into the XObject's content stream to extract text, resolve the `/OC` entry. If the referenced OCG or OCMD is off, skip the entire XObject. This check is independent of any `BDC`/`EMC` marking inside the XObject itself; both must be satisfied for content to be visible.

**Annotations** — annotation dictionaries also support `/OC`:

```
<< /Type /Annot /Subtype /Widget /OC ref_to_ocg ... >>
```

For annotations with appearance streams (`/AP`), the appearance stream text is visible only if the annotation's `/OC` resolves to on. Text from invisible annotation appearances must be excluded.

---

## 8. Multilingual Layer Pattern

A common authoring pattern places translations of the same document on separate OCGs, one per locale, with identical layout geometry. The Usage dictionary's `/Language` subkey carries the BCP 47 tag:

```
OCG_EN: /Usage << /Language << /Lang (en) /Preferred /ON >> >>
OCG_FR: /Usage << /Language << /Lang (fr) /Preferred /OFF >> >>
OCG_ES: /Usage << /Language << /Lang (es) /Preferred /OFF >> >>
```

The `/AS` entry in the default configuration fires on `/Event /View` with `/Category [/Language]`, turning on the preferred language layer and turning off others.

For pdftract, extraction policy options:

- **Default locale extraction** — compute the visible set from `/D` (including `/AS` processing); only extract text from the resulting on-layers. The caller gets clean, single-language output.
- **Target locale extraction** — caller specifies a BCP 47 tag; the library overrides the state table to enable only OCGs whose `/Usage/Language/Lang` matches (exact or prefix match per BCP 47 §4.4) and disables others before extraction.
- **All-layers extraction** — extract all layers regardless of state; tag each span's `ocg_name` with the layer's `/Name` value. The caller can then filter by locale post-extraction.

When all-layers extraction is active, spans at the same position from different language layers will have overlapping bounding boxes. The caller must deduplicate or select by `ocg_name`.

---

## 9. PageElement HF Layers

The `PageElement` usage subtype `/HF` explicitly designates a layer as containing headers and/or footers. This is the PDF specification's own semantic label for running header/footer content.

```
/Usage << /PageElement << /Subtype /HF >> >>
```

Extraction policy for HF layers:

- **Default:** exclude HF-layer content from the primary body text stream; emit it in a separate `headers_footers` bucket or label spans with `zone: HeaderFooter`.
- **Explicit inclusion:** caller opts in via an extraction flag to include HF content merged into body text (rarely useful but sometimes required for form extraction).
- **Detection fallback:** if a layer has no `PageElement` usage entry but its `/Name` matches heuristics like `"Header"`, `"Footer"`, `"Running Head"`, log a warning rather than auto-excluding — only the Usage dictionary is normative.

---

## 10. Extraction Policy

### Default behavior

Extract only content on layers that are **on** in the default viewing state (computed per §5). This matches what a conforming viewer displays. No `ocg_name` metadata is emitted on spans; OCG structure is transparent to the caller.

### Extraction modes

| Mode | Description | `ocg_name` on span |
|---|---|---|
| `DefaultVisible` | Only on-layers per `/D` | absent |
| `TargetLayer(name)` | Only the named OCG by `/Name` match | absent |
| `TargetLocale(lang)` | Only OCGs matching BCP 47 tag in `/Language` | absent |
| `AllLayers` | All layers regardless of state | present |
| `AllLayersVisible` | Only on-layers, but tagged | present |

### Span metadata

When `ocg_name` tagging is active, each span carries:

```rust
pub struct Span {
    pub text: String,
    pub bbox: Rect,
    pub ocg_name: Option<String>,  // None if not inside any OCG marking
    // ... other fields
}
```

`ocg_name` reflects the **innermost** named OCG in the `BDC` stack at the point the span was extracted. If a span is covered by multiple nested OCG markings, the innermost `/Name` is used; all enclosing states must be on for the span to be included in non-`AllLayers` modes.

### Implementation notes

- Build the OCG state table once per document from `/OCProperties/D`; cache it.
- Reuse the same table for all pages — OCG state is document-scoped, not page-scoped.
- The `/Configs` array provides alternative named configurations (e.g., "Print", "Screen"). Expose these to callers who need to extract against a non-default configuration.
- When `/OCProperties` is absent, treat all content as unconditionally visible (the document has no layers).
- Log unresolvable `/OC` references (dangling indirect refs) as warnings; do not silently discard the content — treat it as visible to avoid false negatives.
