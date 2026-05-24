# Installation

pdftract is distributed as a native binary, a Python package, and a Docker image. Choose the installation method that matches your workflow.

## Install via Cargo

```bash
cargo install pdftract
```

This installs the `pdftract` binary in `~/.cargo/bin/`. Make sure `~/.cargo/bin` is in your `PATH`.

### Pre-built Binaries

Pre-built binaries are available from [GitHub Releases](https://github.com/jedarden/pdftract/releases). Download the archive for your platform, extract, and place the binary in your `PATH`.

### Cargo Binstall

For faster installation without compiling from source:

```bash
cargo binstall pdftract
```

This downloads a pre-built binary from the GitHub Release instead of compiling locally.

## Install via pip

pdftract is distributed on PyPI as a native Python extension with PyO3 bindings.

```bash
pip install pdftract
```

The Python package includes the same extraction engine as the CLI, accessible via a Python API. See [Python SDK](./sdk/python.md) for usage.

### Platform Wheels

Wheels are available for:
- Linux `x86_64` (manylinux2014, musllinux)
- macOS `x86_64` and `arm64`
- Windows `x86_64`

If no wheel is available for your platform, pip will fall back to building from source (requires Rust toolchain).

## Install via Homebrew

**Note:** Homebrew formula is deferred to v1.1+. In the meantime, use `cargo install pdftract` or the Docker image.

See the [Non-Goals section](../../plan/plan.md#non-goals) in the project plan for the rationale.

## Install via Docker

Docker images are available on GitHub Container Registry:

```bash
docker pull ghcr.io/jedarden/pdftract:latest
docker run --rm -v $(pwd):/work ghcr.io/jedarden/pdftract:latest extract /work/document.pdf
```

### Image Variants

| Tag | Description |
|---|---|
| `latest` | Default features (vector extraction, basic OCR) |
| `ocr` | Includes Tesseract for full OCR support |
| `full` | All features including PDFium for rasterization |

Multi-arch manifests support `amd64` and `arm64` platforms.

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

For the Python package:

```bash
python -c "import pdftract; print(pdftract.__version__)"
```

### Environment Health Check

After installation, verify your environment is properly configured for pdftract:

```bash
pdftract doctor
```

This validates that all OS-level dependencies (Tesseract, leptonica, libtiff, etc.) are installed and correctly configured. See the [Operations Runbook](../../operations/manual-platform-smoke.md) for detailed troubleshooting of each check.

## Next Steps

Once installed, proceed to the [Quickstart](./quickstart.md) for a five-minute walkthrough of pdftract's core features.
