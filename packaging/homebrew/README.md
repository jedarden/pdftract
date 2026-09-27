# packaging/homebrew — Homebrew formula template

> This directory is a release template only. No Homebrew formula is currently
> published; the placeholders below are filled only after a real release has
> been built and independently verified.

`pdftract.rb.template` is the source of truth for the `pdftract` formula shipped
in the `jedarden/homebrew-tap` tap as `Formula/pdftract.rb`. The file is a
**template**: the release cascade's `pdftract-homebrew-publish` leg renders it
at release time by substituting the placeholders below, then pushes the
rendered formula to the tap. Hand edits to the tap copy are overwritten on
every release — change this template instead.

`render_formula.py` is the local, deterministic renderer and validator for the
same contract. It has no network, git, tap, or release-orchestration behavior;
the later publication child can invoke it after resolving and verifying the
release artifacts:

```sh
python3 packaging/homebrew/render_formula.py \
  --inputs release-inputs.json \
  --output Formula/pdftract.rb
```

The JSON input is an object with exactly these required release fields:

```json
{
  "RELEASE_TAG": "v1.2.3",
  "VERSION": "1.2.3",
  "SOURCE_ARCHIVE_URL": "https://github.com/jedarden/pdftract/archive/refs/tags/v1.2.3.tar.gz",
  "SOURCE_ARCHIVE_SHA256": "<64 lowercase hex characters>",
  "SHA256SUMS_URL": "https://github.com/jedarden/pdftract/releases/download/v1.2.3/SHA256SUMS",
  "SHA256SUMS_SIG_URL": "https://github.com/jedarden/pdftract/releases/download/v1.2.3/SHA256SUMS.sig",
  "SHA256SUMS_PEM_URL": "https://github.com/jedarden/pdftract/releases/download/v1.2.3/SHA256SUMS.pem"
}
```

The caller owns verifying the signed `SHA256SUMS` inputs and extracting the
`source/pdftract-vX.Y.Z.tar.gz` checksum handoff produced by the release leg.
The renderer binds that supplied digest to the formula's `sha256`; it never
downloads or re-hashes an archive and never appends to or rewrites the signed
aggregate checksum file. It rejects prerelease tags, branches, rolling refs,
bare commit SHAs, mismatched version/archive URLs, and incomplete placeholders.
If `ruby` is installed it runs `ruby -c` on both the template and rendered
formula; otherwise it continues with an explicit warning.

Strategy and rationale: `docs/notes/homebrew-tap-strategy.md` (decision record,
bead pdftract-8f4e91aa). This directory was authored by pdftract-2ba6e643; the
render/publish automation is pdftract-da3c85cd; end-to-end verification and the
main-README install-line flip are pdftract-f6cf828b.

The release-to-publication handoff is defined in
`docs/operations/homebrew-release-handoff.md`; it is the authoritative record
of the immutable release inputs, tap destination, ordering, dry-run boundary,
and retry behavior.

## Placeholders

| Placeholder | Meaning | Filled from |
|---|---|---|
| `<VERSION>` | The release version as plain semver — no leading `v`, no prerelease suffix (rc tags are never rendered). Appears twice: in the `url` and the `version` line. | The versioned release tag `vX.Y.Z` the cascade is publishing. |
| `<SHA256>` | sha256 of the exact bytes the `url` serves — the tag's source archive on the GitHub mirror. | Extracted from the signed release `SHA256SUMS` line `source/pdftract-vX.Y.Z.tar.gz`; the release leg hashes the canonical archive once and the Homebrew render leg consumes that handoff without re-downloading it. |

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
- `python3 -m unittest discover -s packaging/homebrew -p 'test_*.py'` — exercises
  successful deterministic rendering, checksum/archive binding, placeholder
  completeness, and rejection of prerelease/floating inputs.
- `brew tap jedarden/tap https://github.com/jedarden/homebrew-tap &&
  brew install jedarden/tap/pdftract && pdftract --version` from a pinned
  `homebrew/brew:<semver>` container is the end-to-end verification the cascade
  leg runs against the client-facing GitHub mirror.

### Current publication verification

The client-facing tap was checked on 2026-09-27. It is reachable, but its
`main` branch (`c7aec641d5cf9e638285fdbc706499f19495f6a2`) contains only
`Formula/.gitkeep`; no `Formula/pdftract.rb` is published. The source archive
URL for the versioned `v1.2.0` tag is reachable and hashes to
`db8390ced458a1e3d3816e43a3488a3e643bcb43a8ac7404f1de38b28f503e75`, but that
is not evidence of a published formula or a completed Homebrew install. The
corresponding `SHA256SUMS`, `SHA256SUMS.sig`, and `SHA256SUMS.pem` release assets
are also not published, so the signed release handoff required to render the
formula is incomplete.

The CLI now accepts `--version`, and the local source smoke prints
`pdftract 0.1.0`. The `v1.2.0` tag still declares workspace version `0.1.0`,
so a future formula rendered as version `1.2.0` must not be treated as
verified until release metadata and the formula agree.

Verification evidence and the exact commands are recorded on bead
`pdftract-ed8d28bf`. The remaining blockers are publication of the signed release
handoff, publication of `Formula/pdftract.rb`, and the full tap/install/version
smoke test on a machine with Homebrew; until then the brew command below is a
validation procedure, not a live install claim.
