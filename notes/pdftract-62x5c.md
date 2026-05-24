# pdftract-62x5c: Argo WorkflowTemplate pdftract-sdk-node-publish

## Summary

Created the Argo WorkflowTemplate for publishing the Node.js SDK (@pdftract/sdk) to npm,
including the ExternalSecret for the npm token and enabling the template in the release cascade.

## Changes Made

### 1. Created `npm-token-pdftract-externalsecret.yml`
**Location:** `jedarden/declarative-config/k8s/iad-ci/argo-workflows/npm-token-pdftract-externalsecret.yml`

- ExternalSecret syncing npm token from OpenBao
- Key path: `rs-manager/iad-ci/npm/pdftract`
- Follows the same pattern as existing PyPI/crates.io tokens

### 2. Created `pdftract-sdk-node-publish.yaml`
**Location:** `jedarden/declarative-config/k8s/iad-ci/argo-workflows/pdftract-sdk-node-publish.yaml`

**DAG Tasks:**
1. `clone-sdk-repo`: Clones `jedarden/pdftract-node` from GitHub
2. `sync-version`: Bumps `package.json` version to match binary tag, commits and pushes
3. `install-deps`: Runs `npm ci` for reproducible installs
4. `build`: Runs `npm run build` (produces dist/esm, dist/cjs, dist/types)
5. `conformance`: Runs `npm test -- conformance` against bundled binary
6. `publish`: Runs `npm publish --access public --provenance`

**Key Features:**
- Container: `node:22-slim` (LTS)
- Pre-release detection: Versions with `-` suffix use `--tag rc`
- Idempotent: Treats npm 409 Conflict (duplicate publish) as success
- Resource limits: CPU 500m-2Gi, Memory 1Gi-4Gi
- TTL: 30min on success, 2hr on failure

### 3. Updated `pdftract-release-cascade.yaml`
**Location:** `jedarden/declarative-config/k8s/iad-ci/argo-workflows/pdftract-release-cascade.yaml`

- Changed `sdk-node-publish.when` from `"false"` to `"true"` to enable the template
- The Node.js SDK now publishes as part of the release cascade

## Acceptance Criteria

- [x] WorkflowTemplate file lands at the documented path in declarative-config
- [x] Follows the same pattern as `pdftract-sdk-go-publish.yaml`
- [x] Uses npm token from ExternalSecret `npm-token-pdftract`
- [x] Conformance step aborts publish if tests fail
- [x] Re-run is idempotent (duplicate publish treated as success)
- [x] Pre-release tags use `--tag rc`
- [x] Enabled in release cascade (`when: "true"`)
- [ ] Test run against a fresh tag publishes `@pdftract/sdk@X.Y.Z` to npm (requires npm token in OpenBao + SDK repo)
- [ ] Published package is installable via `npm install @pdftract/sdk@X.Y.Z` (requires npm token in OpenBao + SDK repo)

## WARN Items

- **npm token not yet in OpenBao**: The ExternalSecret references `rs-manager/iad-ci/npm/pdftract` which must be created manually before the first publish run
- **SDK repo does not exist**: The `jedarden/pdftract-node` repository must be created with a proper `package.json`, `npm run build` script, and conformance tests before the workflow can succeed
- **First publish test requires manual setup**: A full test run requires:
  1. Creating the npm token in OpenBao
  2. Creating the pdftract-node SDK repository
  3. Creating a milestone tag to trigger the cascade

## Notes

- The workflow assumes the SDK repository has a `package.json` with:
  - `name: "@pdftract/sdk"`
  - `scripts.build` that produces `dist/esm`, `dist/cjs`, `dist/types`
  - `scripts.test` with a `conformance` test suite
- The version sync step commits to the SDK repo directly, which means the SDK repo will have a version bump commit for each release
- npm provenance (`--provenance` flag) requires npm CLI v9.5+; if it fails, the workflow will error (graceful degradation could be added in a future bead)

## Commits

- jedarden/declarative-config: `feat(pdftract-62x5c): add pdftract-sdk-node-publish WorkflowTemplate and npm token ExternalSecret`
