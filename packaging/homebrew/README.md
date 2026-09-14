# packaging/homebrew — Homebrew formula template

`pdftract.rb.template` is the source of truth for the `pdftract` formula shipped
in the `jedarden/homebrew-tap` tap as `Formula/pdftract.rb`. The file is a
**template**: the release cascade's `pdftract-homebrew-publish` leg renders it
at release time by substituting the placeholders below, then pushes the
rendered formula to the tap. Hand edits to the tap copy are overwritten on
every release — change this template instead.

Strategy and rationale: `docs/notes/homebrew-tap-strategy.md` (decision record,
bead pdftract-8f4e91aa). This directory was authored by pdftract-2ba6e643; the
render/publish automation is pdftract-da3c85cd; end-to-end verification and the
main-README install-line flip are pdftract-f6cf828b.

## Placeholders

| Placeholder | Meaning | Filled from |
|---|---|---|
| `<VERSION>` | The release version as plain semver — no leading `v`, no prerelease suffix (rc tags are never rendered). Appears twice: in the `url` and the `version` line. | The versioned release tag `vX.Y.Z` the cascade is publishing. |
| `<SHA256>` | sha256 of the exact bytes the `url` serves — the tag's source archive on the GitHub mirror. | Computed by the render leg at render time: download the `url` once and hash the downloaded bytes. Not copied from the release's aggregate `SHA256SUMS` — that file covers the *built* artifacts and is cosign-signed as a unit, so the source-archive hash deliberately lives outside it (strategy §5). The render leg MAY cross-check its computed hash against a `SHA256SUMS` line for the same asset if one ever exists, but no step may modify a published `SHA256SUMS`. |

Substitution of `<VERSION>` must be global (it occurs in both the `url` and the
`version` line); `<SHA256>` must always describe the exact bytes brew will
download from the `url`, never any other artifact.

## Formula shape

- **Build from source** against the versioned-tag source archive, with
  `depends_on "rust"` providing cargo. No bottles in the first iteration — a
  prebuilt-binary formula and official bottles are recorded as future work
  (strategy §9).
- Install runs `cargo install --locked --path crates/pdftract-cli --bin pdftract
  --root prefix --no-track`: the repo root is a virtual Cargo workspace, so the
  build targets the CLI crate that owns the binary; `--bin pdftract` keeps the
  crate's fixture/tooling binaries (gen_lexer_golden, test_glob_discovery, …)
  out of the keg; `--locked` pins the build to the `Cargo.lock` committed in the
  tag; `--no-track` keeps cargo's install bookkeeping out of the keg.
- `license any_of: ["MIT", "Apache-2.0"]` mirrors the workspace
  `license = "MIT OR Apache-2.0"` (workspace Cargo.toml).
- The `test do` block runs `pdftract --version` and asserts the released
  version is printed — the same invocation the cascade's install-verify step
  and `brew test` both use.
- The `livecheck` block watches GitHub releases (`strategy :github_latest`);
  GitHub's latest-release endpoint excludes prereleases, matching the render
  policy that rc tags are never published to the tap.

## Version policy

The formula is rendered only from immutable, non-prerelease versioned tags
`vX.Y.Z`. Moving refs are rejected everywhere in this channel — no
rolling/alias tags, bare git SHAs, or branch names in the formula `url` or
`version`, in any image tag the leg uses, or in tap commits. A botched release
is re-cut under a new tag, producing a new formula version; re-runs of the leg
for the same tag are idempotent (unchanged formula ⇒ no-op push).

## Validation

- `ruby -c packaging/homebrew/pdftract.rb.template` — the template itself is
  parseable Ruby (the placeholders live inside string literals), and the
  rendered output must pass the same check. Run by the render leg where a ruby
  binary exists; `homebrew/brew:<semver>` containers include one.
- `brew tap jedarden/tap https://github.com/jedarden/homebrew-tap &&
  brew install jedarden/tap/pdftract && pdftract --version` from a pinned
  `homebrew/brew:<semver>` container is the end-to-end verification the cascade
  leg runs against the client-facing GitHub mirror.

### Known prerequisite for the test block

The test block asserts on `pdftract --version`. At the time this template was
authored (2026-09-14), the pdftract CLI did not yet accept that flag — the clap
top-level command carries no `version` attribute, so `pdftract --version`
failed with `error: unexpected argument '--version' found` (verified against a
debug build at HEAD b891856c). This is tracked as bead pdftract-8efa24b7; until
that fix lands, the formula's test block — and therefore the cascade's
install-verify — will fail. The template is authored to the target behavior so
the CLI fix needs no template change.
