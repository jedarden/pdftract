# Fix for bf-qf5mh: cargo fuzz build command correction

## Problem
Parent bead bf-4o8a6 had incorrect cargo fuzz build syntax that caused repeated failures:
- Incorrect: `cargo fuzz build --harness profile_yaml`
- Correct: `cargo fuzz build profile_yaml`

## Root cause
cargo-fuzz does not have a `--harness` flag. The target name is passed as a positional argument, not as a flag.

## Verification
```bash
$ cargo fuzz build --help
cargo-fuzz-build 0.13.1
Build fuzz targets

USAGE:
    cargo-fuzz build [OPTIONS] [TARGET]

Arguments:
  [TARGET]
          Name of the fuzz target to build, or build all targets if not supplied
```

## Changes made
Updated bf-4o8a6 description to use correct syntax in both Implementation guidance and Acceptance criteria sections.

## Acceptance criteria status
- PASS: Documented correct cargo fuzz build syntax (see note)
- PASS: Parent bead acceptance criteria updated to use `cargo fuzz build profile_yaml`
- PASS: No reference to --harness flag remains in parent bead
