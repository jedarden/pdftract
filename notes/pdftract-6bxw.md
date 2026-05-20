# Verification Note: pdftract-6bxw - Object Stream (ObjStm) Parser

## Task
Implement object stream (ObjStm) parser with decompress, cache, and /Extends chain.

## Implementation Summary

### Files Modified
- `crates/pdftract-core/src/parser/objstm.rs` - Complete ObjStm parser implementation

### Implementation Details

The ObjectStmParser provides:
1. **Decompression**: Uses Phase 1.5's `decode_stream()` function to decompress ObjStm stream data
2. **Caching**: `Arc<RwLock<HashMap<ObjRef, ObjStmCacheEntry>>>` for thread-safe caching
3. **Extends chain**: Recursive loading with cycle detection and depth limit (MAX_EXTENDS_DEPTH = 16)
4. **API**:
   - `get_object(host_objstm_ref, embedded_index, source, resolve_fn)` - Main API for xref type-2 entry resolution
   - `load_object_stream(obj_stm_ref, stream_dict, source, resolve_fn)` - Bulk loading API
   - `get_cached(obj_ref)` - Check cache
   - `is_cached(obj_ref)` - Check if cached
   - `take_diagnostics()` - Get accumulated diagnostics

### Key Features

1. **Object Stream Format**:
   - Header: N pairs of (object_number, offset) in first `/First` bytes
   - Body: N embedded objects (no `obj`/`endobj` wrapper)
   - Optional `/Extends N G R` for chain to parent ObjStm

2. **Error Handling**:
   - `MissingKey`: Required `/N` or `/First` missing
   - `InvalidFormat`: Malformed header or data
   - `CircularRef`: Cycle detected in `/Extends` chain
   - `DepthExceeded`: `/Extends` chain exceeds 16 levels
   - `DecompressionFailed`: Stream decompression failed

3. **Safety**:
   - Decompression bomb limit enforced (max_decompress_bytes)
   - Embedded streams rejected (spec violation)
   - Generation number must be 0 for embedded objects
   - Thread-safe caching with Arc and RwLock

### Unit Tests

The following tests verify all acceptance criteria:

1. **test_parse_simple_objstm** - Basic ObjStm with N=2 objects
   - Creates flate-compressed stream with header "1 0 2 3" and objects "42", "true"
   - Verifies both objects parse correctly

2. **test_parse_objstm_10_objects** - **CRITICAL TEST** (Acceptance Criterion 1)
   - Creates ObjStm with N=10 objects of all types
   - Verifies all 10 objects dereference correctly by 0-based index

3. **test_objstm_extends_chain** - **Extends chain** (Acceptance Criterion 2)
   - Parent ObjStm with 3 objects, child ObjStm with 2 objects extending parent
   - Verifies both ObjStms' objects are accessible

4. **test_circular_ref_detection** - **Cyclic /Extends** (Acceptance Criterion 3)
   - ObjStm with `/Extends` pointing to itself
   - Verifies `CircularRef` error is emitted

5. **test_truncated_objstm_body** - **Truncated ObjStm** (Acceptance Criterion 4)
   - ObjStm where last object is truncated ("fal" instead of "false")
   - Verifies partial objects returned and diagnostics emitted

6. **test_decompression_bomb_objstm** - **Decompression bomb** (Acceptance Criterion 5)
   - ObjStm with very small max_decompress_bytes limit
   - Verifies STREAM_BOMB diagnostic emitted

7. **test_cache_hit** - **Cache verification** (Acceptance Criterion 6)
   - Loads same ObjStm twice
   - Verifies second call returns cached Arc via `Arc::ptr_eq`

8. **test_get_object_api** - Xref type-2 entry resolution API
   - Tests the `get_object()` method with 0-based index
   - Verifies caching on second call

9. **test_embedded_stream_rejected** - Embedded stream detection
   - Verifies embedded objects that are Streams are rejected with diagnostic

10. **test_extends_depth_exceeded** - Depth limit enforcement
    - Creates chain of 17 ObjStms (exceeds MAX_EXTENDS_DEPTH of 16)
    - Verifies `DepthExceeded` error

11. **test_missing_key_n** - Missing /N key
    - Verifies `MissingKey` error when /N is absent

12. **test_missing_key_first** - Missing /First key
    - Verifies `MissingKey` error when /First is absent

## Acceptance Criteria Status

| Criterion | Status | Test |
|-----------|--------|------|
| Critical test: N=10 objects all dereference | ✅ PASS | test_parse_objstm_10_objects |
| /Extends chain: both ObjStms' objects dereference | ✅ PASS | test_objstm_extends_chain |
| Cyclic /Extends: emits STRUCT_CIRCULAR_REF | ✅ PASS | test_circular_ref_detection |
| Truncated ObjStm: partial objects + diagnostic | ✅ PASS | test_truncated_objstm_body |
| Decompression bomb: emits STREAM_BOMB | ✅ PASS | test_decompression_bomb_objstm |
| Cache hit: returns cached Arc | ✅ PASS | test_cache_hit |

## Integration Points

1. **Phase 1.3 (xref)**: The `get_object()` method is designed to be called by the xref resolver when it encounters a type-2 (compressed) xref entry. The API signature matches the xref resolver's needs.

2. **Phase 1.5 (stream decoder)**: Uses `decode_stream()` function to decompress the ObjStm stream data with full filter pipeline support.

3. **Diagnostics**: Emits diagnostics using the unified `crate::diagnostics` module with proper error codes.

## Notes

- The ObjStm parser implementation is complete and all acceptance criteria are met
- Unit tests cover all critical paths and edge cases
- The code follows the existing patterns in the codebase (Arc/RwLock for caching, Result types for errors)
- Thread-safe design allows concurrent access from multiple threads (important for rayon parallelism in Phase 4)
