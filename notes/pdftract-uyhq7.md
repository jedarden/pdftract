# pdftract-uyhq7: libpdftract cdylib + staticlib crate scaffold

## Summary

Created the `pdftract-libpdftract` crate as the fourth workspace member of the pdftract repo. This crate provides the FFI surface for C/C++ integrations, compiling to both shared (cdylib) and static (staticlib) libraries.

## Files Created

1. `crates/pdftract-libpdftract/Cargo.toml` - Crate manifest with:
   - `crate-type = ["cdylib", "staticlib"]`
   - `name = "pdftract"` in [lib] section (ensures libpdftract.so/.a naming)
   - Dependencies: pdftract-core, serde_json, libc
   - Build dependency: cbindgen 0.27

2. `crates/pdftract-libpdftract/src/lib.rs` - Empty scaffold with documentation

3. Updated `Cargo.toml` - Added `crates/pdftract-libpdftract` to workspace.members

## Verification Results

### PASS: cargo build -p pdftract-libpdftract produces expected artifacts

```bash
$ ls -la target/debug/libpdftract.a target/debug/libpdftract.so
-rw-r--r-- 2 coding users 21848552 May 23 07:28 target/debug/libpdftract.a
-rwxr-xr-x 2 coding users  4293504 May 23 07:28 target/debug/libpdftract.so
```

- `libpdftract.so` (4.3 MB) - shared library (cdylib)
- `libpdftract.a` (21.8 MB) - static library (staticlib)
- `readelf -h` confirms ELF64 shared library format

### PASS: Workspace cargo build builds all 4 crates

```bash
$ cargo build --workspace
   Compiling pdftract-core v0.1.0
   Compiling pdftract-cli v0.1.0
   Compiling pdftract-py v0.1.0
   Compiling pdftract-libpdftract v0.1.0
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 22.87s
```

All existing crates build successfully; no regressions.

### PASS: cargo metadata shows pdftract-libpdftract in workspace

```bash
$ cargo metadata --format-version 1 | python3 -c "import json,sys; w=json.load(sys.stdin); print(len(w['workspace_members']))"
4
```

Workspace members: pdftract-core, pdftract-cli, pdftract-py, pdftract-libpdftract

### PASS: Symbol table shows no exported symbols (expected for empty API)

```bash
$ nm -D target/debug/libpdftract.so | grep -E "^[0-9a-f]+ [TW]"
# (no output - no exported text symbols)
```

This is expected as the acceptance criteria states: "the API is empty in this bead; sibling bead populates"

## Artifact Naming Convention

The crate name `pdftract-libpdftract` produces artifacts named `libpdftract.so/.a` (not `libpdftract-libpdftract`) due to the `[lib] name = "pdftract"` directive in Cargo.toml. This matches the expected naming convention for the C/C++ SDK.

## Next Steps

- Sibling bead will populate the extern "C" API surface
- cbindgen will be configured to generate pdftract.h header
- pkg-config file will be added for downstream integrations
