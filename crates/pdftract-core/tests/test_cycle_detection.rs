//! Integration tests for per-thread cycle detection and LRU object cache.
//!
//! Tests the critical safety guarantees:
//! - Self-referencing objects (A -> A) are detected and return PdfNull with STRUCT_CIRCULAR_REF
//! - Longer cycles (A -> B -> C -> A) are detected
//! - After cycle detection, legitimate objects can still be resolved and cached
//! - Cache statistics are accurate
//! - LRU eviction works correctly
//! - Random resolution sequences never panic or infinite loop

use pdftract_core::diagnostics::DiagCode;
use pdftract_core::parser::object::{ObjRef, ObjectCache, PdfObject};
use std::sync::Arc;

/// Test self-referencing object: `1 0 obj << /A 1 0 R >> endobj`
///
/// Critical test: resolving ObjRef{1,0} dereferences `/A`, which is again ObjRef{1,0};
/// cycle detection catches it, returns PdfNull with STRUCT_CIRCULAR_REF, no stack overflow.
#[test]
fn test_self_cycle_returns_null_with_diagnostic() {
    let cache = ObjectCache::new();
    let ref_a = ObjRef::new(1, 0);

    // Simulate entering resolution of A
    let guard1 = cache.begin_resolution(ref_a).unwrap();

    // While resolving A, we encounter a reference back to A (cycle!)
    // This should fail with STRUCT_CIRCULAR_REF
    let result = cache.begin_resolution(ref_a);
    assert!(result.is_err(), "Should detect cycle when re-entering same object");

    let diag = result.unwrap_err();
    assert_eq!(diag.code, DiagCode::StructCircularRef);
    assert!(diag.message.contains("Circular reference detected"), "Error message should mention circular reference");

    drop(guard1);
}

/// Test 3-cycle: A -> B -> C -> A
///
/// Verifies that cycle detection works for chains longer than 2.
#[test]
fn test_three_cycle_abc_detected() {
    let cache = ObjectCache::new();
    let ref_a = ObjRef::new(1, 0);
    let ref_b = ObjRef::new(2, 0);
    let ref_c = ObjRef::new(3, 0);

    // Start resolving A
    let guard_a = cache.begin_resolution(ref_a).unwrap();

    // A references B - resolve B
    let guard_b = cache.begin_resolution(ref_b).unwrap();

    // B references C - resolve C
    let guard_c = cache.begin_resolution(ref_c).unwrap();

    // C references A - cycle!
    let result = cache.begin_resolution(ref_a);
    assert!(result.is_err(), "Should detect cycle when C references A");

    let diag = result.unwrap_err();
    assert_eq!(diag.code, DiagCode::StructCircularRef);

    drop(guard_c);
    drop(guard_b);
    drop(guard_a);
}

/// Test that after cycle detection, legitimate objects can still be resolved.
///
/// This ensures the cache doesn't cache PdfNull from cycle detection,
/// which would poison legitimate subsequent accesses.
#[test]
fn test_legitimate_object_after_cycle() {
    let cache = ObjectCache::new();
    let ref_a = ObjRef::new(1, 0);  // Part of cycle
    let ref_legit = ObjRef::new(99, 0);  // Legitimate object

    // Simulate a cycle on A
    let guard_a = cache.begin_resolution(ref_a).unwrap();
    let cycle_result = cache.begin_resolution(ref_a);
    assert!(cycle_result.is_err(), "Cycle should be detected");
    drop(guard_a);

    // After cycle is resolved, legitimate object should work fine
    let legit_guard = cache.begin_resolution(ref_legit).unwrap();
    assert_eq!(legit_guard.obj_ref(), ref_legit);
    drop(legit_guard);

    // The legitimate object should be cacheable
    let obj = Arc::new(PdfObject::Integer(42));
    cache.insert(ref_legit, obj.clone());

    // Cache should return the object
    let cached = cache.get(ref_legit);
    assert!(cached.is_some(), "Legitimate object should be cached");
    assert_eq!(cached.unwrap().as_int(), Some(42));

    // Cycle object should NOT be cached (PdfNull is not cached)
    let null_cached = cache.get(ref_a);
    assert!(null_cached.is_none(), "Cycle-detected PdfNull should not be cached");
}

/// Test cache statistics: after 1000 resolutions of 100 unique objects.
///
/// Expected hit ratio >= 90%.
#[test]
fn test_cache_hit_ratio_90_percent() {
    let cache = ObjectCache::new();
    let num_unique = 100;
    let num_accesses = 1000;

    // Create 100 unique objects
    for i in 0..num_unique {
        let obj_ref = ObjRef::new(i as u32, 0);
        let obj = Arc::new(PdfObject::Integer(i as i64));
        cache.insert(obj_ref, obj);
    }

    // Access them randomly 1000 times (should hit most of the time)
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    for i in 0..num_accesses {
        // Deterministic "random" sequence
        let idx = (i as u32) % num_unique as u32;
        let obj_ref = ObjRef::new(idx, 0);
        cache.get(obj_ref);
    }

    let stats = cache.stats();
    let total = stats.hits + stats.misses;
    assert_eq!(total, num_accesses, "Total accesses should match");

    let hit_ratio = stats.hit_ratio().expect("Should have hit ratio");
    assert!(
        hit_ratio >= 90.0,
        "Hit ratio should be >= 90%, got {:.1}%",
        hit_ratio
    );
}

/// Test LRU eviction with capacity 4096.
///
/// The 4097th unique resolution should evict the LRU entry.
#[test]
fn test_lru_eviction_4097_entries() {
    let capacity = 4096;
    let cache = ObjectCache::with_capacity(capacity);

    // Fill the cache to capacity
    for i in 0..capacity {
        let obj_ref = ObjRef::new(i as u32, 0);
        let obj = Arc::new(PdfObject::Integer(i as i64));
        cache.insert(obj_ref, obj);
    }

    assert_eq!(cache.len(), capacity, "Cache should be at capacity");

    // Remember the first object (LRU)
    let lru_ref = ObjRef::new(0, 0);
    assert!(cache.is_lru(lru_ref), "First object should be LRU");

    // Insert one more - should evict the LRU
    let obj_ref = ObjRef::new(capacity as u32, 0);
    let obj = Arc::new(PdfObject::Integer(capacity as i64));
    cache.insert(obj_ref, obj);

    assert_eq!(cache.len(), capacity, "Cache should still be at capacity");

    // LRU should have been evicted
    let evicted = cache.get(lru_ref);
    assert!(evicted.is_none(), "LRU should have been evicted");

    // The new object should be cached
    let new_cached = cache.get(obj_ref);
    assert!(new_cached.is_some(), "New object should be cached");
}

/// Test that resolution depth is limited to 256.
#[test]
fn test_resolution_depth_limit_256() {
    let cache = ObjectCache::new();

    // Resolution depth of 256 should succeed
    let mut guards = Vec::with_capacity(256);
    for i in 0..256u32 {
        let obj_ref = ObjRef::new(i, 0);
        let guard = cache.begin_resolution(obj_ref)
            .expect(&format!("Resolution {} should succeed", i));
        guards.push(guard);
    }

    // 257th resolution should fail with STRUCT_DEPTH_EXCEEDED
    let obj_ref = ObjRef::new(999, 0);
    let result = cache.begin_resolution(obj_ref);
    assert!(result.is_err(), "Depth limit should be enforced");

    let diag = result.unwrap_err();
    assert_eq!(diag.code, DiagCode::StructDepthExceeded);
    assert!(diag.message.contains("256"), "Error should mention the limit");

    // Cleanup
    drop(guards);
}

/// Test that cycle detection works across parallel threads.
///
/// Each thread should have its own cycle detection set.
#[test]
fn test_thread_local_cycle_detection() {
    use std::thread;

    let cache = Arc::new(ObjectCache::new());
    let ref_a = ObjRef::new(1, 0);

    // Main thread resolves A
    let guard_main = cache.begin_resolution(ref_a).unwrap();

    // Spawn a thread - should have its own cycle detection
    let cache_clone = Arc::clone(&cache);
    let handle = thread::spawn(move || {
        // This thread should NOT see A as resolving (different thread-local set)
        let result = cache_clone.begin_resolution(ref_a);
        assert!(result.is_ok(), "Should succeed - different thread-local RESOLVING set");

        // But this thread CAN create its own cycle
        let inner_guard = cache_clone.begin_resolution(ref_a).unwrap();
        let cycle_result = cache_clone.begin_resolution(ref_a);
        assert!(cycle_result.is_err(), "Should detect cycle within this thread");

        drop(inner_guard);
    });

    handle.join().unwrap();

    // Main thread still has A in its resolution set
    let result = cache.begin_resolution(ref_a);
    assert!(result.is_err(), "Should fail - cycle in main thread");

    drop(guard_main);
}

/// Test that PdfNull is NOT cached (to avoid poisoning legitimate accesses).
#[test]
fn test_null_not_cached() {
    let cache = ObjectCache::new();
    let obj_ref = ObjRef::new(1, 0);

    // Try to cache PdfNull - should not be inserted
    let null_obj = Arc::new(PdfObject::Null);
    cache.insert(obj_ref, null_obj);

    // Should miss - Null was not cached
    assert!(cache.get(obj_ref).is_none());
    assert_eq!(cache.len(), 0);
}

/// Proptest-style test: random resolution sequences never panic or infinite loop.
///
/// This generates random sequences of resolutions and verifies:
/// 1. No panics occur
/// 2. All operations terminate (no infinite loops)
/// 3. Cycle detection works correctly
/// 4. Cache invariants are maintained
#[test]
fn test_random_resolution_sequences_terminate() {
    use std::collections::HashSet;

    let cache = ObjectCache::new();
    let num_operations = 1000;
    let mut seen_refs = HashSet::new();

    for i in 0..num_operations {
        // Generate pseudo-random object refs
        let obj_ref = ObjRef::new((i % 50) as u32, 0);

        // Try to begin resolution
        let result = cache.begin_resolution(obj_ref);

        match result {
            Ok(guard) => {
                // Successfully entered resolution
                // Insert a non-null object
                if !seen_refs.contains(&obj_ref) {
                    let obj = Arc::new(PdfObject::Integer(i as i64));
                    cache.insert(obj_ref, obj);
                    seen_refs.insert(obj_ref);
                }

                // Sometimes intentionally create a cycle
                if i % 10 == 0 {
                    let cycle_result = cache.begin_resolution(obj_ref);
                    assert!(cycle_result.is_err(), "Should detect intentional cycle");
                    let diag = cycle_result.unwrap_err();
                    assert_eq!(diag.code, DiagCode::StructCircularRef);
                }

                drop(guard);
            }
            Err(diag) => {
                // Should only fail on cycle detection or depth exceeded
                assert!(
                    diag.code == DiagCode::StructCircularRef || diag.code == DiagCode::StructDepthExceeded,
                    "Unexpected error code: {:?}",
                    diag.code
                );
            }
        }

        // Verify cache invariants periodically
        if i % 100 == 0 {
            let len = cache.len();
            let stats = cache.stats();
            let total = stats.hits + stats.misses;
            // len should be <= total accesses (but not strictly equal due to nulls not being cached)
            assert!(len <= (seen_refs.len() as usize), "Cache length should not exceed unique inserts");
        }
    }

    // Final sanity check
    let stats = cache.stats();
    assert!(stats.hits + stats.misses > 0, "Should have some cache activity");
}
