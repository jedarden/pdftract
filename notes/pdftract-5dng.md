# pdftract-5dng: PDF Name Object Lexer Implementation

## Summary

The PDF name object lexer was already fully implemented in `crates/pdftract-core/src/parser/lexer/mod.rs` (lines 658-767). All acceptance criteria pass with comprehensive test coverage.

## Acceptance Criteria Status

### PASS

All critical tests from the plan pass:

1. **`/Foo` -> `Token::Name(b"Foo")`** - `name_simple` test
2. **`/Foo#20Bar` -> `Token::Name(b"Foo Bar")`** - `name_with_hex_escape_space` test (#20 = space)
3. **`/Foo#00Bar` -> `Token::Name(b"Foo")` + `STRUCT_INVALID_NAME`** - `name_nul_byte_rejected` test
4. **`/AAA...AAA` (128 A's) -> `Token::Name(b"AAA...AAA")` (127 A's) + `STRUCT_INVALID_NAME`** - `name_length_limit_127_bytes` test
5. **`/` (alone) -> `Token::Name(b"")` (no diagnostic)** - `name_empty` test
6. **`/#23#23` -> `Token::Name(b"##")`** - `name_hex_escape_decodes_to_hash` test (#23 = #)
7. **`/Foo#GZ` -> `Token::Name(b"Foo#GZ")` + `STRUCT_INVALID_NAME`** - `name_invalid_hex_escape_keeps_hash_literal` test

### Proptests

- **`name_proptest_never_panics_on_random_bytes`** - PASS
- **`name_proptest_always_produces_valid_token`** - PASS

### INV-8

The implementation maintains INV-8 (lexer invariant). The name lexer properly:
- Tracks raw byte consumption (`raw_consumed`)
- Enforces 127-byte raw length limit before hex expansion
- Rejects NUL bytes (0x00) with `STRUCT_INVALID_NAME` diagnostic
- Truncates at 127 raw bytes, avoiding half-decoded escapes

## Implementation Details

The `lex_name()` function:

1. **Entry**: Position immediately after the leading `/`
2. **Hex escapes (`#XX`)**: Decodes to single byte, checking for valid hex digits
3. **NUL rejection**: Detects both literal NUL and `#00` escape, emits diagnostic, truncates at NUL
4. **Length limit**: 127 raw bytes (before hex expansion), truncates cleanly before incomplete `#XX` sequences
5. **Termination**: Stops at whitespace or any PDF delimiter
6. **Empty name**: `/` followed by delimiter/EOF produces `Token::Name(b"")` with no diagnostic

## Test Results

```
running 23 tests
test parser::lexer::tests::name_case_sensitive ... ok
test parser::lexer::tests::name_hex_escape_decodes_to_hash ... ok
test parser::lexer::tests::name_empty ... ok
test parser::lexer::tests::name_hex_escape_zero_zero_is_nul ... ok
test parser::lexer::tests::name_invalid_hex_escape_keeps_hash_literal ... ok
test parser::lexer::tests::name_invalid_hex_escape_single_digit ... ok
test parser::lexer::tests::name_empty_followed_by_delimiter ... ok
test parser::lexer::tests::name_length_limit_127_bytes ... ok
test parser::lexer::tests::name_length_limit_exact_127_bytes_valid ... ok
test parser::lexer::tests::name_hex_escape_mixed_case ... ok
test parser::lexer::tests::name_simple ... ok
test parser::lexer::tests::name_nul_byte_rejected ... ok
test parser::lexer::tests::name_length_limit_counts_raw_bytes_before_expansion ... ok
test parser::lexer::tests::name_multiple_invalid_hex_escapes ... ok
test parser::lexer::tests::name_literal_nul_byte_rejected ... ok
test parser::lexer::tests::name_truncation_before_incomplete_escape ... ok
test parser::lexer::tests::name_with_bytes_preserved ... ok
test parser::lexer::tests::name_with_all_delimiters ... ok
test parser::lexer::tests::name_with_hex_escape_space ... ok
test parser::lexer::tests::name_with_slash_delimiter ... ok
test parser::lexer::tests::name_zero_byte_not_confused_with_nul ... ok
test parser::lexer::tests::name_proptest_never_panics_on_random_bytes ... ok
test parser::lexer::tests::name_proptest_always_produces_valid_token ... ok

test result: ok. 23 passed; 0 failed; 0 ignored
```

Full lexer suite: 77 tests passed.

## Files Modified

No changes required - implementation was already complete.
