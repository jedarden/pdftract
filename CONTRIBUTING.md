# Contributing to pdftract

Thank you for your interest in contributing to pdftract! This document covers the essential workflows for contributors.

## Licensing and Sign-off

pdftract is dual-licensed under **MIT OR Apache-2.0**. You may choose either license for your use.

### Developer Certificate of Origin (DCO)

This project requires a **Developer Certificate of Origin (DCO)** sign-off on all commits. This certifies that you wrote the code or have the right to pass it on as open-source.

**To sign your commits, use `git commit --signoff` (or `git commit -s`):**

```bash
git commit -s -m "feat: add some feature"
# The "Signed-off-by" trailer is added automatically
```

**No CLA is required.** The DCO is sufficient for this permissive-license project.

### Apache NOTICE File

The Apache-2.0 license includes a NOTICE file requirement, but pdftract does not ship a NOTICE file in the source distribution. This is intentional: the project maintains no contributor list outside of git history, and there are no third-party attribution notices required.

**Downstream redistributors MAY add a NOTICE file** when distributing pdftract as part of their own product. If you choose to add one, it should include:
- Attribution to the pdftract project
- A link to the original source repository
- Any modifications you made (if distributing a modified version)

The absence of a NOTICE file in the upstream distribution does not violate the Apache-2.0 license; the NOTICE requirement applies only when there is something to notice.

## Code of Conduct

This project adopts the [Contributor Covenant v2.1](CODE_OF_CONDUCT.md). All contributors are expected to uphold this code of conduct.

## Reporting Security Issues

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

## Development Setup

### Prerequisites

- **Rust 1.78 or later** — See [Minimum Supported Rust Version (MSRV)](#minimum-supported-rust-version-msrv) below
- **Git** — For cloning and committing

### OCR Feature Dependencies (Optional)

If you're developing OCR-related features (Phase 5), you'll need additional dependencies:

**Linux (Debian/Ubuntu):**
```bash
sudo apt-get install libleptonica-dev libtesseract-dev tesseract-ocr-eng
```

**macOS:**
```bash
brew install tesseract leptonica
```

**Windows:**
- Install Tesseract from the official installers: https://github.com/UB-Mannheim/tesseract/wiki

### Building

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/pdftract.git
cd pdftract

# Build the workspace
cargo build --workspace --locked

# Build release binaries
cargo build --release --workspace
```

### Testing

```bash
# Run all tests
cargo test --workspace --features default

# Run tests with output
cargo test --workspace --features default -- --nocapture

# Run a specific test
cargo test --workspace --features default test_name
```

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

## Local Validation Before Opening a PR

Before submitting a pull request, please run the following commands locally to ensure your changes pass all quality gates:

```bash
# 1. Run all tests (must be all green)
cargo test --workspace --features default

# 2. Lint with clippy (no warnings allowed)
cargo clippy --all-targets --features default -- -D warnings

# 3. Check binary size (must be within budget; target <= 4 MB stripped)
cargo bloat --release --features default

# 4. Check for security advisories (no medium+ issues)
cargo audit

# 5. Check license compliance (no rejected licenses)
cargo deny check licenses

# 6. Check code formatting
cargo fmt --check
```

**Why these checks?** These exact commands are run in the `pdftract-ci` Argo workflow. A green local run predicts a green CI run, reducing review iteration cycles.

### Binary Size Budget

The release binary must be <= 4 MB when stripped. `cargo bloat` helps identify functions contributing most to binary size. If your PR adds significant code:
- Run `cargo bloat --release --features default --crates pdftract-cli`
- Check the top functions in the output
- Consider if large dependencies can be made optional or feature-gated

## CI on Forks — The Argo-CI Caveat

> **IMPORTANT:** Because CI runs on the private `iad-ci` cluster, external contributors cannot trigger CI from their fork.

### How It Works

1. **Fork and open a pull request** against `jedarden/pdftract:main`
2. **A maintainer will trigger the `pdftract-ci` Argo workflow** against your branch
3. **Results are posted as a PR comment** once the workflow completes

### Expected Response Time

- Maintainer-triggered CI: **within 48 hours**
- You'll receive a comment on your PR with the full CI log

### Why This Model?

The `iad-ci` cluster is a private Rackspace Spot cluster accessed via kubectl-proxy over Tailscale. External forks do not have credentials to access this cluster, so they cannot self-serve CI runs. This is unusual, but it allows us to run CI on infrastructure we control without exposing cluster credentials publicly.

### Local Validation is Critical

Since you cannot trigger CI yourself, **please run the full local validation checklist** before opening your PR. This minimizes back-and-forth cycles when the maintainer-triggered CI fails.

## Pull Request Template

All pull requests must follow the [PR template](.github/PULL_REQUEST_TEMPLATE.md). The template requires:

- **Linked issue or RFC** — Every PR should reference an issue or design document
- **Scope statement** — Which Phase / which Acceptance Scenario does this address?
- **Test plan** — How did you verify this works?
- **Manual-test evidence** — Screenshots, terminal output, or example runs
- **Performance impact** — If hot-path code was touched, include benchmark results

## Commit Message Style

This project uses **Conventional Commits** for commit messages. Release notes are auto-generated from commit history using `git-cliff`.

### Format

```
<type>(<scope>): <short summary>

[optional body]

[optional footer]
```

### Types

- `feat:` — A new feature
- `fix:` — A bug fix
- `perf:` — A performance improvement
- `docs:` — Documentation changes
- `chore:` — Maintenance tasks (updates, refactoring, tooling)
- `test:` — Test changes
- `BREAKING CHANGE:` — A breaking change (include in body or footer)

### Examples

```bash
feat(ocr): add Tesseract integration for phase 5
fix(font): handle missing /Widths in Type 3 fonts
perf(extract): cache page tree parsing results
docs(contributing): add Argo-CI caveat section
chore(deps): upgrade lodepng to 0.9.0
```

## Issue Triage

We use issue templates to ensure all necessary information is provided upfront. When opening an issue, please use the appropriate template:

- **Bug report** — Must include `pdftract doctor` output
- **Feature request** — Describe the use case and proposed API
- **Performance regression** — Include before/after benchmarks
- **Security advisory** — Redirects to private disclosure (see [Reporting Security Issues](#reporting-security-issues))

See [`.github/ISSUE_TEMPLATE/`](.github/ISSUE_TEMPLATE/) for the full list.

## Getting Help

- **Documentation:** Check [`docs/`](docs/) for design docs and ADRs
- **Issues:** Search existing issues before opening a new one
- **Discussions:** Use GitHub Discussions for questions and RFCs
- **Security:** See [SECURITY.md](SECURITY.md) for vulnerability reporting

Thank you for contributing to pdftract!
