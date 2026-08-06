# Callback Closure Verification - bf-65dq7i

## Summary
Verified the callback closure construction at `resolver.rs:697-702`. The closure correctly captures all required parameters for Type3 glyph stream resolution.

## Function Context
The closure is defined within the `resolve_type3` function (lines 532-540), which receives:
- `resolver: Option<&XrefResolver>` - reference to XrefResolver for dereferencing
- `source: Option<&dyn ParserPdfSource>` - reference to PDF source for reading stream data
- `doc_decompress_counter: Option<&mut u64>` - mutable reference to decompression counter

## If-Let Pattern (Line 694)
```rust
if let (Some(resolver), Some(source), Some(counter)) = (resolver, source, doc_decompress_counter)
```

This pattern destructures the Optionals, creating shadowed bindings:
- `resolver: &XrefResolver` (non-Optional now)
- `source: &dyn ParserPdfSource` (non-Optional now)
- `counter: &mut u64` (non-Optional now)

## Closure Construction (Lines 700-702)
```rust
let callback = |obj_ref: crate::parser::object::ObjRef| -> Option<Vec<u8>> {
    resolve_stream_bytes(obj_ref, resolver, source, counter)
};
```

### Captured Variables and Capture Modes

| Variable | Type | Capture Mode | Lifetime |
|----------|------|---------------|----------|
| `resolver` | `&XrefResolver` | By reference (borrow) | Inherits from function parameter |
| `source` | `&dyn ParserPdfSource` | By reference (borrow) | Inherits from function parameter |
| `counter` | `&mut u64` | By mutable reference (borrow) | Inherits from function parameter |

### Capture Details
1. **`resolver`** - Captured by immutable reference. The helper function `resolve_stream_bytes` takes `&XrefResolver`, matching the capture mode.

2. **`source`** - Captured by immutable reference. The helper function takes `&dyn ParserPdfSource`, matching the capture mode.

3. **`counter`** - Captured by mutable reference (`&mut u64`). The helper function takes `&mut u64`, allowing modification of the decompression counter during stream resolution.

### Closure Type Signature
```rust
impl Fn(crate::parser::object::ObjRef) -> Option<Vec<u8>>
```

The closure is a `Fn` (not `FnMut` or `FnOnce`) because it only passes references to the helper function and doesn't directly mutate its captures.

### Compatibility with StreamResolverFn
The closure is passed to `rasterize_type3_glyph` as `Option<&StreamResolverFn>`, where:
```rust
pub type StreamResolverFn = dyn Fn(ObjRef) -> Option<Vec<u8>> + Send + Sync;
```

**Compatibility verified:**
- Signature matches: `Fn(ObjRef) -> Option<Vec<u8>>`
- `Send + Sync` bounds satisfied:
  - `&XrefResolver` is `Send + Sync`
  - `&dyn ParserPdfSource` trait object requires `Send + Sync`
  - `&mut u64` is `Send + Sync`
- Lifetime elision works correctly: closure lifetimes are tied to the if-let block scope

## Helper Function (Lines 666-692)
```rust
fn resolve_stream_bytes(
    obj_ref: crate::parser::object::ObjRef,
    resolver: &XrefResolver,
    source: &dyn ParserPdfSource,
    counter: &mut u64,
) -> Option<Vec<u8>>
```

The helper function:
1. Takes ownership of `obj_ref` (the closure passes this by value)
2. Takes `resolver` by shared reference
3. Takes `source` by shared reference
4. Takes `counter` by mutable reference (allows bomb protection counter to increment)

## Verification Results

### ✅ PASS - All Parameters Correctly Captured
- `resolver` is available and captured by reference
- `source` is available and captured by reference
- `doc_decompress_counter` (as `counter`) is available and captured by mutable reference

### ✅ PASS - Capture Modes Correct
- Immutable references for `resolver` and `source`
- Mutable reference for `counter` (required for incrementing decompression counter)

### ✅ PASS - Closure Type Compatible
- The closure implements `Fn` and can be passed as `&StreamResolverFn` to `rasterize_type3_glyph`

### ✅ PASS - Lifetime Handling
- Closure inherits lifetimes from function parameters
- No lifetime violations - the closure's lifetime is tied to the if-let block scope
- All borrowed references (`resolver`, `source`, `counter`) remain valid for the closure's duration

## Architectural Note
This is a workaround pattern for lifetime issues with closures. Rather than having the closure directly capture references (which can cause complex lifetime inference), the code:
1. Defines a standalone helper function with explicit lifetime parameters
2. Creates a closure that delegates to this helper function
3. The closure captures references implicitly through the function call

This pattern avoids explicit lifetime annotations on the closure while maintaining correct borrowing semantics.

## How the Closure is Used
The closure is passed immediately to `rasterize_type3_glyph` at line 704:
```rust
rasterize_type3_glyph(font, &glyph_name, Some(&doc_ctx), Some(&callback))
```

The `rasterize_type3_glyph` function uses this callback to:
1. Receive `ObjRef` instances from Type3 glyph content streams
2. Resolve each reference to actual stream bytes
3. Return decoded bytes for rasterization

This allows Type3 glyphs to reference images or other content in the PDF document, which the rasterizer can dereference through the callback.

## References
- `crates/pdftract-core/src/font/resolver.rs:532-540` - `resolve_type3` function signature
- `crates/pdftract-core/src/font/resolver.rs:666-692` - `resolve_stream_bytes` helper function
- `crates/pdftract-core/src/font/resolver.rs:694-708` - If-let destructuring and closure construction
- `crates/pdftract-core/src/font/resolver.rs:697-702` - Closure construction
- `crates/pdftract-core/src/font/type3_rasterizer.rs:795` - `StreamResolverFn` type definition
