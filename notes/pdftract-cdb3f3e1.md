# pdftract-cdb3f3e1 — Land the pdftract-homebrew-publish WorkflowTemplate in-tree

**Type:** task (child 1 of 5, parent pdftract-da3c85cd)
**Date:** 2026-09-15

## What was found at dispatch (re-checked, per bead instruction)

The bead described the draft as UNTRACKED. That had moved: sibling bead
`pdftract-d0e30c13` (open, unassigned) had already committed the file at
`c43f0482` ("ci(pdftract-d0e30c13): land pdftract-homebrew-publish tap
template in-tree", 2026-09-14 23:55) and pushed it — `origin/main` and local
`main` were identical (0/0 ahead-behind) at dispatch. So criterion 1 (tracked
at HEAD, pushed) was already satisfied, **but the committed content still
carried both known defects**, so the review/fix work was real:

- **Defect A present at HEAD:** `render-formula` unconditionally ran
  `git clone --depth 1 --branch ${TAG}`, so the documented dry-run contract
  (fake tag `v9.9.9`) failed at the clone — the tag has no ref.
- **Defect B present at HEAD:** header asserted an ExternalSecret
  ("homebrew-tap-push-token-externalsecret.yml.disabled alongside this file
  in declarative-config") that does not exist (the credential sibling authors
  it).

Also re-verified nothing else moved:

- declarative-config `origin/main` still carries `dbec30ae` adding the
  cascade leg `homebrew-publish` (invoke-workflow → templateRef
  `pdftract-homebrew-publish`, params `{"tag","version","dry_run"}`,
  `continueOn: failed`) — param names match the template's parameters.
- Cluster (credential-free `kubectl --server=http://traefik-iad-ci:8001`)
  has `pdftract-release-cascade` but **not** `pdftract-homebrew-publish` —
  the dangling templateRef is real until this lands and syncs.

## Changes (both defects fixed, everything else kept as drafted)

`.ci/argo-workflows/pdftract-homebrew-publish.yaml`:

1. **Defect A** — the clone target is now `TEMPLATE_REF`, which is the tag in
   normal runs and the repo default branch `main` in dry runs (with a loud
   `=== DRY RUN: ... ===` marker); the not-present error message names
   `TEMPLATE_REF`. Header, render-formula step comment, and the dry-run
   contract comment updated to match. No code path clones a nonexistent ref
   any more.
2. **Defect B** — the credential paragraph now describes the *intended end
   state* ("authored by the credential sibling; none of it exists yet"), the
   OpenBao path (`secret/rs-manager/iad-ci/forgejo/homebrew-tap-push-token`)
   stays as the corrected §8.3 path, and the push-tap step comment no longer
   implies the ExternalSecret sync already exists.

Images untouched (`alpine:3.19` ×4, `homebrew/brew:4.6.20` default); gate
regex untouched; secretKeyRef reference untouched.

## Verification

| Check | Result |
|---|---|
| YAML parse (`python3 -c yaml.safe_load`) | **PASS** — WorkflowTemplate `pdftract-homebrew-publish`, 6 templates, `activeDeadlineSeconds: 10200` |
| Dry-run contract, fake tag executed | **PASS** — extracted the render step's script verbatim from the YAML (Argo params substituted: `tag=v9.9.9 version=9.9.9 dry_run=true`), ran locally with `apk` stubbed (no apk locally), `git clone` stubbed to serve the real `packaging/homebrew/pdftract.rb.template`, `ruby` stubbed (see WARN), and the two container mount points (`/src`, `/formula`) rewritten to a temp dir. Exit 0: stub archive hashed, template taken **from `main`**, all `<VERSION>`/`<SHA256>` placeholders substituted, no leftover placeholder, exactly one 64-hex sha256 line |
| Non-dry-run rejects fake tag | **PASS** — same script with `dry_run=false` fails at the archive download (curl 404, exit 22) **before** the clone; no `/src` created. The real path can never render a nonexistent tag |
| Dry-run ref reachable in-cluster | **PASS** — `git ls-remote https://github.com/jedarden/pdftract.git refs/heads/main` → `c89bd9d3`; that commit is an ancestor of local `main` (mirror lags Forgejo, not diverged) and `packaging/homebrew/pdftract.rb.template` exists at it (`git cat-file -e`) |
| Gate regex = strategy §6 | **PASS** — simulated the gate for 11 inputs: `v1.2.3`, `v9.9.9`, `v01.0.0` PASS; empty, `v1.2.3-rc.1`, `latest`, `vlatest`, `main`, `abc123def`, `v1.2`, `V1.2.3` all SKIP. SKIP writes `proceed=false`; every downstream DAG task has `when: ... == true` (plus `dry_run != true` on push/wait/verify), so a skip is a Succeeded no-op |
| No credential literal | **PASS** — the only secret wiring is `secretKeyRef: {name: homebrew-tap-push-token, key: token}` (lines 356-358) and the credential-helper line referencing `$TAP_TOKEN` by env indirection; no value anywhere |
| No `:latest` image | **PASS** — 5 grep hits, all prose (comments/gate messages saying `:latest` is rejected); actual images: `alpine:3.19` ×4 + `homebrew/brew:4.6.20` parameter default |
| Header ExternalSecret assertion removed | **PASS** — now "intended end state ... none of it exists yet" |
| `ruby -c` on rendered formula | **WARN** — no ruby on codinghome; stubbed exit-0 in the local render test. The real syntax check runs in-cluster (alpine `apk add ruby`) and is supplied by the final sibling's in-cluster render |

Local render-test scratch (`/tmp/render-test`) removed after the run.

## Out of scope here

- declarative-config landing of the template + in-cluster sync, the
  ExternalSecret/credential provisioning, and the real in-cluster dry-run
  render — the remaining siblings under pdftract-da3c85cd.
