# Contributing to pdftract

Thank you for your interest in contributing to pdftract! This document covers the essential workflows for contributors.

## Lockfile Policy

pdftract uses a workspace-level `Cargo.lock` file that is **checked into version control**. This is intentional: release reproducibility requires that every build from the same commit produces byte-identical artifacts. All CI steps run with `--locked --frozen` to enforce this.

### Updating Dependencies

When adding or updating dependencies:

1. **Targeted updates (preferred):** Update a specific crate and its dependencies:
   ```bash
   cargo update -p crate-name
   ```

2. **Full updates:** Only during release preparation:
   ```bash
   cargo update
   ```

3. **Commit the lockfile:** Always commit `Cargo.lock` alongside any `Cargo.toml` changes:
   ```bash
   git add Cargo.toml Cargo.lock
   git commit -m "deps: upgrade crate-name to X.Y.Z"
   ```

### CI Enforcement

- The `pdftract-ci` Argo workflow runs `cargo check --locked --frozen` as the first step.
- A PR that edits `Cargo.toml` without updating `Cargo.lock` will fail CI.
- Two consecutive builds of `pdftract-build-binaries` against the same tag must produce identical binaries (verified by SHA256 comparison).

### Why Library Crates Have Cargo.lock

The Rust ecosystem convention is that library crates should not check in `Cargo.lock`, allowing downstream consumers to resolve their own dependency versions. pdftract departs from this convention because:

- **Release reproducibility** is paramount for SLSA Level 3 provenance.
- The workspace produces both libraries (`pdftract-core`) and binaries (`pdftract-cli`, `pdftract-py`).
- A single workspace-level `Cargo.lock` applies to all members.
- Downstream consumers can still ignore the lockfile by using `cargo build --frozen` with their own lockfile, or by vendoring.

## Development Workflow

### Building

```bash
cargo build --release
```

### Testing

```bash
cargo test --all
```

### Linting

```bash
cargo clippy --all-targets --all-features
cargo fmt --check
```

## Security

This project uses `cargo-audit` and `cargo-deny` for supply-chain security. New direct dependencies require an ADR or written justification in the PR description.
