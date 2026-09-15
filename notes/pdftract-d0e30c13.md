# pdftract-d0e30c13 — umbrella: Homebrew tap formula publishing chain

Date: 2026-09-15. Status at close: **all three split-children closed with
evidence; every umbrella-level deliverable verified live at HEAD.**

## What this umbrella covers

The README claims `brew install jedarden/tap/pdftract` against
jedarden/homebrew-tap with formulas generated from versioned release
archives. The umbrella owned standing up that pipeline: a WorkflowTemplate
that downloads the versioned archives, verifies them against SHA256SUMS,
renders the formula (version + per-platform SHA256s), and pushes it to the
tap with the standard git identity — plus the credential behind the push and
the client-side install-path verification.

## Structure: the auto-split was already executed

A dispatcher split order against this bead was **already executed earlier on
2026-09-15**: the bead carried `umbrella` + `auto-split-parent`, and a
complete sequential child chain existed at claim time (epoch 6):

```
pdftract-54cad4cb  (credential: OpenBao token + ExternalSecret on iad-ci)
  └─ blocks pdftract-c0413792  (smoke-run the publish template; first formula)
       └─ blocks pdftract-09cd4c1f  (client install path + README claims)
            └─ blocks pdftract-d0e30c13  (this umbrella)
```

All three children carry `split-child` + `parent-pdftract-d0e30c13`.
A second-generation split order (this dispatch, "failed 5 times in a row")
was **declined**: it would create duplicate children over already-satisfied
scope (repo rule: two independent "implement X" beads for the same X is not
fine), and its premise is contradicted by the dispatch's own attempt log —
attempts 4 and 5 resolved `verified_success`; the `failure-count:5` label is
fed by reason-less reopens, not by any failed gate.

## Per-child outcomes and evidence

| Child | Status | Evidence |
|---|---|---|
| `pdftract-54cad4cb` | closed (3rd evidence close) | `notes/pdftract-54cad4cb.md` + 3 addenda; commits `e20ecf8c`, `419d06c8`, `a6bf9282`; declarative-config `1b7e985c`, `1dd760f4`. WARN: dedicated Forgejo token mint needs operator WebAuthn — CI token reused, upgrade path in manifest header |
| `pdftract-c0413792` | closed | `notes/pdftract-c0413792.md`; commit `6dc10b0f`. Smoke runs of the publish template PASS; positive path (formula lands in tap) WARN blocked-on-first-release — tap 404s until the first vX.Y.Z release exists |
| `pdftract-09cd4c1f` | closed | `notes/pdftract-09cd4c1f.md`; commit `22e92864`. Install path verified; stale Homebrew deferral corrected in installation docs |

## Umbrella-level live verification (this session, 2026-09-15)

| Parent criterion | Result |
|---|---|
| WorkflowTemplate generating formulas from release artifacts | **PASS** — `workflowtemplate.argoproj.io/pdftract-homebrew-publish` live on iad-ci (`kubectl --server=http://traefik-iad-ci:8001`); `k8s/iad-ci/argo-workflows/pdftract-homebrew-publish.yaml` at declarative-config origin/main tip |
| Downloads archives, verifies SHA256SUMS, renders formula, commits to tap with standard identity | **PASS** — template steps verified at origin/main: formula rendered from `packaging/homebrew/pdftract.rb.template` ("Formula rendered from … release cascade" commit message, git identity `github@jedarden.com`), push to `git.ardenone.com/jedarden/homebrew-tap.git` via credential helper |
| Credential available to the push leg | **PASS** — ExternalSecret `homebrew-tap-push-token` (argo-workflows, iad-ci): `SecretSynced=True`; backing KV `secret/rs-manager/iad-ci/forgejo/homebrew-tap-push-token` `current_version=1` (metadata via `rs-manager-provision`; value never read) |
| Wired into the release cascade | **PASS** — `pdftract-release-cascade.yaml` at origin/main carries task `homebrew-publish` → templateRef `pdftract-homebrew-publish`, ordered after `github-release` |
| Verify with smoke | **PASS/WARN** — smoke runs exercised by `pdftract-c0413792`; positive path blocked on the first tagged release (infra, out of scope here) |

## Residual WARNs (documented, not blockers)

1. Dedicated least-privilege Forgejo token awaits an operator WebAuthn mint
   (write as the **next KV version** of the existing path — the ESO's 1h
   refresh picks it up unchanged).
2. No formula exists in the tap yet by design: the leg gates on a tagged
   `vX.Y.Z` release (strategy §6). First tagged release publishes the first
   formula; the README install claim becomes literally true then.

## Artifacts

- This note; child notes as tabled above.
- declarative-config (origin/main): `pdftract-homebrew-publish.yaml`,
  `homebrew-tap-push-token-externalsecret.yml`, cascade wiring in
  `pdftract-release-cascade.yaml`.
- OpenBao (rs-manager): `secret/rs-manager/iad-ci/forgejo/homebrew-tap-push-token` v1.
- Strategy: `docs/notes/homebrew-tap-strategy.md` (§4–§8).
