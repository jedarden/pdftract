# pdftract-2bpzs: OutputOptions struct + block-kind filter

## Summary

The `OutputOptions` struct with block-kind filtering and CLI flags was already implemented in the codebase. This note verifies the implementation meets all acceptance criteria.

## Implementation Location

- **Struct definition**: `/home/coding/pdftract/crates/pdftract-core/src/options.rs` (lines 69-185)
- **CLI integration**: `/home/coding/pdftract/crates/pdftract-cli/src/main.rs` (lines 149-171 for flags, 755-760 for wiring)

## Acceptance Criteria Status

| Criteria | Status | Evidence |
|----------|--------|----------|
| Default OutputOptions: all include_* false | PASS | `test_output_options_default` passes |
| --include-headers-footers sets both true | PASS | `test_output_options_include_headers_and_footers` passes |
| --include-invisible-text: include_invisible true | PASS | CLI flag wired correctly (line 758) |
| Filter rejects Header when include_headers false | PASS | `test_output_options_include_block_kind` passes |
| Filter accepts Header when include_headers true | PASS | `test_output_options_include_block_kind_with_flags` passes |
| Watermark filtering ready for Phase 7 | PASS | `include_block_kind("watermark")` implemented |

## Struct Definition

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct OutputOptions {
    pub include_headers: bool,
    pub include_footers: bool,
    pub include_watermarks: bool,
    pub include_invisible: bool,
    pub include_hidden_layers: bool,
    pub include_struct_metadata: bool,
}
```

## CLI Flags

- `--include-headers` - Include header blocks
- `--include-footers` - Include footer blocks
- `--include-headers-footers` - Include both headers and footers
- `--include-invisible-text` - Include invisible text spans (rendering_mode == 3)
- `--include-hidden-layers` - Include hidden-layer text spans (OCG-controlled)
- `--include-watermarks` - Include watermark blocks (no-op until Phase 7)

## Test Results

All 8 OutputOptions tests pass:

```
test options::tests::test_output_options_default ... ok
test options::tests::test_output_options_deserialize ... ok
test options::tests::test_output_options_include_block_kind ... ok
test options::tests::test_output_options_include_block_kind_with_flags ... ok
test options::tests::test_output_options_include_headers_and_footers ... ok
test options::tests::test_output_options_include_span ... ok
test options::tests::test_output_options_include_span_with_flags ... ok
test options::tests::test_output_options_serialize ... ok
```

## Conclusion

The implementation is complete and all acceptance criteria pass. The feature is ready for use.
