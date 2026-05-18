# pdftract-1bn: Cross-compilation build matrix implementation

## Summary

Implemented the cross-compilation build matrix for all 5 release target triples in the `pdftract-ci` WorkflowTemplate. Each target produces a stripped release binary uploaded as an Argo artifact.

## Changes Made

### File: `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/pdftract-ci.yaml`

1. **Added workspace volumeClaimTemplate** (10Gi) to share cloned repo between setup and all build steps
2. **Implemented build-matrix DAG** with 5 target build tasks:
   - `x86_64-unknown-linux-musl` (Linux x86_64 musl)
   - `aarch64-unknown-linux-musl` (Linux ARM64 musl)
   - `x86_64-apple-darwin` (macOS x86_64)
   - `aarch64-apple-darwin` (macOS ARM64)
   - `x86_64-pc-windows-gnu` (Windows x86_64)
3. **Added `continueOn: failed`** to each build task for fault tolerance (one failure doesn't cancel others)
4. **Implemented build-target template** using `ghcr.io/cross-rs/<target>:main` images directly
5. **Configured cargo-cache volume mount** at `/cache/cargo` with `CARGO_HOME` and `CARGO_TARGET_DIR` environment variables
6. **Added SOURCE_DATE_EPOCH** for reproducible builds
7. **Added `--locked` flag** to cargo build for reproducible builds
8. **Added binary stripping** using target-appropriate strip commands
9. **Added artifact upload** with pattern `pdftract-<target>{.exe}`
10. **Updated setup placeholder** to include workspace volume mount

### File: `/home/coding/pdftract/.ci/argo-workflows/pdftract-ci.yaml`

Synced all changes from declarative-config to keep the local copy in sync.

## Acceptance Criteria Status

| Criteria | Status | Notes |
|----------|--------|-------|
| All five build steps in build-matrix DAG | PASS | All 5 targets implemented |
| Binaries upload as artifacts with correct pattern | PASS | Artifact name: `pdftract-<target>{.exe}` |
| Build time <= 8 min for slowest step | WARN | Cannot verify without running pipeline |
| Stripped binary <= 4 MB | WARN | Cannot verify without running pipeline |
| Failure isolation (continueOn) | PASS | Added `continueOn: failed` to all 5 tasks |

## Technical Details

### Build Matrix Structure
```
build-matrix (DAG)
├── build-linux-x86_64-musl (continueOn: failed)
├── build-linux-aarch64-musl (continueOn: failed)
├── build-darwin-x86_64 (continueOn: failed)
├── build-darwin-aarch64 (continueOn: failed)
└── build-windows-x86_64-gnu (continueOn: failed)
```

### Docker Images Used
- Linux: `ghcr.io/cross-rs/x86_64-unknown-linux-musl:main`
- Linux ARM64: `ghcr.io/cross-rs/aarch64-unknown-linux-musl:main`
- macOS x64: `ghcr.io/cross-rs/x86_64-apple-darwin:main`
- macOS ARM64: `ghcr.io/cross-rs/aarch64-apple-darwin:main`
- Windows: `ghcr.io/cross-rs/x86_64-pc-windows-gnu:main`

### Build Features
Default feature set: `default,serve,decrypt` (OCR feature excluded per plan)

### Resource Limits
- Requests: 2 CPU, 4Gi memory
- Limits: 4 CPU, 8Gi memory
- Active deadline: 3600s (1 hour)

## Known Limitations

1. **Setup step is placeholder**: The workspace clone and cargo cache warming logic will be implemented by a sibling Phase 0 bead. Currently, the build-target template expects `/workspace` to contain the cloned repo.

2. **Cannot verify build time**: The 8-minute wall-clock requirement for the slowest step cannot be verified without running the pipeline on iad-ci.

3. **Cannot verify binary size**: The 4 MB budget for stripped binaries cannot be verified without running the pipeline.

4. **macOS/Windows runtime verification**: Per KU-12, these binaries are built but never run in CI. Manual quarterly smoke tests are the verification path (out of scope for this bead).

## References

- Plan section: Phase 0, lines 1001-1009
- ADR-009 (Argo Workflows only)
- Bead ID: pdftract-1bn
