# ADR-001: MPL-2.0 License Exception for cbindgen

## Status
Accepted

## Context
pdftract-libpdftract uses cbindgen (v0.27) as a build dependency to generate C header
files for the C FFI library. cbindgen is licensed under MPL-2.0, which is a copyleft
license not in the default allow list.

## Decision
MPL-2.0 is explicitly allowed for cbindgen as a build-only dependency.

## Rationale
- cbindgen is a **build dependency only** - it is not linked into the final binary
- Build dependencies are compiled and executed during the build process, then discarded
- The MPL-2.0 copyleft terms do not apply to the final pdftract binary or library
- No viable alternative exists for generating C headers from Rust source
- cbindgen is the de-facto standard tool for Rust C FFI (used by Firefox, Servo, etc.)

## Alternatives Considered
- **Manual header maintenance**: Impractical - would diverge from actual FFI signatures
- **Other code generators**: None support Rust's type system adequately for FFI

## Consequences
- pdftract can use cbindgen for C FFI without violating license policy
- The MPL-2.0 license does not affect downstream users of pdftract
- This exception applies to cbindgen as a build dependency only

## References
- cbindgen repository: https://github.com/mozilla/cbindgen
- MPL-2.0 license: https://www.mozilla.org/en-US/MPL/2.0/
