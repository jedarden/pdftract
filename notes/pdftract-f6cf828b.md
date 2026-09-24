# Homebrew verification — `pdftract-f6cf828b`

**Date:** 2026-09-24  
**Verdict:** **WARN — pending the first versioned release and formula publication.**

The client-facing tap exists and is reachable, but the cascade has not published
`Formula/pdftract.rb`: both tap remotes are still at the initial commit and the
GitHub raw formula URL returns 404. GitHub reports no releases and the only tag is
the test-only `v0.1.0-test`, so there is no supported archive for a formula to
install. The shared verification host also has no `brew` or `ruby` executable.

## Acceptance criteria

| Criterion | Result | Evidence |
| --- | --- | --- |
| End-to-end `brew tap` + `brew install` + `pdftract --version` | **WARN** | Not executed: `brew` is not installed, and the client-facing tap has no formula to install. This is not claimed as a PASS. |
| Tap is reachable and mirrored | **PASS** | Forgejo and GitHub `main` both resolve to `c7aec641d5cf9e638285fdbc706499f19495f6a2`; GitHub tree is `Formula`, `Formula/.gitkeep`, `README.md`. |
| Published formula exists | **WARN** | `https://raw.githubusercontent.com/jedarden/homebrew-tap/main/Formula/pdftract.rb` returns HTTP 404; the Forgejo raw endpoint redirects to sign-in (HTTP 303), while its git ref is the same initial commit. |
| Release prerequisite exists | **WARN** | GitHub Releases API reports `count=0`; `git ls-remote --tags` reports only `v0.1.0-test`, not a supported `vX.Y.Z` release. Formula URL and SHA256 therefore cannot yet be checked against a published asset. |
| Template and strategy agree | **WARN** | `packaging/homebrew/pdftract.rb.template` contains the versioned tag URL, `<VERSION>`/`<SHA256>` placeholders, `depends_on "rust"`, the locked CLI install command, and the `pdftract --version` test; the strategy specifies `brew tap jedarden/tap https://github.com/jedarden/homebrew-tap && brew install jedarden/tap/pdftract && pdftract --version`. Local `ruby -c` could not run because Ruby is absent. |
| README reflects the true state | **PASS** | The Homebrew row now says the tap exists while formula and install verification await the first versioned release. |

## Commands and outputs

```text
git ls-remote https://github.com/jedarden/homebrew-tap.git HEAD refs/heads/main
  c7aec641d5cf9e638285fdbc706499f19495f6a2 HEAD
  c7aec641d5cf9e638285fdbc706499f19495f6a2 refs/heads/main
  exit=0

git ls-remote https://git.ardenone.com/jedarden/homebrew-tap.git HEAD refs/heads/main
  c7aec641d5cf9e638285fdbc706499f19495f6a2 HEAD
  c7aec641d5cf9e638285fdbc706499f19495f6a2 refs/heads/main
  exit=0

curl -sS -o /dev/null -w 'HTTP %{http_code}\n' \
  https://raw.githubusercontent.com/jedarden/homebrew-tap/main/Formula/pdftract.rb
  HTTP 404
  exit=0

curl -sS -o /dev/null -w 'Forgejo raw endpoint HTTP %{http_code}\n' \
  https://git.ardenone.com/jedarden/homebrew-tap/raw/branch/main/Formula/pdftract.rb
  Forgejo raw endpoint HTTP 303
  exit=0

curl -fsSL https://api.github.com/repos/jedarden/pdftract/releases | ...
  count=0
  exit=0

git ls-remote --tags https://github.com/jedarden/pdftract.git 'refs/tags/v*'
  28d26fa1a23399f996184f0fe0f738088e65c42f refs/tags/v0.1.0-test
  exit=0

command -v brew
  brew: not installed (command -v exit 1)

command -v ruby
  ruby: not installed (command -v exit 1)

curl -fsSL 'https://api.github.com/repos/jedarden/homebrew-tap/git/trees/main?recursive=1' | ...
  tree=Formula Formula/.gitkeep README.md
  exit=0

rg -n 'class Pdftract|url |version |sha256 |depends_on|cargo.*install|test do|--version|<VERSION>|<SHA256>' \
  packaging/homebrew/pdftract.rb.template
  exit=0; matched the expected template contract

rg -n -A2 -B1 'brew tap jedarden/tap' docs/notes/homebrew-tap-strategy.md
  exit=0; matched the documented client-facing install command
```

The exact install command must be re-run from a pinned Homebrew container after
the cascade publishes a supported `Formula/pdftract.rb` and `vX.Y.Z` release.
