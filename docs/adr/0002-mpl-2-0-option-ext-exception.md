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
