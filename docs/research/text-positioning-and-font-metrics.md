# Text Positioning, Font Metrics, and Spacing Precision in PDF Extraction

## PDF Text State Parameters

PDF rendering is driven by a text state that accumulates across operators within a BT/ET block. pdftract must maintain all seven text state scalars in its content stream interpreter, because each one contributes to the final glyph position and advance.

**Tc (character spacing)** is a scalar in unscaled text space units added to the horizontal advance of every glyph after it is placed. It is additive with the advance width from the font. Because Tc accumulates on every character, even a Tc of 0.5 pt visibly spreads a long word. pdftract must apply Tc after each glyph's advance, including glyphs inside a TJ array, before applying any TJ kerning number that follows.

**Tw (word spacing)** is a scalar applied in addition to Tc, but only when the glyph code is 0x20 (the ASCII space). For multi-byte CIDFont encodings the space character may map to a different code point; pdftract must consult the ToUnicode CMap to identify which glyph code represents U+0020 before applying Tw. When Tw is nonzero, a space glyph effectively has advance = glyph_advance + Tc + Tw.

**Tz (horizontal scaling)** is a percentage value (100 = normal). It scales all horizontal distances — including advance widths and Tc and Tw — in text space. The scaling factor is Tz / 100. It does not affect vertical positioning or ascent/descent. pdftract must multiply every horizontal displacement by this factor when computing the glyph's contribution to the text matrix.

**TL (text leading)** is the vertical distance between baselines when the T* operator is used or when TD is called. It is stored but does not affect individual glyph placement; it determines the vertical offset applied by T*.

**Tf (font and size)** selects the current font resource and sets the text font size. The font size is a scale factor applied to glyph coordinates expressed in font units. pdftract must look up the named font in the current resource dictionary, decode its subtype (Type1, TrueType, CIDFont via Type0, Type3), and load its metrics accordingly.

**Tr (rendering mode)** affects whether the glyph is filled, stroked, clipped, or invisible. For text extraction purposes only Tr = 3 (invisible) is functionally significant; pdftract should suppress glyphs with Tr = 3 from the output since they are typically used for clipping without visual rendering.

**Ts (text rise)** shifts the baseline vertically, positive values moving the glyph upward, used for superscripts and subscripts. The rise is in unscaled text space and must be added to the y-component of the text position before transforming to user space. pdftract must include Ts in the baseline coordinate used for line grouping, since a superscript glyph on the same nominal line as its base text will have a different baseline y value and should not be merged into the same text span.

## The Text Matrix Tm and Line Matrix Tlm

At the start of a BT block, both the text matrix Tm and the line matrix Tlm are initialized to the identity matrix. These are 3×3 homogeneous matrices maintained in parallel.

The **Tm operator** sets both Tm and Tlm to the supplied matrix. It completely replaces the current position; it does not accumulate.

The **Td operator** (lowercase) moves the text position by (tx, ty) relative to the start of the current line: Tlm = [[1,0,0],[0,1,0],[tx,ty,1]] × Tlm, and Tm is set to the new Tlm. The **TD operator** (uppercase) is identical but also sets TL = −ty.

The **T* operator** is equivalent to Td(0, −TL).

After each glyph is rendered, the text position advances. The advance in text space is:

```
tx = (glyph_advance_in_font_units / 1000) * font_size * (Tz / 100) + Tc * (Tz / 100)
```

plus Tw * (Tz/100) if the glyph is the space character. This advance is applied to Tm by post-multiplying a translation matrix. Tlm is not modified by glyph advance; it retains the position of the start of the current line until a line-movement operator is encountered.

To convert the text position to user space, pdftract multiplies the current text position vector by Tm and then by the current transformation matrix (CTM) accumulated from the graphics state stack.

## Font Units vs. User Space Units

Glyph metrics in PDF fonts are expressed in font units. For Type 1 and most TrueType fonts the coordinate system is defined so that 1 em = 1000 font units. Type 3 fonts define their own font matrix via the /FontMatrix entry; the standard value is [0.001, 0, 0, 0.001, 0, 0], mapping the 1000-unit space into a 1-unit space consistent with Type 1.

The conversion from font units to text space is:

```
text_space_units = font_units * font_size / 1000
```

For Type 3 fonts, pdftract must apply the /FontMatrix to glyph coordinates before applying the font size, because the font matrix may not be the standard 1/1000 scaling.

The resulting text-space coordinate is then transformed to user space by Tm, and user space is transformed to device space by the CTM. For bounding box extraction in a device-independent form, pdftract should output coordinates in user space (points, where 1 pt = 1/72 inch) by applying Tm but stopping before the CTM, unless the caller requests device-pixel coordinates.

## Width Arrays and WX Entries

Width information for each glyph is critical for accurate advance computation. pdftract must load widths from the font dictionary rather than relying on glyph outlines, which may not be embedded.

For **simple fonts** (Type1, TrueType, Type3), the /Widths array contains the advance widths for glyph codes from /FirstChar to /LastChar, in font units. The /MissingWidth entry in the font descriptor provides the default for codes not covered by /Widths. pdftract must handle the case where /Widths is absent (relying entirely on /MissingWidth) gracefully.

For **CIDFonts** (used as descendants of Type0 fonts), the /W array uses a compact, sparse encoding. It alternates between two forms: a range form `c_first [w1 w2 ... wn]` mapping consecutive CIDs starting at c_first, or a run form `c_first c_last w` assigning the same width w to all CIDs in the range. The /DW entry gives the default width for CIDs not covered by /W. pdftract must parse both forms and build a lookup table for O(1) advance retrieval per glyph.

When the TJ operator provides kerning numbers, those numbers are in thousandths of a text space unit and are subtracted from the current x position (negative = leftward displacement). Glyph widths from /W or /Widths are in font units (thousandths of an em). These are different scales: a TJ kerning value of −100 means a displacement of 0.1 text space units at the current font size, while a glyph width of 1000 in /W means a full em.

## Horizontal vs. Vertical Writing Modes

The /WMode entry in a CIDFont's /CIDSystemInfo or in the CMap determines writing direction: 0 for horizontal (default), 1 for vertical.

In **horizontal mode**, the advance vector after each glyph is along the positive x-axis in text space. The text matrix is updated by tx as described above; ty is zero for normal glyphs.

In **vertical mode**, the advance vector is along the negative y-axis. pdftract must use the /W2 array, which provides vertical advance widths (v) and glyph origin offsets (w1x, w1y) for each glyph. The origin offset shifts the glyph's origin from the default position at the top-center of the advance rectangle. The effective advance after each glyph is:

```
ty = -(v / 1000) * font_size
```

with the horizontal offset w1x applied as a one-time shift at the start of the glyph. Vertical text extraction requires grouping glyphs by their x-coordinate (the column baseline) rather than the y-coordinate.

## Kerning in TJ Arrays and Word Boundary Reconstruction

The TJ operator accepts an array whose elements are either byte strings (rendered as glyphs) or numbers (kerning adjustments). A negative number shifts the text position leftward, effectively closing space between glyphs; a positive number opens space. The displacement in text space units is:

```
displacement = -(kerning_number / 1000) * font_size * (Tz / 100)
```

When reconstructing word boundaries, pdftract must decide whether a TJ number represents intentional word spacing or typographic kerning. A reliable heuristic: if the absolute displacement exceeds a threshold (typically 0.2 to 0.3 times the space character's advance width at the current font size), treat it as a word gap and inject a synthetic space. Below this threshold, treat it as kerning and accumulate it into the preceding glyph's bounding box.

## Character Spacing and Word Spacing Interaction

Tc and Tw interact with TJ kerning in a specific order. For each glyph emitted by a Tj or TJ string segment: first apply the glyph's advance from /W or /Widths, scaled to text space; then add Tc scaled by Tz/100; then add Tw scaled by Tz/100 if the glyph code is the space character. TJ kerning numbers are applied between string segments, not between individual glyphs within a segment.

This means that if a word is encoded as a single string inside a TJ array, Tc accumulates across every character in that string. pdftract must not apply Tc only once per string — it applies once per glyph code emitted.

## Sub-pixel Precision

PDF coordinates are floating-point. A character at x = 72.35 pt and another at x = 72.36 pt are distinct positions that affect layout analysis. pdftract must preserve at least two decimal places in all bounding box coordinates. Rounding glyph positions to integers introduces errors that misalign characters on the same baseline by enough to break line-grouping logic, particularly in justified text where inter-word spacing is adjusted to sub-point precision.

All intermediate computations — Tm multiplication, advance accumulation, Tc/Tw addition — must be carried out in 64-bit floating point. Output bounding boxes should be serialized at two decimal places minimum.

## Bounding Box Computation

A glyph's bounding box in user space is computed as follows. The lower-left corner x-coordinate is the current text position's x after transforming through Tm and CTM. The width of the bounding box is:

```
bb_width = (glyph_advance + Tc + Tw_if_space - kerning_applied_after) * (Tz / 100) * font_size / 1000
```

where kerning is the TJ number that follows this glyph in the array, if any. For the vertical extent, pdftract uses the font's /Ascent and /Descent values from the font descriptor, scaled by font_size / 1000. These are the typographic ascent and descent in font units and define the bounding box height from baseline − |Descent| to baseline + Ascent, adjusted by Ts.

For a tight span bounding box spanning multiple glyphs on the same baseline, x_min is the text position at the first glyph and x_max is the text position after the last glyph's advance (including Tc, excluding post-span kerning). The vertical extent uses the maximum Ascent and minimum Descent across all fonts used in the span.

## Baseline Grid and Line Detection

After transforming through Tm and CTM, each glyph has a baseline y-coordinate in user space. Glyphs with identical or nearly identical baseline y values belong to the same line. Because floating-point arithmetic in PDF generators is imprecise, pdftract must use a tolerance for baseline comparison — a reasonable value is 0.5 pt.

The grouping algorithm should sort glyphs by their baseline y-coordinate and cluster them into lines using single-linkage: two glyphs join the same line if their baseline y values differ by less than the tolerance. Within a line, glyphs are sorted by their x-coordinate (or y-coordinate in vertical mode). After clustering, pdftract checks for text rise: glyphs within a line that have nonzero Ts should be tagged as superscript or subscript rather than merged into the main text run, since their visual y position differs but their logical line membership is retained for reading-order purposes.

The final output — an ordered sequence of glyph records each carrying Unicode text, user-space bounding box, font size, and baseline y — gives downstream consumers the data needed for accurate word segmentation, column detection, and table extraction without requiring re-processing of the raw content stream.
