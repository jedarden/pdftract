# Dockerfile for pdftract
# Build arg FEATURES selects the feature set: default | ocr | full
# - default: baseline features, distroless base (~20 MB)
# - ocr: default + OCR (Tesseract), debian-slim base (~120 MB)
# - full: all features, debian-slim base (~140 MB)

ARG FEATURES=default

# Build stage: use Debian slim for Rust toolchain
FROM debian:bookworm-slim AS builder

ARG FEATURES
# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust via rustup
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal && \
    rustup target add x86_64-unknown-linux-gnu

WORKDIR /usr/src/pdftract

# Copy source
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build with requested features
RUN cargo build --release --features ${FEATURES}

# Runtime stage: conditional base based on FEATURES
FROM gcr.io/distroless/cc-debian12 AS runtime-default
FROM debian:bookworm-slim AS runtime-ocr
FROM debian:bookworm-slim AS runtime-full

# Select runtime stage based on FEATURES
FROM runtime-${FEATURES}

# Install Tesseract for ocr/full variants
ARG FEATURES
RUN if [ "${FEATURES}" != "default" ]; then \
    apt-get update && \
    apt-get install -y --no-install-recommends \
    tesseract-ocr \
    tesseract-ocr-eng \
    libliblept5 \
    && rm -rf /var/lib/apt/lists/*; \
    fi

# Copy binary from builder
COPY --from=builder /usr/local/cargo/bin/pdftract /usr/local/bin/pdftract

# Copy license files to /usr/share/doc/pdftract/
RUN mkdir -p /usr/share/doc/pdftract
COPY LICENSE-MIT LICENSE-APACHE /usr/share/doc/pdftract/

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/pdftract"]
CMD ["--help"]
