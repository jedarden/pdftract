# detect_char_proc_type Implementation Review

## Overview

The `detect_char_proc_type` function in `type3_rasterizer.rs` classifies PDF objects to determine their type for Type 3 CharProc validation. This review documents the implementation logic, edge cases, and test strategy.

## CharProcType Variants

The `CharProcType` enum (line 118) has four variants:

1. **`Stream`** - PDF stream object (contains content stream bytes)
2. **`Dict`** - PDF dictionary object (contains key-value pairs)
3. **`Unknown`** - Unknown type (returned when reference dereferencing fails or no context provided)
4. **`Other(String)`** - Any other PDF object type with a descriptive name

## Detection Logic

### Three Function Layers

1. **`detect_char_proc_type`** (line 163) - Main public API
   - Simple delegation to `detect_char_proc_type_with_context`
   - Ensures circular reference protection is always used

2. **`detect_char_proc_type_with_context`** (line 208) - Reference handling wrapper
   - Accepts optional `DocumentContext` for dereferencing
   - Initializes empty visited set for cycle detection
   - Delegates to implementation function

3. **`detect_char_proc_type_with_context_impl`** (line 216) - Core logic with cycle detection
   - Takes `visited: &mut HashSet<ObjRef>` parameter
   - Handles all classification logic

### Classification Flow

```
PdfObject::Stream(_)        → CharProcType::Stream
PdfObject::Dict(_)          → CharProcType::Dict
PdfObject::Ref(obj_ref)     → [Reference handling logic]
Other types                 → CharProcType::Other(type_name())
```

### Reference Handling Logic (Ref objects)

When encountering `PdfObject::Ref(obj_ref)`:

1. **Check for circular reference**
   - If `visited.contains(obj_ref)` → `CharProcType::Other("circular-reference")`
   - Mark as visited: `visited.insert(*obj_ref)`

2. **Try dereferencing if `doc_context` provided**
   - **Success**: Recursively classify the dereferenced object
   - **Failure - `MissingCharProcRef` error** → `CharProcType::Other("error")`
   - **Failure - `Io` error** → `CharProcType::Unknown`
   - **Failure - `CircularRef` error** → `CharProcType::Unknown`
   - **Failure - Other errors** → `CharProcType::Unknown`

3. **No doc_context provided** → `CharProcType::Unknown`

### Other Types (non-Stream/Dict/Ref)

For all other PDF object types, the function returns:
```
CharProcType::Other(object.type_name())
```

The `type_name()` method (defined in `parser/object/types.rs` line 258) returns:
- `"null"` - Null objects
- `"boolean"` - Bool objects
- `"integer"` - Integer objects
- `"real"` - Real objects
- `"string"` - String objects
- `"name"` - Name objects
- `"array"` - Array objects
- `"reference"` - Ref objects (should never reach this match arm due to early handling)
- `"stream"` - Stream objects (handled by early match)
- `"dictionary"` - Dict objects (handled by early match)
- `"indirect"` - Indirect objects

## Edge Cases and Special Behaviors

1. **Circular references** - Detected via visited set, returns `Other("circular-reference")`

2. **Missing references** - Returns `Other("error")` for not-found errors (test expectation matches this)

3. **I/O errors** - Returns `Unknown` (truly unresolved cases)

4. **No context provided** - Returns `Unknown` for Ref objects (cannot dereference without context)

5. **Recursive classification** - Dereferenced objects are classified recursively, maintaining the visited set

6. **Validation errors during dereferencing** - Returns `Unknown` (these shouldn't occur during dereferencing but are handled)

## Test Strategy

### Existing Test Coverage

The `type3_rasterizer_test.rs` file already has tests for:
- `test_detect_char_proc_type_dict` - Dictionary objects
- `test_detect_char_proc_type_stream` - Stream objects (regression check)
- `test_detect_char_proc_type_other_integer` - Integer objects
- `test_detect_char_proc_type_other_string` - String objects
- `test_detect_char_proc_type_other_name` - Name objects
- `test_detect_char_proc_type_other_array` - Array objects
- `test_detect_char_proc_type_other_boolean` - Boolean objects (true and false)
- `test_detect_char_proc_type_other_null` - Null objects

### Additional Test Coverage Needed

1. **Reference handling with context**:
   - Successful dereferencing → Stream
   - Successful dereferencing → Dict
   - Successful dereferencing → Other type
   - Missing reference → `Other("error")`
   - I/O error → `Unknown`
   - Circular reference → `Other("circular-reference")`

2. **Reference handling without context**:
   - Ref without doc_context → `Unknown`

3. **Recursive references**:
   - Ref → Ref → Stream (multi-level)
   - Ref → Ref → Dict (multi-level)

4. **Edge case types**:
   - Real objects → `Other("real")`
   - Indirect objects → `Other("indirect")`

5. **DocumentContext interaction**:
   - Test with minimal test context (helper exists at line 35)

## Key Implementation Notes

1. **Error categorization**: The implementation distinguishes between "error" (missing refs) and "unknown" (I/O errors, no context) for debugging purposes

2. **Visited set lifecycle**: The visited set is passed through recursive calls to detect cycles across the entire reference chain

3. **Type preservation**: The function preserves type information through the `type_name()` call for non-special types

4. **Context-dependent behavior**: The function behaves differently based on whether `doc_context` is provided, enabling both basic classification and deep dereferencing

## References

- Implementation: `crates/pdftract-core/src/font/type3_rasterizer.rs` lines 118-281
- CharProcType enum: line 118
- detect_char_proc_type: line 163
- detect_char_proc_type_with_context: line 208
- detect_char_proc_type_with_context_impl: line 216
- type_name() method: `crates/pdftract-core/src/parser/object/types.rs` line 258
- Existing tests: `crates/pdftract-core/src/font/type3_rasterizer_test.rs` lines 1361-1520
