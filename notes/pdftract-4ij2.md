# pdftract-4ij2: Per-thread cycle detection + LRU object cache

## Summary

This bead's implementation was **already complete** in the codebase. The per-thread cycle detection and LRU object cache features have been fully implemented with comprehensive tests.

## Implementation Verification

### Files Modified (already exist)

| File | Purpose |
|------|---------|
| `crates/pdftract-core/src/parser/object/cycle.rs` | Per-thread cycle detection with `thread_local! RefCell<HashSet<ObjRef>>` |
| `crates/pdftract-core/src/parser/object/cache.rs` | LRU cache with `Mutex<LruCache<ObjRef, Arc<PdfObject>>>`, 4096 capacity |
| `crates/pdftract-core/src/parser/object/types.rs` | ObjRef, PdfObject types |
| `crates/pdftract-core/tests/test_cycle_detection.rs` | Integration tests |

### Acceptance Criteria Status

| Criterion | Status | Test |
|----------|--------|------|
| Self-cycle returns PdfNull + STRUCT_CIRCULAR_REF, no stack overflow | ✅ PASS | `test_self_cycle_returns_null_with_diagnostic` |
| 3-cycle (A->B->C->A) detection | ✅ PASS | `test_three_cycle_abc_detected` |
| Legitimate objects cached after cycle detection | ✅ PASS | `test_legitimate_object_after_cycle` |
| Cache hit ratio >= 90% (1000 accesses of 100 objects) | ✅ PASS | `test_cache_hit_ratio_90_percent` |
| LRU eviction at 4097 entries (4096 capacity) | ✅ PASS | `test_lru_eviction_4097_entries` |
| Random resolution sequences terminate | ✅ PASS | `test_random_resolution_sequences_terminate` |

### Implementation Details Verified

#### Per-thread Cycle Detection (`cycle.rs`)
- ✅ `thread_local! { static RESOLVING: RefCell<HashSet<ObjRef>> ... }`
- ✅ `ResolutionGuard` RAII guard for automatic cleanup
- ✅ `is_resolving(obj_ref)` function for cycle checking
- ✅ Thread-local isolation for rayon parallelism

#### LRU Object Cache (`cache.rs`)
- ✅ `lru = "0.12"` crate in `Cargo.toml`
- ✅ `Mutex<LruCache<ObjRef, Arc<PdfObject>>>` with 4096 capacity
- ✅ `CacheStats { hits: u64, misses: u64 }` tracking
- ✅ `MAX_RESOLUTION_DEPTH: u16 = 256` depth limiting
- ✅ Thread-local depth counter with `RESOLUTION_DEPTH`
- ✅ PdfNull NOT cached (prevents poisoning)
- ✅ `CacheResolutionGuard` combining cycle + depth tracking

### Test Results

```bash
$ cargo test --package pdftract-core --test test_cycle_detection
running 9 tests
test test_self_cycle_returns_null_with_diagnostic ... ok
test test_three_cycle_abc_detected ... ok
test test_legitimate_object_after_cycle ... ok
test test_cache_hit_ratio_90_percent ... ok
test test_lru_eviction_4097_entries ... ok
test test_resolution_depth_limit_256 ... ok
test test_thread_local_cycle_detection ... ok
test test_null_not_cached ... ok
test test_random_resolution_sequences_terminate ... ok

test result: ok. 9 passed; 0 failed; 0 ignored
```

### Unit Tests (cache.rs)

All 21 unit tests in `cache.rs` pass:
- Cache hit/miss tracking
- Hit ratio calculation
- Null not cached
- LRU eviction
- Cycle detection
- Depth limit enforcement
- Thread-local independence
- Guard cleanup on panic
- peek_lru/is_lru helpers

### Diagnostic Codes Verified

| Code | Emitted When | Message Includes |
|------|--------------|------------------|
| `STRUCT_CIRCULAR_REF` | Cycle detected | ObjRef in msg |
| `STRUCT_DEPTH_EXCEEDED` | Depth > 256 | Limit value in msg |

## Conclusion

**The implementation is complete and all acceptance criteria PASS.** No code changes were required for this bead—the feature was already implemented.
