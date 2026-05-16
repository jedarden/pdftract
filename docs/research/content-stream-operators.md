# PDF Content Stream Operator Reference for Text Extraction

## Overview

A PDF content stream is a sequence of operands followed by operators, processed left to right. Text extraction requires accurate parsing of this stream, including correct handling of operator arguments, encoding subtleties, and interactions with the graphics state. This document covers every operator class relevant to pdftract's content stream parser.

---

## 1. Text Object Delimiters: BT and ET

Text objects are bracketed by `BT` (begin text) and `ET` (end text). The text matrix (Tm) and text line matrix (Tlm) are initialized to the identity matrix at `BT` and discarded at `ET`. Text operators are only valid inside a text object; invoking them outside is an error that real-world PDFs nonetheless commit. pdftract must tolerate `BT`/`ET` mismatches — unpaired or nested occurrences exist in producer output — and should maintain a nesting counter rather than a simple boolean flag.

---

## 2. Font and Size: Tf

`name size Tf` sets the current font to the resource named `name` (a PDF name object) and the text font size to `size` in unscaled text space units. The font resource must be looked up in the current resource dictionary's `Font` subdictionary. Failure to track the current font and size means character-to-Unicode mapping cannot be performed, because glyph encoding is font-specific. Every `Tf` invocation must trigger a font cache lookup or load.

---

## 3. Text Positioning Operators: Tm, Td, TD, T*

`a b c d e f Tm` sets both the text matrix and the text line matrix to the provided six-element matrix. This is an absolute positioning operation that replaces, not concatenates, the existing text matrix.

`tx ty Td` moves the text position by `(tx, ty)` in text space and sets the text line matrix to the new position. `tx ty TD` is equivalent to `-ty TL` followed by `tx ty Td` — it simultaneously updates the leading parameter `TL`.

`T*` moves to the next line, equivalent to `0 -TL Td` where `TL` is the current text leading value. Tracking `TL` across `TD` and `TL` operator invocations is required for correct line break detection.

---

## 4. String Show Operators: Tj, TJ, ', "

`string Tj` paints the glyphs for the given string and advances the text position by the sum of the glyph widths plus character spacing and word spacing adjustments.

`array TJ` accepts a PDF array alternating between string objects and numeric objects. Each string element is rendered in sequence; each numeric element adjusts the horizontal text position by `-value / 1000` text units before rendering the next string. The sign convention is critical: a positive number moves left (tightens spacing), and a negative number moves right (adds space). Treating the sign incorrectly reverses kern direction and corrupts word boundary detection. Multiple strings within a single `TJ` array are logically concatenated text — they must be joined without inserting spurious word separators unless a sufficiently negative numeric element indicates a word gap.

`string '` is exactly equivalent to `T* string Tj`: it moves to the next line and then shows the string. This is a shorthand that must not be confused with the PDF string delimiter `'`, which is not a valid PDF string delimiter at all — strings use parentheses or angle brackets.

`Tw Tc string "` sets the word spacing to `Tw` and the character spacing to `Tc`, then moves to the next line and shows the string, equivalent to `Tw Tw Tc Tc T* string Tj`. The two numeric operands precede the string operand. Misidentifying `"` as a two-argument operator versus a one-argument operator will cause operand stack corruption for all subsequent operators in the stream.

---

## 5. String Encoding: Literal and Hex Strings

Both `Tj` and `TJ` accept PDF string arguments in one of two encodings.

Literal strings are enclosed in parentheses: `(Hello)`. Parentheses must be balanced or escaped with a backslash. A backslash followed by a digit sequence introduces an octal escape. A backslash at a line break signals a continuation with no newline character in the string value.

Hex strings are enclosed in angle brackets: `<48656C6C6F>`. Each pair of hex digits encodes one byte. An odd number of hex digits is completed with an implicit trailing zero. Hex strings may contain whitespace between digit pairs for readability, which must be ignored during decoding.

Both encodings can represent arbitrary byte values, including null bytes (0x00). Some parsers terminate string reading at a null byte. pdftract must treat null bytes as valid string content and pass them through to the character mapping stage. Encodings such as UTF-16BE prefix their content with a BOM (0xFE 0xFF) and embed null bytes for ASCII characters; failing to read past nulls silently truncates text.

---

## 6. Graphics State Operators Affecting Text: q, Q, cm, gs

`q` pushes a copy of the entire graphics state — including all text state parameters (font, size, Tc, Tw, TL, Tr, Ts, and the text and text line matrices) — onto the graphics state stack. `Q` pops and restores it. Form XObjects commonly bracket their content with `q`/`Q`, so every recursive call into an XObject stream must begin with an implicit save and end with an implicit restore; a `q` without a matching `Q` inside an XObject is contained by the recursive frame.

`a b c d e f cm` concatenates the provided matrix with the current transformation matrix (CTM). Because text coordinates are transformed through the CTM into device space, changes to the CTM affect the computed position of rendered text even when no text positioning operator is invoked. pdftract must maintain the full current transformation matrix to compute accurate bounding boxes or reading order.

`name gs` applies a named entry from the current resource dictionary's `ExtGState` subdictionary. ExtGState dictionaries may set font (`Font` key), character spacing (`CA`, `ca` are opacity but `TC`, `TW`, `TL`, `Ts`, `Tf` are text), rendering mode (`TR`/`TR2`), and other text-relevant parameters. pdftract must inspect the ExtGState dictionary and update its internal text state accordingly.

---

## 7. Inline Images: BI, ID, EI

An inline image is introduced by `BI`, followed by key-value pairs describing the image (width, height, color space, filter, etc.), then `ID` on its own line, followed immediately by the raw binary image data, then `EI`.

Detecting `EI` is non-trivial because the raw image data may contain the byte sequence `EI` as part of its payload. The robust algorithm is: compute the expected byte length of the image data from the width, height, bits per component, and color space, applying any compression filter to determine the compressed length; read exactly that many bytes after `ID`; then expect `EI` as the next token. If the filter length is not determinable (e.g., the filter is unknown), fall back to scanning for a whitespace-preceded `EI` followed by whitespace or an operator name — but this heuristic can misfire. pdftract should prefer length-based detection wherever possible and treat inline images as opaque blobs; they contain no text operators.

---

## 8. Marked Content: BDC, BMC, EMC, MP, DP

Marked content operators are the structural hooks for tagged PDF. `tag BDC` and `tag properties BDC` begin a marked content sequence with an optional property dictionary; `tag BMC` begins one without properties. `EMC` ends the innermost open marked content sequence. `tag MP` and `tag properties DP` are point operators that mark a location without delimiting a span.

For text extraction, marked content enables mapping of content to logical structure (headings, paragraphs, table cells, artifacts). The `Artifact` tag marks content that should be excluded from extracted text (headers, footers, page numbers, decorative rules). The `ActualText` attribute in a property dictionary provides an explicit Unicode string to substitute for the rendered glyph sequence, handling ligatures, special characters, and layout artifacts. pdftract should track open marked content sequences and expose their tags and properties to the extraction layer.

---

## 9. The Do Operator and XObjects

`name Do` invokes the XObject named `name` from the current resource dictionary's `XObject` subdictionary. The named object is either a Form XObject or an Image XObject.

A Form XObject is a PDF stream with its own content stream and its own resource dictionary. It must be processed recursively: the current graphics state is saved, the Form's matrix is concatenated with the CTM, the Form's resource dictionary becomes the active resource context, and the Form's content stream is parsed and executed as if it were inline. Text generated inside a Form XObject appears in device space at positions determined by the combined transformation. Failing to recurse into Form XObjects silently drops text.

An Image XObject is a raster image. It contains no text operators and must be skipped. The distinction between Form and Image XObjects is the `Subtype` key in the XObject's dictionary: `/Form` versus `/Image`.

---

## 10. Compatible Extensions: BX and EX

`BX` and `EX` bracket a sequence of operators that may not be defined in the PDF specification version being parsed. A conforming reader that does not understand operators inside a `BX`/`EX` pair must skip them without error. pdftract must track `BX`/`EX` nesting depth and, when inside a compatible section, consume and discard all tokens — including any operands — until the matching `EX` is reached. Crashing or raising an error on unknown operators inside `BX`/`EX` violates the PDF specification and will fail on real-world files produced by non-standard tools.

---

## 11. Tokenizer Edge Cases

PDF defines six whitespace characters: space (0x20), horizontal tab (0x09), carriage return (0x0D), line feed (0x0A), form feed (0x0C), and null (0x00). Any combination of these may appear between operands and operators. The tokenizer must skip all whitespace between tokens and must not treat any whitespace character as significant except inside string literals and when determining line endings for the `'` and `"` operators.

Comments begin with `%` and extend to the end of the line (the next CR, LF, or CR+LF sequence). Comment content is ignored. However, comments may appear anywhere between tokens — including between an operand and the operator it belongs to. The tokenizer must treat comments exactly as whitespace.

PDF files commonly begin with a high-bit-byte comment such as `%âãÏÓ` or `%¥±ë` immediately after the `%PDF-1.x` header line. This comment signals to transfer protocols that the file is binary. The tokenizer must handle these high-byte characters without misinterpreting them as tokens; since they appear in a comment, they are discarded before any token scanning begins.

Binary data at the start of compressed streams (after `stream\n`) may begin with bytes that coincidentally match operator names. pdftract must never parse stream data as operator tokens; the stream body is always accessed through its decoded filter output, not by scanning raw bytes inline in the content stream tokenizer.

Operator names in PDF are composed of characters from a defined set; some operators use characters outside the alphanumeric range (`'`, `"`, `*`). The tokenizer must include these in its operator character set and must distinguish them from the delimiters `(`, `)`, `<`, `>`, `[`, `]`, `{`, `}`, `/`, `%` that begin other object types.
