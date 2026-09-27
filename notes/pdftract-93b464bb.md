# pdftract-93b464bb — Integrate versioned Homebrew publication into the release cascade

Dispatch 2026-09-27 (attempt 9, claim_epoch 9). This bead arrived as an auto-split
order (failure-count:8), but the split already existed: the dispatcher itself had
created two child chains (54af116f→e72cc825→fc061b84→52f8e9ac and
09049a49→287f57bf→a242f32c→35ffe0a7), all eight children closed, and all three of
this bead's blockers (incl. sibling fb684d69) closed. Attempts 6–8 show the
split-template loop does not terminate here. Per the documented treadmill
resolution, the terminal action is re-derive-at-HEAD + evidence close.

## What this dispatch found and did

Re-verifying at HEAD found one genuinely open item the child closes did not
cover: **declarative-config was stale**. Its last homebrew sync (`04887db1`,
2026-09-26T20:52-04:00) predates the in-tree signed-handoff rework series
(e204d49b 05:37Z … acdb6501 07:37Z), so the live iad-ci cascade still ran the
pre-contract generation: the leg recomputed the archive digest from a second
download and the release producer wrote no `source/pdftract-vX.Y.Z.tar.gz` line
— exactly what AC 2 and `docs/operations/homebrew-release-handoff.md` reject.

Fixed by syncing the two contract-paired templates (consumer + producer were
changed together by e204d49b; syncing only one would leave the live pair
fail-closed):

- `k8s/iad-ci/argo-workflows/pdftract-homebrew-publish.yaml`
- `k8s/iad-ci/argo-workflows/pdftract-github-release.yaml`

declarative-config commit `1681e445` (pushed, HEAD on origin/main; pre-commit
incl. gitleaks Passed). Other drifted `.ci/` files (ci, docker-build,
nightly-fuzz, …) are out of this bead's scope and left untouched.

## Acceptance criteria — verified at HEAD

| AC | Verdict | Evidence |
| --- | --- | --- |
| Cascade invokes formula rendering; publishes `Formula/pdftract.rb` only for a versioned release tag | PASS | Live `pdftract-release-cascade` (iad-ci) has `homebrew-publish` step, `dependencies: [github-release]`, `template-name: pdftract-homebrew-publish`, `continueOn: failed`; `versioned-tag-gate` accepts only `^v[0-9]+\.[0-9]+\.[0-9]+$` (declarative-config `pdftract-release-cascade.yaml:383-405`, `.ci/argo-workflows/pdftract-homebrew-publish.yaml:259-294`); render re-gates as defense in depth (`:485`). Test: `test_non_versioned_tags_are_successful_noops` |
| Workflow consumes the release archive and SHA256SUMS, not a recomputed/floating artifact | PASS (in-tree; landed live via 1681e445) | `verify-release-metadata` downloads published SHA256SUMS/.sig/.pem, cosign-verifies against the iad-ci OIDC identity, extracts the signed source-archive digest; render consumes the handoff and never re-hashes (`.ci/.../pdftract-homebrew-publish.yaml:338-417`); producer writes the signed line (`.ci/.../pdftract-github-release.yaml`). Tests: `test_release_producer_hands_off_signed_source_archive_digest`, `test_render_consumes_checksum_handoff_without_archive_fetch_or_rehash`, `test_invalid_handoff_fails_before_renderer_and_push` |
| No `:latest` image or release URL; no credential value in manifests/args/logs/docs | PASS | Images pinned (`alpine:3.19`, `homebrew/brew:4.6.20`); the only `:latest` strings are policy comments rejecting it. `TAP_TOKEN` only via `secretKeyRef: homebrew-tap-push-token` (ExternalSecret from OpenBao `secret/rs-manager/iad-ci/forgejo/homebrew-tap-push-token`) → git credential helper; never in URL/argv/log. Tests: `test_workflow_images_are_not_floating`, `test_approved_tap_urls_are_pinned_and_secrets_are_only_referenced` |
| Documented failure/retry behavior; no silent success on tap failure | PASS | "Failure and retry behavior" section (`.ci/.../pdftract-homebrew-publish.yaml:61-95`) + handoff doc "Failure and retry contract"; bounded retries, wait-for-mirror times out LOUD, brew-verify fails unless `pdftract --version` matches, cascade `continueOn` keeps the leg Failed. Tests: `test_failed_tap_update_retries_without_false_success`, `test_render_failure_is_retryable_but_cannot_reach_tap`, `test_publication_waits_for_render_validation_and_uses_bounded_retry` |
| Dry-run/validation proves the rendered formula before any push | PASS | `dry_run=true` renders via the real `render_formula.py` + `ruby -c` against a deterministic checksum fixture; push/wait/verify steps omitted; push Secret never mounted (`.ci/.../pdftract-homebrew-publish.yaml:48-59,363-376`). Tests: `test_dry_run_proves_archive_and_sha256sums_handoff_locally`, `test_unchanged_formula_is_an_idempotent_success` |

## Commands run this dispatch

```
$ pytest packaging/homebrew/test_workflow_contract.py packaging/homebrew/test_render_formula.py -q
18 passed, 5 subtests passed in 0.47s   (exit 0)

$ python3 packaging/homebrew/test_workflow_contract.py    # 12 tests, OK
$ python3 packaging/homebrew/test_render_formula.py       # 6 tests, OK

$ git merge-base --is-ancestor HEAD origin/main           # pdftract AND declarative-config: both pushed
```

## Live-sync verification

declarative-config `1681e445` pushed to Forgejo origin → GitHub push mirror
(sync_on_commit) → ArgoCD (rs-manager) → iad-ci. Live template observed at
`kubectl --server=http://traefik-iad-ci:8001 get workflowtemplate
pdftract-homebrew-publish ...` — see the close reason for the observed state at
close time.

## Relation to child beads

The eight closed children under `parent-pdftract-93b464bb` authored the pieces
(handoff contract 54af116f/09049a49, render stage e72cc825, dry-run gate
287f57bf, guarded publication fc061b84/a242f32c, cascade + failure-path
verification 52f8e9ac/35ffe0a7); this dispatch closed the remaining
integration gap (live template generation) and verifies the parent ACs against
HEAD.
