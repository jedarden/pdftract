# Bead bf-18zzm6: Add CharProcType enum to type3_rasterizer

## Summary

Added the `CharProcType` enum to `type3_rasterizer.rs` to represent the classification of PDF objects for Type 3 CharProc detection.

## Changes Made

### File: `crates/pdftract-core/src/font/type3_rasterizer.rs`

Added the `CharProcType` enum definition after the imports (line 32):

```rust
/// Classification of PDF objects for Type 3 CharProc detection.
///
/// Represents the type of a PDF object referenced by a Type 3 font's
/// CharProcs dictionary. Used by detection functions to determine
/// whether an object is a stream, dictionary, or other type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharProcType {
    /// PDF stream object (contains content stream bytes)
    Stream,
    /// PDF dictionary object (contains key-value pairs)
    Dict,
    /// Any other PDF object type with a descriptive name
    Other(String),
}
```

## Acceptance Criteria

- ✅ CharProcType enum exists in type3_rasterizer.rs
- ✅ Enum has exactly three variants: Stream, Dict, Other(String)
- ✅ Enum compiles without errors (verified with `cargo check`)
- ✅ Enum is public (pub) so it can be used by tests
- ✅ Appropriate derives: Debug, Clone, PartialEq, Eq (minimal needed)
- ✅ Well-documented with doc comments explaining each variant

## Verification

```bash
# Compilation check
cargo check --package pdftract-core
# Result: SUCCESS (no errors or warnings)
```

The enum is now ready to be used by the detection function that will be implemented in a future bead (referenced by parent bead bf-3czm40).

## Related

- Parent bead: bf-3czm40 (PDF object type detection)
- Plan reference: lines 3851-3890 (PDF object type detection)
