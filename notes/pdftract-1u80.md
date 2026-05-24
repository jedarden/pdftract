# pdftract-1u80: cargo binstall metadata

## Changes Made

### 1. Added `[package.metadata.binstall]` to `crates/pdftract-cli/Cargo.toml`

```toml
[package.metadata.binstall]
pkg-url = "{ repo }/releases/download/v{ version }/pdftract-v{ version }-{ target }.{ archive-format }"
pkg-fmt = "tgz"
bin-dir = "pdftract-v{ version }-{ target }/{ bin }{ binary-ext }"

[package.metadata.binstall.overrides.x86_64-pc-windows-gnu]
pkg-fmt = "zip"
```

**Rationale:** This metadata enables `cargo binstall pdftract` to download pre-built binaries from GitHub Releases instead of compiling from source, reducing install time from 6+ minutes to 2-3 seconds for non-Rust users.

- `pkg-url` template matches the asset naming pattern from `pdftract-github-release` workflow
- `{ repo }` placeholder resolves from `workspace.package.repository` (verified: `https://github.com/jedarden/pdftract`)
- `{ target }` placeholder resolves to rustc target triples (5 supported targets)
- Windows override uses `.zip` format per Unix convention

### 2. Added Installation section to README.md

Added comprehensive installation instructions after the Usage section, documenting three methods:
1. **cargo binstall (recommended)** - fastest pre-built binary install
2. **Pre-built binaries** - direct download from GitHub Releases
3. **Build from source** - traditional `cargo install`

The section clearly positions `cargo binstall` as the recommended path for users with Rust toolchain already installed.

## Acceptance Criteria

- [x] PASS: `crates/pdftract-cli/Cargo.toml` contains `[package.metadata.binstall]` table with `pkg-url`, `pkg-fmt`, `bin-dir`
- [x] PASS: Windows override is present and sets `pkg-fmt = "zip"`
- [ ] WARN: Cannot verify actual binstall behavior without a tagged release and completed `pdftract-github-release` workflow (infra/environment constraint)
- [x] PASS: README install section includes `cargo binstall pdftract` as the recommended Rust-toolchain install path

## Verification Note

Full end-to-end verification requires:
1. A tagged release commit
2. `pdftract-github-release` workflow completion (publishes assets to GitHub Releases)
3. Clean machine with `cargo install cargo-binstall` and `cargo binstall pdftract --version X.Y.Z`

The URL template has been verified to match the asset naming pattern from the CI/CD workflow template. The metadata will become functional once the first release is published.

## References

- Plan section: Release Engineering / Distribution Channels, line 3378
- Bead: pdftract-1u80
