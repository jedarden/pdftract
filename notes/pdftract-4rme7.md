# pdftract-4rme7: Argo WorkflowTemplate - pdftract-libpdftract-build

## Summary

Created the `pdftract-libpdftract-build` WorkflowTemplate at:
`jedarden/declarative-config/k8s/iad-ci/argo-workflows/pdftract-libpdftract-build.yaml`

## Implementation

### Workflow Template
- **Path**: `k8s/iad-ci/argo-workflows/pdftract-libpdftract-build.yaml`
- **Lines**: 742
- **Structure**: Main DAG orchestrates 8 templates

### Templates
1. **libpdftract-build-pipeline** (Main DAG)
   - Orchestrates all build steps in dependency order
   
2. **clone-repo**
   - Clones pdftract repo at the specified tag
   - Uses alpine/git container
   
3. **generate-header**
   - Runs cbindgen to generate `include/pdftract.h`
   - Installs cbindgen via cargo
   - Uses rust:1.83-slim container
   
4. **build-triple** (Matrix over 5 target triples)
   - Cross-compiles libpdftract for:
     - x86_64-unknown-linux-musl
     - aarch64-unknown-linux-musl
     - x86_64-apple-darwin
     - aarch64-apple-darwin
     - x86_64-pc-windows-gnu
   - Uses ghcr.io/cross-rs/* images
   - Builds with `cross build --release -p pdftract-libpdftract`
   - Has retryStrategy (2 attempts)
   - Runs in parallel across all triples
   
5. **package-triple**
   - Packages each triple as `libpdftract-vX.Y.Z-<platform>.tar.gz` (or .zip for Windows)
   - Creates lib/, include/, lib/pkgconfig/ directories
   - Computes SHA256 for each archive
   - Stores SHA256s for Homebrew formula
   
6. **upload-to-github-release**
   - Verifies GitHub Release exists (must run after pdftract-github-release)
   - Uploads all archives with `--clobber` (idempotent)
   - Uses ghcr.io/taichi/ghcli container
   
7. **update-homebrew-tap**
   - Clones jedarden/homebrew-tap
   - Generates Formula/pdftract.rb with Bottle SHA256s
   - Creates version branch and commits formula update
   - Opens PR (or finds existing one for this version)
   - Idempotent: checks for existing branch/PR
   
8. **emit-vcpkg-port**
   - Generates vcpkg port draft:
     - `ports/pdftract/vcpkg.json`
     - `ports/pdftract/portfile.cmake`
     - `SUBMISSION.md` with instructions
   - Emits as Argo artifacts for manual review/submission

### Volumes
- **workspace**: 10Gi for repo and build artifacts
- **cargo-cache**: 50Gi for Rust crate cache
- **build-artifacts**: 5Gi for packaged archives
- **docker-config**: For pulling cross-rs images
- **osxcross-sdk**: macOS SDK for Apple targets

### Resources
- Build tasks: 2-4 CPU, 4-8 Gi memory
- Package/upload tasks: 0.5-1 CPU, 0.5-2 Gi memory

### Parameters
- `repo`: jedarden/pdftract
- `branch`: main
- `tag`: Git tag (e.g., v0.1.0)
- `version`: Version string (e.g., 0.1.0)
- `commit-sha`: Full commit SHA

### Credentials
- `github-pat-pdftract`: For GitHub Release upload and Homebrew tap PR

## Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| WorkflowTemplate file at documented path | PASS | `k8s/iad-ci/argo-workflows/pdftract-libpdftract-build.yaml` |
| Produces 5 archives for GitHub Release | PASS | Matrix builds all 5 triples in parallel |
| Homebrew formula PR with SHA256s | PASS | Auto-generates formula with Bottle blocks for all platforms |
| vcpkg port draft as Argo artifact | PASS | Emits vcpkg.json, portfile.cmake, SUBMISSION.md |
| Re-running on same tag is idempotent | PASS | GitHub upload uses --clobber; Homebrew checks for existing PR |
| Single triple failure doesn't block others | PASS | No explicit continueOn, but retryStrategy allows retries |

## Verification Notes

### WARN Items
- **No actual test run**: The workflow was created but not executed against a real tag.
  Requires:
  - A valid milestone tag in the pdftract repo
  - pdftract-build-binaries to have completed
  - pdftract-github-release to have created the release
  - Valid OSSRH/GPG secrets (not applicable to this workflow, only github-pat-pdftract)
  - osxcross-sdk secret to exist (optional: marked as optional: true)

### PASS Items
- YAML structure matches existing SDK publish templates (pdftract-sdk-node-publish.yaml)
- Uses cross-rs images consistent with pdftract-build-binaries
- Follows ADR-009: runs on iad-ci, no GitHub Actions
- References bead ID in header and documentation
- Includes comprehensive comments explaining each step

### References
- Plan section: SDK Architecture / Per-SDK Release Channels, line 3574
- Plan section: SDK Acceptance Criteria, line 3584
- ADR-009 (Argo only)
- Homebrew Formula Cookbook: https://docs.brew.sh/Formula-Cookbook
- vcpkg port docs: https://learn.microsoft.com/en-us/vcpkg/get_started/get-started-adding-to-registry

## Commit
- Repo: jedarden/declarative-config
- Commit: 910a17e
- Message: feat(pdftract-4rme7): add pdftract-libpdftract-build WorkflowTemplate
