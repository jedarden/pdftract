# pdftract-1lw3 Verification Note

## Bead: Argo WorkflowTemplate: pdftract-release-cascade (orchestrate all per-channel templates from a tag)

Date: 2026-05-20
Commit: b06a448d65861c4de7005d7c3b9a274098656e76

## Summary

Implemented the top-level pdftract-release-cascade WorkflowTemplate that orchestrates all per-channel publish templates in correct dependency order. Also created the Argo Events sensor that automatically triggers the cascade on milestone tag pushes.

## Files Created/Modified

### jedarden/declarative-config

1. k8s/iad-ci/argo-workflows/pdftract-release-cascade.yaml (442 lines)
   - DAG topology with correct dependency ordering
   - parallelism: 10 for wide fan-out to SDK templates
   - continueOn: failed for failure isolation
   - Re-runnable via argo retry
   - Release summary exit handler with per-channel status reporting
   - SDK publish templates as placeholders (when: false) for Phase 8

2. k8s/iad-ci/argo-events/pdftract-tag-trigger.yaml (139 lines)
   - Argo Events Sensor for Forgejo webhook integration
   - Filters for v*.*.* milestone tag pushes only
   - Extracts tag, version, commit-sha from webhook payload
   - Triggers cascade with correct parameters

3. k8s/iad-ci/argo-events/forgejo-eventsource.yml (+4 lines)
   - Added pdftract webhook entry on port 12000, endpoint /pdftract

4. k8s/iad-ci/argo-events/webhook-ingressroute.yml (+6 lines)
   - Added Traefik route for /pdftract path on webhooks-ci.ardenone.com

## Acceptance Criteria Status

### PASS
- [PASS] WorkflowTemplate file lands at correct path in jedarden/declarative-config
- [PASS] The Argo Events Sensor lands at correct path in jedarden/declarative-config
- [PASS] The Sensor is wired to the pdftract repo's tag-push events via forgejo-eventsource and webhook-ingressroute
- [PASS] DAG topology correctly encodes dependencies
- [PASS] Failure isolation implemented via continueOn: failed at per-channel task level
- [PASS] Parallelism set to 10 to avoid cluster starvation
- [PASS] SDK templates are placeholders (when: false) for Phase 8 implementation
- [PASS] Release summary exit handler reports per-channel status
- [PASS] Manual launch instructions documented in sensor comments
- [PASS] ADR-009 compliance: cascade orchestration is purely on iad-ci cluster

### WARN
- [WARN] Test cascade submission against a fresh tag could not be verified (kubectl not available)
- [WARN] Simulated failure of pdftract-py-ci could not be tested (kubectl not available)
- [WARN] Cascade re-run behavior could not be verified (kubectl not available)
- [WARN] Slack/Discord notification is implemented as console output; actual webhook integration would require additional ExternalSecret

## DAG Topology

pdftract-build-binaries (root)
  ├── pdftract-py-ci (Python wheels to PyPI)
  ├── pdftract-crates-publish (Rust crates to crates.io)
  │   └── pdftract-docs-build (mdBook to Cloudflare Pages)
  ├── pdftract-docker-build (multi-arch images to GHCR)
  ├── pdftract-sdk-node-publish (npm - Phase 8, when: false)
  ├── pdftract-sdk-go-publish (go module - Phase 8, when: false)
  ├── pdftract-sdk-java-publish (Maven Central - Phase 8, when: false)
  ├── pdftract-sdk-dotnet-publish (NuGet - Phase 8, when: false)
  ├── pdftract-sdk-ruby-publish (RubyGems - Phase 8, when: false)
  ├── pdftract-sdk-php-publish (Packagist - Phase 8, when: false)
  ├── pdftract-sdk-swift-publish (SPM - Phase 8, when: false)
  └── pdftract-github-release (joins on all above)

## Webhook URL
https://webhooks-ci.ardenone.com/pdftract

## References
- Plan section: Release Engineering / Argo WorkflowTemplates, line 3382
- Plan section: Release Engineering Acceptance Criteria, lines 3439-3448
- ADR-009: Argo-only CI/CD
