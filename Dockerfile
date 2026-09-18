# syntax=docker/dockerfile:1.7

# FEATURES is a Docker variant selector, not a Cargo feature name. The
# supported selectors are default and full. The full selector expands to the
# buildable CLI feature names below because the workspace does not define a
# Cargo feature literally named "full". OCR, remote, profiles, and PDFium
# remain planned until their feature-gated code is buildable in a clean
# checkout.
ARG FEATURES=default
ARG FULL_FEATURES=serve,mcp,inspect,grep,cache,receipts,markdown

# Rust 1.90.0 is pinned by both version and immutable multi-arch manifest.
FROM rust:1.90.0-bookworm@sha256:3914072ca0c3b8aad871db9169a651ccfce30cf58303e5d6f2db16d1d8a7e58f AS builder

ARG FEATURES
ARG FULL_FEATURES
WORKDIR /usr/src/pdftract

# Keep the builder's native toolchain explicit. The current default/full tiers
# do not link native OCR libraries; this also keeps the final images
# distroless.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy the workspace inputs used by pdftract-cli's selected binary target.
# In particular, the CLI manifest names helper binaries under tests/ and
# tools/, and its source embeds the schema and built-in profiles.
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY docs/schema/ ./docs/schema/
COPY profiles/ ./profiles/
COPY tests/ ./tests/
COPY tools/ ./tools/

RUN set -eux; \
    case "${FEATURES}" in \
        default) \
            cargo build --locked --release -p pdftract-cli --bin pdftract \
            ;; \
        full) \
            cargo build --locked --release -p pdftract-cli --bin pdftract \
                --features "${FULL_FEATURES}" \
            ;; \
        *) \
            echo "unsupported FEATURES=${FEATURES}; use default or full" >&2 \
            exit 2 \
            ;; \
    esac

# Distroless has no shell, package manager, or mkdir. All filesystem and
# package setup therefore happens in the named runtime stages below, before
# the final selector stage is chosen.
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f AS runtime-default
COPY --from=builder /usr/src/pdftract/target/release/pdftract /usr/local/bin/pdftract
COPY LICENSE-MIT LICENSE-APACHE /usr/share/doc/pdftract/
ENTRYPOINT ["/usr/local/bin/pdftract"]
CMD ["--help"]

# The current full tier is still shell-less and uses the same minimal runtime.
FROM runtime-default AS runtime-full

ARG FEATURES
FROM runtime-${FEATURES} AS runtime
