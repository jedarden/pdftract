# bf-1b7od: Verify cargo-fuzz installation and fuzz project configuration

## Findings

### PASS: cargo-fuzz installation
```bash
$ cargo fuzz --version
cargo-fuzz 0.13.1
```

### PASS: fuzz/Cargo.toml exists with proper dependencies
- File exists at `/home/coding/pdftract/fuzz/Cargo.toml`
- Contains `libfuzzer-sys = { version = "0.4", features = ["arbitrary-derive"] }`
- Package metadata properly marked with `cargo-fuzz = true`

### FAIL: profile_yaml fuzz target missing
- `fuzz/fuzz_targets/profile_yaml.rs` does NOT exist
- fuzz/Cargo.toml lists 6 targets but NOT profile_yaml:
  1. lexer (fuzz_targets/lexer.rs)
  2. object_parser (fuzz_targets/object_parser.rs)
  3. xref (fuzz_targets/xref.rs)
  4. stream_decoder (fuzz_targets/stream_decoder.rs)
  5. cmap_parser (fuzz_targets/cmap_parser.rs)
  6. content (fuzz_targets/content.rs)

## Existing fuzz targets
```
$ ls -la fuzz/fuzz_targets/
-rw-r--r-- 1 coding users 1179 May 20 18:13 cmap_parser.rs
-rw-r--r-- 1 coding users  822 Jul  5 18:05 content.rs
-rw-r--r-- 1 coding users  752 May 20 18:13 lexer.rs
-rw-r--r-- 1 coding users  793 May 20 18:13 object_parser.rs
-rw-r--r-- 1 coding users 1375 May 22 17:26 stream_decoder.rs
-rw-r--r-- 1 coding users  733 May 20 18:13 xref.rs
```

## Status
Acceptance criteria NOT met: profile_yaml fuzz target is missing from configuration.
