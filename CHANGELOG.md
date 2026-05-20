# Changelog

All notable changes to pdftract will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- MSRV gate in CI: `msrv-check` step builds with `rust:1.78-slim` to detect new-Rust feature usage

## Minimum Supported Rust Version (MSRV) Policy

The **Minimum Supported Rust Version (MSRV)** for pdftract is **1.78**.

### Bumping the MSRV

Bumping the MSRV is a **MINOR version event** with the following requirements:

1. **At least one release of warning**: The changelog must announce the planned MSRV bump at least one minor release before it takes effect. For example:
   - `v0.2.0`: "MSRV will bump to 1.79 in v0.3.0"
   - `v0.3.0`: MSRV actually bumps to 1.79

2. **Never in a PATCH release**: MSRV bumps must not occur in patch releases (e.g., `v0.2.0` -> `v0.2.1`), as downstream consumers expect patch versions to remain compatible with their existing toolchain.

3. **Update all locations** when bumping:
   - Root `Cargo.toml`: `[workspace.package] rust-version`
   - CI workflow: `rust:` image tag in the `msrv-check` step
   - README: MSRV badge
   - `clippy.toml`: `msrv` setting
   - This CHANGELOG.md: entry announcing the bump

4. **CI enforcement**: The `msrv-check` step in `pdftract-ci` will fail if any source file requires a newer Rust version than declared. This prevents silent MSRV drift.

### Why MSRV Matters

- Downstream consumers (library users, binary redistributors) may be stuck on older Rust versions due to platform constraints, distribution policies, or transitive dependency requirements.
- Silent MSRV drift (e.g., using `let-else`, `core::error::Error`, or async-fn-in-trait) breaks these consumers without warning.
- The MSRV gate makes Rust-version drift a code-review-time conversation, not a post-release surprise.

### Current MSRV: 1.78

Declared in `Cargo.toml` via `rust-version = "1.78"` under `[workspace.package]`. Both `pdftract-core` and `pdftract-cli` inherit this value.
