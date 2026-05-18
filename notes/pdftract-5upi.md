# pdftract-5upi: Structural Token Lexer

## Summary

The structural token lexer was already fully implemented. This verification confirms that all acceptance criteria tests pass. The only change made was fixing a pre-existing compilation error in `xref.rs` by adding the missing `parse_obj_header_at_memory` function.

## Acceptance Criteria Status

### All Critical Tests PASS

1. **Array delimiters** (`[1 2 3]`): `array_delimiters` test PASSED
   - ArrayStart, Integer(1), Integer(2), Integer(3), ArrayEnd, Eof

2. **Dict delimiters** (`<< /A 1 >>`): `dict_delimiters` test PASSED
   - DictStart, Name(b"A"), Integer(1), DictEnd, Eof

3. **Hex string not dict** (`<48>`): `hex_string_odd_length_single_nibble` test PASSED
   - String(b"\x48"), Eof — correctly dispatches `<` followed by non-`<` to hex lexer

4. **Dict start, hex string, dict end** (`<<<48>>>`): `hex_string_dict_start_hex_string_dict_end` test PASSED
   - DictStart, String(b"\x48"), DictEnd

5. **Boolean and null keywords** (`true false null`): `bool_literals` and `null_keyword` tests PASSED
   - Bool(true), Bool(false), Null, Eof

6. **Object keywords** (`12 0 obj null endobj`): `obj_keywords` test PASSED
   - Integer(12), Integer(0), Obj, Null, EndObj, Eof

7. **Indirect reference** (`5 0 R`): `indirect_ref_keyword` test PASSED
   - Integer(5), Integer(0), IndirectRef, Eof

8. **Stream keywords** (`stream\n...endstream`): `stream_keywords` and `stream_header_valid_line_endings` tests PASSED
   - Token::Stream, then Token::EndStream

9. **Invalid stream header** (`stream\rxxx`): `stream_header_lone_cr_emits_diagnostic` test PASSED
   - Token::Stream + `STRUCT_INVALID_STREAM_HEADER` diagnostic (lone `\r` is invalid)

10. **Case-mismatched keyword** (`True`): `bool_case_sensitive` test PASSED
    - Token::Keyword(b"True"), Eof (object parser will reject)

### Proptests PASS

- `proptest_hex_string_never_panics_on_random_bytes`: PASSED
- `proptest_hex_string_roundtrip_via_reencode`: PASSED
- `proptest_string_never_panics_on_random_bytes`: PASSED
- `proptest_valid_string_roundtrips`: PASSED
- `name_proptest_never_panics_on_random_bytes`: PASSED
- `name_proptest_always_produces_valid_token`: PASSED

## Implementation Details

The structural token lexer dispatches from `next_token()` as follows:

- `[` / `]` → ArrayStart / ArrayEnd (direct return)
- `<` → peek next byte: if `<`, return DictStart (advance 2); else hex string lexer
- `>` → peek next byte: if `>`, return DictEnd (advance 2); else emit STRUCT_UNEXPECTED_BYTE
- `t` → check for "true" (Bool(true)) or "trailer" (Keyword), else lex_keyword
- `f` → check for "false" (Bool(false)), else lex_keyword
- `n` → check for "null" (Null), else lex_name
- `o` → check for "obj" (Obj), else lex_name
- `e` → check for "endstream" (EndStream) or "endobj" (EndObj), else lex_name
- `s` → check for "stream" (Stream with line ending validation) or "startxref" (Keyword)
- `R` → IndirectRef
- `x` → check for "xref" (Keyword)
- `%` → check for "%%EOF" (Keyword) or skip comment

### Stream Header Validation

Per PDF spec 7.3.8.1, the `stream` keyword must be followed by `\n` or `\r\n`. A lone `\r` is INVALID:

```rust
// In lex_s_keyword():
if let Some(&b'\n') = self.bytes.first() {
    self.advance(1); // \n is valid
} else if let Some(&b'\r') = self.bytes.first() {
    self.advance(1);
    if let Some(&b'\n') = self.bytes.first() {
        self.advance(1); // \r\n is valid
    } else {
        // Lone \r - emit STRUCT_INVALID_STREAM_HEADER
    }
}
```

## Changes Made

Fixed a pre-existing compilation error in `xref.rs` by adding the missing `parse_obj_header_at_memory` function. This function is a variant of `parse_obj_header_at` that works directly with a byte slice instead of a `PdfSource`, used by the `forward_scan_memory` function for efficient scanning of small files.

File: `crates/pdftract-core/src/parser/xref.rs`
- Added `parse_obj_header_at_memory` function (lines 1120-1189)

## INV-8 Status

INV-8 (lexer never panics on invalid input) is maintained:
- All proptests use random byte sequences and verify no panics
- Every lexer branch handles EOF gracefully
- Unknown keywords emit Token::Keyword instead of panicking
