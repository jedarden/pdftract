# Building Python Wheels for pdftract

This document describes how to build binary Python wheels for pdftract across all supported platforms using cross-compilation from a Linux host.

## Target Platforms

pdftract builds wheels for 5 target triples (ABI3, cp310-abi3):

| Platform | Triple | manylinux / macosx / win tag |
|----------|--------|------------------------------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | `manylinux_2_28_x86_64` |
| Linux aarch64 | `aarch64-unknown-linux-gnu` | `manylinux_2_28_aarch64` |
| macOS Intel | `x86_64-apple-darwin` | `macosx_11_0_x86_64` |
| macOS Apple Silicon | `aarch64-apple-darwin` | `macosx_11_0_arm64` |
| Windows x86_64 | `x86_64-pc-windows-gnu` | `win_amd64` |

All wheels use the stable ABI (abi3) with minimum Python 3.10, producing a single wheel per platform: `pdftract-{version}-cp310-abi3-{platform_tag}.whl`

## Prerequisites

### On Linux (Ubuntu/Debian)

```bash
# Install cross-compilation toolchains
sudo apt install \
    gcc-aarch64-linux-gnu \
    g++-aarch64-linux-gnu \
    gcc-x86-64-linux-gnu \
    g++-x86-64-linux-gnu

# Install mingw-w64 for Windows cross-compilation
sudo apt install mingw-w64

# Install Rust targets
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
rustup target add x86_64-apple-darwin aarch64-apple-darwin
rustup target add x86_64-pc-windows-gnu

# Install cross tool
cargo install cross --git https://github.com/cross-rs/cross
```

### macOS SDK for Linux→macOS Cross-Compilation

macOS cross-compilation requires Xcode SDK files due to Apple licensing:

```bash
# Create SDK directory
mkdir -p ~/.macos-sdks

# Download macOS 11 SDK (minimum for Apple Silicon support)
# Use the osxcross toolchain setup
git clone https://github.com/tpoechtrager/osxcross.git ~/.osxcross
cd ~/.osxcross

# Download and unpack SDK (follow osxcross instructions)
# Requires Xcode command line tools from Apple (free developer account)
# Then build the toolchain:
./build.sh

# The cross crate will automatically detect osxcross if installed in ~/.osxcross
```

### Using Docker for Linux Wheels

For manylinux compliance, use the official manylinux Docker images:

```bash
# Linux x86_64 (native, no cross needed)
docker run --rm -v $(pwd):/io ghcr.io/rust-cross/manylinux_2_28-x86_64:latest \
    maturin build --release --out wheels/

# Linux aarch64 (cross from x86_64 host)
docker run --rm -v $(pwd):/io ghcr.io/rust-cross/manylinux_2_28-aarch64:latest \
    maturin build --release --target aarch64-unknown-linux-gnu --out wheels/
```

## Building Wheels

### Native Build (Linux x86_64)

```bash
cd crates/pdftract-py
maturin build --release --out dist/
# Produces: pdftract-{version}-cp310-abi3-manylinux_2_28_x86_64.whl
```

### Cross-Compiled Builds

#### Linux aarch64 (from x86_64 host)

```bash
# Install aarch64 target
rustup target add aarch64-unknown-linux-gnu

# Build
maturin build --release --target aarch64-unknown-linux-gnu --out dist/
# Produces: pdftract-{version}-cp310-abi3-manylinux_2_28_aarch64.whl
```

#### macOS Intel (from Linux host)

```bash
# Requires osxcross installation (see Prerequisites above)
rustup target add x86_64-apple-darwin

# Build
maturin build --release --target x86_64-apple-darwin --out dist/
# Produces: pdftract-{version}-cp310-abi3-macosx_11_0_x86_64.whl
```

#### macOS Apple Silicon (from Linux host)

```bash
# Requires osxcross with ARM64 support
rustup target add aarch64-apple-darwin

# Build
maturin build --release --target aarch64-apple-darwin --out dist/
# Produces: pdftract-{version}-cp310-abi3-macosx_11_0_arm64.whl
```

#### Windows x86_64 (from Linux host)

```bash
# Install MinGW toolchain (see Prerequisites)
rustup target add x86_64-pc-windows-gnu

# Build
maturin build --release --target x86_64-pc-windows-gnu --out dist/
# Produces: pdftract-{version}-cp310-abi3-win_amd64.whl
```

### Using the cross crate

For consistent cross-compilation across all platforms:

```bash
# Install cross
cargo install cross --git https://github.com/cross-rs/cross

# Build for any target
cross build --release --target x86_64-unknown-linux-gnu
cross build --release --target aarch64-unknown-linux-gnu
cross build --release --target x86_64-apple-darwin
cross build --release --target aarch64-apple-darwin
cross build --release --target x86_64-pc-windows-gnu

# Then build wheels with maturin
maturin build --release --target <triple> --out dist/
```

The cross crate handles Docker environment creation and toolchain configuration automatically.

## Reproducible Builds

To ensure reproducible wheels across builds, set `SOURCE_DATE_EPOCH`:

```bash
# Use git commit timestamp or fixed epoch
export SOURCE_DATE_EPOCH=$(git show -s --format=%ct HEAD)
maturin build --release --target x86_64-unknown-linux-gnu --out dist/

# Verify reproducibility
sha256sum dist/*.whl
```

With `SOURCE_DATE_EPOCH` set, the same source will produce byte-identical wheels across builds.

## Wheel Naming Convention

Wheels follow PEP 491 naming:

```
{distribution}-{version}(-{build tag})?-{python tag}-{abi tag}-{platform tag}.whl
```

For pdftract:
- Distribution: `pdftract`
- Version: from Cargo.toml (e.g., `0.1.0`)
- Python tag: `cp310` (minimum version for abi3)
- ABI tag: `abi3` (stable ABI, forward compatible)
- Platform tag: varies by platform

Examples:
- `pdftract-0.1.0-cp310-abi3-manylinux_2_28_x86_64.whl`
- `pdftract-0.1.0-cp310-abi3-macosx_11_0_arm64.whl`
- `pdftract-0.1.0-cp310-abi3-win_amd64.whl`

## CI/CD Integration

The `pdftract-ci` Argo WorkflowTemplate (in `jedarden/declarative-config`) builds all 5 wheels in parallel using a build matrix.

See `.ci/argo-workflows/pdftract-ci.yaml` for the full CI configuration.

## Troubleshooting

### macOS SDK Not Found

If maturin can't find the macOS SDK during cross-compilation:

```bash
# Set SDK path explicitly
export MACOSX_DEPLOYMENT_TARGET=11.0
export SDKROOT=$(xcrun --sdk macosx --show-sdk-path)  # on macOS
export OSXCROSS_ROOT=~/.osxcross  # on Linux with osxcross
```

### Windows Cross-Compilation Fails

Ensure MinGW is installed and in PATH:

```bash
# Verify MinGW installation
x86_64-w64-mingw32-gcc --version

# If missing, install via apt
sudo apt install mingw-w64
```

### manylinux Version

We use `manylinux_2_28` (RHEL 8 compatible) as the baseline. This is the modern standard; older `manylinux2014` (RHEL 7) is deprecated as RHEL 7 is EOL.

## References

- [maturin User Guide](https://www.maturin.rs/)
- [PEP 491 - Wheel Binary Package Format](https://peps.python.org/pep-0491/)
- [PEP 600 - Platform Tag](https://peps.python.org/pep-0600/)
- [PyO3 Building and Distribution](https://pyo3.rs/main/building-and-distribution)
- [cross-rs](https://github.com/cross-rs/cross)
