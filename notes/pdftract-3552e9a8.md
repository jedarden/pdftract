# pdftract-3552e9a8 — Dry-run evidence pass for the homebrew publish leg

**Type:** task (child 5 of 5, terminal; parent pdftract-da3c85cd)
**Date:** 2026-09-15
**Cluster:** iad-ci, namespace `argo-workflows`
**Template:** `pdftract-homebrew-publish` (synced; creationTimestamp 2026-09-15T06:25:00Z)

## What this pass proves

The tap-publish leg's **gate → render → validate** path executes end to end in
dry_run with a fake version, nothing is published anywhere, no credential is
touched, and a non-versioned ref skips the whole leg. This is the evidence the
strategy note sanctions while no real release exists
(`docs/notes/homebrew-tap-strategy.md` §6 version policy, §7 cascade placement,
§8 item 1: dry-run is the sanctioned validation).

## Preconditions checked first

- Template actually synced to the cluster (not just in git):
  `kubectl --server=http://traefik-iad-ci:8001 get workflowtemplate
  pdftract-homebrew-publish -n argo-workflows` → exists, created
  2026-09-15T06:25:00Z (child 4's declarative-config landing).
- Template DAG shape (from the live template): `versioned-tag-gate` →
  `render-formula` when `proceed == true`; `push-tap`, `wait-for-mirror`,
  `brew-verify` additionally require `{{workflow.parameters.dry_run}} != true`.
- `homebrew-tap-push-token` Secret/ExternalSecret does **not** exist in the
  namespace — consistent with child 2's close reason (token mint is a pending
  operator action; the leg is dry_run-only until it lands). The dry run must
  not need it.

## Runs submitted

Sanctioned manual submission (`~/CLAUDE.md` "Submitting a workflow manually"):
`kubectl --kubeconfig=~/.kube/iad-ci.kubeconfig create -f -` with
`generateName: pdftract-homebrew-publish-manual-`, `workflowTemplateRef.name:
pdftract-homebrew-publish`, and `arguments.parameters` overrides. Five runs
total — runs 2, 3, 5 are identical re-submissions of run 1, made only because
`podGC: OnPodCompletion` deletes each pod the moment it finishes, so the first
attempt's logs were unrecoverable (two watcher bugs of mine, then a working
live-streaming watcher; details at the bottom).

| # | Workflow | tag | version | dry_run | Phase | gate | render | push-tap | wait-for-mirror | brew-verify |
|---|----------|-----|---------|---------|-------|------|--------|----------|-----------------|-------------|
| 1 | `...-78wcj` | v9.9.9 | 9.9.9 | true | **Succeeded** (41s) | Succeeded, `proceed=true` | Succeeded | Skipped | Skipped | Skipped |
| 2 | `...-48782` | v9.9.9 | 9.9.9 | true | **Succeeded** (30s) | Succeeded, `proceed=true` | Succeeded | Skipped | Skipped | Skipped |
| 3 | `...-lv4d8` | v9.9.9 | 9.9.9 | true | **Succeeded** | Succeeded | Succeeded | Skipped | Skipped | Skipped |
| 4 | `...-z99n9` | **main** | (empty) | true | **Succeeded** (10s) | Succeeded, `proceed=false` | **Skipped** | Skipped | Skipped | Skipped |
| 5 | `...-7mvtq` | v9.9.9 | 9.9.9 | true | **Succeeded** | Succeeded — log captured | Succeeded — log captured | Skipped | Skipped | Skipped |

Node phases above are from each workflow's `status.nodes` (workflow objects
carry a 30-min success TTL; node/parameter evidence below was extracted before
reaping where cited).

## Evidence A — gate passes for a versioned tag (runs 1/2/5)

Run 5 gate pod log (captured live, verbatim):

```
Gate PASS: v9.9.9 is a non-prerelease versioned tag
...level=INFO msg="sub-process exited" argo=true error=<nil>
...level=INFO msg="saving parameter" argo=true src=/tmp/proceed
```

Gate output parameter in all three v9.9.9 workflow objects: `proceed=true`.

## Evidence B — render + validate executes (runs 1/2/5)

Run 5 render pod log (captured live, validation sequence verbatim):

```
=== DRY RUN: stubbing the source archive for v9.9.9 (no fetch) ===
=== source archive sha256: 1f6189084d26cc9bc8436275db35c0358dc8792cd21c2276db18e41eb186ad43
=== DRY RUN: template from default branch 'main' (stub tag v9.9.9 has no ref to clone) ===
=== Fetching packaging/homebrew/pdftract.rb.template from main ===
Syntax OK
=== Rendered Formula/pdftract.rb for v9.9.9 (dry_run=true) ===
<Class pdftract ... full formula text ...>
```

`Syntax OK` is `ruby -c`'s output — the in-container Ruby parse check the
criterion names. The step runs under `set -eu` with `ruby -c ... || exit 1`,
so the Succeeded phase independently implies the parse passed in runs 1-3 as
well.

Cross-check from run 1's workflow object (independent of logs): the
`render-formula` node's `outputs.parameters.formula-b64` is stored in
`status.nodes`, and decoding it yields the rendered formula — `version "9.9.9"`,
`url ".../archive/refs/tags/v9.9.9.tar.gz"`, exactly one
`sha256 "1f618908…ad43"` 64-hex line, zero leftover `<VERSION>`/`<SHA256>`
placeholders. The stub archive (not a real fetch) is what that sha256 hashes —
expected and correct for a dry run.

## Evidence C — gate skips a non-versioned ref (run 4)

`tag=main`, `version=""`, `dry_run=true` → workflow **Succeeded in 10s**.
Node dump: `versioned-tag-gate` Succeeded with output `proceed=false`;
`render-formula`, `push-tap`, `wait-for-mirror`, `brew-verify` all **Skipped**
(Argo omits the tasks on the `when` condition). This is the gate-skip shape
per strategy §6: a skipped leg is a Succeeded workflow with every downstream
step omitted, never a failed release.

## Evidence D — nothing published by any run

- Baseline (before any run) and post-run (after all 5 runs) `git ls-remote`,
  both remotes, byte-identical:

  ```
  Forgejo  https://git.ardenone.com/jedarden/homebrew-tap.git  main=c7aec641d5cf9e638285fdbc706499f19495f6a2
  GitHub   https://github.com/jedarden/homebrew-tap.git        main=c7aec641d5cf9e638285fdbc706499f19495f6a2
  ```

  `c7aec641` is child 1's initial README commit — the tap is exactly as the
  tap-creation sibling left it. Mirror and origin agree.
- No `push-tap`/`wait-for-mirror`/`brew-verify` pod ever executed in any run —
  every node dump shows those three as `Skipped` (the DAG `when` conditions).
- No secret read: `kubectl get secrets -n argo-workflows` before vs after the
  runs is identical (diff empty); no `homebrew-tap-push-token` ExternalSecret
  or Secret was created; the template's only secret reference is push-tap's
  `secretKeyRef`, inside a step that never ran.

## Acceptance criteria

| Criterion | Verdict |
|---|---|
| dry_run=true + fake tag v9.9.9: gate passes, render-formula executes, rendered formula validates (`ruby -c` in-container), phase Succeeded, push-tap/wait-for-mirror/brew-verify absent | **PASS** (Evidence A, B; node tables) |
| Non-versioned-tag run: Succeeded with the whole leg omitted | **PASS** (Evidence C, run 4) |
| Nothing published anywhere by either run (ls-remote unchanged on both tap remotes) | **PASS** (Evidence D; holds across all five runs) |
| Verification notes committed to the pdftract repo with explicit pathspecs; no credential touched or needed | **PASS** (this note + `notes/pdftract-da3c85cd.md`; every submission was anonymous — the write kubeconfig touches only workflow creation, and no value-bearing path was read or written) |
| brew install not executed (no real release exists) | **WARN (allowed)** — that is post-umbrella sibling `pdftract-f6cf828b`'s evidence ("Verify brew install pdftract end-to-end", blocked by parent pdftract-da3c85cd) |

## Operational notes for whoever runs the real release

- `podGC: OnPodCompletion` means step logs are unrecoverable after a run — the
  rendered-formula text survives in the workflow object
  (`status.nodes[render-formula].outputs.parameters.formula-b64`, base64) until
  the TTL reaps the workflow (success: 1800s). To keep logs, stream them live
  (`kubectl logs -f` from pod creation) or temporarily override podGC on a
  debug workflow.
- All five runs were pure dry-runs: no tap commit, no secret, no state change
  anywhere. The four extra submissions cost nothing but cluster minutes.
- Verify-tooling footnote: my first two log-capture attempts failed for mundane
  reasons (a `#!/bin/bash` shebang on NixOS, a doubled `manual-` prefix in a
  label selector) — not template behavior. Recorded here so nobody re-litigates
  the template over those.

## Related

- Parent umbrella: `pdftract-da3c85cd` (index note: `notes/pdftract-da3c85cd.md`)
- Child 1 `pdftract-6ea73858` — tap repos created (Forgejo + GitHub, push mirror)
- Child 2 `pdftract-fa57398f` — credential path wired by reference (value pending operator mint)
- Child 3 `pdftract-cdb3f3e1` — template fixed + landed in-tree (`notes/pdftract-cdb3f3e1.md`)
- Child 4 `pdftract-3898462e` — declarative-config landing verified (byte-identical, synced)
