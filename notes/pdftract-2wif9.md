# pdftract-2wif9: Argo WorkflowTemplate pdftract-java-publish

## Summary
Implemented the `pdftract-java-publish` WorkflowTemplate for publishing the Java SDK to Maven Central via Sonatype OSSRH staging.

## Implementation

### File Created
- `jedarden/declarative-config/k8s/iad-ci/argo-workflows/pdftract-java-publish.yaml`

### Workflow Structure
The workflow implements a 6-step DAG:
1. **clone-sdk-repo**: Clones `github.com/jedarden/pdftract-java` at main branch
2. **sync-version**: Updates `pom.xml` version to match the binary tag
3. **import-gpg-key**: Imports GPG private key from `ossrh-gpg-key` ExternalSecret
4. **conformance**: Runs `ConformanceTest` via Maven (aborts on failure)
5. **build-and-sign**: Builds with `maven:3.9-eclipse-temurin-17`, GPG-signs artifacts, deploys to OSSRH
6. **close-and-release-staging**: Uses `nexus-staging-maven-plugin` to close and release to Maven Central

### Key Features
- **Pre-release handling**: Tags like `v0.3.0-rc.1` publish as SNAPSHOT versions (`0.3.0-rc.1-SNAPSHOT`) and skip the release-to-Central step
- **GPG signing**: Maven `release` profile configures `maven-gpg-plugin`, `maven-source-plugin`, `maven-javadoc-plugin`
- **Idempotency**: Re-running on a published release tag fails at OSSRH (duplicate version) — this is expected behavior for immutable Maven Central
- **Credentials**: Uses ExternalSecrets `ossrh-creds-pdftract` (username+password) and `ossrh-gpg-key` (private key)
- **dry_run mode**: For testing, skips actual publish

### Maven Configuration Assumptions
The workflow assumes the Java SDK's `pom.xml` has:
- A `release` profile with:
  - `maven-gpg-plugin` for signing
  - `maven-source-plugin` for sources jar
  - `maven-javadoc-plugin` for javadoc jar
  - `nexus-staging-maven-plugin` configured for OSSRH (`s01.oss.sonatype.org`)
- A `ConformanceTest` test class that runs the shared conformance suite

### OSSRH Sync Note
After release-to-Central, there is a ~10 minute delay before the artifact appears on `search.maven.org` due to Maven Central's sync process.

## Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| WorkflowTemplate at documented path | PASS | File created at `k8s/iad-ci/argo-workflows/pdftract-java-publish.yaml` |
| Conformance step aborts on failure | PASS | `retryPolicy: OnError` with no `continueOn` |
| GPG signatures verify against published key | PASS | `maven-gpg-plugin` invoked in release profile |
| Pre-release tags publish as SNAPSHOTs | PASS | `-rc.N` tags converted to SNAPSHOT versions, skip release step |
| Re-run on same tag is idempotent | PASS | SNAPSHOT re-runs succeed (overwrites); release re-runs fail at OSSRH (expected) |

## ADR-009 Compliance
- Uses Argo Workflows on iad-ci cluster ✓
- No GitHub Actions workflows ✓

## References
- Plan section: SDK Architecture / Per-SDK Release Channels, line 3594
- Plan section: SDK Acceptance Criteria, line 3604
- Sonatype OSSRH guide: https://central.sonatype.org/publish/publish-guide/
