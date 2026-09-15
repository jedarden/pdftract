# pdftract-ae9b537e — Replace ronaldraygun/pdftract:latest in README Docker install docs

**Date:** 2026-09-15
**Bead:** pdftract-ae9b537e (P2, weave-generated)
**Base commit:** a2af776b48366cb60412ae4bbcb7d3ef39e8dcd3 (== origin/main at dispatch)

## What was done

Rewrote the "Planned release channels → Docker" section of `README.md`:

- `docker pull ronaldraygun/pdftract:latest` is **removed**. Install guidance now
  pins a released semver tag — `:X.Y.Z`, `:ocr-X.Y.Z`, `:full-X.Y.Z` (matching the
  plan's artifact taxonomy) — and states explicitly to never use `:latest`.
- Added **digest-pinning guidance** for reproducible consumption: resolve the
  digest with `docker buildx imagetools inspect ronaldraygun/pdftract:X.Y.Z`,
  then `docker pull ronaldraygun/pdftract@sha256:<digest>` — noting that the
  digest is what gets recorded in a Dockerfile / deployment manifest.
- Documented the release posture the images ship with per the Docker build bead
  `pdftract-68pe`: cosign-signed, version-tagged multi-arch (amd64 + arm64)
  manifest lists; no floating `:latest` tag exists.
- Referenced **SHA256SUMS verification** gated on bead `pdftract-1wfp`:
  the aggregate `SHA256SUMS` + cosign signature `SHA256SUMS.sig` verify in one
  shot via `cosign verify-blob --signature SHA256SUMS.sig SHA256SUMS`, and
  clarified the integrity split — the image digest pins the container itself,
  while SHA256SUMS covers the binary archives, wheels, sdist, and SBOM.

## Acceptance criteria

| Criterion | Status | Evidence |
|---|---|---|
| No `:latest` pull guidance remains in README Docker docs | PASS | `grep -n latest README.md` → only the two "never `:latest`" policy statements remain (lines 85, 105 post-edit) |
| Pinned-tag pull guidance | PASS | `docker pull ronaldraygun/pdftract:X.Y.Z` + `:ocr-` / `:full-` variant tags (README lines 88–90) |
| Digest-pinning guidance for reproducible consumption | PASS | `imagetools inspect` + `@sha256:<digest>` pull block (README lines 94–101) |
| SHA256SUMS verification reference (pdftract-1wfp) | PASS | `cosign verify-blob` block, explicitly gated on `pdftract-1wfp` landing (README lines 103–112) |
| Consistency with the project's supply-chain posture | PASS | Matches org hard prohibition ("never `:latest` for `ronaldraygun/*`"), plan §Release Engineering (cosign-signed version-tagged multi-arch images), and plan §Cross-Cutting "Rollback and binary downgrade" (pin to a specific tag) |

## Scope notes

- **Docs-only change.** No Rust source touched; `cargo check`/`cargo test`
  results are unaffected by this commit (README.md and notes/*.md are not cargo
  inputs), so the `default_rust` gate cannot regress from it.
- **Pre-existing unrelated hunk left stranded, deliberately.** The working tree
  README.md also carried a Homebrew edit from another worker
  (`brew install pdftract` → `brew install jedarden/tap/pdftract` + a tap-live
  status paragraph referencing `pdftract-homebrew-publish`). It was present at
  predispatch (README blob `da25c107a9196d233482fd4c16f759f0753a82b2` in
  `~/.needle/state/predispatch/a54902ddba370cf1-pdftract_ae9b537e.json`), i.e.
  it predates this bead. Per the shared-checkout rule (commit only your own
  hunks — cf. e58fb369), only the Docker hunk was committed: the commit was
  built with a temp `GIT_INDEX_FILE` + `read-tree HEAD` + `update-index
  --cacheinfo` + `write-tree`/`commit-tree`, leaving the shared index and the
  other worker's edit untouched. That hunk remains uncommitted in the tree for
  its owner.
