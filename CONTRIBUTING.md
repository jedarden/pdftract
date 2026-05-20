# Contributing to pdftract

Thank you for your interest in contributing to pdftract! This document covers the essential workflows for contributors.

## Minimum Supported Rust Version (MSRV)

The **Minimum Supported Rust Version (MSRV)** for pdftract is **1.78**. This is the oldest Rust version that can successfully build the project. The MSRV is declared in `Cargo.toml` via the `rust-version` field and enforced in CI.

### MSRV Policy

- **MSRV is 1.78** for the public crates (`pdftract-core`, `pdftract-cli`)
- **Bumping MSRV is a MINOR version event** — it requires at least one release of warning in the changelog
- **Never bump MSRV in a PATCH release** — this breaks downstream consumers without notice
- **CI enforces MSRV** — the `msrv-check` step builds with `rust:1.78-slim` and fails if newer Rust features are used

### When bumping MSRV

If you need to use a Rust feature newer than 1.78:

1. **Open an issue or ADR** documenting the required feature and why it's necessary
2. **Update all locations**:
   - Root `Cargo.toml`: `[workspace.package] rust-version`
   - CI workflow: `rust:` image tag in the `msrv-check` step
   - README: MSRV badge
   - `clippy.toml`: `msrv` setting
3. **Add a CHANGELOG entry** announcing the bump with at least one release of warning
4. **Wait for the next MINOR release** — never include in a PATCH

### Code review guidelines

- **New dependencies** whose declared MSRV exceeds 1.78 are rejected at code-review time
- The `msrv-check` CI step catches most MSRV violations automatically
- Reviewers should verify that new code doesn't use Rust 1.79+ features (e.g., `core::error::Error` in stable, `let-else`, certain async-fn-in-trait features)

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

### Responsible Disclosure

If you discover a security vulnerability, please do **NOT** open a public issue or pull request. Instead, report it privately:

1. **Email (preferred):** [security@jedarden.com](mailto:security@jedarden.com)
   - PGP-encrypted emails are strongly encouraged
   - PGP key: [`docs/security/pgp-public-key.asc`](docs/security/pgp-public-key.asc)

2. **GitHub Private Vulnerability Reporting:**
   - Use the [Security tab](https://github.com/jedarden/pdftract/security/advisories)

See [`SECURITY.md`](SECURITY.md) for our full disclosure policy, including:
- Supported versions and security fix timeline
- 90-day disclosure window
- CVE assignment process
- Safe harbor for good-faith researchers

### Supply-Chain Security

This project uses `cargo-audit` and `cargo-deny` for supply-chain security. New direct dependencies require an ADR or written justification in the PR description.
