# pdftract-39g4j: --receipts CLI flag + ExtractionOptions.receipts threading

## Summary

Implemented the `--receipts` CLI flag with clap `value_parser` for runtime validation of allowed values ("off", "lite", "svg"). Verified that the MCP tools args already have the `receipts` field properly defined and the schema validation passes.

## Changes Made

### CLI (`crates/pdftract-cli/src/main.rs`)
- Added `value_parser = ["off", "lite", "svg"]` to the `--receipts` flag (line 84)
- This makes clap validate the receipts mode at parse time with a helpful error message

### Already in Place (no changes needed)
- `ReceiptsMode` enum in `crates/pdftract-core/src/options.rs` (with `from_str()` and `as_str()` methods)
- `ExtractionOptions` struct with `receipts: ReceiptsMode` field
- `Receipt` struct with `lite()` and `with_svg()` constructors in `crates/pdftract-core/src/receipts/mod.rs`
- `SpanJson` and `BlockJson` with optional `receipt` field in `crates/pdftract-core/src/schema/mod.rs`
- MCP tools args with `receipts: Option<String>` field in `crates/pdftract-cli/src/mcp/tools/args.rs`

## Acceptance Criteria Status

### PASS
- **pdftract extract --receipts=bogus file.pdf** → CLI parse error from clap value_parser: "error: invalid value 'bogus' for '--receipts <MODE>' [possible values: off, lite, svg]"
- **CLI help shows proper values**: `--receipts <MODE> Receipt mode: off (default), lite, or svg [default: off] [possible values: off, lite, svg]`
- **ExtractionOptions struct serializes the receipts field** (already implemented in options.rs with serde derive)
- **MCP tools args have receipts field** (ExtractArgs, ExtractTextArgs, ExtractMarkdownArgs all include `receipts: Option<String>`)
- **Schema validation tests pass** (test_extract_tool_schema, test_registry_has_all_tools)

### WARN (pending full extraction implementation)
- **pdftract extract --receipts=lite file.pdf → JSON output's spans have non-null receipt fields** - CLI accepts the flag, but full extraction is stubbed (TODO in cmd_extract: line 296)
- **pdftract extract --receipts=svg file.pdf → JSON output's spans have receipt fields including svg_clip** - Same as above, pending extraction implementation
- **Block-level receipts** - Pending extraction implementation
- **Performance criterion (<=10% overhead for lite, <=25% for svg)** - Pending benchmark implementation with actual extraction

### NOTE
The actual threading of `ExtractionOptions` through the extraction pipeline and the integration of receipt generation in span/block builders is deferred to the extraction implementation beads (Phase 6). This bead focused on the CLI/MCP entry points, which are now properly wired.

## Files Modified
- `crates/pdftract-cli/src/main.rs`: Added `value_parser = ["off", "lite", "svg"]` to --receipts flag

## Files Verified (no changes needed)
- `crates/pdftract-core/src/options.rs`: ReceiptsMode enum and ExtractionOptions struct
- `crates/pdftract-core/src/receipts/mod.rs`: Receipt struct with constructors
- `crates/pdftract-core/src/schema/mod.rs`: SpanJson and BlockJson with receipt field
- `crates/pdftract-cli/src/mcp/tools/args.rs`: MCP tools args with receipts field

## Testing

```bash
# CLI validation works
./target/release/pdftract extract --receipts=bogus /dev/null
# error: invalid value 'bogus' for '--receipts <MODE>'
#   [possible values: off, lite, svg]

# CLI help shows proper values
./target/release/pdftract extract --help | grep receipts
# --receipts <MODE>      Receipt mode: off (default), lite, or svg [default: off] [possible values: off, lite, svg]

# MCP schema tests pass
cargo test -p pdftract-cli test_extract_tool_schema --lib
cargo test -p pdftract-cli test_registry_has_all_tools --lib
```
