# ADR-002: MPL-2.0 License Exception for option-ext

## Status
Accepted

## Context
option-ext (v0.2.0) is a transitive dependency brought in by the dirs crate
(v5.0.1), which pdftract-cli uses for resolving platform-specific configuration
directories (e.g., ~/.config/pdftract on Linux, ~/Library/Application Support on macOS).

## Decision
MPL-2.0 is explicitly allowed for option-ext as a transitive dependency with no
viable alternative.

## Rationale
- option-ext is a **transitive dependency** - not directly chosen by pdftract
- The dirs crate is the de-facto standard for cross-platform config directory resolution
- No viable alternative to dirs exists that avoids the option-ext transitive dependency
- option-ext provides a single trivial function (Option::zip) - minimal code surface
- The MPL-2.0 copyleft effect is limited to the option-ext crate itself

## Alternatives Considered
- **Hardcode platform paths**: Would break on niche platforms and future OS versions
- **Use a different dirs crate**: No alternative exists; all similar crates pull in option-ext
- **Fork dirs without option-ext**: Impractical maintenance burden for a single function

## Consequences
- pdftract can use dirs for cross-platform config directory resolution
- The MPL-2.0 license does not affect downstream users of pdftract
- This exception applies to option-ext as a transitive dependency only

## Future Work
- Monitor the dirs crate for future versions that may eliminate the option-ext dependency
- Consider contributing a PR to dirs to remove the option-ext dependency if feasible

## References
- dirs repository: https://github.com/dirs-dev/dirs-rs
- option-ext repository: https://github.com/kvsari/option-ext

## Review addendum — 2026-10-06 (pdftract-26b61246)

- **Installed versions and tree:** `cargo tree -i option-ext --locked` reports
  `option-ext 0.2.0 ← dirs-sys 0.4.1 ← dirs 5.0.1`; both `pdftract-cli` and
  `pdftract-core` depend on `dirs`. The exception is still needed for the
  committed dependency selection.
- **Published versions:** [crates.io lists `dirs 7.0.0`](https://crates.io/api/v1/crates/dirs)
  (published 2026-09-05). Its [dependency metadata](https://crates.io/api/v1/crates/dirs/7.0.0/dependencies)
  requires `dirs-sys ^0.5.0`; [`dirs-sys 0.5.0` still requires
  `option-ext ^0.2.0`](https://crates.io/api/v1/crates/dirs-sys/0.5.0/dependencies).
  Updating to the current major release would therefore not eliminate the
  exception.
- **Upstream sources and action:** The project moved from the archived
  [GitHub repository](https://github.com/dirs-dev/dirs-sys-rs) to
  [Codeberg](https://codeberg.org/dirs/dirs-sys-rs). A 2026-10-06 check of its
  [open pull requests](https://codeberg.org/dirs/dirs-sys-rs/pulls) found no
  option-ext removal. The maintainer declined removal in
  [issue #21](https://github.com/dirs-dev/dirs-sys-rs/issues/21) and later
  [PR #26](https://github.com/dirs-dev/dirs-sys-rs/pull/26); earlier
  [PR #22](https://github.com/dirs-dev/dirs-sys-rs/pull/22) and
  [PR #24](https://github.com/dirs-dev/dirs-sys-rs/pull/24) also closed without
  removal. A duplicate PR has no realistic path to acceptance on this evidence.
  Retain the exception and make no dependency or upstream change.
- **Next due:** Recheck at the next dependency update or `dirs`/`dirs-sys`
  release checkpoint, no later than 2027-09-15. Any dependency change needs a
  separate implementation bead. This recurring review remains owned by
  `pdftract-26b61246`.
