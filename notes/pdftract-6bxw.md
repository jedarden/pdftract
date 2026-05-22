# Verification Note: pdftract-6bxw - Object Stream (ObjStm) Parser

## Task
Implement object stream (ObjStm) parser with decompress, cache, and /Extends chain.

## Implementation Summary

### Files
- `crates/pdftract-core/src/parser/objstm.rs` - Complete ObjStm parser implementation (1280 lines)
- `crates/pdftract-core/src/parser/mod.rs` - Re-exports ObjStm types

### Implementation Details

The ObjectStmParser provides:
1. **Decompression**: Uses Phase 1.5's `decode_stream()` function to decompress ObjStm stream data
2. **Caching**: `Arc<RwLock<HashMap<ObjRef, ObjStmCacheEntry>>>` for thread-safe cached access
3. **Extends chain**: Recursive loading with cycle detection (HashSet in_progress) and depth limit (MAX_EXTENDS_DEPTH = 16)
4. **API**:
   - `get_object(host_objstm_ref, embedded_index, source, resolve_fn)` - Main API for xref type-2 entry resolution
   - `load_object_stream(obj_stm_ref, stream, source, resolve_fn)` - Bulk loading API
   - `get_cached(obj_ref)` - Check cache without loading
   - `is_cached(obj_ref)` - Check if cached
   - `take_diagnostics()` - Get accumulated diagnostics

### Key Features

1. **Object Stream Format**:
   - Header: N pairs of (object_number, offset) in first `/First` bytes
   - Body: N embedded objects (no `obj`/`endobj` wrapper per spec)
   - Optional `/Extends N G R` for chain to parent ObjStm

2. **Error Handling** (ObjStmError enum):
   - `MissingKey`: Required `/N` or `/First` missing → DiagCode::StructMissingKey
   - `InvalidFormat`: Malformed header or data → DiagCode::StructInvalidObjstm
   - `CircularRef`: Cycle detected in `/Extends` chain → DiagCode::StructCircularRef
   - `DepthExceeded`: `/Extends` chain exceeds 16 levels → DiagCode::StructDepthExceeded
   - `DecompressionFailed`: Stream decompression failed → DiagCode::StreamDecodeError

3. **Safety**:
   - Decompression bomb limit enforced via doc_decompress_counter
   - Embedded streams rejected (spec violation) → STRUCT_INVALID_OBJSTM diagnostic
   - Thread-safe caching with Arc<Vec<...>> for concurrent reads
   - Cycle detection prevents infinite loops in /Extends chains

## Acceptance Criteria Status

| Criterion | Status | Test |
|-----------|--------|------|
| Critical test: N=10 objects all dereference correctly | ✅ PASS | test_parse_objstm_10_objects |
| /Extends chain: both ObjStms' objects dereference correctly | ✅ PASS | test_objstm_extends_chain |
| Cyclic /Extends: emits STRUCT_CIRCULAR_REF, no infinite loop | ✅ PASS | test_circular_ref_detection |
| Truncated ObjStm: partial objects + STRUCT_INVALID_OBJSTM | ✅ PASS | test_truncated_objstm_body |
| Decompression bomb: emits STREAM_BOMB | ✅ PASS | test_decompression_bomb_objstm |
| Cache hit: returns cached Arc (Arc::ptr_eq verified) | ✅ PASS | test_cache_hit |
| Missing /N or /First: emits STRUCT_MISSING_KEY | ✅ PASS | test_missing_key_n, test_missing_key_first |
| /Extends depth exceeded: emits STRUCT_DEPTH_EXCEEDED | ✅ PASS | test_extends_depth_exceeded |
| Embedded stream rejected: emits STRUCT_INVALID_OBJSTM | ✅ PASS | test_embedded_stream_rejected |
| get_object API for type-2 entries | ✅ PASS | test_get_object_api |

## Test Results (2026-05-22)

```
running 16 tests
test parser::objstm::tests::test_max_extends_depth ... ok
test parser::objstm::tests::test_missing_key_first ... ok
test parser::objstm::tests::test_circular_ref_detection ... ok
test parser::objstm::tests::test_obj_stm_parser_default ... ok
test parser::objstm::tests::test_missing_key_n ... ok
test parser::objstm::tests::test_obj_stm_error_display ... ok
test parser::objstm::tests::test_obj_stm_parser_new ... ok
test parser::objstm::tests::test_decompression_bomb_objstm ... ok
test parser::objstm::tests::test_cache_hit ... ok
test parser::objstm::tests::test_get_object_api ... ok
test parser::objstm::tests::test_embedded_stream_rejected ... ok
test parser::objstm::tests::test_parse_simple_objstm ... ok
test parser::objstm::tests::test_truncated_objstm_body ... ok
test parser::objstm::tests::test_objstm_extends_chain ... ok
test parser::objstm::tests::test_parse_objstm_10_objects ... ok
test parser::objstm::tests::test_extends_depth_exceeded ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 442 filtered out
```

## Integration Points

1. **Phase 1.3 (xref)**: The `get_object()` method is designed to be called by the xref resolver when it encounters a type-2 (compressed) xref entry (`XrefEntry::Compressed { obj_stm_nr, index }`). The API signature accepts `(host_objstm_ref, embedded_index)` and returns `PdfObject`.

2. **Phase 1.5 (stream decoder)**: Uses `decode_stream()` function to decompress the ObjStm stream data with full filter pipeline support (FlateDecode, ASCII85Decode, etc.).

3. **Diagnostics**: Emits diagnostics using the unified `crate::diagnostics` module with proper error codes (StructMissingKey, StructCircularRef, StructDepthExceeded, StructInvalidObjstm, StreamBomb).

## References
- Plan section: Phase 1.2 line 1072 (object streams)
- PDF spec 7.5.7 (Object Streams)
- INV-8 (never panic, always return partial data on errors)
