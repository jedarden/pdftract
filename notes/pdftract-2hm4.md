# pdftract-2hm4: Implement PDF hex string lexer with odd-length zero padding

## Summary

This bead implements the PDF hex string lexer that decodes hex strings of the form `<...>` into byte sequences. The implementation was already present in the codebase; this bead renamed diagnostic codes to use the `STRUCT_` prefix as specified in the bead description.

## Work Done

### 1. Renamed Diagnostic Codes

The lexer's `DiagCode` enum variants were renamed to use the `STRUCT_` prefix:
- `InvalidName` -> `StructInvalidName`
- `InvalidHex` -> `StructInvalidHex`
- `InvalidOctal` -> `StructInvalidOctal`
- `InvalidStreamHeader` -> `StructInvalidStreamHeader`
- `UnexpectedEof` -> `StructUnexpectedEof`
- `UnterminatedString` -> `StructUnterminatedString`

All references throughout the lexer module were updated accordingly.

### 2. Existing Implementation Verified

The hex string lexer (`lex_hex_string()`) was already implemented with:
- Hex digit pair decoding: `<48656C6C6F>` -> `b"Hello"`
- Embedded whitespace handling (PDF spec 7.2.2 whitespace ignored)
- **Odd-length zero padding**: `<4>` -> `b"\x40"` (dangling nibble becomes HIGH nibble with LOW nibble 0)
- Both lowercase and uppercase hex digits accepted
- Invalid character handling with `STRUCT_INVALID_HEX` diagnostic
- Unterminated string handling with `STRUCT_UNTERMINATED_STRING` diagnostic

### 3. Files Modified

- `crates/pdftract-core/src/parser/lexer/mod.rs`: Renamed 6 `DiagCode` enum variants and updated all references

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| `<>` -> `b""` | PASS | hex_string_empty |
| `<4>` -> `b"\x40"` (NOT `\x04`) | PASS | hex_string_odd_length_single_nibble |
| `<48656C6C6F>` -> `b"Hello"` | PASS | hex_string_hello_world |
| `<aBcD>` -> `b"\xAB\xCD"` | PASS | hex_string_mixed_case |
| `<48 65>` -> whitespace ignored | PASS | hex_string_with_whitespace |
| Unterminated `<48` -> diagnostic | PASS | hex_string_unterminated_emits_diagnostic |
| proptest: random bytes never panic | PASS | proptest_string_never_panics_on_random_bytes |
| proptest: roundtrip property | PASS | proptest_valid_string_roundtrips |
| INV-8 maintained | PASS | All error paths use diagnostics, no panics |

## Test Results

```
cargo test --lib parser::lexer::tests::hex_string
test result: ok. 16 passed; 0 failed
```

All hex string tests pass:
- `hex_string_empty`: `<>` -> `b""`
- `hex_string_odd_length_single_nibble`: `<4>` -> `b"\x40"` (critical test)
- `hex_string_hello_world`: `<48656C6C6F>` -> `b"Hello"`
- `hex_string_mixed_case`: `<aBcD>` -> `b"\xAB\xCD"`
- `hex_string_with_whitespace`: `<48 65 6C\n6C 6F>` -> `b"Hello"`
- `hex_string_invalid_char_emits_diagnostic`: `<48Z65>` -> `b"\x48\x65"` + `STRUCT_INVALID_HEX`
- `hex_string_unterminated_emits_diagnostic`: `<4865` -> `b"\x48\x65"` + `STRUCT_UNTERMINATED_STRING`
- `hex_string_unterminated_with_dangling_nibble`: `<48657` -> `b"\x48\x65\x70"` + diagnostic
- And 8 more tests covering edge cases

Proptests also pass:
- `proptest_string_never_panics_on_random_bytes`: Random bytes never panic
- `proptest_valid_string_roundtrips`: Decode+encode roundtrip property

## Implementation Notes

The critical odd-length padding behavior is correct:
- `<4>` -> `b"\x40"` (the nibble `4` becomes the HIGH nibble, LOW nibble is 0)
- `<48657>` -> `b"\x48\x65\x70"` (dangling `7` becomes `0x70`, NOT `0x07`)

This is the opposite of intuition (trailing zero is LOW, not HIGH) and is a common bug source.

## References

- Plan section: Phase 1.1 Lexer, line 1032-1033 (hex strings, odd-length zero padding); line 1046 (critical test)
- PDF spec 7.3.4.3 (Hexadecimal Strings)
- Files modified:
  - crates/pdftract-core/src/parser/lexer/mod.rs
