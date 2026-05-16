# PDF/X Prepress and Print Production PDF Extraction

## Overview

PDF/X is a family of ISO standards designed for reliable, blind exchange of print-ready content between creators and print service providers. Unlike general-purpose PDFs, PDF/X documents carry strict conformance requirements that constrain what is allowed inside the file—mandating font embedding, controlling color spaces, restricting transparency, and specifying page geometry boxes. These constraints, while imposing on document creators, are a benefit for extraction: a conformant PDF/X file makes strong guarantees that simplify several of pdftract's jobs. Understanding where those guarantees apply, and where prepress production artifacts complicate extraction, is essential to handling magazine spreads, packaging artwork, and newspaper print PDFs correctly.

## The PDF/X Conformance Family

The PDF/X lineage spans five major standards, each building on or refining the previous.

**PDF/X-1a** (ISO 15930-1, later revised as 15930-4) is the strictest and most widely deployed standard for commercial print. It prohibits all device-independent color spaces in favor of DeviceCMYK, DeviceGray, Separation (for spot colors), and DeviceN. RGB images and ICC-tagged objects are forbidden outright. PDF/X-1a also disallows transparency, requiring fully flattened artwork, and mandates that all fonts be embedded without subsetting restrictions. Because color space choices are locked to CMYK and spot, extraction does not need to resolve color management to identify text color—text is either black, a gray level, a CMYK mix, or a named separation color.

**PDF/X-3** (ISO 15930-3) relaxes the color restriction. ICC-tagged RGB color spaces are permitted alongside CMYK, provided the document includes an OutputIntent that names the intended output device profile. Transparency remains prohibited. PDF/X-3 is common in European print workflows where ICC-managed RGB photography is delivered to print alongside CMYK editorial content. Extraction must be prepared to encounter RGB text objects alongside CMYK content in the same file.

**PDF/X-4** (ISO 15930-7) is the current preferred standard for high-quality commercial printing. It permits live transparency—meaning objects with non-opaque alpha can appear in the content stream without prior flattening. PDF/X-4 also allows optional content groups (layers) to be present. For extraction, PDF/X-4 files may contain text objects inside transparency groups, requiring that pdftract track the graphics state alpha stack (detailed in the graphics-state-tracking research) rather than assuming all text is fully opaque.

**PDF/X-4p** extends PDF/X-4 by allowing the ICC output profile to be stored externally rather than embedded in the file. The `DestOutputProfileRef` dictionary entry points to a named external profile. This has no practical effect on text extraction.

**PDF/X-5** covers partial exchange—used when referencing external graphical content (external ICC profiles, external artwork). PDF/X-5 documents may legitimately omit embedded content that would be resolved at output. For pdftract, this means some image content may be absent (represented by OPI proxies), but text content should still be fully present per the standard.

The conformance level can be read from the `GTS_PDFXVersion` key in the document's XMP metadata (`pdfxid:GTS_PDFXVersion`) or from the `Info` dictionary's `GTS_PDFXVersion` entry. pdftract should capture this value and tag extraction output accordingly. Knowing the conformance level tells pdftract which assumptions are safe: on PDF/X-1a, transparency never needs to be resolved; on PDF/X-4, it does.

## OutputIntent and Device Classification

Every PDF/X file must contain an `/OutputIntents` array with at least one entry describing the intended output device. The entry is a dictionary with `/DestOutputProfile` (an embedded ICC profile stream), `/OutputConditionIdentifier` (a string like `FOGRA39` or `U.S. Web Coated (SWOP) v2`), and `/Info`.

pdftract does not need to parse the ICC profile binary to benefit from this structure. The presence of a valid OutputIntent is sufficient to classify the document as a print-production PDF. The `OutputConditionIdentifier` string can be surfaced in metadata output as a hint about the intended press standard. If pdftract is producing structured output for downstream consumers (such as content management ingest pipelines for magazine archives), tagging the document class as `print_production` with the output condition identifier gives consumers useful provenance without requiring pdftract to interpret colorimetry.

The `/OutputIntents` array key is also used by PDF/A. pdftract should check both `/GTS_PDFX` and `/GTS_PDFA` subtype strings to distinguish the two archival families.

## Page Geometry Boxes and the Bleed Zone Problem

PDF/X mandates specific usage of the page geometry boxes defined in the PDF specification. Understanding these boxes is critical for separating body content from print production artifacts.

The **TrimBox** defines the final finished page dimensions—the boundary where the physical paper will be cut after printing. All editorial content (article text, photographs, page numbers) intended to be read by the end user lives within the TrimBox. The **BleedBox** extends beyond the TrimBox by the bleed amount, typically 3mm on each side. Content in the bleed zone is intentionally printed beyond the trim edge to prevent white slivers appearing if the cut is slightly off-register. The **ArtBox**, when present, describes the meaningful content area as defined by the document creator, which may be inset from the TrimBox.

For text extraction, the TrimBox is the authoritative boundary for body content. Text objects whose bounding rectangles fall entirely outside the TrimBox should be labeled `bleed_content` in pdftract's zone classification. Text objects that straddle the TrimBox boundary—partially inside, partially outside—are the most ambiguous case. These are typically glyphs at the edge of a page-bleed background element or production labels placed in the bleed zone. pdftract should classify straddling glyphs based on whether their typographic origin (the baseline point) falls within the TrimBox. A glyph whose origin is inside the TrimBox but whose descenders extend into the bleed zone is body content; a glyph whose origin is in the bleed zone is bleed content.

The `/CropBox`, if present, typically matches or is larger than the TrimBox. Some PDF/X workflows set the CropBox to the BleedBox size to show the full bleed when viewed on screen. pdftract must use TrimBox as the primary boundary for content classification and not assume CropBox represents finished dimensions.

## Spot Colors and Separation Spaces

PDF/X-1a files are saturated with Separation and DeviceN color spaces carrying Pantone names, brand color names, or custom identifiers. A text object might be specified in `[/Separation /PANTONE-485-C /DeviceCMYK <...alternate tint function...>]`. The tint function provides a CMYK fallback for screen preview and low-fidelity output, but the canonical color is the named separation.

For extraction, the separation name itself is semantically meaningful in packaging and magazine workflows. pdftract should record the spot color name for text objects rendered in Separation spaces, rather than resolving to the CMYK alternate. This allows downstream systems to identify, for instance, that a logo mark or legal notice is printed in a specific Pantone ink—information that might affect content priority or processing rules in a brand asset workflow.

DeviceN spaces, which combine multiple colorants into a single space, may name several spot inks simultaneously. A DeviceN array like `[/PANTONE-485-C /PANTONE-Cool-Gray-9-C]` identifies a duotone or multi-ink object. pdftract records all named colorants from the DeviceN components array when annotating text color.

## OPI Proxy Images

OPI (Open Prepress Interface) is a workflow mechanism where high-resolution images are replaced by low-resolution proxies during creative layout. The proxy carries OPI comments—either 1.3 (%%BeginOPI...%%EndOPI PostScript-style comments wrapped in a marked content sequence) or 2.0 (an `/OPI` dictionary in the image XObject's dictionary or in a surrounding content stream marked content section)—pointing to the path of the full-resolution original.

For text extraction, OPI images are irrelevant but noteworthy. pdftract does not attempt to retrieve or process the high-resolution original. However, if pdftract is producing a document structure report alongside the text extraction, the presence of OPI comments should be flagged. OPI images appear in prepress-stage PDFs that have not yet been through final output processing; a document containing OPI 2.0 dictionaries may also contain other pre-output artifacts that affect fidelity.

The OPI dictionary is found at the `/OPI` key within an image XObject's dictionary, containing subkeys `/1.3` or `/2.0` with the original file path. pdftract's image enumeration pass should check for this key and emit an `opi_proxy_detected` diagnostic when present.

## Font Embedding and Extraction Confidence

PDF/X mandates that all fonts—without exception—be fully embedded. Subsetting is permitted, but the subsetting restriction flag that would prevent copying or editing must not be set. This is a meaningful signal for extraction confidence. A PDF/X-conformant file with valid fonts should yield clean ToUnicode CMap coverage for all glyphs present, which means character-to-Unicode mapping should succeed without heuristic fallback.

pdftract's confidence scoring for individual text spans can be elevated when the enclosing document carries a valid `GTS_PDFXVersion` identifier and all fonts encountered are fully embedded (Flags bit 2 not set, `FontDescriptor` present with `FontFile`, `FontFile2`, or `FontFile3`). This is distinct from PDF/A, where embedding is also mandated but ToUnicode presence is more strictly required for Level A conformance.

When a PDF/X file is encountered with a missing or incomplete font embedding—a non-conformance—pdftract should treat it identically to any other font-missing case (triggering the fallback glyph recognition pipeline) but emit a conformance violation diagnostic. The document's claim of PDF/X conformance does not guarantee it.

## Overprint Settings

Overprint is fundamental to CMYK prepress. When an object overprints, its ink is added on top of the ink already on the substrate rather than knocking out the background. This is controlled by the `/OP` (overprint for stroking), `/op` (overprint for filling), and `/OPM` (overprint mode, 0 or 1) entries in ExtGState dictionaries applied via `/gs` operators.

For text in PDF/X prepress documents, overprint is frequently set for black text to ensure it prints correctly on top of CMYK imagery (100% K overprinting is standard practice). This does not affect which Unicode characters are present, but it does affect visual rendering—overprinting black on a colored background produces rich black rather than a white knockout.

pdftract records overprint settings from the current graphics state when processing text runs but does not simulate the visual overprint result. This is correct behavior: extraction concerns itself with textual content, not ink simulation. The recorded OPM and overprint flags can be surfaced in structured output if a downstream consumer needs them.

## Trapping Annotations

PDF/X-3 and later versions permit TrapNet annotations—annotations of subtype `/TrapNet` that encode trap geometry for the press. These are rectangles or paths describing where ink spread has been applied at color boundaries to prevent misregistration gaps. They appear in the `/Annots` array of a page dictionary.

TrapNet annotations contain no text content and are irrelevant to extraction. pdftract should enumerate and skip them without raising warnings. The presence of TrapNet annotations is a useful provenance signal (the document was processed by a trapping engine before delivery) and can be noted in the document structure report.

## Print Production Artifacts in the Bleed Zone

Commercial print PDFs—particularly magazine advertising pages and packaging—routinely include production marks placed outside the TrimBox in the bleed and slug zones: crop marks (hairlines showing where the sheet is to be cut), registration marks (bull's-eye targets for aligning color separations), color bars (rows of color swatches for press calibration), and cut guides. These marks frequently include text: the name of the print service provider, a job number, a timestamp, Pantone color names next to the color bar swatches, and instructional text such as "CUT HERE" or "DO NOT PRINT."

All of this text is production metadata, not editorial content. Because these elements are positioned outside the TrimBox, pdftract's zone classifier will label them `bleed_content` by the spatial rule described above. When pdftract is configured for clean body text output (the default extraction mode), bleed_content zones are excluded from the primary text stream. When pdftract is in full-page or audit mode, bleed_content is included in a separate output section labeled accordingly.

## Magazine and Newspaper PDFs

Magazine and newspaper PDFs are the dominant real-world use case for PDF/X-1a and PDF/X-4. They exhibit several structural patterns that extraction must accommodate.

Multi-column editorial layouts are nearly universal. Text runs in narrow columns with precisely controlled gutter spacing. Adjacent columns may contain independent articles, requiring that pdftract's reading-order heuristics identify column boundaries before linearizing text. The column detection approach documented in the complex-layout-reading-order research applies directly here.

Article threading links article text across non-contiguous pages using the PDF `/Threads` array of `/Thread` dictionaries, each containing a chain of `/Bead` dictionaries. Each bead references a page and a rectangle. Threading is the PDF mechanism for encoding "article continues on page 47"—the beads mark the reading order of article fragments across the publication. pdftract should traverse the thread beads to reconstruct article continuity, appending thread-linked text boxes in bead order rather than page order, and flagging the result as a threaded article in structured output.

Pull quotes—enlarged excerpts from the article body, set in display type and overlaid on the column layout—are a common source of duplicate text. The same sentence appears once in the body text run and once as a pull quote in a larger font size at a different position. pdftract's post-extraction deduplication pass should identify these by comparing text proximity and string similarity, preferring the body text instance and tagging the pull quote as `display_duplicate`.

Headlines, decks (subheadlines), bylines, and section labels are all in the trim box but at different font sizes and positions. pdftract's zone classifier should distinguish these by font size and position relative to column boundaries, surfacing them as `headline`, `byline`, and `section_label` zones rather than body text.

## Summary

PDF/X print production files offer pdftract several reliable signals: font embedding is guaranteed (raising extraction confidence), the conformance level constrains color spaces and transparency behavior, and OutputIntent provides device classification without profile parsing. The critical extraction challenge is spatial—separating editorial body text inside the TrimBox from the rich ecosystem of bleed content, production marks, and prepress artifacts that surround it. By anchoring zone classification on the TrimBox boundary, recording spot color names rather than resolving them, skipping OPI and TrapNet entries, and threading article beads for multi-page continuity, pdftract can deliver clean body text from even the most complex commercial print PDF.
