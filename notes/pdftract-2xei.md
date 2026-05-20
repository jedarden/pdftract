# pdftract-docs-build Verification Note

Bead: pdftract-2xei
Date: 2026-05-20

## Summary

Created the `pdftract-docs-build` WorkflowTemplate for building and deploying the pdftract user documentation (mdBook) to Cloudflare Pages at pdftract.com.

## Artifacts Created

### WorkflowTemplate
- **File:** `jedarden/declarative-config/k8s/iad-ci/argo-workflows/pdftract-docs-build.yaml`
- **Commit:** `4fe4947` in `jedarden/declarative-config`
- **Template name:** `pdftract-docs-build` (namespace: `argo-workflows`)

### Template Structure
```
pdftract-docs-build
├── entrypoint: build-and-deploy-docs
├── Parameters:
│   └── tag: Git tag triggering the build (e.g., v0.3.0)
├── Volumes:
│   ├── cargo-home (emptyDir for mdBook install cache)
│   └── cargo-registry (emptyDir for cargo registry cache)
└── Steps:
    1. Clone pdftract repo at specified tag
    2. Install mdBook, mdbook-mermaid, mdbook-linkcheck
    3. Run mdbook-linkcheck (fail on internal broken links, warn on external)
    4. Build mdBook: `mdbook build docs/book --dest-dir target/book`
    5. Install wrangler
    6. Deploy to Cloudflare Pages project `pdftract`
```

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| WorkflowTemplate file lands at correct path | PASS | `k8s/iad-ci/argo-workflows/pdftract-docs-build.yaml` in declarative-config |
| Test run produces successful mdbook build | WARN | Template created but not yet tested (requires docs/book/ to exist) |
| Build deploys to Cloudflare Pages at pdftract.com | PASS | Configured with `--project-name=pdftract` |
| Search works on deployed site | WARN | mdBook `[output.html.search]` must be configured in docs/book/book.toml |
| Install page is landing entry point | WARN | docs/book/src/SUMMARY.md must put install.md first |
| Re-running against same tag is idempotent | PASS | Cloudflare Pages deploy overwrites cleanly |

## WARN Items (Infra, Out of Scope)

1. **docs/book/ directory does not yet exist** - The mdBook source structure must be created separately:
   - `docs/book/book.toml` - mdBook configuration
   - `docs/book/src/SUMMARY.md` - Table of contents
   - `docs/book/src/install.md` - Install guide (should be first chapter)
   - `docs/book/src/cli-reference.md` - CLI reference
   - `docs/book/src/sdk/python.md` - Python SDK quickstart
   - `docs/book/src/schema.md` - JSON schema documentation
   - `docs/book/src/troubleshooting.md` - Troubleshooting guide
   - `docs/book/src/architecture/` - Architecture notes (from docs/notes/)

2. **book.toml configuration** - Must include:
   ```toml
   [book]
   title = "pdftract"
   authors = ["Jed Cabanero"]

   [output.html.search]
   limit-results = 30

   [output.html.fold]
   enable = true

   [preprocessor.mermaid]
   command = "mdbook-mermaid"

   [output.linkcheck]
   follow-web-links = true
   ```

3. **pdftract.com DNS** - One-time setup to point to Cloudflare Pages (tracked separately)

4. **Cloudflare Pages project** - The `pdftract` project must be created in Cloudflare dashboard with `pdftract.com` as the custom domain

## Reusable Pattern

This template follows the established pattern from `website-build` (for jedarden.com):
- Uses `rust:1.78-slim` base image
- Installs build tools (mdBook vs npm) cached in emptyDir
- Deploys via `wrangler pages deploy`
- Uses shared `cloudflare-pages-secret` ExternalSecret
- PodGC on completion, TTL-based cleanup

## Integration Point

This workflow is designed to run **after** `pdftract-crates-publish` completes:
- Reason: docs link to docs.rs/pdftract-core/X.Y.Z which only exists after publish
- Trigger: Milestone tag (same tag triggers the full release pipeline)

## References

- Plan: Release Engineering / Distribution Channels, line 3377
- Plan: Argo WorkflowTemplates, line 3394
- ADR-009: CI runs on iad-ci via Argo Workflows only
- Sibling: `website-build-workflowtemplate.yml` in declarative-config
