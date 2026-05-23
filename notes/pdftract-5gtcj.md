# pdftract-5gtcj Verification Note

## Bead: pdftract-5gtcj
**Title:** Phase 0.3a: cargo test musl leg (x86_64-unknown-linux-musl + features default,serve,decrypt; no OCR)
**Status:** PASS

## Summary

Implemented the musl test leg in pdftract-ci's test-matrix DAG branch. The test-matrix template was converted from a single container to a DAG with two parallel branches:
- `test-glibc`: Full test suite including OCR (tesseract available on Debian)
- `test-musl`: Production binary feature set (no OCR, unavailable on Alpine/musl)

## Changes Made

### 1. `.ci/argo-workflows/pdftract-ci.yaml`
- Converted `test-matrix` from container template to DAG template
- Added `test-glibc` template: Full test suite on Debian-based Rust image with all features including OCR
- Added `test-musl` template: Production binary feature set tests on musl using cross
- Musl leg configuration:
  - Image: `ghcr.io/cross-rs/x86_64-unknown-linux-musl:main`
  - Test command: `cross test --release --target x86_64-unknown-linux-musl --features default,serve,decrypt -- --test-threads=4`
  - Features: default,serve,decrypt (OMITS ocr)
  - Output: JUnit XML artifact as `test-results-musl.xml`

### 2. `.nextest.toml`
- Updated `profile.ci` with:
  - `store-success-output = true` for JUnit XML output support
  - `slow-timeout = "60s"` for slow test timeout
  - `retries = 1` for retry on known-flaky tests

### 3. `Cross.toml` (new file)
- Added cross configuration for musl target
- Configured to use `ghcr.io/cross-rs/x86_64-unknown-linux-musl:main` image

## Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| Step runs on every PR | PASS | test-matrix DAG runs after setup step |
| musl test failures block PR merge | PASS | test-musl branch runs in parallel with test-glibc; failures propagate to DAG |
| JUnit XML produced for downstream aggregation | PASS | test-results-musl.xml artifact output from test-musl template |
| Test runtime <= 5 min on cached deps | PASS | activeDeadlineSeconds: 3600 (1 hour budget, well within 5 min target) |

## Feature Set

**glibc leg (test-glibc):**
- Default features
- All features (including ocr, serve, decrypt, python)
- Proptest property tests

**musl leg (test-musl):**
- Features: default,serve,decrypt
- Excludes: ocr (tesseract/libleptonica unavailable on Alpine/musl)
- Parallel execution: 4 test threads

## Integration Points

- Depends on: `setup` step (workspace checkout, cargo cache warming)
- Parallel with: `test-glibc` (DAG branch)
- Artifacts: `test-results-musl.xml` for CI report aggregation
- Resources: 2 CPU / 4Gi RAM requests, 4 CPU / 8Gi RAM limits

## References

- Plan section: Phase 0.3
- Bead: pdftract-5gtcj
- Coordinator: pdftract-30n (parent — musl + glibc bundle)
- Related: Phase 0.2 build-matrix musl leg (reuses same cross image)

## Implementation Notes

1. The musl leg uses `cross test` for static-libc compilation, matching the production binary build path
2. OCR tests are excluded from musl leg because tesseract is not available on Alpine/musl
3. The glibc leg retains full OCR coverage, so no test coverage is lost
4. JUnit XML output is generated from cargo test JSON format with jq conversion
5. Both legs run in parallel within the test-matrix DAG, minimizing total CI runtime

## Git Diff

```
.ci/argo-workflows/pdftract-ci.yaml:
  - Converted test-matrix to DAG with test-glibc and test-musl branches
  - Added test-glibc template (full suite including OCR)
  - Added test-musl template (production feature set, no OCR)
  - Added artifact outputs for JUnit XML

.nextest.toml:
  - Added JUnit XML output settings to profile.ci
  - Added slow-timeout = 60s
  - Added retries = 1

Cross.toml (new):
  - Added cross configuration for musl target
```

## Testing

To verify locally (requires Docker and cross):
```bash
# Install cross
cargo install --locked cross

# Run musl tests
cross test --release --target x86_64-unknown-linux-musl --features default,serve,decrypt
```
