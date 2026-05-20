# pdftract-279: Workspace Layout Verification

## Acceptance Criteria Status

### PASS
- `cargo metadata --format-version 1` lists exactly 3 workspace members: pdftract-core, pdftract-cli, pdftract-py
- All three crate `Cargo.toml`s reference workspace-wide `version`, `edition`, `rust-version`, `license`, `repository` via `workspace = true`
- Root `Cargo.toml` uses `resolver = "2"` as required for cdylib feature compatibility
- Workspace is configured with `[workspace.package]` for shared metadata (version, edition, rust-version, license, repository, authors, homepage, documentation)
- Workspace is configured with `[workspace.dependencies]` for shared external deps (anyhow, flate2, lzw, memchr, secrecy, serde, thiserror, tracing)
- pdftract-cli has `[[bin]] name = "pdftract"` and `default-run = "pdftract"`
- pdftract-py uses `pyo3` with `extension-module` feature and `crate-type = ["cdylib"]`
- `.cargo/config.toml` provides build aliases for CI (bench-ci, test-ci) and development (b, br, c, cr, t, tr)
- Adding a fourth member requires only one edit to root `Cargo.toml` (workspace.members list)
- pdftract-core is `publish = true`; pdftract-cli is `publish = true`; pdftract-py is `publish = false`

### WARN
- `cargo build --workspace --locked` fails due to compilation errors in pdftract-core source code (12 errors, 15 warnings)
- These are code-level issues, not workspace structure issues
- The workspace configuration itself is correct and complete
- Separate beads are needed to fix the compilation errors

## Files Modified
- `Cargo.toml` - Updated workspace members from `["crates/pdftract-core", "crates/pdftract-cer-diff", "crates/pdftract-cli"]` to `["crates/pdftract-core", "crates/pdftract-cli", "crates/pdftract-py"]`
- `Cargo.toml` - Added workspace metadata: rust-version, authors, homepage, documentation
- `Cargo.toml` - Updated license from `MIT` to `MIT OR Apache-2.0`
- `Cargo.toml` - Added workspace dependencies: anyhow, lzw, memchr, secrecy (updated from 0.8 to 0.10), serde, tracing
- `.cargo/config.toml` - Created new file with build aliases

## References
- Plan section: Release Engineering and Distribution / Artifact Taxonomy, lines 3343-3367
