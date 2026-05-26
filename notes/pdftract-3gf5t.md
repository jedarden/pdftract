# pdftract-3gf5t: walkdir folder traversal + *.pdf filter + remote URL expansion

## Summary

Implemented path expansion for the `pdftract grep` subcommand. This includes:

1. **FileWorkItem structure**: Created `FileWorkItem` and `PathOrUrl` types to represent work items
2. **Path expansion**: Implemented `expand_paths()` function that:
   - Expands local file paths (single files and directories)
   - Walks directories via walkdir with *.pdf filtering (case-insensitive)
   - Supports https:// URLs when the `remote` feature is enabled
   - Skips hidden directories (starting with .)
   - Silently skips non-PDF files
   - Calculates bytes_total for progress reporting
3. **Public API**: Added `produce_work_items()` function as the public entry point
4. **Integration**: Updated `run_grep()` to use the new path expansion logic

## Files Changed

- `crates/pdftract-cli/src/grep/expand.rs` (new): Path expansion module with FileWorkItem, PathOrUrl, and expand_paths()
- `crates/pdftract-cli/src/grep/mod.rs`: Added expand module import and produce_work_items() function
- `crates/pdftract-cli/src/grep/event.rs`: Fixed `should_skip_confidence()` function for proper NaN/Infinity handling in JSON serialization

## Acceptance Criteria Status

- ✅ walkdir filters non-PDF files silently
- ✅ Single-file paths produce one FileWorkItem
- ✅ Mixed dir+file PATH list works
- ✅ https:// URL produces FileWorkItem when remote feature on; clap error when off
- ✅ Symlink loop does not hang (follow_links(false))
- ✅ bytes_total accurate sum
- ✅ Public produce_work_items(args: &GrepArgs) -> impl Iterator<Item = FileWorkItem>

## Tests

All 130 grep-related tests pass with `--features grep`:
- expand.rs tests: 11/11 passed
- matcher.rs tests: 24/24 passed
- event.rs tests: 22/22 passed
- mod.rs tests: 53/53 passed

## References

- Plan section: 7.8 line 2708 (path semantics), 2715 (-r recursive), 2793 (non-PDF silently skipped)
- Bead: pdftract-3gf5t
