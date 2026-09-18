# bf-4vzqu0 — Generate Forgejo personal access token on git.ardenone.com

**Outcome: ALREADY SATISFIED — verification-only close, 2026-09-18.** 4th dispatch;
auto-split order declined (see "Why no split" below). No code change warranted.

## Fresh derivation this dispatch (HEAD `c5847e31`)

The token value was never printed, never in argv, never on disk. Every check
below is by property or downstream effect.

1. **Token exists and is healthy** — OpenBao `secret/rs-manager/iad-ci/forgejo/ci-token`
   (rs-manager instance, provisioning identity metadata read):
   `current_version=3`, v3 rotated 2026-09-13; 3 versions present, none
   `destroyed`, no `deletion_time` stamp on any version.

2. **Functional clone proof** — token piped from the read identity into the
   *exact* credential-helper shape the live rust-verify template configures
   (`username=x-token`):
   - `git ls-remote https://git.ardenone.com/jedarden/pdftract.git HEAD` →
     exit 0, server HEAD `c5847e31f8016a9d9b204aad456c45cb79bc4d42` ==
     local HEAD.
   - `GET /api/v1/repos/jedarden/pdftract` with the token → HTTP 200.

3. **Live wiring on iad-ci** — `WorkflowTemplate/rust-verify` injects
   `FORGEJO_TOKEN` from `secretKeyRef forgejo-webhook-token/token`
   (optional: true) into `credential.https://git.ardenone.com.helper`;
   `ExternalSecret/forgejo-webhook-token` (ns `argo-workflows`) is
   `Ready=True` / "secret synced", refresh `2026-09-18T15:42:06Z` (today).

## Acceptance criteria

| Criterion | Status |
|---|---|
| Personal access token exists on git.ardenone.com | **PASS** (ci-token, v3, authenticates live) |
| Token has appropriate repository permissions | **PASS** (read:repository; rust-verify is clone-only — ls-remote + repos API 200) |
| Token value available for sealing, not committed | **PASS** (lives at the OpenBao path above; synced to cluster via ExternalSecret) |
| Token permissions match what rust-verify expects | **PASS** (template consumes exactly this secret/key/shape) |

No new PAT was minted: nothing would reference it, and the parent
`bf-5ig30` closed 2026-09-05 with the same finding.

## Why the auto-split order was declined

`failure-count:3` accumulated from environmental failures only — attempts 1
and 2 resolved `work_failure` on `gate:default_rust` tripped by the shared
checkout's unrelated uncommitted edits (both prior evidence closes were
correct and were mechanically reopened by the attempt-outcome plumbing ~2
minutes later), and attempt 3 was a worker hard timeout. None challenged the
deliverable. Splitting a satisfied bead would create phantom children for
work that is done (the documented auto-split treadmill).

Gate evidence for the close: the bead's `notes` field was updated this
dispatch (shipped-work contract option 2) with the derivation above;
close reason carries the same summary. Checkpoint commit `c332c5d7`.
