# Installation

pdftract has one supported installation channel today: build from this
repository. No crates.io package, PyPI wheel, release archive, container
image, Homebrew formula, or hosted documentation site is published yet.

## Build from source

The workspace does not publish a root package, so target the CLI crate:

```bash
git clone https://github.com/jedarden/pdftract.git
cd pdftract
cargo install --locked --path crates/pdftract-cli
```

This installs the `pdftract` binary in `~/.cargo/bin/`. Make sure that
directory is in your `PATH`.

For a local container build, use the checked-in Dockerfile. `FEATURES` is a
Docker variant selector (`default` or `full`), not a Cargo feature called
`full`:

```bash
docker build --pull --build-arg FEATURES=default --tag pdftract:local .
docker run --rm \
  --mount "type=bind,src=$PWD/document.pdf,dst=/work/document.pdf,readonly" \
  pdftract:local extract /work/document.pdf --json -
```

The automated local/CI contract is
[`scripts/verify-docker.sh`](../../../scripts/verify-docker.sh). It builds the
default and full variants, runs `pdftract --version`, prints compiled feature
contents, and extracts the canonical W3C fixture.

## Planned channels — not published

These channels are release targets, not installation instructions. Wait for a
versioned release and an updated verification record before using them:

| Channel | Planned location | Current status |
|---|---|---|
| Rust library and CLI | crates.io | Not published |
| Python bindings and wheels | PyPI | Not published |
| Pre-built binaries | GitHub Releases | No release exists |
| Homebrew | `jedarden/homebrew-tap` | No formula exists |
| Container images | `ghcr.io/jedarden/pdftract` | Not published |
| Hosted user guide | `pdftract.com` | Not a supported channel |

The release workflow must publish and independently verify immutable versioned
references before planned commands are documented as usable.

## Platform Support

### Supported Platforms

| Platform | CI Status | Notes |
|---|---|---|
| Linux `x86_64` (glibc) | Fully CI-tested | Primary development platform |
| Linux `x86_64` (musl) | Fully CI-tested | Alpine-compatible |
| Linux `arm64` (glibc) | Fully CI-tested | ARM64 servers (e.g., Graviton) |
| Linux `arm64` (musl) | Fully CI-tested | Alpine ARM64 |
| macOS `x86_64` | Build-tested | See caveat below |
| macOS `arm64` | Build-tested | See caveat below |
| Windows `x86_64` | Build-tested | See caveat below |

### Cross-Platform Test Limitation (KU-12)

> **Linux is fully CI-tested; macOS and Windows are build-tested and manually smoke-tested per release.**

Per project architecture decision ADR-009, the CI pipeline runs on Linux-only infrastructure (`iad-ci`). macOS and Windows binaries are **built** via cross-compilation but are never **executed** in automated CI. This is acknowledged as Known Unknown KU-12 with the following mitigation:

- A manual smoke-test runbook is executed by the release lead before each milestone against at least one physical macOS machine and one Windows VM
- User bug reports for platform-specific issues are acknowledged within 48 hours and addressed in the next patch release
- No claim of "tested on macOS/Windows" appears in CI status badges

If you encounter a platform-specific issue on macOS or Windows, please file a bug report. The project is committed to fixing platform bugs promptly.

### Minimum Rust Version

If building from source, pdftract requires Rust 1.78 or later. The MSRV is pinned in `Cargo.toml` and tested on every PR.

## Verifying Installation

Run the following command to verify your installation:

```bash
pdftract --version
```

You should see output like:

```
pdftract 0.1.0
```

The Python binding is planned but has no published wheel, so there is no
supported Python installation command to verify yet.

### Environment Health Check

After installation, verify your environment is properly configured for pdftract:

```bash
pdftract doctor
```

This validates the dependencies used by the selected build. OCR-specific
system checks apply only to a future OCR-enabled build. See the [Operations
Runbook](../../operations/manual-platform-smoke.md) for detailed
troubleshooting of each check.

## Next Steps

Once installed, proceed to the [Quickstart](./quickstart.md) for a five-minute walkthrough of pdftract's core features.
