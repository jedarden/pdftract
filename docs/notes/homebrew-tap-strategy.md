# Homebrew Tap Strategy — Distribution Decision Record

**Date:** 2026-09-14
**Purpose:** Decide how pdftract reaches Homebrew users, before any formula or CI
automation exists. This note is the contract the later Homebrew sibling beads build
against: the formula template (pdftract-2ba6e643), the release-cascade publish leg
(pdftract-da3c85cd), and end-to-end verification + README flip (pdftract-f6cf828b).
**Bead:** pdftract-8f4e91aa (docs-only child 1 of 4 from parent pdftract-65433634)
**Status:** DECIDED — later siblings should treat the choices below as settled and
mechanical; deviations must reopen this note, not silently fork the strategy.

**Audit update (2026-09-27):** The decision remains unchanged. Both tap refs exist,
but the client-facing mirror currently contains only `Formula/.gitkeep` and no
`Formula/pdftract.rb`. The versioned `v1.2.0` source archive is reachable, while
`SHA256SUMS`, `SHA256SUMS.sig`, and `SHA256SUMS.pem` for that release are absent.
Therefore there is no valid release handoff and no claim that Homebrew installation
is live; the concrete publication and verification gates remain in §8.

---

## 1. Decision summary

| Question | Decision |
|---|---|
| Distribution vehicle | **Dedicated tap repo `jedarden/homebrew-tap`** — not a homebrew-core submission (for now) |
| Tap repo (source of truth) | `https://git.ardenone.com/jedarden/homebrew-tap` (Forgejo, **public**) |
| Tap repo (client-facing) | `https://github.com/jedarden/homebrew-tap` (read-only Forgejo push mirror, **public**) |
| Formula path in the tap | `Formula/pdftract.rb` (short form: `brew install jedarden/tap/pdftract`) |
| Formula source in this repo | `packaging/homebrew/pdftract.rb.template` (authored by pdftract-2ba6e643, rendered by the cascade) |
| Formula kind | **Build-from-source** from the versioned-tag source archive; `depends_on "rust"` |
| Formula `url` | `https://github.com/jedarden/pdftract/archive/refs/tags/v<VERSION>.tar.gz` |
| Formula `sha256` | sha256 of the exact bytes the `url` serves, handed from the signed release `SHA256SUMS` (see §5) |
| Version policy | Versioned `vX.Y.Z` release tags only; **`:latest`, bare SHAs, branch names, and rc tags are rejected** (§6) |
| homebrew-core | Deferred until the notability criteria are met (§3); tap ships now |
| User install command | `brew install jedarden/tap/pdftract` |

## 2. Why a tap, and why now

- A tap ships immediately: creating `jedarden/homebrew-tap` is two repo creations and
  a mirror, with no review gate. homebrew-core requires the project to clear
  notability thresholds pdftract does not meet today (§3).
- `brew install jedarden/tap/pdftract` works from day one and is the exact command the
  README's release-channels section already advertises; if the project later clears
  notability, the core submission reuses the same formula body and the tap remains as
  the early-channel/override path.
- The plan (docs/plan/plan.md:3493) originally deferred native package-manager
  distribution "beyond cargo/PyPI/Docker" to v1.1+. This decision deliberately pulls
  **tap-only** Homebrew distribution forward ahead of that deferral — the tap is cheap
  (no bottle infrastructure, no review dependency) and the parent bead
  (pdftract-65433634) schedules it now. homebrew-core submission stays deferred per
  the original plan intent. Plan-vs-built reviewers should read §3 as the scoped
  exception, not a contradiction.

## 3. Why not homebrew-core (yet) — live research, 2026-09-14

From the Homebrew Package Acceptance Policy
(<https://docs.brew.sh/Package-Acceptance-Policy>, fetched 2026-09-14): a new package
"must demonstrate public interest beyond its author", normally via:

- "at least 30 forks, 30 watchers or 75 stars"; or
- "at least 90 forks, 90 watchers or 225 stars **for a self-submission by the
  repository owner**" (our case — jedarden submitting its own project);
- plus: "A code repository less than 30 days old is normally not eligible", and "The
  metrics apply to the canonical upstream repository, not an unendorsed mirror or
  code-hosting fork." (pdftract's canonical upstream is the Forgejo repo; a future
  core submission would need to argue "equivalent public evidence" or point at the
  GitHub mirror's accrued metrics as the practical proxy.)

Software below the bar "can generally be maintained in a third-party tap", with the
explicit caveat that tap distribution "does not imply Homebrew endorsement or
support." homebrew-core additionally requires that sources use "an immutable release
archive, tag or revision and downloaded archives must be verified with SHA-256", and
that the formula "must either build from source or install portable,
platform-independent output" (<https://docs.brew.sh/Acceptable-Formulae>) — the
formula design in §5 already conforms, so the only gap at core-submission time is
notability.

**Decision:** tap now; revisit homebrew-core when the self-submission thresholds
(90/90/225) are plausibly met. No homebrew-core work is scheduled in the sibling
beads.

## 4. Hosting: Forgejo source of truth, GitHub mirror (org standard)

The tap is a git repository and follows the org hosting standard unchanged: Forgejo
(`git.ardenone.com`) is the source of truth; GitHub is a read-only mirror that
Homebrew clients actually fetch from.

Why the tap still goes through Forgejo even though `brew tap jedarden/tap` resolves
GitHub-natively:

- **The org rule exists for exactly this shape.** "All repos are hosted on Forgejo as
  the source of truth. GitHub is a read-only mirror." A GitHub-only tap would make
  GitHub the only copy of a repo we own, and would make CI write *directly* to GitHub
  — the pattern the org standard forbids ("never set up client-side dual-push" is
  avoided precisely because the Forgejo server-side mirror does it instead).
- **Disaster recovery:** if the GitHub repo is lost, locked, or renamed, the tap is
  rebuilt by re-pointing the mirror; the canonical history never lived only on GitHub.
- The cost is one moving part: **mirror propagation lag**. `sync_on_commit: true`
  makes it seconds, but it is asynchronous — see the bounded-wait requirement in §8.
- End users are unaffected: `brew install jedarden/tap/pdftract` auto-taps
  `github.com/jedarden/homebrew-tap` (the mirror). A user who prefers the origin can
  tap it explicitly: `brew tap jedarden/tap https://git.ardenone.com/jedarden/homebrew-tap.git`.

**Rejected alternative:** the stranded draft `.ci/argo-workflows/pdftract-homebrew-publish.yaml`
(untracked in the worktree, from a failed parent attempt) argues the tap is a
"publication artifact" that should live GitHub-only and be written by CI with
`github-pat-pdftract`. That argument is rejected: a tap is a versioned git repo
(formula history per release), the org standard covers it, and the mirror already
gives us the GitHub-native short form. Sibling pdftract-da3c85cd **replaces** that
draft, it does not adopt it — the draft also renders a prebuilt-binary formula,
diverging from §5.

### Repo creation steps (one-time setup; owner: pdftract-da3c85cd)

1. **Forgejo (push-to-create — do NOT use the repos API; the stored token lacks
   `write:user`):**

   ```bash
   mkdir homebrew-tap && cd homebrew-tap && git init
   git branch -m main
   # seed: README.md (install line + generated-formula warning), Formula/.gitkeep
   git add README.md Formula/.gitkeep
   git commit -m "init: jedarden homebrew tap"
   git remote add origin https://git.ardenone.com/jedarden/homebrew-tap.git
   git push -u origin main          # auto-creates the repo on Forgejo
   ```

2. **Flip Forgejo visibility to public** (push-created repos start private; the tap
   is only useful public — users fetch it unauthenticated):

   ```bash
   FORGEJO_TOKEN="$(git credential fill <<< 'protocol=https
   host=git.ardenone.com
   ' | grep password | cut -d= -f2)"
   curl -s -X PATCH "https://git.ardenone.com/api/v1/repos/jedarden/homebrew-tap" \
     -H "Authorization: token $FORGEJO_TOKEN" -H "Content-Type: application/json" \
     -d '{"private": false}'
   ```

3. **GitHub repo (empty; the mirror populates it):**

   ```bash
   gh repo create jedarden/homebrew-tap --public \
     --description "Homebrew tap for jedarden projects (pdftract) — mirror of git.ardenone.com/jedarden/homebrew-tap"
   ```

4. **Forgejo server-side push mirror to GitHub** (the org step-4 pattern; the GH
   token lives in the Forgejo mirror config server-side, never in this repo, never in
   CI, never in docs):

   ```bash
   GH_TOKEN="$(gh auth token)"
   FORGEJO_TOKEN="$(git credential fill <<< 'protocol=https
   host=git.ardenone.com
   ' | grep password | cut -d= -f2)"
   curl -s -X POST "https://git.ardenone.com/api/v1/repos/jedarden/homebrew-tap/push_mirrors" \
     -H "Authorization: token $FORGEJO_TOKEN" -H "Content-Type: application/json" \
     -d "{\"remote_name\": \"github-mirror\", \"remote_address\": \"https://jedarden:${GH_TOKEN}@github.com/jedarden/homebrew-tap.git\", \"sync_on_commit\": true, \"interval\": \"8h\"}"
   ```

   Precedent: the jedarden/pdftract Forgejo→GitHub push mirror was verified healthy
   with `sync_on_commit: true` (docs/notes/git-mirroring.md).

## 5. Formula design: source archive, build from source

**Decision:** the formula builds from source against the versioned-tag **source
archive** published by the GitHub mirror of this repo:

```ruby
url "https://github.com/jedarden/pdftract/archive/refs/tags/v<VERSION>.tar.gz"
sha256 "<SHA256>"     # see below
```

with `depends_on "rust"` and an install step that runs a locked cargo install into
Homebrew's `bin` — the exact template is pdftract-2ba6e643's deliverable
(`packaging/homebrew/pdftract.rb.template`); this section fixes only the strategy:

- **Why build-from-source:** pdftract has no bottle infrastructure and the cascade's
  binary archives target 5 triples × 2 feature variants — more surface than a first
  formula needs. Homebrew's own core policy expects a formula to "build from source"
  (Acceptable-Formulae), so this shape is also the one a future core submission
  reuses. A prebuilt-binary formula and official bottles are recorded as future work
  (§9), not first-iteration scope.
- **Why the GitHub source archive:** it exists automatically for every tag, needs no
  new release asset, and the mirror repo is **public** (verified 2026-09-14 via
  `gh repo view jedarden/pdftract` → `PUBLIC`), so unauthenticated brew clients can
  download it.
- **`sha256` source:** the release leg downloads the canonical `url` once and
  records its digest in the signed aggregate as
  `source/pdftract-vX.Y.Z.tar.gz`. The Homebrew render leg extracts that exact
  handoff and does not independently download or hash the archive. The
  aggregate remains immutable after signing; no Homebrew step appends to or
  rewrites it.
- **Platform coverage:** source build works wherever a rust toolchain installs via
  Homebrew (macOS x86_64/aarch64, Linux x86_64/aarch64). No Windows formula —
  Homebrew does not distribute Windows binaries.
- **License:** `license any_of: ["MIT", "Apache-2.0"]` (workspace `license =
  "MIT OR Apache-2.0"`, Cargo.toml).
- **Test block:** required — `pdftract --version` must succeed post-install
  (pdftract-2ba6e643 criterion). A `livecheck` block watching GitHub releases is
  recommended.
- **Generated-file warning:** the formula header must state it is generated by the
  release cascade and edits are overwritten — hand fixes go to
  `packaging/homebrew/pdftract.rb.template` in this repo.

### Formula-generation handoff contract

The next packaging child consumes one immutable release tuple and its release
integrity metadata. For a release tag `vX.Y.Z`, the required inputs and their
canonical URLs are:

| Input | Required value |
|---|---|
| Release tag | `vX.Y.Z`, where `X.Y.Z` is non-prerelease semver; the tag must resolve to the release commit and must not be rewritten. |
| Formula version | `X.Y.Z` — the tag with its leading `v` removed. |
| Source archive | `https://github.com/jedarden/pdftract/archive/refs/tags/vX.Y.Z.tar.gz`; the release leg records the exact bytes' digest as `source/pdftract-vX.Y.Z.tar.gz` in signed `SHA256SUMS`, and the generator consumes that 64-hex handoff. |
| Aggregate checksums | `https://github.com/jedarden/pdftract/releases/download/vX.Y.Z/SHA256SUMS`; consume the file as published, with no append or rewrite. |
| Checksum signature | `https://github.com/jedarden/pdftract/releases/download/vX.Y.Z/SHA256SUMS.sig` and the matching `SHA256SUMS.pem`; verify these against the downloaded `SHA256SUMS` before accepting the release metadata. |
| Formula source | `packaging/homebrew/pdftract.rb.template` from the same `vX.Y.Z` tag. |

`SHA256SUMS` is both the release-integrity input and the source of the formula's
`sha256` handoff. The release producer adds the canonical source archive line
before signing; the Homebrew leg must not invent, append, or rewrite it. If the
release is missing `SHA256SUMS`, its signature or certificate, the unique source
archive line, or the required checksum verification fails, formula generation
stops.

The handoff to the formula-rendering child is the resolved tuple
`{RELEASE_TAG, VERSION, SOURCE_ARCHIVE_URL, SOURCE_ARCHIVE_SHA256,
SHA256SUMS_URL, SHA256SUMS_SIG_URL, SHA256SUMS_PEM_URL}` plus the tagged
template path above. That child must emit the rendered formula at
`Formula/pdftract.rb` for the tap and preserve the generated-file warning; tap
publication and client-facing install verification are later outputs, not inputs
to rendering.

The following refs are prohibited in every input or output of this channel:
`:latest`, `latest`, `releases/latest`, branch names such as `main`, bare git
SHAs, unversioned archive paths, and any other moving or unversioned ref. This
includes release URLs, formula URLs/version fields, source/template fetches, tap
commits, and verification images. A release that cannot satisfy the immutable
`vX.Y.Z` tuple is skipped rather than guessed.

## 6. Version policy — versioned tags only, `:latest` explicitly rejected

- Formulas are rendered **only** from versioned release tags `vX.Y.Z` (semver,
  non-prerelease). Pre-release tags (`vX.Y.Z-rc.N`) are **not** published to the tap
  — the cascade leg gates on a non-prerelease versioned tag, mirroring how
  `pdftract-github-release` marks rc uploads `--prerelease`.
- **`:latest`, bare git SHAs, branch names, and any moving ref are rejected
  everywhere in this channel** — in the formula `url`/`version`, in every container
  image tag the leg uses, and in the tap commits. This extends the org rule for
  `ronaldraygun/*` images ("never `:latest` or a bare git SHA; pin a semver tag") to
  the whole Homebrew surface. The verification container pins
  `homebrew/brew:<semver>` (e.g. `4.6.20`), never `:latest`. This is also what
  Homebrew's core policy demands of sources ("immutable release archive, tag or
  revision"), so the channel stays core-eligible by construction.
- Tags are immutable: a botched release is re-cut under a **new** tag (no
  force-push, org rule), producing a new formula version. Re-runs of the leg for the
  same tag are idempotent (unchanged formula ⇒ no-op push).

## 7. Place in the release cascade

The Homebrew leg is a step of `pdftract-release-cascade` (pdftract-1lw3's
WorkflowTemplate, in-tree at `.ci/argo-workflows/`, synced to jedarden/declarative-config
`k8s/iad-ci/argo-workflows/`; ADR-009: Argo-only, no GitHub Actions):

```
... → pdftract-github-release (publishes tag release + SHA256SUMS)
        └── pdftract-homebrew-publish (render → push to tap → verify)
```

- **Ordering:** strictly after `pdftract-github-release` — the leg fetches the tag's
  source archive, which only resolves once the tag exists (tag push precedes the
  release), and verification installs the released version.
- **Failure isolation:** consistent with the cascade's per-channel `continueOn`
  design (pdftract-1lw3), a tap-publish failure must not fail the whole release run
  unless the tap is that run's deliverable (pdftract-da3c85cd acceptance criterion).
- **Pipeline inside the leg:** render (substitute `<VERSION>`/`<SHA256>` into
  `packaging/homebrew/pdftract.rb.template`, validate with `ruby -c` where available)
  → push to the **Forgejo** tap repo → bounded mirror wait (§8) → install-verify.
- **Verification:** from a pinned `homebrew/brew:<semver>` container:
  `brew tap jedarden/tap https://github.com/jedarden/homebrew-tap && brew install
  jedarden/tap/pdftract && pdftract --version` reporting the released version —
  i.e. verification exercises the **client-facing mirror path**, not the Forgejo
  origin, because that is the path users take.
- Cluster changes land via declarative-config (in-tree `.ci/argo-workflows/` updated
  together), pushed, ArgoCD-synced — never `kubectl apply`.

## 8. Prerequisites the later siblings depend on

The original setup baseline was checked 2026-09-14; the current publication audit
was checked 2026-09-27. Owners remain in parentheses.

1. **A complete versioned release handoff is still missing.** The `v1.2.0` tag and
   its immutable source archive exist, but the release URLs for `SHA256SUMS`,
   `SHA256SUMS.sig`, and `SHA256SUMS.pem` return not-found. The first green release
   cascade must publish all three assets, with the unique
   `source/pdftract-vX.Y.Z.tar.gz` line in the signed aggregate, before the
   Homebrew leg can render anything. A dry-run fixture may validate the renderer,
   but it is not a release handoff. (pdftract-da3c85cd)
2. **The signed checksum handoff is authoritative.** `pdftract-github-release`
   (pdftract-2x7y; template in-tree at `.ci/argo-workflows/pdftract-github-release.yaml`)
   must publish `SHA256SUMS` and its signature material before the Homebrew leg
   runs. Per §5, the release producer's signed source-archive line supplies the
   formula's sha256; the aggregate is not modified post-signature. (pdftract-da3c85cd)
3. **Tap push credential — OpenBao path reference, never a literal:**
   `secret/rs-manager/iad-ci/forgejo/homebrew-tap-push-token` on the
   **rs-manager** instance.
   *(Corrected 2026-09-15: this note originally specified
   `secret/ardenone-cluster/forgejo-iad-ci/homebrew-tap-push-token` on
   openbao-v2, but iad-ci's ClusterSecretStore `openbao`
   (`k8s/iad-ci/external-secrets/cluster-secret-store.yml`) serves the
   rs-manager prefix — every live iad-ci ExternalSecret reads
   `rs-manager/iad-ci/*` — so an ardenone-cluster path cannot be synced into
   the cluster. The provisioning identity in the command below is likewise
   `rs-manager-provision`, which does not exist yet (rs-manager has only the
   read identity until its reconciler bootstraps it) — provisioning is blocked
   on that operator setup.)*
   Contents: a Forgejo token scoped to repo `jedarden/homebrew-tap` write only.
   Provisioning follows the secrets-by-reference rule: generate and store via pipe —
   `openssl rand -base64 32 | bao-as rs-manager-provision bao kv put -cas=N
   secret/rs-manager/iad-ci/forgejo/homebrew-tap-push-token token=-` — verify
   by property (`bao kv metadata get ... | jq .data.current_version`), and sync it to
   the `argo-workflows` namespace on iad-ci via an ExternalSecret. The workflow step
   references the path / secret name only; grep the diff to prove no value appears.
   The workflow contract records the ExternalSecret as enabled and
   `SecretSynced=True` on 2026-09-26, but that synchronization signal is not a
   successful tap publication. Before a live push, verify the current secret
   property in the target cluster and verify success only through the downstream
   push/mirror effects. No token value belongs in this repository, workflow
   parameters, URLs, arguments, logs, or bead evidence. (pdftract-da3c85cd)
4. **Tap repos exist, but the formula does not.** The Forgejo origin and GitHub
   mirror both expose the tap's current `main` commit (`c7aec641d5cf9e638285fdbc706499f19495f6a2`),
   and the mirror contains only `Formula/.gitkeep` plus its README. The publication
   child must still render and push exactly `Formula/pdftract.rb`; until that
   commit is mirrored, the install command is a validation procedure, not a live
   channel claim. (pdftract-da3c85cd creates; pdftract-f6cf828b verifies)
5. **GitHub mirror of pdftract is public** — ✅ verified 2026-09-14
   (`gh repo view jedarden/pdftract` → `PUBLIC`), so the formula `url` is
   anonymously fetchable. No action.
6. **Mirror propagation tolerance** — because CI pushes the formula to Forgejo and
   verification taps GitHub, the verify step must **bounded-poll** the mirror with
   `git ls-remote` for the expected immutable tap commit (up to ~5 min), then
   inspect that commit's `Formula/pdftract.rb` before `brew tap`; on timeout, fail
   the verify step with a "mirror lag" message, not a generic failure.
   `sync_on_commit: true` makes this seconds, not minutes, in the normal case.
   (pdftract-da3c85cd)

## 9. Future work (explicitly out of sibling scope)

- **homebrew-core submission** once §3 thresholds are plausibly met (self-submission
  needs 90 forks / 90 watchers / 225 stars on the canonical upstream).
- **Prebuilt-binary formula / bottles**: consume the cascade's default-variant
  archives for the four darwin/musl triples to drop the rust build dependency;
  official bottles via Homebrew's bottle workflow. (The stranded draft's rendering
  logic is a useful reference for the archive-naming then, but is otherwise
  superseded.)
- **`libpdftract` formula** in the same tap — plan.md:3594/3691 route the C/C++
  shared-library channel through a Homebrew formula; it lands in
  `jedarden/homebrew-tap` alongside pdftract when that channel ships.
- Source-archive SHA in `SHA256SUMS`: if a future cascade change wants the tag
  source archive covered by the cosign-signed aggregate, it must be added as a
  release *asset* (e.g. a `git archive`-produced `pdftract-vX.Y.Z-source.tar.gz`
  built in `pdftract-build-binaries`) *before* signing — never appended afterwards.

## 10. References

- README.md release-channels section (`#### Homebrew`)
- Parent umbrella: pdftract-65433634; siblings: pdftract-2ba6e643 (formula template),
  pdftract-da3c85cd (cascade leg), pdftract-f6cf828b (verify + README flip)
- Release chain the tap consumes: pdftract-1lw3 (cascade), pdftract-2x7y (GitHub
  Release + SHA256SUMS), pdftract-5x3u (crates.io ordering precedent)
- docs/plan/plan.md:3493 (original v1.1+ deferral), :3485 (channel/credential table),
  :3594 & :3691 (libpdftract formula routing)
- docs/notes/git-mirroring.md (Forgejo→GitHub push-mirror precedent, `sync_on_commit`)
- <https://docs.brew.sh/Package-Acceptance-Policy> (notability thresholds, fetched
  2026-09-14); <https://docs.brew.sh/Acceptable-Formulae> (immutable-archive +
  SHA-256 + build-from-source requirements, fetched 2026-09-14)
