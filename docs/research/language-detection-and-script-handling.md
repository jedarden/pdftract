# Language Detection and Script Handling in pdftract

## Overview

Multilingual PDF documents expose three distinct problems for a text extraction library: identifying which Unicode script a sequence of codepoints belongs to, reconstructing logical order from glyphs that may have been stored in visual order, and normalizing script-specific presentation variants to canonical Unicode forms. This document covers each problem, the relevant standards, and the implementation strategy for `pdftract`.

---

## 1. Script Detection from Glyph Data

### Unicode Script Property (UAX #24)

Every Unicode codepoint carries a `Script` property defined in UAX #24. The Unicode Character Database (UCD) ships `Scripts.txt` and the companion `ScriptExtensions.txt`. Script extensions matter because some codepoints — most common-use punctuation, digits U+0030–U+0039, and combining marks — are legitimately shared across scripts and carry the `Common` or `Inherited` value rather than a specific script name.

A `pdftract` span classifier should resolve script assignments in this priority order:

1. **Specific script** — codepoints with a single non-`Common`, non-`Inherited` script assignment are classified directly.
2. **Script extensions** — codepoints with multiple entries in `ScriptExtensions.txt` (e.g., U+0300 COMBINING GRAVE ACCENT extends into `Latin`, `Greek`, `Cyrillic`) inherit the script of the surrounding run.
3. **Common/Inherited** — treated as transparent; they attach to the script of the nearest resolved codepoint within the same bidi run.

### Mixed-Script Spans

A single PDF text object can contain codepoints from multiple scripts (e.g., a Japanese sentence with embedded Latin product names). The standard approach is **script-run segmentation**: scan the codepoint sequence left to right, maintaining a current script state, and emit a new span boundary whenever the resolved script changes from one specific value to another. `Common` and `Inherited` codepoints do not trigger boundaries.

The Unicode `ScriptExtensions` data can be used to suppress spurious splits: if a `Common` punctuation character appears between two Latin spans with no intervening RTL text, it should remain in the Latin span rather than producing a one-character `Common` fragment.

### CJK Script Identification

CJK requires distinguishing four overlapping script blocks:

| Script | Key Ranges |
|--------|-----------|
| Han | U+4E00–U+9FFF (BMP), U+3400–U+4DBF (Extension A), U+20000–U+2A6DF (Extension B) |
| Hiragana | U+3041–U+3096 |
| Katakana | U+30A1–U+30FA, U+31F0–U+31FF |
| Hangul | U+AC00–U+D7A3 (syllables), U+1100–U+11FF (jamo) |

Han is shared across Chinese, Japanese, and Korean. Language detection (Section 7) must disambiguate Han-dominant runs; script detection alone cannot.

### PDF `/Lang` Attribute

Tagged PDFs may carry a `/Lang` entry (BCP 47 language tag) on the document catalog, individual structure elements, or marked-content sequences. When present, `/Lang` is a strong prior:

- `ja` → expect Han + Hiragana + Katakana, writing mode potentially vertical.
- `ar` or `he` → expect RTL bidi direction, visual-order glyph storage likely.
- `zh-TW` vs. `zh-CN` → disambiguates Traditional vs. Simplified Han.

When `/Lang` is absent or when extracted text falls outside the declared language's expected scripts, fall back to character-level detection. Never suppress the fallback entirely: many PDFs carry a top-level `/Lang` that does not apply uniformly to all content (e.g., an English document with a Hebrew quotation).

---

## 2. Unicode Bidirectional Algorithm (UBA, UAX #9)

### Algorithm Structure

UAX #9 defines a multi-pass algorithm over a paragraph of codepoints. Each codepoint has a **bidi character type** (Strong: L/R/AL; Weak: EN/ES/ET/AN/CS/NSM/BN; Neutral: B/S/WS/ON; Explicit: LRE/RLE/LRO/RLO/PDF/LRI/RLI/FSI/PDI).

Key steps:

1. **Paragraph embedding level**: if the first strong character is R or AL, the paragraph is RTL (embedding level 1); otherwise LTR (level 0).
2. **Explicit level runs**: `LRE`/`RLE` push a new embedding level; `PDF` pops. The isolate controls (`LRI`/`RLI`/`FSI`/`PDI`, introduced in Unicode 6.3) create isolated bidi contexts that do not affect the surrounding paragraph's level stack.
3. **Weak type resolution**: sequences of weak types are resolved based on surrounding strong types per a finite-state table.
4. **Neutral resolution**: neutral characters between two same-direction strong runs take that direction; between opposing runs they take the paragraph direction.
5. **Reorder**: within each level run, apply the level-based reordering algorithm to produce visual order.

### Why PDF Breaks Bidi

PDF authoring tools generally emit glyphs in **visual order** for RTL text rather than in logical (Unicode) order. The content stream positions each glyph individually on the page via the text matrix; there is no implicit cursor advance that encodes reading direction. An Arabic sentence rendered right-to-left appears in the content stream starting from the rightmost glyph.

Consequences for extraction:

- Naively reading content-stream character codes left-to-right from a page produces reversed Arabic/Hebrew words.
- Mixed LTR/RTL content is interleaved in spatial order: the leftmost object on the page comes first in the stream, regardless of its logical position in the paragraph.

### Detecting and Reversing Visual-Order RTL

Detection heuristic: after Unicode recovery, if a run of characters with strong R or AL bidi type appears in left-to-right spatial order (i.e., X coordinates increase as the content-stream position increases), the run is stored in visual order and must be reversed. The threshold for "increasing X" should tolerate per-glyph kerning noise (±2 units in text space).

Reversal procedure:

1. Identify the visual-order run boundaries (the span between two LTR-direction glyphs or page-object boundaries).
2. Reverse the codepoint sequence within each RTL word (space-delimited or width-gap-delimited).
3. Apply UBA to the reassembled logical string to verify paragraph direction.

Note: some PDF producers (notably newer versions of Adobe Acrobat) do store RTL text in logical order with correct ToUnicode. The detection heuristic must be conditional, not unconditional.

---

## 3. Arabic and Hebrew Specifics

### Arabic Shaping and Presentation Forms

Arabic uses a joining model: each base letter has up to four contextual glyph forms — **isolated**, **initial**, **medial**, and **final** — determined by whether the character joins to the preceding and/or following letter. Critically, all four forms map to the same base Unicode codepoint. A PDF font may embed glyphs named `uniFE8D` (isolated alef) or `uniFE8E` (final alef), which are Arabic Presentation Forms from the block U+FB50–U+FDFF (Presentation Forms-A) and U+FE70–U+FEFF (Presentation Forms-B).

Normalization: apply Unicode compatibility decomposition (NFKD or NFKC) to map presentation forms to their base codepoints. For the ligature block (U+FB50–U+FDFF), some entries (e.g., U+FB8A ARABIC LETTER TCHEH WITH THREE DOTS ABOVE) lack a NFKC decomposition and should be preserved as-is. After normalization, the shaping context is lost, but the logical character identity is recovered — which is what text extraction requires.

Mandatory ligatures such as **lam-alef** (U+0644 + U+0627 and variants) have precomposed forms in the presentation block. These should be expanded back to their two-codepoint sequences during normalization.

### Hebrew Vowel Points and Cantillation

Hebrew base letters (U+05D0–U+05EA) may be followed by **nikud** (vowel points, U+05B0–U+05C7) and **cantillation marks** (U+0591–U+05AF). These are combining characters with `Inherited` bidi type, which means they correctly attach to the preceding base letter in logical order. For plain-text extraction, nikud and cantillation can be optionally stripped or preserved depending on the output mode; `pdftract` should expose a normalization flag `strip_combining_marks: bool` per script.

### RTL Word Boundaries Without Spaces

Some Arabic PDFs omit inter-word spaces in the content stream (words are positioned by glyph advances rather than space characters). Word boundary detection falls back to **X-gap analysis**: a gap between adjacent glyphs significantly larger than the average intra-word advance (heuristic: > 0.25 × em) is treated as a word boundary.

---

## 4. CJK Handling

### Horizontal vs. Vertical Writing Modes

PDF CMaps carry a `/WMode` entry: `0` = horizontal, `1` = vertical. A font may embed two CMaps — a horizontal CMap (name ending in `-H`) and a vertical CMap (name ending in `-V`). The content stream selects between them via the font resource's `/Encoding` or via direct CIDFont reference.

CJK punctuation normalization: fullwidth forms (U+FF01–U+FF60) are compatibility equivalents of their ASCII counterparts. For prose extraction, map fullwidth to halfwidth via NFKC unless the output is destined for layout-sensitive consumers. The `pdftract` normalization pipeline should apply NFKC only to `Common`-script fullwidth/halfwidth punctuation, not to Han or Kana characters (NFKC decomposes some compatibility Kana which should be preserved).

### CJK Line-Break Rules (UAX #14)

The Unicode Line Breaking Algorithm (UAX #14) defines **non-starter** characters (closing brackets, closing quotation marks, Japanese small kana: ぁぃぅぇぉっゃゅょ) that cannot begin a line, and **non-ender** characters (opening brackets) that cannot end a line. When `pdftract` reassembles lines from individual glyphs, these rules inform the merge heuristic: a glyph with a non-starter break class that appears at the apparent start of a new line in the spatial layout should be joined to the preceding line.

---

## 5. Vertical Text

### PDF Encoding of Vertical CJK

In vertical writing mode, the text matrix in the content stream applies a 90-degree rotation: the current transformation matrix (CTM) component produces a glyph that advances downward rather than rightward. The glyph's width in the font metrics becomes its vertical advance, and the horizontal dimension becomes the em-square height.

Detection: examine the `Tm` (text matrix) operator. A matrix of the form `[0 -1 1 0 tx ty]` or `[0 1 -1 0 tx ty]` indicates vertical text. Combined with `/WMode 1` in the CMap, this is a reliable signal.

Reconstruction: to recover horizontal reading order from a vertical column:

1. Sort glyphs by decreasing Y within a column (top-to-bottom).
2. Sort columns by increasing X (left-to-right for vertical text flowing left-to-right between columns, which is the default for Japanese).
3. Assign direction `ttb` to the span.

### Tate-Chu-Yoko

Tate-chu-yoko (縦中横) is a typographic convention where a short horizontal sequence (typically 2–4 Latin characters or digits) is set horizontally within a vertical line. In PDF, these glyphs appear without the 90-degree rotation applied to surrounding CJK glyphs. Detection: within a vertical column, glyphs with a non-rotated text matrix and Latin/digit script classification form a tate-chu-yoko inline sequence. They should be extracted as a single horizontal sub-span with direction `ltr`, embedded within the enclosing `ttb` span.

---

## 6. Ligatures and Script-Specific Normalization

### Unicode Normalization Forms

| Form | Definition | Use in pdftract |
|------|-----------|----------------|
| NFC | Canonical decomposition then canonical composition | Default for Latin, Greek, Cyrillic output |
| NFD | Canonical decomposition only | Internal processing of combining marks |
| NFKC | Compatibility decomposition then canonical composition | Arabic presentation forms, fullwidth CJK punctuation |
| NFKD | Compatibility decomposition only | Intermediate step for specific scripts |

Apply NFKC selectively: Arabic (to collapse presentation forms), fullwidth punctuation (U+FF01–U+FF60), and Latin ligatures from the Alphabetic Presentation Forms block (U+FB00–U+FB06: ff, fi, fl, ffi, ffl, ſt, st).

### Latin Ligatures

The glyphs `fi`, `fl`, `ff`, `ffi`, `ffl` have explicit Unicode codepoints (U+FB01, U+FB02, U+FB00, U+FB03, U+FB04). PDF fonts commonly use these as single glyphs mapped via ToUnicode to either the precomposed ligature or the two-character sequence. For text search and NLP compatibility, always expand to the constituent characters: `fi` → U+0066 U+0069. Preserve the original ligature codepoint in a `raw_codepoints` field if the consumer needs to reconstruct original layout.

### Devanagari Conjunct Consonants

Devanagari conjunct consonants (Sanskrit: saṃyuktākṣara) are encoded in Unicode as a base consonant + virama (U+094D) + following consonant. PDF fonts may embed precomposed conjunct glyphs that have no standard Unicode representation. Recovery requires mapping via the font's glyph name (e.g., `kka` → U+0915 U+094D U+0915) using a glyph-name-to-sequence table. NFD decomposition of Devanagari preserves the logical structure and should be preferred over NFC for output.

---

## 7. Language Detection

### Statistical and Dictionary Approaches

For runs of 50+ characters with a known script, statistical **n-gram language identification** is reliable. The `whatlang` crate (Rust) uses trigram frequency profiles for 69 languages; the `lingua` crate supports 75 languages with a higher-accuracy bigram + unigram model at the cost of a larger compiled profile set. Both crates accept `&str` and return a language tag with confidence score.

For shorter spans (10–50 characters), dictionary-based detection — checking whether the top-N most frequent words from a candidate language appear in the span — outperforms n-gram models. Maintain per-script stop-word lists (the 200 most frequent words per language) compiled into the binary.

### Using `/Lang` as a Prior

When the PDF supplies `/Lang`, use it to bias detection: if the extracted text scores above 0.4 confidence for the declared language, accept the declaration. If the text scores below 0.4 for the declared language but above 0.7 for another, emit a `lang_conflict` warning and use the detected language. If detection confidence is below 0.4 for all candidates, emit `und` (undetermined).

Confidence threshold summary:

| Condition | Output |
|-----------|--------|
| `/Lang` present, detection ≥ 0.4 for declared | Use `/Lang` tag |
| `/Lang` present, conflict detected (other ≥ 0.7) | Use detected tag, warn |
| `/Lang` absent, detection ≥ 0.6 | Use detected tag |
| Any path, confidence < 0.4 | `und` |

---

## 8. Output Metadata on Spans and Blocks

Each extracted `Span` and `Block` in the `pdftract` JSON output carries the following language and script metadata:

```json
{
  "text": "مرحباً بالعالم",
  "lang": "ar",
  "script": "Arab",
  "direction": "rtl",
  "normalization": ["nfkc", "visual_order_reversed"],
  "lang_confidence": 0.92,
  "writing_mode": "horizontal"
}
```

Field definitions:

- **`lang`** — BCP 47 language tag (e.g., `ar`, `he`, `ja`, `zh-TW`, `und`). Sourced from `/Lang` or detection.
- **`script`** — ISO 15924 four-letter script code (e.g., `Arab`, `Hebr`, `Hani`, `Hira`, `Hang`, `Deva`, `Thai`, `Latn`). Derived from UAX #24 per-codepoint classification, taking the dominant script of the span.
- **`direction`** — One of `ltr`, `rtl`, or `ttb`. Derived from UBA paragraph direction for horizontal text; `ttb` set when vertical writing mode is detected via CTM analysis and `/WMode 1`.
- **`normalization`** — Array of normalization operations applied, in application order. Valid values: `nfc`, `nfkc`, `nfd`, `nfkd`, `visual_order_reversed`, `ligature_expanded`, `presentation_forms_collapsed`, `combining_marks_stripped`.
- **`lang_confidence`** — Float in [0.0, 1.0] from the language detector. Omitted when `lang` is sourced from `/Lang` and no conflict was detected. Set to `null` when `lang` is `und`.
- **`writing_mode`** — `horizontal` or `vertical`. `vertical` implies `direction` is `ttb`; tate-chu-yoko sub-spans within a vertical block carry `direction: ltr` and `writing_mode: horizontal`.

Blocks aggregate span metadata: the `script` and `lang` of a block are the modal values across its constituent spans. Blocks containing spans from more than one script carry a `mixed_script: true` flag and list all scripts in a `scripts` array alongside the dominant `script` field.
