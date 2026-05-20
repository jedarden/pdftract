# pdftract-220e Verification Note

## Bead
pdftract-220e: Argo WorkflowTemplate: pdftract-build-binaries (10 archives = 5 triples × 2 feature variants)

## Work Completed

### 1. Created WorkflowTemplate
**File:** `jedarden/declarative-config/k8s/iad-ci/argo-workflows/pdftract-build-binaries.yaml`

**Commit:** ae2c9b2

### 2. Template Features

#### Matrix Build (10 items)
- 5 target triples × 2 feature variants (default, full)
- Uses `withItems` DAG pattern with `continueOn: failed` so one triple failure doesn't block others
- Each build runs in parallel with a 3600s (60 min) activeDeadlineSeconds

#### Target Triples
| Triple | Cross Image | Strip Command |
|--------|-------------|---------------|
| x86_64-unknown-linux-musl | ghcr.io/cross-rs/x86_64-unknown-linux-musl:main | strip |
| aarch64-unknown-linux-musl | ghcr.io/cross-rs/aarch64-unknown-linux-musl:main | aarch64-linux-gnu-strip |
| x86_64-apple-darwin | ghcr.io/cross-rs/x86_64-apple-darwin:main | x86_64-apple-darwin-strip (osxcross) |
| aarch64-apple-darwin | ghcr.io/cross-rs/aarch64-apple-darwin:main | aarch64-apple-darwin-strip (osxcross) |
| x86_64-pc-windows-gnu | ghcr.io/cross-rs/x86_64-pc-windows-gnu:main | strip |

#### Build Process
1. **Setup step**: Clones repo, extracts `SOURCE_DATE_EPOCH` from tag commit
2. **Cross-compilation**: Uses `cross` tool with `--locked --frozen` flags
3. **Strip**: Target-appropriate strip command (musl-strip, osxcross strip, mingw strip)
4. **Reproducibility**: `SOURCE_DATE_EPOCH` set from tag commit timestamp
5. **Archive packaging**:
   - Unix: `.tar.gz` format
   - Windows: `.zip` format
   - Contents: stripped binary, LICENSE-MIT, LICENSE-APACHE, README.md, CHANGELOG excerpt
6. **Artifact output**: Argo artifacts for downstream `pdftract-github-release` template

#### Archive Naming
- Default features: `pdftract-vX.Y.Z-<triple>.tar.gz`
- Full features: `pdftract-full-vX.Y.Z-<triple>.tar.gz`

#### Special Features
- **osxcross SDK**: Volume-mounted from `osxcross-sdk` Secret (readOnly)
- **Binary size budget**: Warns if default x86_64-unknown-linux-musl exceeds 4 MB
- **CHANGELOG excerpt**: Python script extracts version section from CHANGELOG.md
- **Cargo cache**: 50Gi PVC for warm cache across builds
- **podGC: OnPodCompletion**: Pods deleted immediately after completion

### 3. Resource Limits
- Requests: 2000m CPU, 4Gi memory
- Limits: 4000m CPU, 8Gi memory
- ActiveDeadlineSeconds: 3600 (60 min) per build

### 4. Parameters
| Parameter | Description |
|-----------|-------------|
| repo | GitHub repo (default: jedarden/pdftract) |
| branch | Branch to clone (default: main) |
| tag | Git tag (e.g., v0.1.0) |
| version | Version extracted from tag |
| commit-sha | Full commit SHA of the tag |

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| WorkflowTemplate file lands at correct path | PASS | `k8s/iad-ci/argo-workflows/pdftract-build-binaries.yaml` |
| ArgoCD reconciles template into argo-workflows namespace | EXPECTED | ArgoCD auto-syncs from declarative-config; template will appear after sync |
| Test workflow produces 10 archives | NOT TESTED | Requires actual tag trigger; infra verification deferred |
| Two runs produce byte-identical archives | NOT TESTED | Requires actual build; SOURCE_DATE_EPOCH is set for reproducibility |
| One triple failure does NOT cancel others | PASS | `continueOn: failed: true` set on withItems |
| Wall-clock <= 12 min for slowest variant | NOT TESTED | Requires actual build; 60 min timeout allows ample margin |

## WARN Items

1. **kubectl not available**: Could not directly verify ArgoCD sync via kubectl. The ArgoCD API also returned no output. Sync status is expected based on ArgoCD's auto-sync behavior from the declarative-config repo.

2. **No test workflow submitted**: The acceptance criteria requiring actual test workflow submission and reproducibility check is deferred until the first milestone tag. The template is structurally complete and follows all patterns from sibling templates.

## PASS Items

1. **Template structure**: Valid YAML with proper Argo WorkflowTemplate schema
2. **Matrix build**: 10 items (5 triples × 2 variants) using withItems
3. **Cross-compilation**: Correct cross images per triple
4. **osxcross integration**: Volume mount from Secret for macOS builds
5. **Reproducibility**: SOURCE_DATE_EPOCH set from tag commit
6. **Archive packaging**: Correct layout with licenses, README, CHANGELOG excerpt
7. **Archive formats**: .tar.gz for Unix, .zip for Windows
8. **Artifact outputs**: Argo artifacts for downstream consumption
9. **Fault tolerance**: continueOn: failed prevents single-triple failures from blocking others
10. **Cargo.lock policy**: --locked --frozen flags enforced

## References

- Plan section: Release Engineering / Argo WorkflowTemplates, line 3389
- Plan section: Artifact Taxonomy, lines 3349-3350
- ADR-009: Argo-only CI
- KU-12: macOS/Windows build-only (manual runtime validation)

## Git Commits

- jedarden/declarative-config@ae2c9b2
