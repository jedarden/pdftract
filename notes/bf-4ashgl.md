# Type3 Glyph Call Chain Documentation

**Bead**: bf-4ashgl  
**Date**: 2026-08-06  
**Purpose**: Document the complete call chain from font resolver to Type3 glyph rasterization

## Overview

This document traces the complete call path from the public API entry point for Type3 glyph resolution through to the actual rasterization of glyph content streams.

## Call Chain Summary

```
resolve_type3() [resolver.rs:591]
  └─> resolve_type3_level4() [resolver.rs:683]
       └─> resolve_stream_bytes() [resolver.rs:725] (local helper)
            └─> rasterize_type3_glyph() [type3_rasterizer.rs:1330]
                 └─> RasterizerContext::execute_content_stream()
```

## Detailed Function Signatures

### 1. Entry Point: `resolve_type3()`

**Location**: `crates/pdftract-core/src/font/resolver.rs:591`

**Signature**:
```rust
pub fn resolve_type3(
    font: &Type3Font,
    to_unicode: Option<&ToUnicodeMap>,
    char_code: u8,
    resolver: Option<&XrefResolver>,
    source: Option<&dyn ParserPdfSource>,
    doc_decompress_counter: Option<&mut u64>,
    diagnostics: &mut Vec<Diagnostic>,
) -> ResolvedGlyph
```

**Purpose**: Public API for resolving Type3 font glyphs. Implements a 4-level resolution strategy:
- Level 1: ToUnicode CMap lookup
- Level 2: Encoding → AGL lookup
- Level 3: SKIPPED (no embedded program in Type3)
- Level 4: Shape recognition via rasterization (when shape-db feature is enabled)

**Parameters available**:
- `resolver: Option<&XrefResolver>` - PDF xref resolver
- `source: Option<&dyn ParserPdfSource>` - PDF source for reading streams
- `doc_decompress_counter: Option<&mut u64>` - Decompression byte counter

---

### 2. Level 4 Handler: `resolve_type3_level4()`

**Location**: `crates/pdftract-core/src/font/resolver.rs:683`

**Signature**:
```rust
#[cfg(feature = "shape-db")]
fn resolve_type3_level4(
    font: &Type3Font,
    char_code: u8,
    glyph_name: Option<Arc<str>>,
    resolver: Option<&XrefResolver>,
    source: Option<&dyn ParserPdfSource>,
    doc_decompress_counter: Option<&mut u64>,
    diagnostics: &mut Vec<Diagnostic>,
) -> ResolvedGlyph
```

**Purpose**: Rasterizes glyph content stream to 32×32 bitmap, computes pHash, and looks up in shape database.

**Key operations**:
1. Gets glyph name from encoding (lines 692-709)
2. Verifies glyph exists in `/CharProcs` (lines 711-720)
3. Creates stream resolver callback if document context available (lines 723-761)
4. Calls `rasterize_type3_glyph()` (lines 763, 766)
5. Downsamples result to 32×32 (via `downscale_to_32x32()`)
6. Computes pHash and queries shape database

**Parameters available**:
- `resolver: Option<&XrefResolver>` - Same as from `resolve_type3()`
- `source: Option<&dyn ParserPdfSource>` - Same as from `resolve_type3()`
- `doc_decompress_counter: Option<&mut u64>` - Same as from `resolve_type3()`

---

### 3. Local Helper: `resolve_stream_bytes()`

**Location**: `crates/pdftract-core/src/font/resolver.rs:725` (nested inside `resolve_type3_level4`)

**Signature**:
```rust
fn resolve_stream_bytes(
    obj_ref: crate::parser::object::ObjRef,
    resolver: &XrefResolver,
    source: &dyn ParserPdfSource,
    counter: &mut u64,
) -> Option<Vec<u8>>
```

**Purpose**: Resolves an ObjRef to decoded stream bytes. Used as a closure-compatible workaround for lifetime issues.

**Operations**:
1. Resolves object reference: `resolver.resolve_with_source(obj_ref, source)` (line 734)
2. Extracts stream from resolved object (lines 737-740)
3. Decodes stream: `decode_stream(&stream, source, &ExtractionOptions::default(), counter)` (lines 743-748)

**Parameters available**:
- `resolver: &XrefResolver` - Borrowed from outer scope
- `source: &dyn ParserPdfSource` - Borrowed from outer scope
- `counter: &mut u64` - Borrowed from outer scope

---

### 4. Rasterizer Entry: `rasterize_type3_glyph()`

**Location**: `crates/pdftract-core/src/font/type3_rasterizer.rs:1330`

**Signature**:
```rust
pub fn rasterize_type3_glyph<'a, R>(
    font: &Type3Font,
    glyph_name: &str,
    doc_context: Option<&'a DocumentContext<'a>>,
    resolve_stream: Option<&R>,
) -> Option<Vec<u8>>
where
    R: Fn(ObjRef) -> Option<Vec<u8>> + ?Sized,
```

**Purpose**: Rasterizes a Type3 glyph content stream to a bitmap.

**Operations**:
1. Gets char_proc ObjRef from font: `font.char_proc(glyph_name)?` (line 1340)
2. Calls stream resolver callback if provided (lines 1346-1350)
3. Creates `RasterizerContext` and executes content stream (lines 1355-1356)
4. Returns bitmap bytes (line 1357)

**Parameters available**:
- `doc_context: Option<&DocumentContext<'a>>` - Contains resolver and source
- `resolve_stream: Option<&R>` - Callback to resolve streams

**Current TODO comment** (line 1342-1343):
```rust
// Document context is passed for potential future use (e.g., form XObject resolution)
// Stream resolution happens via the resolver callback pattern
```

**Note**: The function currently **ignores** `doc_context` (line 1344: `let _doc_context = doc_context;`). It only uses the `resolve_stream` callback.

---

## Supporting Types

### `DocumentContext<'a>` Struct

**Location**: `crates/pdftract-core/src/font/type3_rasterizer.rs:37`

```rust
pub struct DocumentContext<'a> {
    /// PDF document resolver for looking up indirect references
    pub resolver: Option<&'a XrefResolver>,
    /// PDF source for reading stream data
    pub source: Option<&'a dyn PdfSource>,
}
```

### `StreamResolverFn` Type Alias

**Location**: `crates/pdftract-core/src/font/type3_rasterizer.rs:1085`

```rust
pub type StreamResolverFn = dyn Fn(ObjRef) -> Option<Vec<u8>> + Send + Sync;
```

### `Type3DocumentContext` Import (in resolver.rs)

**Location**: `crates/pdftract-core/src/font/resolver.rs:26`

```rust
use crate::font::type3_rasterizer::{rasterize_type3_glyph, 
    DocumentContext as Type3DocumentContext, StreamResolverFn};
```

This is an alias for `DocumentContext` to avoid naming conflicts.

---

## Parameter Flow Analysis

### Where parameters are currently available:

| Parameter | Available at `resolve_type3()` | Available at `resolve_type3_level4()` | Available at `rasterize_type3_glyph()` |
|-----------|-------------------------------|----------------------------------------|------------------------------------------|
| `resolver` | ✅ Yes (as `Option<&XrefResolver>`) | ✅ Yes (as `Option<&XrefResolver>`) | ❌ No (only via callback) |
| `source` | ✅ Yes (as `Option<&dyn ParserPdfSource>`) | ✅ Yes (as `Option<&dyn ParserPdfSource>`) | ❌ No (only via callback) |
| `doc_decompress_counter` | ✅ Yes (as `Option<&mut u64>`) | ✅ Yes (as `Option<&mut u64>`) | ❌ No (only via callback) |

### How parameters flow to `rasterize_type3_glyph()`:

1. **When all parameters are `Some`** (lines 753-763):
   - Creates `Type3DocumentContext { source }` containing the `source`
   - Creates a closure callback capturing `resolver`, `source`, and `counter`
   - Passes `Some(&doc_ctx)` and `Some(&callback)` to `rasterize_type3_glyph()`

2. **When any parameter is `None`** (lines 764-767):
   - Passes `None` for both `doc_context` and `resolve_stream`
   - Results in `None` return (can't rasterize without stream resolution)

---

## Key Observations

### 1. Current Architecture Pattern

The current implementation uses a **callback pattern** for stream resolution:
- The callback captures `resolver`, `source`, and `doc_decompress_counter`
- `rasterize_type3_glyph()` doesn't directly access these parameters
- This avoids lifetime complexity but limits flexibility

### 2. DocumentContext Underutilization

- `DocumentContext` struct exists and contains `resolver` and `source`
- `rasterize_type3_glyph()` receives `doc_context` but **immediately ignores it** (line 1344)
- The TODO comment acknowledges this is "for potential future use (e.g., form XObject resolution)"

### 3. Parameter Limitations

When `rasterize_type3_glyph()` needs to resolve nested XObjects (e.g., form XObjects in Type3 content streams), it currently cannot:
- It only has access to the `resolve_stream` callback
- The callback is designed for char_proc streams, not general XObjects
- `doc_context` is passed but unused

---

## Related Code Sections

### Type3 Font Structure

**Location**: `crates/pdftract-core/src/font/type3.rs`

The `Type3Font` struct contains:
- `encoding: FontEncoding` - Character encoding
- `char_procs: IndexMap<Arc<str>, ObjRef>` - Mapping of glyph names to content stream references
- `font_bbox: [f32; 4]` - Font bounding box
- `matrix: [f32; 4]` - Font matrix

### RasterizerContext

**Location**: `crates/pdftract-core/src/font/type3_rasterizer.rs` (around line 900+)

`RasterizerContext` manages:
- Path building state
- Graphics state stack (save/restore)
- Bitmap rendering
- Content stream operator execution

---

## Call Sites (Test Usage)

The function is tested in `resolver.rs`:
- Line 1162: Test with ToUnicode
- Line 1200: Test without ToUnicode
- Line 1233: Test for encoding-only case

All test calls pass `None` for `resolver`, `source`, and `doc_decompress_counter`, meaning they don't test actual rasterization (only the resolution logic before it).

---

## Summary

The call chain flows from public API → 4-level resolver → shape recognition → rasterization. Parameters (`resolver`, `source`, `doc_decompress_counter`) are available at the top two levels but only reach the rasterizer via a callback closure. The `DocumentContext` struct exists but is currently unused by `rasterize_type3_glyph()`, representing a known limitation for future XObject resolution support.
