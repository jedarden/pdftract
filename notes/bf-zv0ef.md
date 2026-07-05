# bf-zv0ef: Commit CLI changes and build artifacts (Cargo.lock, build/agl.json)

## Investigation Summary

Bead scope: Commit changes to crates/pdftract-cli/src, Cargo.lock, and build/agl.json.

## Findings

Upon investigation, no uncommitted changes were found for the files specified in the bead scope:

1. **pdftract-cli/src**: No uncommitted changes detected
2. **Cargo.lock**: No uncommitted changes detected  
3. **build/agl.json**: File not found at expected location (found at `crates/pdftract-core/build/agl.json` instead)

The working tree showed only `.needle-predispatch-sha` modified (a metadata file).

## Issue Found and Fixed

While running `cargo check --all-targets` (acceptance criterion 1), a compilation error was discovered:

```
error[E0432]: unresolved import `pdftract_core::cmap::tokenize_cjk_bytes`
  --> crates/pdftract-core/benches/cmap_tokenize.rs:6:27
   |
6  | use pdftract_core::cmap::{tokenize_cjk_bytes, CodespaceRange, CodespaceRanges};
   |                           ^^^^^^^^^^^^^^^^^^ no `tokenize_cjk_bytes` in `cmap`
note: found an item that was configured out
  --> crates/pdftract-core/src/cmap/mod.rs:14:19
   |
13 | #[cfg(feature = "cjk")]
   |       --------------- the item is gated behind the `cjk` feature
14 | pub use tokenize::tokenize_cjk_bytes;
   |                   ^^^^^^^^^^^^^^^^^^
```

**Root cause**: The `cmap_tokenize` benchmark uses CJK tokenization functions that are behind the `cjk` feature flag, but the benchmark target configuration did not specify this requirement.

**Fix applied**: Added `required-features = ["cjk"]` to the `cmap_tokenize` benchmark configuration in `crates/pdftract-core/Cargo.toml`:

```toml
[[bench]]
name = "cmap_tokenize"
harness = false
required-features = ["cjk"]
```

## Verification

**PASS**:
- `cargo check --all-targets` passes with no errors after fix
- Benchmark fix committed with conventional commit format
- All commits pushed to origin main (forgejo at git.ardenone.com)

**WARN**:
- No uncommitted CLI changes found to commit (bead scope files already committed)
- build/agl.json location discrepancy (expected at `build/agl.json`, found at `crates/pdftract-core/build/agl.json`)

## Commit Details

**Commit**: `fix(bf-zv0ef): add required-features to cmap_tokenize benchmark`

File changed: `crates/pdftract-core/Cargo.toml` (1 insertion)

SHA: `8b3aa8e1`

## Conclusion

The CLI changes and build artifacts mentioned in the bead scope were already committed in the 150 commits ahead of github/main. The bead acceptance criteria were met by:
1. Fixing and verifying `cargo check --all-targets` passes
2. Committing the benchmark fix with proper rationale
3. Pushing all commits to forgejo main (origin main)

The bead is complete with the benchmark fix as the substantive work product.
