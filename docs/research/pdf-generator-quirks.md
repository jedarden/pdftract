# PDF Generator Identification and Per-Generator Extraction Quirks

Different PDF generators leave distinctive fingerprints in the files they produce, and those fingerprints predict the extraction problems pdftract will encounter. Knowing which tool created a PDF allows the pipeline to apply targeted workarounds rather than generic fallbacks. This document covers how to identify the generator and exactly what extraction behavior to expect from each major source.

---

## 1. Generator Identification

Every PDF may carry two generator strings in its `/Info` dictionary: `/Creator` and `/Producer`. These serve distinct purposes. `/Creator` names the authoring application — the tool the human used to compose the document (Microsoft Word, Adobe InDesign, LibreOffice Writer). `/Producer` names the PDF conversion engine — the component that rendered the final byte stream (Acrobat PDFMaker, pdfTeX, Ghostscript, Quartz PDFContext). In workflows with a single tool, both fields may name the same application. In multi-step workflows (for example, Word → Distiller, or LaTeX → dvips → Ghostscript), they diverge and reveal the pipeline.

XMP metadata in the `/Metadata` stream duplicates much of this information using `xmp:CreatorTool` and `pdf:Producer`, often with more detail than the `/Info` dictionary allows. When `/Info` strings are truncated or absent, XMP is the fallback.

pdftract should extract and normalize both strings early in the parsing phase, before any text extraction begins, and use the normalized values to select generator-specific processing modes. Matching should be case-insensitive substring search, not exact equality, because version numbers and build identifiers vary.

---

## 2. Microsoft Word (PDFMaker and Save as PDF)

Word-produced PDFs carry `/Creator` values such as `Microsoft Word`, `Microsoft Office Word`, or simply `Word`, and `/Producer` values of `Adobe PDF Library` (when PDFMaker is used) or `Microsoft: Print To PDF` (when using the built-in Save as PDF driver introduced in Office 2013).

**Encoding:** Word embeds ToUnicode CMaps reliably. Character identity is rarely the problem.

**Character spacing:** Older Word versions (pre-2013) inconsistently apply the `Tc` (character spacing) operator. A non-zero `Tc` in the graphics state may persist across text objects where it should have been reset, causing pdftract to miscalculate inter-character gaps when reconstructing word boundaries. The workaround is to honor `Tc` only within the immediately enclosing `BT/ET` block and treat carry-over as a bug rather than intent.

**Word spacing in TJ arrays:** Word frequently uses TJ arrays to encode text with embedded kerning values. These values are in thousandths of a text space unit and are typically negative (closing gaps). Positive values beyond a threshold — commonly 250 units at 1000 units/em — represent intentional word breaks and should be treated as spaces even when no explicit space character appears in the string operand.

**Structure tree:** Word documents prior to Office 365 (version 16.0) almost never include a StructTree. Logical reading order must be inferred from geometry. Word 2016 and later can produce accessible PDFs with a partial StructTree when the author uses the Accessibility Checker and exports with the `Document structure tags for accessibility` option enabled. These StructTrees are shallower than InDesign output and may omit figure alt-text even when accessibility options are on.

---

## 3. LibreOffice Writer

LibreOffice Writer sets `/Creator` to `Writer` and `/Producer` to `LibreOffice N.N` or `OpenOffice.org N.N` for older releases.

**ToUnicode:** Generally present and correct for Latin scripts. The failure mode is ligatures. The glyphs `fi`, `fl`, `ff`, `ffi`, and `ffl` are sometimes encoded as single-slot glyphs in the font's private encoding without a corresponding ToUnicode entry and without the `ActualText` attribute in a StructTree (which LibreOffice does not produce). The extraction result is a missing ligature character. The workaround is to identify single-glyph operands in known ligature codepoint slots and substitute the Unicode decomposition based on glyph name.

**Word spacing:** LibreOffice sometimes omits explicit space characters between words when the inter-word gap is encoded entirely as a large negative TJ kerning value. The threshold for interpreting TJ gaps as spaces is the same as for Word, but the frequency is higher. pdftract's span-merging pass must apply this heuristic consistently to avoid run-together words in LibreOffice output.

---

## 4. Adobe InDesign

InDesign is the highest-quality PDF generator in common use. `/Creator` is `Adobe InDesign` with a version number; `/Producer` is `Adobe PDF Library`.

**Encoding and structure:** Accessible InDesign exports (File → Export → PDF → with `Create Tagged PDF` enabled) produce well-formed StructTrees with `ActualText` on ligature spans, role maps for custom tag names, and article threads for multi-column reading order. ToUnicode CMaps are always present and correct.

**Optical kerning:** InDesign's optical kerning algorithm produces large numbers of small TJ adjustments — often individual character pairs with sub-5-unit corrections. These are legitimate and should not be misinterpreted as word breaks. pdftract's gap threshold logic must use a higher threshold (around 500–600 units at 1000 units/em) when it detects InDesign output to avoid false word-break insertions between tightly-set glyphs.

**Spot colors:** InDesign preserves spot color separations (Pantone, custom inks) in DeviceN and Separation color spaces. This is irrelevant for text extraction but can cause confusion if the pipeline attempts to rasterize pages for OCR confidence scoring — the DeviceN color values will not render correctly without the spot color lookup table.

**Article threads:** Older InDesign exports (pre-CS6) encode reading order for multi-column layouts as article threads in the `/Threads` array rather than in the StructTree. pdftract should extract article threads as a fallback reading-order source when the StructTree is absent or incomplete.

---

## 5. LaTeX (pdflatex, LuaLaTeX, XeLaTeX, and dvips)

LaTeX generator detection is covered in depth in `latex-and-scientific-pdf-patterns.md`. The relevant `/Producer` strings are: `pdfTeX-N.N` for pdflatex, `XeTeX` for xelatex, `LuaTeX` for lualatex, and `GPL Ghostscript` or `Acrobat Distiller` for the legacy dvips pipeline.

**dvips artifacts:** The `latex` → `dvips` → `ps2pdf` pipeline produces PDFs with no ToUnicode CMaps. Ghostscript does not synthesize them from the PostScript source. Character identity must be recovered entirely from glyph names and font encoding vectors. Very old dvips output may also include Type 3 fonts built from PK bitmap rasterizations of Metafont glyphs; these have no outline and no reliable glyph name. pdftract must fall back to raster OCR for pages dominated by such fonts.

**hyperref metadata:** When the hyperref package is loaded, it populates `/Info` fields (`/Title`, `/Author`, `/Subject`, `/Keywords`) and creates a PDF outline (bookmarks) from section headings. This is useful for extraction — bookmarks can supplement or replace geometric heading detection. However, hyperref also emits PDF destinations for every `\label`, which multiplies the number of named destinations in the cross-reference dictionary; pdftract should not attempt to extract those destinations as meaningful text.

---

## 6. Google Docs and Google Slides

Google Docs exports carry `/Creator` of `Google Docs` or `Google Slides` and a `/Producer` string beginning with `Skia/PDF` followed by a build milestone number (for example, `Skia/PDF m128`). This overlaps with Chrome's producer string; the `/Creator` field disambiguates.

**Unicode and encoding:** Google's export engine produces correct ToUnicode CMaps. Character identity is reliable.

**Header and footer duplication:** Google Docs repeats header and footer content on every page as independent text streams with no structural marker distinguishing them from body text. The text appears at the top and bottom of each page at consistent Y coordinates. pdftract should detect repeated text blocks at fixed page-relative positions across three or more consecutive pages and classify them as headers or footers, suppressing duplicates in continuous extraction output.

**Inline images:** Images in Google Docs PDFs are always converted to JPEG and inlined in the content stream. They are not referenced as XObject Form resources. This means image extraction must scan inline image operators (`BI`/`EI`) in addition to `Do` operators.

**Structure tree:** Google Docs and Slides do not emit StructTrees. Reading order is entirely geometry-driven.

---

## 7. macOS Print to PDF (Core Graphics / Quartz)

macOS system-level PDF generation sets `/Producer` to `Mac OS X N.N.N Quartz PDFContext`. The `/Creator` is the application that initiated the print job.

**Font handling:** Core Graphics subsets fonts aggressively, retaining only the glyphs used in the document. Subset names carry the standard six-character uppercase prefix. ToUnicode CMaps are present and correct for all text.

**Page thumbnails:** Quartz-generated PDFs frequently embed page thumbnails as JPEG images in the `/Thumb` entry of each page dictionary. These are rendering artifacts and should not be processed as content.

**Quality:** Quartz output is generally clean. The main extraction challenge arises when the printing application does not expose logical text to the PDF layer — for example, when a canvas-based web application prints via WebKit, the output may be paths rather than text operators.

---

## 8. Ghostscript

Ghostscript (`/Producer` beginning with `GPL Ghostscript N.N`) typically appears as a downstream converter, transforming PostScript into PDF. It may also appear as the engine in Linux print-to-PDF and in some server-side document conversion pipelines.

**Encoding errors:** When converting PostScript that uses Symbol or Dingbats fonts, Ghostscript sometimes misidentifies glyph slots during re-encoding, producing incorrect character substitutions. A Symbol font encoded with the standard Symbol encoding should map slot 0x61 to the alpha character (U+03B1); Ghostscript has been observed mapping some slots to their Latin equivalents instead. pdftract should treat any text run in a font named `Symbol` or `ZapfDingbats` as suspect and apply the canonical encoding table rather than trusting the embedded ToUnicode.

**Type 3 promotion:** When Ghostscript converts PostScript Type 1 fonts it cannot fully resolve, it may re-emit them as Type 3 fonts with charproc streams. These Type 3 glyphs do not carry glyph names and require shape-based recovery. Detection: font `/Subtype` is `Type3` and `/FontMatrix` is not the identity matrix.

**No ToUnicode synthesis:** Ghostscript does not add ToUnicode CMaps to PostScript-derived content that lacked them. If the upstream PostScript had no encoding information, the PDF will not either. dvips-to-Ghostscript output is the canonical case.

---

## 9. Browser Print-to-PDF (Chrome/Chromium and Firefox)

**Chrome:** `/Producer` is `Skia/PDF mXX` where XX is a Chromium milestone number. `/Creator` is absent or set to the page title. Chrome's PDF renderer is based on the Skia graphics library and produces clean ToUnicode CMaps.

**Firefox:** `/Producer` is `Mozilla/N.N` with a version number.

**Fragmented text runs:** Both browsers may decompose text into single-character `Tj` operations in some rendering paths, particularly for complex CSS typography (letter-spacing, text-shadow, mixed bidirectional content). A paragraph that reads as one logical span in the DOM becomes dozens of individual positioned glyphs in the PDF. pdftract's span-merging pass must reconstruct these into word and line sequences by clustering glyphs whose inter-character gaps fall within a font-size-relative threshold. The merge step should apply before any word-boundary heuristics, not after.

**Baseline variation:** Web pages with inline SVG or mixed font sizes can produce text runs with small vertical offsets within a single visual line. The line-grouping pass should use a tolerance band of roughly 20% of the dominant font size when assigning characters to the same text line.

---

## 10. Scanning Software and OCR Layers

Scanned PDFs produced by NAPS2, Adobe Scan, Microsoft Office Lens, and similar tools carry an invisible text layer rendered with text rendering mode 3 (`Tr 3` — neither filled nor stroked). The background is a raster image; the text layer is OCR output aligned to the image.

**Producer strings:**
- NAPS2: `/Producer` is `NAPS2` with a version
- Adobe Scan: `/Creator` contains `Adobe Scan`
- Office Lens: `/Creator` contains `Microsoft Office Lens`
- Generic OCR pipelines using ABBYY FineReader may report `/Producer` as `ABBYY FineReader`
- Tesseract-based pipelines (including some open-source scan apps) report `/Producer` as `tesseract N.N.N`

**OCR engine quality:** ABBYY FineReader output typically has higher character-level accuracy and better word-spacing reconstruction than Tesseract, particularly for non-Latin scripts and degraded print. Apple Vision (used in iOS scan apps) is competitive with ABBYY for English. Tesseract output requires more aggressive post-OCR normalization.

**Confidence signals:** Tesseract embeds per-word confidence values in the invisible text layer as custom ActualText or via font size variation tricks. ABBYY encodes confidence differently and less consistently. pdftract should compute its own confidence signal for scan layers: the ratio of recognizable Unicode characters to total character count in the `Tr 3` layer, cross-checked against the visual character density in the corresponding raster region. A high-confidence scan layer can be used directly; a low-confidence one should trigger a re-OCR pass using pdftract's internal raster pipeline.

**Text alignment:** OCR-placed text in scan PDFs is positioned to match the corresponding raster glyph but may use a single monospace font regardless of the original typeface. Inter-word gaps are encoded as explicit space characters rather than TJ kerning, which makes word boundary reconstruction straightforward — the OCR engine has already done it. The primary extraction task is simply reading the `Tr 3` text stream, filtering out the rendering mode, and normalizing the spacing.

---

## Summary: Generator Detection to Extraction Mode

| `/Producer` pattern | Likely source | Key extraction concerns |
|---|---|---|
| `Adobe PDF Library` + `/Creator` Word | Word via PDFMaker | Tc carry-over, TJ word gaps, no StructTree pre-365 |
| `Microsoft: Print To PDF` | Word Save as PDF | As above, lighter kerning |
| `LibreOffice N.N` | LibreOffice Writer | Ligature gaps, TJ space encoding |
| `Adobe PDF Library` + `/Creator` InDesign | InDesign | Optical kerning threshold, article threads |
| `pdfTeX-N.N` | pdflatex | OT1 encoding, partial ToUnicode |
| `XeTeX` / `LuaTeX` | xelatex / lualatex | Good Unicode, math block mapping |
| `GPL Ghostscript` | Ghostscript / dvips | No ToUnicode, Type 3 promotion, Symbol re-encoding |
| `Skia/PDF mXX` + no Creator | Chrome | Fragmented single-char Tj runs |
| `Mozilla/N.N` | Firefox | Fragmented runs, baseline variation |
| `Mac OS X N.N.N Quartz PDFContext` | macOS print | Clean output, thumbnail noise |
| `Google Docs` / `Google Slides` (Creator) | Google export | Header/footer dedup, inline JPEG |
| `ABBYY FineReader` / `tesseract N.N` | Scan + OCR | Tr 3 layer, confidence scoring |

Detecting the generator is a one-time operation at parse time and costs negligible overhead. The payoff is that every subsequent heuristic — gap thresholds, ligature substitution, StructTree reliance, span merging aggressiveness — can be tuned to the actual source rather than a generic average. pdftract should treat `/Producer` detection as a first-class preprocessing step, not an optional diagnostic.
