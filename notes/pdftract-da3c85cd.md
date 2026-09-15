# pdftract-da3c85cd — Homebrew tap publish leg: children evidence index

**Type:** index note for the umbrella bead `pdftract-da3c85cd`
**Date:** 2026-09-15

The umbrella was auto-split (2026-09-15) into 5 chained children. True state at
split time: the cascade leg was already wired+synced in declarative-config
(`dbec30ae`) but its templateRef was dangling — the drafted template sat
untracked in-tree with a dry_run defect, and the tap repos did not exist. This
index ties each child to its evidence so the umbrella can close on it.

| # | Child | Scope | Status | Evidence |
|---|-------|-------|--------|----------|
| 1 | `pdftract-6ea73858` | Create `jedarden/homebrew-tap` on Forgejo + GitHub | Closed 2026-09-15T05:32Z | Close reason: Forgejo repo exists via push-to-create, `main=c7aec641d5cf9e638285fdbc706499f19495f6a2` (initial README commit; content cross-verified); GitHub repo public, default branch main; Forgejo→GitHub push mirror present with `sync_on_commit=true`, interval 8h |
| 2 | `pdftract-fa57398f` | Wire the tap-push credential path by reference (OpenBao → ExternalSecret → k8s Secret → git credential helper) | Closed 2026-09-15T12:00Z | Close reason: identity/policy wiring verified live (`rs-manager-provision` AppRole authenticates; capabilities on `secret/data/rs-manager/iad-ci/forgejo/homebrew-tap-push-token`); value itself blocked at the Forgejo token mint boundary — a named operator action. The ExternalSecret is deliberately NOT enabled until then, so the leg runs dry_run-only — exactly what child 5's pass exercised |
| 3 | `pdftract-cdb3f3e1` | Fix + land the drafted template in-tree | Closed 2026-09-15T04:50Z | `notes/pdftract-cdb3f3e1.md`; commit `8007c6b3` pushed (server tip verified). Fixed the dry_run contract: `render-formula` takes the formula template from default branch `main` when dry_run, so a fake tag never clones a nonexistent ref |
| 4 | `pdftract-3898462e` | Publish the template through declarative-config and confirm cluster sync | Closed 2026-09-15T11:20Z (attempt 3) | Close reason: `origin/main` of declarative-config carries `k8s/iad-ci/argo-workflows/pdftract-homebrew-publish.yaml` byte-identical to the in-tree file (landing `1a3fca4f`, verified against the working tree and the committed blob at HEAD); the gate that failed attempts 1-2 was repaired and landed. Live template observed synced in-cluster by child 5 (created 2026-09-15T06:25:00Z) |
| 5 | `pdftract-3552e9a8` | Dry-run evidence pass (this pass) | Closed 2026-09-15 | `notes/pdftract-3552e9a8.md` — 5 standalone dry-run submissions on iad-ci: fake-tag `v9.9.9` runs Succeed with gate PASS, render executed, in-container `ruby -c` → `Syntax OK`, and push-tap/wait-for-mirror/brew-verify all Skipped; `tag=main` run Succeeds with the whole leg omitted (`proceed=false`); `git ls-remote` on both tap remotes unchanged before/after (`c7aec641…`); zero secret churn; no credential touched |

## State after all children close

- Template in-tree and synced; cascade leg `homebrew-publish` invokes it with
  `continueOn: failed` after `pdftract-github-release`.
- Tap repos exist and are empty-but-for the README; origin and mirror agree.
- The leg is **live for dry runs only** until the Forgejo tap-push token is
  minted and the ExternalSecret enabled (operator action named in child 2's
  close reason). Enabling it turns on real pushes; nothing else needs to change.
- The end-to-end `brew install` verification is deliberately **not** part of
  this umbrella: it belongs to `pdftract-f6cf828b` ("Verify brew install
  pdftract end-to-end and update the README channel status"), which is blocked
  by this umbrella and needs a real release to exist.
