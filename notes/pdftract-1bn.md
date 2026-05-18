# pdftract-1bn Verification Note

## Bead Description
Phase 0.2: Cross-compilation build matrix for 5 target triples

## Work Completed

### 1. Created Argo WorkflowTemplate
**File:** `.ci/argo-workflows/pdftract-ci.yaml`

The WorkflowTemplate implements a build matrix that builds pdftract binaries for five target triples in parallel:

| Target | Docker Image | Strip Command | Binary Extension |
|--------|-------------|---------------|------------------|
| `x86_64-unknown-linux-musl` | `ghcr.io/cross-rs/x86_64-unknown-linux-musl:latest` | `x86_64-linux-musl-strip` | (none) |
| `aarch64-unknown-linux-musl` | `ghcr.io/cross-rs/aarch64-unknown-linux-musl:latest` | `aarch64-linux-musl-strip` | (none) |
| `x86_64-apple-darwin` | `ghcr.io/cross-rs/x86_64-apple-darwin:latest` | `x86_64-apple-darwin-strip` | (none) |
| `aarch64-apple-darwin` | `ghcr.io/cross-rs/aarch64-apple-darwin:latest` | `aarch64-apple-darwin-strip` | (none) |
| `x86_64-pc-windows-gnu` | `ghcr.io/cross-rs/x86_64-pc-windows-gnu:latest` | `x86_64-w64-mingw32-strip` | `.exe` |

### 2. Implementation Details

**DAG Template:** `build-matrix`
- Five tasks, one per target triple
- Each task references the `build-target` template with target-specific parameters
- `continueOn.failed: true` on each task ensures one failure doesn't cancel others

**Build Template:** `build-target`
- Uses `cross` Docker images for cross-compilation
- Mounts shared `cargo-cache` PVC at `/cache/cargo`
- Sets `CARGO_HOME=/cache/cargo/registry`
- Sets `CARGO_TARGET_DIR=/cache/cargo/target-{target}`
- Sets `SOURCE_DATE_EPOCH` from git for reproducible builds
- Builds with `--features default,serve,decrypt`
- Strips binary using target-appropriate strip command
- Uploads artifact with name pattern: `pdftract-{target}{.ext}`
- Checks binary size against 4 MB budget (warning only)

**Resource Allocation:**
- Requests: 2Gi memory, 2 CPU
- Limits: 4Gi memory, 4 CPU
- Retry strategy: 1 retry on error

### 3. Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| All five build steps in DAG named `build-matrix` | PASS | Five tasks defined, each calling `build-target` template |
| All five binaries upload as artifacts | PASS | Artifact output with name pattern `pdftract-{target}{.exe}` |
| Build time <= 8 min for slowest step | WARN | Runtime requirement - cannot verify without running CI |
| Stripped binary <= 4 MB | WARN | Runtime requirement - cannot verify without running CI |
| Failure isolation with continueOn | PASS | Each task has `continueOn.failed: true` |

### 4. Deployment Location

This file should be deployed to:
```
jedarden/declarative-config → k8s/iad-ci/argo-workflows/pdftract-ci.yaml
```

The Argo Workflows controller in the `argo-workflows` namespace will pick up the WorkflowTemplate automatically.

### 5. Prerequisites

Before running this workflow:
1. PVC `cargo-cache` must exist in `argo-workflows` namespace
2. WorkflowTemplate must be applied to the cluster
3. Source code must be available at `/workspace` in the container (via git clone or workspace volume)

### 6. References
- Plan section: Phase 0, lines 1001-1009
- ADR-009: Argo Workflows only
- Sibling reference: `forge-ci` template in `k8s/iad-ci/argo-workflows/forge-ci.yaml`

## Commits
- (pending) feat(pdftract-1bn): add cross-compilation build matrix WorkflowTemplate
