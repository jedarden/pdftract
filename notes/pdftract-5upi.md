# pdftract-5upi: Structural Token Lexer Bug Fix

## Summary

Fixed incorrect fallback behavior in keyword lexer functions. Four functions (`lex_e_keyword`, `lex_o_keyword`, `lex_r_keyword`, `lex_n_keyword`) were incorrectly calling `lex_name()` instead of `lex_keyword()` when keywords didn't match, causing incorrect parsing of unrecognized keywords starting with those letters.

## Bug Description

When a PDF contains an unrecognized word starting with `e`, `o`, `n`, or `R` (e.g., a typo like "endob" instead of "endobj"), the lexer should fall back to generic keyword parsing, which emits `Token::Keyword(bytes)`. Instead, these functions were calling `lex_name()`, which expects to parse a name object (always starting with `/`).

**Why this matters:** Names in PDF always start with `/`. The `lex_name()` function immediately advances past the leading `/`. When called on input like "endob" (no leading `/`), it would incorrectly skip the first byte and try to parse the rest as a name, producing wrong results.

## Changes Made

File: `crates/pdftract-core/src/parser/lexer/mod.rs`

1. **`lex_e_keyword()` (line 1037)**: Changed `self.lex_name()` to `self.lex_keyword()`
   - Handles: "endstream", "endobj"
   - Fallback example: "endob" → `Token::Keyword(b"endob")` (was incorrectly parsed as name)

2. **`lex_o_keyword()` (line 1050)**: Changed `self.lex_name()` to `self.lex_keyword()`
   - Handles: "obj"
   - Fallback example: "ob" → `Token::Keyword(b"ob")` (was incorrectly parsed as name)

3. **`lex_r_keyword()` (line 1060)**: Changed `self.lex_name()` to `self.lex_keyword()`
   - Handles: "R" (indirect reference)
   - Fallback example: "Ref" → `Token::Keyword(b"Ref")` (was incorrectly parsed as name)

4. **`lex_n_keyword()` (line 1074)**: Changed `self.lex_name()` to `self.lex_keyword()`
   - Handles: "null"
   - Fallback example: "nul" → `Token::Keyword(b"nul")` (was incorrectly parsed as name)

## Verification

### Pre-existing Implementation Status

The structural token lexer was already fully implemented. This fix only corrected the fallback behavior:

| Feature | Status | Location |
|---------|--------|----------|
| Array delimiters `[` `]` | ✅ | Lines 379-380 |
| Dict delimiters `<<` `>>` | ✅ | Lines 866-870, 952-956 |
| Boolean keywords `true` `false` | ✅ | Lines 474-505 |
| Null keyword `null` | ✅ | Lines 1064-1074 |
| Obj keywords `obj` `endobj` | ✅ | Lines 1028-1050 |
| Stream keywords `stream` `endstream` | ✅ | Lines 969-1036 |
| Indirect ref `R` | ✅ | Lines 1053-1061 |
| Xref keywords `xref` `trailer` `startxref` | ✅ | Lines 483-518, 1007-1014 |
| `%%EOF` marker | ✅ | Lines 521-528 |
| Stream header validation (PDF 7.3.8.1) | ✅ | Lines 978-1002 |

### Acceptance Criteria Tests (all present in code)

1. **Array with integers** (line 2016): `[1 2 3]` → ArrayStart, Integer(1), Integer(2), Integer(3), ArrayEnd, Eof
2. **Dict with name and integer** (line 2028): `<< /A 1 >>` → DictStart, Name(b"A"), Integer(1), DictEnd, Eof
3. **Hex string (not dict)** (line 1437): `<48>` → String(b"\x48"), Eof
4. **Dict-hex-dict ambiguity** (line 1576): `<<<48>>>` → DictStart, String(b"\x48"), DictEnd
5. **Boolean and null** (line 2061): `true false null` → Bool(true), Bool(false), Null, Eof
6. **Indirect object header** (line 2039): `12 0 obj null endobj` → Integer(12), Integer(0), Obj, Null, EndObj, Eof
7. **Indirect reference** (line 2051): `5 0 R` → Integer(5), Integer(0), IndirectRef, Eof
8. **Case-sensitive keyword** (line 1134): `True` → Keyword(b"True"), Eof
9. **Stream header validation** (lines 1188-1221):
   - `stream\nbody` → Stream, no diagnostics
   - `stream\r\nbody` → Stream, no diagnostics
   - `stream\rbody` → Stream + STRUCT_INVALID_STREAM_HEADER
   - `stream body` → Stream + STRUCT_INVALID_STREAM_HEADER

### Proptest Properties (INV-8 Compliance)

- `proptest_random_bytes_never_panics` (line 2071): Verifies no panic on any input
- All lexer branches handle EOF gracefully
- Unknown keywords emit `Token::Keyword(bytes)` instead of panicking

## Additional Bug Fix (2026-05-20)

### Commit: `7818f22` - `fix(pdftract-5upi): remove diagnostic emission for unknown keywords`

**Issue**: The `lex_keyword()` function was incorrectly emitting `StructUnexpectedByte` diagnostics for unknown keywords.

**Fix**: Removed diagnostic emission from `lex_keyword()` function (lines 540-564).

**Rationale**:
1. Many valid keywords (trailer, xref, etc.) are not in the initial dispatch table
2. The object parser is responsible for validating keywords against known operators
3. Emitting diagnostics here causes false positives for valid PDF constructs

This change aligns with the task requirement that unknown keywords emit `Token::Keyword` without a diagnostic, letting the object parser handle `STRUCT_UNKNOWN_KEYWORD` if needed.

## Notes

The lexer module compiles successfully. Full integration tests cannot run due to unrelated pre-existing compilation errors in other modules (missing `LZWDecoder`, `Diagnostic` type mismatches in catalog.rs, pages.rs, ocg.rs). These errors are not caused by this change.
