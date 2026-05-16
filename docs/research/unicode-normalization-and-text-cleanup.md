# Unicode Normalization, Text Cleanup, and Post-Processing Pipeline

## Overview

Raw text extracted from PDF streams is rarely clean Unicode. Glyph-to-character mappings in PDF fonts encode text in forms optimized for rendering, not for downstream consumption: ligature glyphs stand in for character sequences, soft hyphens interrupt words at line breaks, Private Use Area code points mask unresolved glyphs, and combining diacritics may arrive in visual rather than logical order. The pdftract post-processing pipeline exists to resolve all of these issues in a defined, predictable sequence before text reaches the caller.

This document specifies the precise Unicode transformations applied in that pipeline stage.

---

## 1. Unicode Normalization Form

pdftract outputs text in **NFC** (Canonical Decomposition followed by Canonical Composition, as defined in Unicode Standard Annex #15). This choice is mandated by the PDF/UA-2 standard, which requires ActualText and ToUnicode output to be in NFC. Beyond standards compliance, NFC is the most compact canonical form and is what most downstream tools—text search indexes, NLP tokenizers, and string comparison routines—expect to receive.

The distinction between normalization forms matters here. **NFD** decomposes every precomposed character into its base letter and combining diacritical sequence (e.g., `é` → `e` + U+0301), which is useful for diacritic-stripping but produces inflated code unit counts. **NFKD** and **NFKC** additionally apply compatibility decompositions, which collapse ligatures and other compatibility variants into their canonical equivalents—useful for search indexing, but too destructive for general-purpose extraction output where the caller may need to distinguish `ﬁ` from `fi` for layout reconstruction. pdftract applies compatibility decompositions selectively and explicitly (see Section 2) rather than globally via NFKC, then composes the result to NFC.

The normalization step runs last in the pipeline, after all other transformations, so that earlier steps do not produce NFD intermediate forms that subsequently compose incorrectly.

---

## 2. Selective Compatibility Decomposition

PDF fonts frequently encode typographic ligatures as single glyphs and map them to Compatibility Area code points rather than multi-character sequences. A naïve extraction that preserves these code points produces text where `"efficient"` is stored as `"e` U+FB03 `ient"`, which breaks substring search, spell checking, and word-boundary detection.

pdftract applies the following compatibility decompositions unconditionally when the code point appears in body text content:

| Code Point | Name | Expansion |
|---|---|---|
| U+FB00 | LATIN SMALL LIGATURE FF | `ff` |
| U+FB01 | LATIN SMALL LIGATURE FI | `fi` |
| U+FB02 | LATIN SMALL LIGATURE FL | `fl` |
| U+FB03 | LATIN SMALL LIGATURE FFI | `ffi` |
| U+FB04 | LATIN SMALL LIGATURE FFL | `ffl` |
| U+FB05 | LATIN SMALL LIGATURE LONG S T | `st` |
| U+FB06 | LATIN SMALL LIGATURE ST | `st` |
| U+FB00–U+FB4F | Full Alphabetic Presentation Forms block | per Unicode decomposition mapping |

For **superscript and subscript digits** (e.g., U+00B2 SUPERSCRIPT TWO, U+00B3 SUPERSCRIPT THREE, U+2070–U+2079, U+2080–U+2089), pdftract applies compatibility decomposition only when the character is in a run of body text where no mathematical context is detected. When the span is tagged as a formula, equation, or appears within a mathematical font context identified during the glyph-mapping stage, the superscript/subscript code points are preserved verbatim so that the caller can reconstruct notation correctly. Body text heuristics check for surrounding alphanumeric characters and the absence of mathematical operator adjacency; when ambiguous, the code point is preserved and flagged in the confidence metadata.

Alphabetic Presentation Forms outside the ligature set (e.g., U+FB50–U+FDFF Arabic Presentation Forms-A, U+FE70–U+FEFF Arabic Presentation Forms-B) are decomposed only when the current script run is Latin. For Arabic text, these forms carry distinct semantic weight in some legacy encodings and are left for the Arabic shaping logic in Section 10.

---

## 3. Private Use Area Handling

Code points in the Private Use Area (U+E000–U+F8FF, supplementary plane U+F0000–U+FFFFF) appear when a PDF's ToUnicode CMap assigns PUA values to glyphs whose actual characters are unknown—commonly in hand-crafted or scanned documents with embedded bitmapped fonts where the font vendor used PUA internally.

pdftract does **not** silently drop PUA code points and does not attempt heuristic substitution. Instead, each PUA code point is preserved verbatim in the output string and annotated in the structured JSON output with `confidence_source: "Synthetic"` and `confidence: 0.0`. This makes the gap visible to downstream processors without corrupting the surrounding text or altering character offsets. Callers that need clean plain text can filter on the confidence metadata; callers that need to audit extraction quality can locate every unresolved glyph precisely.

PUA cleanup is out of scope for pdftract: resolving these requires either a per-font encoding table provided by the caller or OCR fallback, both of which are caller responsibilities.

---

## 4. Soft Hyphen Handling

U+00AD (SOFT HYPHEN) is inserted by TeX and similar typesetters at potential line-break positions within words. In the rendered PDF the glyph may or may not be visible depending on whether the line actually broke at that point. When text is extracted naïvely, these soft hyphens appear mid-word in the output stream regardless of rendering context.

pdftract resolves soft hyphens using glyph position data from the PDF content stream. When a U+00AD is followed by a line break in the glyph sequence (detected by a vertical position delta exceeding the line height threshold) and the first character on the next line is a lowercase letter, the soft hyphen and the line break are both removed and the two word fragments are joined. This heuristic covers the dominant TeX case. When the next line begins with an uppercase letter or a digit, the soft hyphen is removed but a space is inserted, under the assumption that the break was between words. In all other cases the soft hyphen is removed unconditionally—U+00AD has no display semantics in plain text output and is never useful to callers.

---

## 5. Non-Breaking Space Normalization

U+00A0 (NO-BREAK SPACE), U+202F (NARROW NO-BREAK SPACE), and U+2007 (FIGURE SPACE) are treated differently depending on the output mode and the detected content type.

In **body text** spans—paragraphs, headings, captions—all three are normalized to U+0020 (SPACE). Typographers use NBSP to prevent line breaks at specific positions, but that layout intent is lost in text extraction; preserving NBSP in body text output causes unexpected behavior in search indexes and tokenizers that do not normalize it.

In **formatted content** spans—table cells, form fields, code blocks, and structured data regions detected by layout analysis—the original code points are preserved. A figure space (U+2007) in a numeric column is semantically significant for alignment; a narrow NBSP in a date or unit string (`100 km`) is intentional and its removal would corrupt the value.

The `--text` output mode applies body-text normalization globally. The JSON structured output preserves the original code points and annotates the span's content-type classification so the caller can make their own normalization decision.

---

## 6. Control Character Filtering

C0 control characters (U+0000–U+001F) and C1 control characters (U+0080–U+009F) appear in extracted text as artifacts of encoding errors, particularly in documents that mix single-byte encodings or use MacRoman/WinANSI code pages where byte values in the C1 range map to printable characters in the source encoding but are incorrectly interpreted as Unicode.

pdftract strips all C0 and C1 control characters from body text with two exceptions: U+0009 (CHARACTER TABULATION) and U+000A (LINE FEED) are retained when they appear in form field values, where they are legitimate content. U+000D (CARRIAGE RETURN) is normalized to U+000A rather than stripped, because some PDF generators use CR to terminate form field lines. The null byte U+0000 is stripped unconditionally regardless of context.

---

## 7. Zero-Width Characters

U+200B (ZERO WIDTH SPACE), U+FEFF (BYTE ORDER MARK / ZERO WIDTH NO-BREAK SPACE), U+200C (ZERO WIDTH NON-JOINER), and U+200D (ZERO WIDTH JOINER) require distinct treatment.

**U+200B** and **U+FEFF** are stripped from all output. ZWSP is used by some PDF generators as an internal glyph separator with no semantic content; BOM has no meaning within a string body. Neither survives into pdftract output.

**U+200C (ZWNJ)** and **U+200D (ZWJ)** affect shaping in Arabic, Indic, and other complex scripts. In runs where the span's detected language is Arabic (`ar`), Persian (`fa`), Hindi (`hi`), or any other language whose script relies on ZWJ/ZWNJ for correct glyph selection, these code points are preserved verbatim. Stripping a ZWNJ from a Persian compound word or a ZWJ from an Indic conjunct consonant produces incorrect text that cannot be faithfully re-rendered. In Latin-script spans where these code points appear due to encoding errors, they are stripped.

Language detection for this decision uses the script-run classification produced during the glyph-mapping stage, falling back to Unicode script property of the surrounding characters.

---

## 8. Smart Quotes and Typographic Punctuation

U+2018 (LEFT SINGLE QUOTATION MARK), U+2019 (RIGHT SINGLE QUOTATION MARK), U+201C (LEFT DOUBLE QUOTATION MARK), U+201D (RIGHT DOUBLE QUOTATION MARK), U+2013 (EN DASH), and U+2014 (EM DASH) are **preserved as-is** in all output modes.

These are correct Unicode characters, not encoding errors. Normalizing them to ASCII apostrophes, straight quotation marks, or hyphens would constitute lossy transformation that destroys typographic information present in the source document. Downstream tools that require ASCII-only punctuation must perform their own substitution; pdftract does not make that decision for the caller.

---

## 9. Whitespace Collapse in Plain Text Output

The `--text` output mode applies a final whitespace normalization pass that is not applied to JSON structured output:

- Multiple consecutive U+0020 SPACE characters within a line are collapsed to a single space.
- U+000D, U+000D U+000A, and U+000C (FORM FEED) line endings are normalized to U+000A.
- Trailing whitespace is stripped from every line.
- Multiple consecutive blank lines are collapsed to a single blank line.

This pass runs after all other transformations. It is intentionally absent from JSON output, where span-level whitespace reflects the actual character sequence returned by the extraction engine and the caller controls presentation.

---

## 10. Combining Character Ordering

Some legacy PDF generators, particularly those targeting RTL scripts, write glyph sequences in visual order rather than logical Unicode order. A base character may be followed by combining diacritics in the order they appear left-to-right on screen rather than in Unicode canonical combining class (ccc) order. When combining marks with different canonical combining classes are in the wrong sequence, Unicode normalization produces a different composed character than intended—or fails to compose at all.

pdftract detects out-of-order combining sequences by examining the canonical combining class of each code point in a combining character sequence. When the sequence is not in non-decreasing ccc order, the marks are sorted by ccc before the final NFC composition pass. This reordering is applied only to sequences where all marks belong to the same base character (i.e., the sequence is a single combining character sequence in Unicode terms) to avoid incorrectly reordering marks that belong to adjacent base characters.

For Arabic and Hebrew text where the visual-to-logical reordering problem is pervasive, pdftract additionally applies the Unicode Bidirectional Algorithm to the extracted character sequence before the combining character sort, ensuring that the logical string order matches Unicode's expected representation for RTL text.

---

## Pipeline Execution Order

The transformations above execute in the following sequence to avoid interactions between steps:

1. Control character filtering (C0/C1 strip)
2. Zero-width character handling (strip ZWSP/BOM; preserve ZWJ/ZWNJ in complex-script spans)
3. PUA annotation (flag and pass through)
4. Soft hyphen resolution (requires raw glyph positions, must precede whitespace normalization)
5. Ligature and compatibility decomposition (selective, as specified in Section 2)
6. Superscript/subscript resolution (context-dependent)
7. Non-breaking space normalization (body text only)
8. Combining character reordering
9. NFC normalization (final composition)
10. Whitespace collapse (`--text` mode only)

Smart quote and typographic punctuation preservation requires no active transformation—it is an absence of normalization—and is therefore not a discrete step.
