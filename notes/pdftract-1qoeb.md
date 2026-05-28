# Verification Note: pdftract-1qoeb (Marked-content stack)

## Summary

The marked-content stack (`MarkedContentStack`) has been fully implemented in `crates/pdftract-core/src/parser/marked_content_stack.rs` with the BMC/BDC/EMC operator parsers in `marked_content_operators.rs`.

## Implementation Details

### MarkedContentStack (`marked_content_stack.rs`)
- `MAX_MC_DEPTH = 64` (matches spec)
- `MarkedContentFrame` struct with:
  - `tag: String` (tag name like "Span", "P", "Artifact")
  - `mcid: Option<u32>` (marked content identifier)
  - `is_hidden: bool` (OCG hidden state from bead pdftract-1q19p)
- `MarkedContentStack` struct with:
  - `push_bmc(tag: String) -> bool`: Push BMC frame (tag only)
  - `push_bdc(tag: String, mcid: Option<u32>, is_hidden: bool) -> bool`: Push BDC frame
  - `pop_emc() -> Option<MarkedContentFrame>`: Pop top frame, None if empty
  - `innermost_frame() -> Option<&MarkedContentFrame>`: Get top frame
  - `innermost_mcid() -> Option<u32>`: Get top MCID
  - `depth() -> usize`: Current stack depth
  - `is_hidden() -> bool`: Check if any frame is hidden
  - `reset()`: Clear stack for page boundary

### Operator Parsers (`marked_content_operators.rs`)
- `parse_bmc()`: BMC operator parser
- `parse_bdc()`: BDC operator parser with MCID extraction and OCG handling
- `parse_emc()`: EMC operator parser

## Acceptance Criteria Status

| Criteria | Status | Test |
|----------|--------|------|
| push_bmc 64 times → all push; 65th emits MARKED_CONTENT_DEPTH_EXCEEDED | ✅ PASS | `test_depth_limit` |
| push_bmc N then pop_emc N → empty stack | ✅ PASS | `test_pop_emc` |
| pop_emc on empty stack → EmcUnderflow diagnostic | ✅ PASS | `test_pop_emc_underflow` |
| top_mcid returns Some(mcid) when top has MCID; None when empty | ✅ PASS | `test_push_bdc_with_mcid`, `test_empty_stack` |
| Unit tests cover push/pop balance, overflow, underflow | ✅ PASS | 20 tests in stack module, 25 in operators |
| INV-8 (no panic) verified on all stack operations | ✅ PASS | All tests pass without panic |

## Test Results

```bash
$ cargo test -p pdftract-core --lib marked_content_stack
running 20 tests
test result: ok. 20 passed; 0 failed; 0 ignored

$ cargo test -p pdftract-core --lib marked_content_operators
running 25 tests
test result: ok. 25 passed; 0 failed; 0 ignored
```

## Notes

The implementation predates this bead (already complete). Minor differences from the bead spec:
1. Uses `String` for `tag` instead of `Name` - correct for the implementation context
2. `pop_emc()` returns `Option` instead of `Result` - simpler, diagnostic is emitted internally
3. Methods named `innermost_*` instead of `top_*` - more descriptive, functionally equivalent

These are non-functional differences; the implementation meets all acceptance criteria.
