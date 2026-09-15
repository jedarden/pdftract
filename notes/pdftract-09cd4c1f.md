# Verification — pdftract-09cd4c1f (client-facing tap install path + README tap-claim reconciliation)

**Date:** 2026-09-15 · **Bead:** pdftract-09cd4c1f (terminal link of the pdftract-d0e30c13 split chain)
**Verdict up front:** the chain's precondition — "run it only after the formula actually exists on the
tap" — is **unmet**. No `Formula/pdftract.rb` was ever pushed: `github.com/jedarden/homebrew-tap`
contains one commit (`c7aec64 init: jedarden homebrew tap`) with only `README.md` and
`Formula/.gitkeep`, and the raw URL a Homebrew client fetches returns **404**. The smoke-run sibling
closed blocked-on-release (no qualifying vX.Y.Z release exists), so there was nothing to propagate.
Everything below documents what *is* true, what could be verified, and the one real doc drift found
and fixed in-tree.

## Split-chain state (template synced → credential enabled → smoke run → client path)

| Link | Bead | State | Evidence |
|---|---|---|---|
| Template synced | pdftract-2ba6e643 (closed) | `packaging/homebrew/pdftract.rb.template` in-tree at HEAD; matches the strategy note §5 contract (class `Pdftract`, `url` = GitHub-mirror tag archive, `<VERSION>`/`<SHA256>` placeholders, `depends_on "rust"`, `test do` block asserting `pdftract --version`, generated-file header) | file inspected this session |
| Publish leg (incl. credential) | pdftract-da3c85cd (closed) | `.ci/argo-workflows/pdftract-homebrew-publish.yaml` present; render step validates `ruby -c` before push (yaml line 288); credential precondition **live-verified this session**: ExternalSecret `homebrew-tap-push-token` (ns `argo-workflows`) = `SecretSynced True`, `Ready` | [notes/pdftract-da3c85cd.md](pdftract-da3c85cd.md) |
| Smoke run | pdftract-c0413792 (closed, **blocked-on-release**) | Negative-path run Succeeded (versioned-tag gate held, no push, tap stayed `c7aec64`); dry-run rendered+validated a stub formula and its sha256 was reproduced out-of-band. No formula pushed. Missing artifact named: first vX.Y.Z release (tag + mirror source archive + SHA256SUMS) — 0 `pdftract-github-release` runs ever; only tag is `v0.1.0-test` | [notes/pdftract-c0413792.md](pdftract-c0413792.md), commit `6dc10b0f` |
| **Client path (this bead)** | pdftract-09cd4c1f | Mirror sync **healthy** (nothing to propagate); formula **absent on both ends**; docs drift in `installation.md` **fixed in-tree**; README claims verified true as written | this note, commit cited in the close reason |

## Acceptance criteria — PASS/WARN/FAIL

**WARN — GitHub raw URL returns 200 with content equal to the Forgejo copy: FAIL as literally
specified, healthy as a mirror.** `https://raw.githubusercontent.com/jedarden/homebrew-tap/main/Formula/pdftract.rb`
→ `HTTP 404` (the client-facing fact of record). The Forgejo↔GitHub mirror itself is fine:
`git ls-remote` HEADs converge at `c7aec641d5cf9e638285fdbc706499f19495f6a2` on both, and full
clones of both diff **byte-identical** (`diff -r --brief` clean over `README.md` + `Formula/.gitkeep`).
The 404 is a *content* gap (nothing was ever pushed), not a *sync* gap. Note on method: Forgejo
raw-over-HTTP anonymously 303s to the sign-in page (instance requires sign-in), so content equality
was proven via git clones, not curl — the GitHub mirror is the only anonymous raw path, which is
exactly the client-facing route the strategy note intends.

**WARN — static validation of the published formula: not runnable (nothing published); substitutes
executed.** (a) `ruby -c`: no Ruby interpreter exists on codinghome (only nix `.drv` build plans in
`/nix/store`, no `ruby`/`irb` in PATH) — recorded per the bead's explicit allowance. Substituting
evidence: the publish workflow itself hard-gates on `ruby -c` of the rendered formula
(`pdftract-homebrew-publish.yaml:288`) and the c0413792 dry-run executed that gate successfully.
(b) Archive URL `https://github.com/jedarden/pdftract/archive/refs/tags/v<VERSION>.tar.gz`: **cannot
resolve — no vX.Y.Z tag exists** (`git ls-remote --tags` → only `v0.1.0-test`; GitHub REST
rate-limited from this IP, git protocol used instead). Same missing artifact the smoke run named.
(c) sha256 vs SHA256SUMS: **the bead's phrasing is corrected by the decided strategy** —
`docs/notes/homebrew-tap-strategy.md` §5: the formula `sha256` is computed at render time from the
exact bytes the `url` serves, and deliberately lives **outside** the aggregate `SHA256SUMS` (which
covers built artifacts, is cosign-signed, and must not be appended-to); the render leg may
cross-check a `SHA256SUMS` line only if one ever exists. The correct future static check is
`sha256(archive at v<VERSION>.tar.gz) == formula sha256`, not a SHA256SUMS lookup. (d) Template
structure vs strategy: class name, version stanza, license `any_of: ["MIT","Apache-2.0"]`, and test
block all conform (checked this session).

**PASS — README/installation tap claims verified true, drifted claim corrected in-tree.**
- `README.md` Homebrew section (line ~136-142): every falsifiable claim verified true — tap repo
  live at `github.com/jedarden/homebrew-tap` (public, mirror in sync); the publish step exists at
  `.ci/argo-workflows/pdftract-homebrew-publish.yaml` and verifies `brew install` +
  `pdftract --version` in a pinned `homebrew/brew:<semver>` container per release (yaml lines 10-12);
  "The first formula lands with the first milestone release" is accurate and forward-looking, so the
  install command block above it is properly conditioned as written. **No edit made** — the README
  line flip on an executed install belongs to sibling `pdftract-f6cf828b` (in_progress, claim held by
  `claude-code-glm-5.3-glm53-ibkr`, whose criteria own exactly that line; editing it from here would
  collide on the shared index). One nuance passed to that bead: the README's "generated from the
  versioned release archives **and SHA256SUMS**" carries the §5 imprecision discussed above.
- `docs/user-docs/src/installation.md`: **drift fixed** (last touched 2026-05-24, `ebb3a8ff` — four
  months before the tap strategy existed). It claimed "Homebrew formula is deferred to v1.1+", which
  the DECIDED strategy note explicitly overturned ("deliberately pulls tap-only Homebrew distribution
  forward ahead of that deferral") and which the README already contradicted. Replaced with the true
  state: tap live, formula generated from versioned release archives by the publish step, first
  formula lands with the first vX.Y.Z release, command not yet installable until then, homebrew-core
  still deferred, link to the strategy note (`../../notes/homebrew-tap-strategy.md` — path verified
  from the doc's location).

**WARN — brew audit / brew install smoke: skipped, Homebrew not installed on codinghome.** Per the
bead and standing org practice, Homebrew was **not** installed on this shared machine. The
substantive client-path PASS arrives with the first release, via the publish leg's own in-runner
`brew tap` + `brew install` + `pdftract --version` verification (pinned container), which is the
executed-install evidence `pdftract-f6cf828b` will record.

## Commands / outputs (key ones)

```
git ls-remote https://github.com/jedarden/homebrew-tap.git   HEAD refs/heads/main → c7aec641… (×2)
git ls-remote https://git.ardenone.com/jedarden/homebrew-tap.git HEAD refs/heads/main → c7aec641… (×2)
curl -sS -o /dev/null -w '%{http_code}' https://raw.githubusercontent.com/jedarden/homebrew-tap/main/Formula/pdftract.rb
  → 404
clone both taps → diff -r --brief (minus .git) → TREES IDENTICAL
git ls-remote --tags https://github.com/jedarden/pdftract.git → refs/tags/v0.1.0-test only
kubectl --server=http://traefik-iad-ci:8001 get externalsecret homebrew-tap-push-token -n argo-workflows
  → SecretSynced True, Ready
which ruby → none (PATH); /nix/store ruby artifacts are .drv build plans only
```

## What must re-run when the first vX.Y.Z release lands

1. `pdftract-homebrew-publish` executes its positive path → `Formula/pdftract.rb` on Forgejo tap.
2. Mirror propagates (`ls-remote` HEADs converge; GitHub raw URL → 200, byte-equal to Forgejo).
3. This bead's criteria re-verify at HEAD: raw 200 + content equality; `ruby -c` (CI gate output);
   `sha256(formula) == sha256(tag archive)`; install command live. Re-dispatch of this bead or
   `pdftract-f6cf828b` covers it — do not trust this note past the release.

## Scope calls recorded

- **README not edited despite "fix any drift" wording:** the README's claims are individually true as
  written; the conditioned install-command flip is sibling `pdftract-f6cf828b`'s owned deliverable
  (claimed, in flight). Avoided a two-worker collision on the same lines of a shared checkout.
- **No formula rebuilt here:** the bead says "Already true (verify, do not rebuild)"; the landing
  work is gated on the release cascade's first run, which is outside this bead's scope.
