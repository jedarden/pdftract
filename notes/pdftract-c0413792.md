# pdftract-c0413792 — pdftract-homebrew-publish smoke run; positive path blocked-on-release

Date: 2026-09-15. Parent umbrella: pdftract-d0e30c13. Preceded in-chain by
pdftract-54cad4cb (credential bead — its precondition verified below).

## Outcome in one line

The never-run `pdftract-homebrew-publish` WorkflowTemplate was exercised
standalone on iad-ci for the first time (two runs, both **Succeeded**): the
required negative-path run proved the versioned-tag re-gate is a clean no-op
skip, and a supplementary dry-run proved the render+validate half end to end
with a hash verified out-of-band. The positive path (render+push landing
`Formula/pdftract.rb` on the tap) is **blocked-on-release**: no vX.Y.Z tag or
release exists anywhere yet, so that leg of the contract cannot run until the
first `pdftract-github-release` happens. Exact missing artifact documented
below.

## Pre-flight checks

| Check | Result |
|---|---|
| Template applied on iad-ci | ✅ `workflowtemplate/pdftract-homebrew-publish` present; args: `repo`, `tap-repo`, `tap-url`, `tag`, `version`, `dry_run`, `brew-image` |
| Cascade arg shape (`pdftract-release-cascade.yaml` ~L389) | ✅ leg passes `{"tag", "version", "dry_run"}` only — all other params use template defaults |
| Credential precondition (prior bead's deliverable) | ✅ ExternalSecret `homebrew-tap-push-token` (ns `argo-workflows`, ClusterSecretStore `openbao`): **SecretSynced=True**, ready ~48m before the run |
| Prior homebrew runs | ✅ 0 homebrew-named workflows ever existed on iad-ci (full workflow sweep) — these are the first |
| Qualifying input tag | ❌ none exists — see "Positive path blocked-on-release" |

## Run 1 (required deliverable): negative-path no-op

`kubectl create` (iad-ci kubeconfig) of a standalone Workflow with
`workflowTemplateRef: pdftract-homebrew-publish`,
`tag=v0.1.0-test` (a **real** tag — the only one in the repo — but not
`vX.Y.Z`-shaped), `version=0.1.0-test`, `dry_run=false`.

**Run:** `pdftract-homebrew-publish-manual-fpvlw`
**Phase: Succeeded** — 21:52:52Z → 21:53:02Z (10 s).

Node status (the controller's own durable record):

| Node | Phase | Evidence |
|---|---|---|
| `versioned-tag-gate` | Succeeded | exitCode 0, output parameter **`proceed=false`** (captured from `/tmp/proceed`) |
| `render-formula` | Skipped | `when 'false == true' evaluated false` |
| `push-tap` | Skipped | `when 'false == true && false != true' evaluated false` |
| `wait-for-mirror` | Skipped | same |
| `brew-verify` | Skipped | same |

The gate rejected a real, resolvable, immutable tag purely on version shape —
exactly the contract (strategy §6: rc/prerelease/alias/branch/SHA refs are
never published). With `dry_run=false` (credential path armed), the run still
touched nothing: tap `main` remained at `c7aec64` before and after
(`git ls-remote`), i.e. no push, no credential use.

**Log-evidence caveat (why node status, not pod logs):** the gate pod lives
~1–2 s and `podGC: OnPodCompletion` deletes it on exit; iad-ci has no artifact
repository, so stdout of a completed pod is unrecoverable by design. Two
attach attempts (one live 1 s-poll loop started immediately after create)
missed the sub-second pod. The Workflow object's node status — including the
captured `proceed` output parameter value and each Skipped node's evaluated
`when` expression — is the durable evidence for this run, quoted in full
above. The template's own design acknowledges this (formula travels as a
base64 *parameter*, not an artifact, for the same reason).

## Run 2 (supplementary): dry-run render+validate

`tag=v9.9.9`, `version=9.9.9`, `dry_run=true` (the template header's
documented pre-first-release exercise mode — renders against a stub archive,
takes the template from `main`, stops before any push; no credential
involved).

**Run:** `pdftract-homebrew-publish-dryrun-w2z6f`
**Phase: Succeeded** — 21:57:22Z → 21:58:02Z (40 s).

| Node | Phase | Note |
|---|---|---|
| `versioned-tag-gate` | Succeeded | gate PASS direction also exercised (v9.9.9 is versioned-shaped) |
| `render-formula` | Succeeded | full render+validate path: stub hashed, template fetched from `main`, placeholders substituted, exactly-one-sha256-line check, 64-hex check, `ruby -c` parse — all passed |
| `push-tap` / `wait-for-mirror` / `brew-verify` | Skipped | `dry_run=true` — as designed |

The rendered formula was recovered from the node status itself
(`outputs.parameters["formula-b64"]`, base64-decoded): structurally complete —
`url …/archive/refs/tags/v9.9.9.tar.gz`, `version "9.9.9"`, single
`sha256 "1f6189084d26cc9bc8436275db35c0358dc8792cd21c2276db18e41eb186ad43"`,
`depends_on "rust"`, build-from-source `cargo install --locked`, livecheck,
test block. (Render pod also finished before live attach; same podGC caveat —
the recorded output parameter is the evidence, and it is the actual payload
bytes, strictly stronger than a log line.)

**Out-of-band hash verification (acceptance criterion's "never trust the
workflow log alone"):** the dry-run stub archive is deterministic, so it was
regenerated locally from the template's exact two `printf` statements —
local `sha256sum` yields `1f6189…ad43`, identical to the sha256 recorded in
the rendered formula. The render step's hash-and-substitute pipeline is
thereby verified independently of anything the workflow printed.

## Positive path blocked-on-release — exact missing artifact

**Missing: the first `vX.Y.Z` release itself.** Verified 2026-09-15:

- Tags on both Forgejo and the GitHub mirror: only `v0.1.0-test` (fails the
  gate regex `^v[0-9]+\.[0-9]+\.[0-9]+$`; proven by Run 1).
- Releases: Forgejo API `repos/jedarden/pdftract/releases` → `[]`;
  GitHub releases → none. Therefore **no** release-published source archive
  at `https://github.com/jedarden/pdftract/archive/refs/tags/vX.Y.Z.tar.gz`
  and **no** `SHA256SUMS` release asset to cross-check against.
- Upstream producer: **0** `pdftract-github-release` or
  `pdftract-release-cascade` workflows have ever run on iad-ci — the leg that
  creates the tag, publishes the release assets, and emits the aggregate
  `SHA256SUMS` (+ cosign sig) has never executed.

Everything the positive path needs *besides* the release is proven ready:
template applied, push credential synced (`SecretSynced=True`), gate proven in
both PASS and SKIP directions, render+validate proven end to end with an
out-of-band-verified hash, and the Forgejo tap push target reachable (its
anonymous clone — the same URL the push step clones — was exercised for the
before/after state checks). When the first vX.Y.Z release lands, re-run this
template standalone with `-p tag=vX.Y.Z -p version=X.Y.Z`; the render step
will hash the real archive bytes the formula `url` serves, and the run
delivers `Formula/pdftract.rb` + mirror catch-up + `brew install` verify.

## Acceptance criteria

- **PASS (primary, render+push on a real tag):** not applicable this run —
  covered instead by the sanctioned fallback below (no qualifying tag exists;
  this is *not* a WARN on the workflow, which behaved exactly to spec).
- **PASS (fallback):** negative-path run `…-manual-fpvlw` proved the re-gate
  no-op (Succeeded, `proceed=false`, all four downstream nodes Skipped, tap
  untouched at `c7aec64`); positive path documented blocked-on-release with
  the exact missing artifact (§ above).
- **PASS (evidence + hygiene):** run names, phases, and per-node evidence
  recorded in this note; no orphaned processes
  (`pgrep -af 'pdftract[ ]mcp|TH[_]0|TH[-]0'` → empty; both background
  watchers exited cleanly).
- **WARN allowance (mirror lag):** not triggered — no push occurred, so there
  was no mirror event to lag.

## References

- Template: `.ci/argo-workflows/pdftract-homebrew-publish.yaml` (also live on
  iad-ci; declarative-config dbec30ae)
- Cascade leg: `declarative-config/k8s/iad-ci/argo-workflows/pdftract-release-cascade.yaml`
  (~L389, `continueOn: failed`, strictly after `github-release`)
- Credential bead: `notes/pdftract-54cad4cb.md` (token path, ESO enablement)
- Strategy: `docs/notes/homebrew-tap-strategy.md` §4–§8
- Tap state: `https://git.ardenone.com/jedarden/homebrew-tap` @ `c7aec64`
  (Formula/.gitkeep only — first formula awaits the first release)
