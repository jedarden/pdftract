# PDF Object Model and Data Types

**Reference for pdftract's PDF lexer and object parser**

---

## Overview

The PDF specification defines eight fundamental object types that together form the basis of every PDF document: Boolean, Integer, Real, String, Name, Array, Dictionary, Stream, and Null. A correct PDF parser must handle every syntactic variant of each type, including edge cases that the specification acknowledges but that real-world generators produce in unexpected ways. This document provides a precise reference for implementing the pdftract lexer and object parser such that extraction never fails due to parsing ambiguity or malformed object encoding.

---

## 1. Boolean and Null

The Boolean and Null objects are keyword literals. `true` and `false` represent Boolean values; `null` represents the Null object. All three are case-sensitive — `True`, `FALSE`, and `Null` are not valid PDF keywords. A Null value in a dictionary is distinct from an absent key: an absent key typically implies a default value defined by the specification, whereas an explicit `null` value means the key is present but its value is the Null object. Parsers must preserve this distinction rather than treating both cases identically.

---

## 2. Integer and Real Numbers

Integer objects have no decimal point and may be preceded by an optional `+` or `-` sign. Real objects contain a decimal point, such as `3.14` or `-0.5`. The PDF specification states that integers fit within 32-bit signed range and reals within 32-bit float precision; however, documents produced by non-conforming generators routinely exceed these bounds. pdftract should store integers as `i64` and reals as `f64` to avoid truncation or overflow silently corrupting extracted data.

Integer/real disambiguation during lexing is straightforward: the presence of a `.` character in the token makes it a real. Some generators also emit real values using scientific notation (`1.5e3`), which the specification does not sanction but which appears in files produced by certain software. A robust lexer must recognize this form and parse it correctly.

Negative zero (`-0`) is syntactically valid. As an integer it should parse to zero. As a real it may produce negative zero in IEEE 754, which is semantically equivalent to positive zero for PDF purposes but must not cause a parse error.

---

## 3. String Objects

PDF strings come in two syntactic forms: literal strings and hexadecimal strings.

**Literal strings** are enclosed in parentheses. The content may include any byte except unescaped unbalanced parentheses. Parentheses may be nested without escaping, provided they remain balanced — `(foo (bar) baz)` is a single valid string containing the inner parentheses. Backslash escape sequences allow the inclusion of special characters:

- `\n` — LINE FEED (0x0A)
- `\r` — CARRIAGE RETURN (0x0D)
- `\t` — HORIZONTAL TAB (0x09)
- `\\` — backslash
- `\(` and `\)` — literal parentheses, bypassing balance counting
- `\ddd` — one to three octal digits representing a byte value (e.g., `\101` = `A`)
- `\` followed by a newline — line continuation; the backslash and newline are both ignored, allowing long strings to span source lines

A backslash followed by any character not in the above list should be treated as that character alone (the backslash is discarded). Parsers must handle the case where a `\ddd` octal sequence has fewer than three digits before the next non-octal character.

**Hexadecimal strings** are enclosed in angle brackets: `<4F6E65>`. Each pair of hex digits represents one byte. Whitespace between digit pairs is permitted and must be ignored. If the total number of hex digits is odd, the final digit is treated as if followed by a zero — `<4F6>` decodes as bytes `0x4F` and `0x60`. Case is not significant for hex digits.

**UTF-16 strings** appear in PDF 1.7 for text that cannot be represented in PDFDocEncoding. They begin with the big-endian byte-order mark `FE FF`. When the parser encounters this BOM at the start of a string (literal or hex-decoded), it must decode the remainder as UTF-16BE rather than as PDFDocEncoding. A robust implementation converts these to Rust's native UTF-8 during parsing, surfacing any malformed surrogate pairs as replacement characters rather than panicking.

---

## 4. Name Objects

Name objects begin with a forward slash and are followed by regular characters. The slash itself is not part of the name's value. Names are case-sensitive: `/Type` and `/type` are different names.

Within a name, the sequence `#xx` — where `xx` is exactly two hexadecimal digits — is a hex escape representing the byte with that value. This allows names to contain bytes that would otherwise be delimiter or whitespace characters. Hex escapes are case-insensitive. The byte `#00` (the null byte) is syntactically valid inside a name and must not cause the parser to truncate the name at that point. In Rust, names must be stored as `Vec<u8>` or a newtype wrapper rather than `String`, since Rust strings cannot contain interior null bytes. Callers needing a string representation can use a lossy conversion or handle the null as an escaped sequence.

The PDF specification imposes no hard limit on name length, but the recommended maximum for interoperability is 127 bytes. In practice, generators may produce longer names; the parser should not truncate them.

---

## 5. Array and Dictionary Objects

Arrays are delimited by `[` and `]`. Elements are any object types, separated by whitespace. Arrays may be nested to arbitrary depth. Array elements may include indirect references (described below), allowing deferred resolution.

Dictionaries are delimited by `<<` and `>>`. Each entry consists of a Name key followed by a value of any object type. Keys must be Name objects; values may be any object, including nested dictionaries, arrays, or indirect references. A dictionary containing no entries — whitespace only between `<<` and `>>` — is a valid empty dictionary and must not be treated as a parse error. Dictionary keys should be deduplicated in the parser's output; the PDF specification states that duplicate keys produce undefined behavior, but in practice the last occurrence typically wins.

Both structures may be deeply nested. The parser must not use a fixed-depth recursion limit that would cause it to reject valid (if pathological) documents. Iterative parsing using an explicit stack is preferable to naive recursion for production use.

---

## 6. Stream Objects

A stream object always consists of a dictionary followed by the keyword `stream`, the stream data, and the keyword `endstream`. The line ending after `stream` must be either a single LINE FEED or a CARRIAGE RETURN followed by a LINE FEED — a standalone CR is not permitted, though lenient parsers may accept it. The stream data is binary and extends for exactly as many bytes as specified by the `/Length` entry in the preceding dictionary.

The `/Length` key is authoritative for determining the extent of the stream body. After reading `/Length` bytes, the parser skips optional whitespace and expects the `endstream` keyword. Common `/Length` errors in real-world PDFs include:

- **Off-by-one** errors where the declared length is one byte too long or too short relative to `endstream`
- **CRLF miscounts** where the generator counted a newline as one byte but stored two
- **Zero-length streams** which are valid and must not cause errors
- **Missing /Length** which requires scanning forward for `endstream` as a recovery strategy

A correct recovery implementation scans forward from the `stream` keyword to find `endstream`, uses the actual byte count as the effective length, and emits a recoverable parse warning. This ensures that a /Length error does not cause extraction failure for the remainder of the document.

---

## 7. Indirect References

An indirect reference takes the form `N G R`, where `N` is the object number, `G` is the generation number, and `R` is the literal keyword. For example, `12 0 R` refers to object 12 at generation 0. Indirect references may appear wherever any object may appear: as dictionary values, array elements, or standalone objects. The object number and generation number are non-negative integers.

Resolution of an indirect reference requires a lookup in the cross-reference table (xref). The xref maps object number and generation to a byte offset within the file where the corresponding `N G obj` ... `endobj` block begins. Circular references are technically invalid but must not cause the parser to loop indefinitely; a resolution depth limit or a visited-objects set provides a safe guard.

---

## 8. Comments

A `%` character outside of a string or stream begins a comment that extends to the end of the line. Comments may appear between any tokens and are treated as whitespace by the parser.

The conventional PDF header `%PDF-1.x` is followed on the next line by a comment containing four bytes with values above 127, such as `%âãÏÓ`. This high-bit-byte convention signals to FTP clients that the file is binary and should not be transferred in text mode. The lexer should handle this comment correctly rather than treating the high bytes as malformed input.

---

## 9. Parser Correctness: Edge Cases

Several lexical edge cases deserve specific attention during implementation:

**Whitespace** in PDF includes space (0x20), horizontal tab (0x09), carriage return (0x0D), line feed (0x0A), form feed (0x0C), and null (0x00). The null byte as whitespace is rare but must not terminate tokenization prematurely.

**Delimiter characters** — `(`, `)`, `<`, `>`, `[`, `]`, `{`, `}`, `/`, `%` — terminate a token without consuming the delimiter itself. The `<<` and `>>` tokens must be distinguished from a single `<` or `>`, which denotes a hex string or comparison operator in PostScript but not in PDF object streams.

**Very long strings or names** must not overflow fixed buffers. The parser should accumulate token content into a growable structure (e.g., Rust's `Vec<u8>`) without imposing an arbitrary size ceiling.

**Malformed hex strings** with an odd digit count are recoverable by appending a trailing `0` before decoding. A hex string containing non-hex, non-whitespace characters inside the `<>` delimiters is malformed; the parser should emit a warning and skip the invalid character, treating the remaining digits as the string's content.

**Empty dictionaries** (`<< >>`) are valid. Parsers that expect at least one key-value pair before `>>` will incorrectly reject them.

**Negative zero** (`-0`) is syntactically valid for both integer and real tokens. Integer negative zero should produce `0i64`. Real negative zero should produce `-0.0f64` without error.

---

*This document is a reference for pdftract's lexer and object parser implementation. Correctness at the syntax level — particularly for edge cases involving escape sequences, /Length discrepancies, null bytes in names, and deeply nested structures — is foundational to reliable text extraction across the full range of real-world PDF files.*
