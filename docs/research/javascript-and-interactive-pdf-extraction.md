# JavaScript, Interactive Elements, and Dynamic Content in PDFs

## Overview

Interactive PDFs present a unique extraction challenge: they contain content whose visible form depends on runtime behavior — JavaScript execution, form field state, and in some cases entirely XML-driven rendering. pdftract must correctly extract static field values and form content from these documents without executing any code and without misrepresenting computed or absent values as extracted text.

---

## PDF JavaScript: Structure and Extraction Implications

PDF supports JavaScript at multiple levels of the document structure, governed by ISO 32000 and the Adobe Acrobat JavaScript API extensions. At the document level, named JavaScript functions are registered in the document catalog's `/Names` tree under the `/JavaScript` key. Each entry maps a name string to a script object containing the source text. These functions may be called by event-driven actions elsewhere in the document.

JavaScript actions can also appear as `/OpenAction` in the document catalog, which triggers script execution when the file is opened. Individual pages carry `/AA` (Additional Actions) dictionaries with entries for page open (`/O`) and page close (`/C`) events. Form fields carry their own `/AA` dictionaries covering events such as keystroke (`/K`), validation (`/V` within `/AA`, distinct from the value key `/V` in the field dictionary), format (`/F`), and calculate (`/C`).

From pdftract's perspective, JavaScript has no effect on the byte-level content of the PDF. The page content streams, font tables, glyph sequences, and form field dictionaries are static data structures laid down at write time. JavaScript manipulates the viewer's runtime state — it can set field values, trigger document actions, or produce alert dialogs — but it cannot alter the bytes that pdftract reads during extraction. This means pdftract's correct posture is to treat JavaScript as inert: never execute it, never interpret it as extractable text, and never depend on it to understand field values.

However, pdftract must not crash when encountering JavaScript objects. A document containing `/JavaScript` in its names tree or `/OpenAction` pointing to a script action should be flagged as containing JavaScript (a metadata annotation on the extraction result), and those script objects should be silently skipped during content traversal.

---

## AcroForm Field Extraction

AcroForm is the traditional PDF interactive form model. Form fields are described by widget annotation dictionaries linked through the `/AcroForm` entry in the document catalog. Each field has a `/FT` (field type) key and a `/V` (value) key. The `/V` key holds the current field value as a PDF object — string, name, or array, depending on field type.

### Field Types

**Text fields** (`/Tx`) store their value as a UTF-16BE or PDFDocEncoding string in `/V`. Multiline fields may contain embedded newline characters. pdftract reads `/V` directly.

**Choice fields** (`/Ch`) represent dropdowns and listboxes. The selected value is stored in `/V` as the export value string, while the display label lives in the `/Opt` array. Each entry in `/Opt` is either a string (export value equals display label) or a two-element array `[export_value, display_label]`. pdftract should extract both the raw `/V` export value and the resolved display label by looking up `/Opt`.

**Button fields** (`/Btn`) encompass three distinct subtypes determined by the `/Ff` (field flags) bitmask. Bit 17 marks the field as a pushbutton. Bit 16 marks it as radio. A button with neither bit set is a checkbox. Pushbuttons carry no intrinsic value — they trigger actions when clicked and pdftract should record them as action elements with no value. Radio button groups store the name of the currently selected option in `/V` as a PDF Name object. Checkboxes store their state as a Name: `/Yes` (or a custom on-state name defined in the `/AP` (Appearance) dictionary) when checked, and `/Off` when unchecked. To determine what "on" means for a given checkbox, pdftract inspects the keys of the `/AP/N` (Normal appearance) sub-dictionary: the key that is not `/Off` is the on-state name.

**Signature fields** (`/Sig`) hold a signature dictionary in `/V` when signed, or are absent when unsigned. pdftract should note the presence or absence of a signature value without attempting to validate cryptographic content.

### Field Hierarchy and Name Construction

AcroForm fields are organized in a tree. Each field dictionary may contain a `/Kids` array pointing to child fields, and each field has a `/T` (partial name) key. The fully qualified field name is constructed by concatenating `/T` values from ancestor to descendant, separated by periods. Parent fields may omit `/FT` and serve only as containers for organizing children.

Certain attributes — particularly `/DA` (default appearance string, which specifies font and color for text rendering) and `/DR` (default resource dictionary) — are inherited downward through the hierarchy. pdftract must walk the ancestor chain to resolve these inherited values when they are absent from a leaf field dictionary.

### Rich Text in Text Fields

When a text field contains formatted content, the `/RV` (rich value) key holds rich text as an XML string conforming to a subset of XHTML. This XML uses `<span>` elements with `style` attributes to encode bold, italic, font size, and color. When `/RV` is present, pdftract should extract text from the `/RV` XML rather than from `/V`, since `/V` in this case is a plain-text fallback that loses all inline formatting. The `/RV` payload should be parsed as XML, with text content extracted from element nodes and formatting annotations preserved in the extraction metadata.

---

## Calculated Fields and the computed_empty State

Fields with a `/AA/C` (calculate) action have their displayed value determined by JavaScript at runtime. The static `/V` in the field dictionary reflects the last value that was written to disk — which may be the result of a previous rendering session, or may be entirely absent if the document was generated programmatically and never opened in an interactive viewer.

pdftract's extraction policy for calculated fields:

- If `/V` is present and non-empty, extract it and annotate the field as `calculated` in the output metadata.
- If `/V` is absent or is an empty string, annotate the field as `computed_empty` and emit no value. This distinction is important for downstream consumers: a `computed_empty` field is not a blank field the user left unfilled — it is a field whose value requires JavaScript execution to produce.

pdftract must never attempt to evaluate the JavaScript in `/AA/C`. The calculation logic may depend on other field values, external data sources, or viewer-specific globals that are unavailable at extraction time.

---

## XFA: Static vs. Dynamic Forms

XFA (XML Forms Architecture) is an alternative form model that represents the form in an embedded XML stream. The `/XFA` key in the AcroForm dictionary points to this stream (or an array of name/stream pairs). XFA exists in two operational modes that have fundamentally different implications for extraction.

**Static XFA** renders the form layout to conventional PDF page content. The page content streams contain actual glyph sequences and positioned text, exactly as in a non-XFA document. pdftract extracts static XFA documents through the normal content stream pipeline, with no special handling required. The XFA XML is present but the page content is self-contained.

**Dynamic XFA** does not render to PDF page content at all. The page streams may be entirely empty or contain only placeholder content. The actual form fields, their layout, and their current values exist solely within the XFA XML. A document using dynamic XFA is essentially a PDF container around an XFA application. If pdftract attempts normal content-stream extraction on a dynamic XFA document, the result will be empty or misleading.

Detection: pdftract checks whether the document has an XFA stream. If it does, it examines whether the PDF page content streams contain substantive operator sequences. If the pages are empty and XFA is present, the document is classified as dynamic XFA. pdftract then parses the XFA XML directly, locating field nodes and their values within the XFA namespace. The XFA field value elements (typically `<field>` nodes with child `<value>` elements) are extracted and mapped to their XFA name paths, producing a structured field output analogous to AcroForm extraction.

---

## Submit and Reset Actions

Many interactive forms include submit (`/S` type `/SubmitForm` or `/JavaScript`) and reset (`/S` type `/ResetForm`) actions attached to pushbutton fields or to `/AA` entries. These actions are irrelevant to extraction — they describe what happens when the form is submitted, not what values the fields contain.

pdftract detects submit actions and annotates the document-level metadata to indicate that the form was designed for electronic submission. This is useful for downstream classification: a document flagged as submission-oriented may be a fillable form that was never completed, which informs how empty fields should be interpreted. No attempt is made to follow submit action targets or reconstruct submission payloads.

---

## Extraction Policy Summary

The following policy governs pdftract's handling of interactive PDF content:

- **JavaScript** is never executed. Documents containing JavaScript in `/JavaScript` names, `/OpenAction`, or field `/AA` entries are flagged with `contains_javascript: true` in extraction metadata.
- **Static field values** (`/V`) are always extracted for all field types that carry meaningful values: text, choice, radio, and checkbox.
- **Calculated fields** with a non-empty `/V` are extracted and annotated as `calculated`. Calculated fields with empty or absent `/V` are annotated as `computed_empty` with no value emitted.
- **Rich text fields** with `/RV` use the XML payload as the authoritative text source.
- **Choice fields** resolve export values against `/Opt` to provide display labels.
- **Pushbutton fields** produce no value output; they are recorded as action widgets.
- **Signature fields** are noted as signed or unsigned; cryptographic content is not extracted.
- **Static XFA** is handled through normal page content extraction.
- **Dynamic XFA** triggers XFA XML parsing; field values are extracted from XFA element nodes.
- **JavaScript source text** is never emitted as extracted content, regardless of where it appears in the document structure.

This policy ensures that pdftract produces accurate, reproducible extraction results from interactive PDFs: static values are faithfully reported, absent computed values are honestly flagged, and the extraction pipeline remains deterministic regardless of what JavaScript the document author intended to run.
