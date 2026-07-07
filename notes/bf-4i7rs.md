# Fuzz/Cargo.toml Dependency Analysis (bf-4i7rs)

**Date:** 2026-07-06
**Status:** PASS - All dependencies correctly configured, all fuzz targets building successfully

## Summary

The current `fuzz/Cargo.toml` configuration is **correct and complete**. All 7 fuzz targets build successfully with their required pdftract-core features enabled. No dependency issues were found.

## Fuzz Target Analysis

### 1. lexer.rs
- **APIs used:** `pdftract_core::parser::lexer::Lexer`
- **Features required:** None (core lexer API)
- **Status:** ✅ Building successfully (59MB binary)

### 2. object_parser.rs
- **APIs used:** `pdftract_core::parser::object::ObjectParser`
- **Features required:** None (core object parser API)
- **Status:** ✅ Building successfully (59MB binary)

### 3. xref.rs
- **APIs used:** `pdftract_core::parser::xref`, `pdftract_core::parser::stream::MemorySource`
- **Features required:** None (core xref parser API)
- **Status:** ✅ Building successfully (59MB binary)

### 4. stream_decoder.rs
- **APIs used:** `pdftract_core::parser::stream::{FlateDecoder, ASCII85Decoder, ASCIIHexDecoder, LZWDecoder}`
- **Features required:** None (core stream decoder API)
- **Status:** ✅ Building successfully (59MB binary)

### 5. cmap_parser.rs
- **APIs used:** `pdftract_core::parser::lexer::Lexer`
- **Features required:** None (uses lexer for name/string parsing)
- **Status:** ✅ Building successfully (59MB binary)

### 6. content.rs
- **APIs used:** `pdftract_core::content_stream::{process_with_mode, ProcessingMode}`, `pdftract_core::parser::resources::ResourceDict`
- **Features required:** None (core content stream API)
- **Status:** ✅ Building successfully (59MB binary)

### 7. profile_yaml.rs
- **APIs used:** `pdftract_core::profiles::load_profile_yaml`
- **Features required:** **`profiles`** (enables serde_yaml)
- **Status:** ✅ Building successfully (60MB binary, serde_yaml symbols confirmed via `nm`)

## Current fuzz/Cargo.toml Configuration

```toml
[dependencies]
pdftract-core = { path = "../crates/pdftract-core", features = ["profiles", "serde", "decrypt", "quick-xml", "cjk"] }
libfuzzer-sys = { version = "0.4", features = ["arbitrary-derive"] }
```

### Feature Analysis

| Feature | Purpose | Required By | Status |
|---------|---------|-------------|--------|
| `profiles` | Enables serde_yaml for profile loading | profile_yaml.rs | ✅ Correctly enabled |
| `serde` | Enables serde/serde_json/schemars | - | ⚪ Not required by current targets |
| `decrypt` | Enables PDF decryption (RC4/AES) | - | ⚪ Not required by current targets |
| `quick-xml` | Enables quick-xml for XML parsing | - | ⚪ Not required by current targets |
| `cjk` | Enables CJK text extraction | - | ⚪ Not required by current targets |

**Note:** The `serde`, `decrypt`, `quick-xml`, and `cjk` features are enabled but not currently used by any fuzz target. They may be useful for future comprehensive fuzzing.

## Dependency Chain Verification

### serde_yaml Availability
```
fuzz/Cargo.toml
  └─ pdftract-core features = ["profiles"]
      └─ crates/pdftract-core/Cargo.toml: profiles = ["dep:serde_yaml"]
          └─ serde_yaml = { version = "0.9", optional = true }
```

✅ **Verified:** serde_yaml is correctly available through the `profiles` feature flag.

### Workspace Path Syntax
```toml
pdftract-core = { path = "../crates/pdftract-core", ... }
```

✅ **Verified:** Correct relative path from `fuzz/` to `crates/pdftract-core/`.

## Build Verification

```bash
$ ls fuzz/target/x86_64-unknown-linux-gnu/release/
lexer          object_parser   xref
stream_decoder cmap_parser     content
profile_yaml  (60MB - includes serde_yaml)
```

All 7 fuzz binaries built successfully with no compilation errors or warnings.

## serde_yaml Symbol Verification

```bash
$ nm profile_yaml | grep serde_yaml | head -3
000000000019a7e0 t _RINv...10serde_yaml...
00000000001bef90 T _RINv...10serde_yaml...
000000000019bdf0 t _RINv...10serde_yaml...
```

✅ **Confirmed:** serde_yaml symbols are present in the profile_yaml binary.

## Historical Context

This bead (bf-4i7rs) was created to document fuzz/Cargo.toml dependency issues, but **no issues were found**. The current configuration:

1. ✅ Correctly enables the `profiles` feature for profile_yaml.rs
2. ✅ Uses correct workspace-relative path syntax
3. ✅ All fuzz targets compile and link successfully
4. ✅ serde_yaml is properly available through the profiles feature

## Recommendations

1. **Keep current configuration** - it's correct and complete
2. **Consider removing unused features** - `serde`, `decrypt`, `quick-xml`, `cjk` are not required by current targets but add build complexity
3. **Monitor future fuzz targets** - if new targets are added, ensure required features are enabled

## Minimal Configuration (Alternative)

If you want to minimize feature flags to only what's currently required:

```toml
[dependencies]
pdftract-core = { path = "../crates/pdftract-core", features = ["profiles"] }
libfuzzer-sys = { version = "0.4", features = ["arbitrary-derive"] }
```

However, keeping the extra features (`serde`, `decrypt`, `quick-xml`, `cjk`) is reasonable for comprehensive fuzzing coverage.

## Conclusion

**PASS:** All fuzz/Cargo.toml dependencies are correctly configured. All 7 fuzz targets build successfully with their required features. No changes are needed to the current configuration.
