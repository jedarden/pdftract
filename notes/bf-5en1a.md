# bf-5en1a: Lower max_decompress_bytes default to 512MB and propagate

## Work Completed

This bead's changes were implemented in commit `e94f2ab` (fix(bf-49wmw): fix PNG-predictor unbounded pre-allocation).

### Changes Made

1. **Core constant** (`crates/pdftract-core/src/parser/stream.rs:35`):
   - Changed `DEFAULT_MAX_DECOMPRESS_BYTES` from `2 * 1024_u64.pow(3)` (2 GiB) to `512 * 1024_u64.pow(2)` (512 MiB)
   - Updated documentation comment

2. **ExtractionOptions default** (`crates/pdftract-core/src/parser/stream.rs:1021`):
   - Already uses `DEFAULT_MAX_DECOMPRESS_BYTES`, no change needed

3. **CLI** (`crates/pdftract-cli/src/main.rs`):
   - Uses `ExtractionOptions::default()`, inherits the 512 MiB limit
   - No hardcoded values to change

4. **Python bindings** (`crates/pdftract-py/src/lib.rs`):
   - Stub implementation, no `max_decompress_bytes` exposure yet

5. **MCP server** (`crates/pdftract-cli/src/mcp/server.rs`):
   - Stub implementation, no service yet

6. **test_bomb_limit_flate** (`crates/pdftract-core/src/parser/stream.rs:966`):
   - Uses custom limit of 3 bytes for testing
   - No change needed - test verifies bomb limit behavior, not the specific default value

## Acceptance Criteria

- [x] PASS: `DEFAULT_MAX_DECOMPRESS_BYTES` is 512 MiB
- [x] PASS: `ExtractionOptions::default()` uses the constant
- [x] PASS: CLI inherits the default
- [x] PASS: Tests pass (`test_bomb_limit_flate`, `test_extraction_options_default`)
- [x] WARN: Python bindings are stub (no exposure yet)
- [x] WARN: MCP server is stub (no service yet)

## Verification

```bash
# Verify constant value
grep "DEFAULT_MAX_DECOMPRESS_BYTES" crates/pdftract-core/src/parser/stream.rs
# Output: pub const DEFAULT_MAX_DECOMPRESS_BYTES: u64 = 512 * 1024_u64.pow(2);

# Verify ExtractionOptions default
cargo test test_extraction_options_default --lib
# Output: test result: ok. 1 passed

# Verify bomb limit test
cargo test test_bomb_limit_flate --lib
# Output: test result: ok. 1 passed
```

## References

- Plan: `/home/coding/pdftract/docs/plan/plan.md` line 75 (512 MB default)
- Research doc: `docs/research/adversarial-inputs-and-parser-security.md`
- Implementation commit: `e94f2ab` (fix(bf-49wmw))
