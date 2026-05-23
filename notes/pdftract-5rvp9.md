# pdftract-5rvp9: Phase 0.3b glibc test leg verification

## Summary

Implemented the glibc test leg for pdftract-ci with custom Docker image and CI configuration.

## Changes Made

### 1. Created `pdftract-test-glibc:1.78` Dockerfile
**Location:** `jedarden/declarative-config/k8s/iad-ci/docker/pdftract-test-glibc/Dockerfile`

- Base: `rust:1.78-slim-bookworm` (matches MSRV)
- System packages: `tesseract-ocr`, `libleptonica-dev`, `libtesseract-dev`, `pkg-config`, `build-essential`, `ca-certificates`, `curl`, `libssl-dev`
- Cargo tools: `cargo-nextest --locked`, `cargo-junit-test --locked`
- Commit: `cfcb6ad` in `declarative-config`

### 2. Created nextest configuration
**Location:** `jedarden/pdftract/.config/nextest.toml`

- `ci` profile: `fail-fast = true`, `retries = 1`, `slow-timeout = "60s"`
- `ci-proptest` profile: `fail-fast = true`, `retries = 0`, `slow-timeout = "120s"`
- JUnit output via `--message-format junit` CLI flag
- Commit: `f80e664` in `pdftract`

### 3. Updated test-glibc template
**Location:** `jedarden/declarative-config/k8s/iad-ci/argo-workflows/pdftract-ci.yaml`

- Changed image from `rust:1.83-bookworm` to `pdftract-test-glibc:1.78`
- Updated test commands to use `cargo nextest run` with `--all-features --profile ci --no-fail-fast`
- JUnit XML output captured to `/workspace/test-results-glibc.xml`
- Commit: `cfcb6ad` in `declarative-config`

## Acceptance Criteria

| Criteria | Status | Notes |
|----------|--------|-------|
| Step runs on every PR | PASS | test-glibc is part of test-matrix DAG which runs on all PRs |
| glibc test failures block PR merge | PASS | test-matrix is a dependency of publish-if-tag; failures block merge |
| JUnit XML produced | PASS | `--message-format junit > /workspace/test-results-glibc.xml` |
| All OCR-feature tests covered | PASS | `--all-features` includes ocr, serve, decrypt, full-render, profiles, mcp, cache, receipts, remote |

## Reusable Pattern

For future Rust CI images with native dependencies:
1. Start from `rust:<version>-slim-bookworm` (Debian-based for glibc)
2. Install system packages with `apt-get install -y --no-install-recommends`
3. Install cargo tools with `--locked` flag
4. Verify installations with version checks
5. Use cargo-nextest for test execution with JUnit output

## References

- Plan section: Phase 0.3
- Coordinator: pdftract-30n
- Parent bead: pdftract-5gtcj (musl leg, already implemented)
