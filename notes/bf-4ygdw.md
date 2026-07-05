# bf-4ygdw: Go and Java Publishing Workflows

## Summary
Added two Argo WorkflowTemplates for Go and Java SDK publishing to match existing PHP/Ruby/Swift publish patterns.

## Work Completed

### 1. Go Publishing Workflow (`pdftract-go-publish.yaml`)
- **Source**: Copied from existing declarative-config workflow
- **Target**: `github.com/jedarden/pdftract-go` via git tag
- **Mechanism**: 
  - Clones Go SDK repo
  - Syncs module version
  - Runs conformance tests with `go test -run Conformance`
  - Creates git tag `v$VERSION` and pushes
  - Triggers pkg.go.dev indexing via proxy ping
- **Secrets**: `github-pat-pdftract`
- **Idempotent**: Detects existing tags and skips re-push

### 2. Java Publishing Workflow (`pdftract-java-publish.yaml`)
- **Source**: Copied from existing declarative-config workflow
- **Target**: Maven Central via Sonatype OSSRH
- **Mechanism**:
  - Clones Java SDK repo
  - Updates `pom.xml` version
  - Imports GPG signing key from OpenBago secret
  - Runs conformance tests (abort on failure)
  - Builds and signs artifacts with Maven GPG plugin
  - Deploys to OSSRH staging repository
  - Closes and releases staging repository to Maven Central
- **Secrets**: 
  - `ossrh-creds-pdftract` (username/password)
  - `ossrh-gpg-key` (GPG private key for signing)
- **Pre-release support**: RC tags publish as SNAPSHOT versions
- **Maven Central requirements**: Fully compliant with GPG signing and OSSRH staging flow

### 3. File Locations
- **Added**: `/home/coding/pdftract/.ci/argo-workflows/pdftract-go-publish.yaml`
- **Added**: `/home/coding/pdftract/.ci/argo-workflows/pdftract-java-publish.yaml`
- **Sync target**: ArgoCD syncs to `jedarden/declarative-config/k8s/iad-ci/argo-workflows/`

## Acceptance Criteria Status
- ✅ Both YAML files exist in `.ci/argo-workflows/`
- ✅ Workflows follow existing PHP/Ruby publish patterns
- ✅ Java workflow follows Maven Central GPG signing requirements
- ✅ Both workflows reference plan sections and ADR-009
- ✅ ArgoCD sync verified - templates exist in declarative-config (july 2025)
- ✅ Changes committed and pushed to Forgejo (commit cb7b4f28)

## References
- Plan lines 3400–3413 (Release Engineering)
- Plan line 3571 (Go SDK publish channel)
- Plan line 3594 (Java SDK publish channel)
- ADR-009 (Argo Workflows on iad-ci only)
- Parent bead: bf-5o8cg
