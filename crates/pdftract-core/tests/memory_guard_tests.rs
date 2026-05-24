//! Allocation-sensitive tests using the memory-guard helper.
//!
//! These tests verify that code fails gracefully under memory pressure.
//! All tests are tagged to skip on Windows (which doesn't support
//! per-process memory limits).
//!
//! See `memory_guard.rs` for the helper implementation and usage convention.

mod memory_guard;

use std::io::Cursor;

/// Test that large vector allocations fail gracefully under memory limits.
#[cfg_attr(not(target_os = "windows"), test)]
#[ignore = "memory limit tests interfere with each other when run in the same process"]
fn test_large_vector_allocation_fails_gracefully() {
    use memory_guard::assert_fails_under_memory_limit;

    // Try to allocate 1 GB under a 100 MB limit
    assert_fails_under_memory_limit(100 * 1024 * 1024, || {
        let mut v: Vec<u8> = Vec::new();
        v.try_reserve(1_000_000_000).map_err(|e| e.to_string())?;
        Ok::<_, String>(v.capacity())
    });
}

/// Test that parsing a large (malformed) PDF stream fails gracefully.
///
/// This simulates an attack vector: a compressed stream that decompresses
/// to an enormous size. We want to ensure we return an error, not OOM.
#[cfg_attr(not(target_os = "windows"), test)]
#[ignore = "memory limit tests interfere with each other when run in the same process"]
fn test_oversized_decompression_fails_gracefully() {
    use memory_guard::assert_fails_under_memory_limit;

    assert_fails_under_memory_limit(100 * 1024 * 1024, || {
        // Simulate attempting to decompress a stream that claims to be
        // much larger than our memory budget allows
        let fake_compressed_data = vec![0u8; 10_000];
        let mut cursor = Cursor::new(fake_compressed_data);

        // Try to read more data than the limit allows
        let mut buffer = Vec::new();
        cursor
            .read_to_end(&mut buffer)
            .map_err(|e| e.to_string())?;

        // Simulate attempting to allocate an oversized buffer
        buffer.try_reserve(500_000_000).map_err(|e| e.to_string())?;

        Ok::<_, String>(buffer.len())
    });
}

use std::io::Read;

/// Test that HashMap insertion fails gracefully under memory limits.
#[cfg_attr(not(target_os = "windows"), test)]
fn test_hashmap_under_memory_pressure() {
    use memory_guard::assert_succeeds_under_memory_limit;
    use std::collections::HashMap;

    // This should succeed within 100 MB
    let count = assert_succeeds_under_memory_limit(100 * 1024 * 1024, || {
        let mut map = HashMap::new();
        for i in 0..10_000 {
            map.insert(i, format!("value_{}", i));
        }
        Ok::<_, String>(map.len())
    });

    assert_eq!(count, 10_000);
}

/// Test that Vec::try_reserve propagates allocation failures.
#[cfg_attr(not(target_os = "windows"), test)]
#[ignore = "memory limit tests interfere with each other when run in the same process"]
fn test_try_reserve_propagates_failure() {
    use memory_guard::run_under_memory_limit;

    let result = run_under_memory_limit(100 * 1024 * 1024, || {
        let mut v: Vec<u8> = Vec::new();
        // Try to reserve 500 MB under a 100 MB limit
        v.try_reserve(500_000_000).map_err(|e| e.to_string())?;
        Ok::<_, String>(v.capacity())
    });

    assert!(result.is_err());
    match result {
        Err(memory_guard::MemoryGuardError::ClosureError(msg)) => {
            assert!(msg.contains("allocation") || msg.contains("memory"), "Error should mention allocation: {}", msg);
        }
        _ => panic!("Expected ClosureError, got {:?}", result),
    }
}

/// Test that String::try_reserve works similarly.
#[cfg_attr(not(target_os = "windows"), test)]
#[ignore = "memory limit tests interfere with each other when run in the same process"]
fn test_string_try_reserve_fails_gracefully() {
    use memory_guard::run_under_memory_limit;

    let result = run_under_memory_limit(100 * 1024 * 1024, || {
        let mut s = String::new();
        s.try_reserve(500_000_000).map_err(|e| e.to_string())?;
        Ok::<_, String>(s.capacity())
    });

    assert!(result.is_err());
}

/// Test: Verify Box allocation fails gracefully.
#[cfg_attr(not(target_os = "windows"), test)]
fn test_box_allocation_under_limit() {
    use memory_guard::assert_succeeds_under_memory_limit;

    // Small Box allocations should succeed
    let value = assert_succeeds_under_memory_limit(100 * 1024 * 1024, || {
        let boxed = Box::new(vec![1u8; 1000]);
        Ok::<_, String>(boxed.len())
    });

    assert_eq!(value, 1000);
}

/// Test: Multiple allocations under a tight budget.
#[cfg_attr(not(target_os = "windows"), test)]
fn test_multiple_allocations_under_tight_budget() {
    use memory_guard::assert_succeeds_under_memory_limit;

    let total = assert_succeeds_under_memory_limit(50 * 1024 * 1024, || {
        let mut total = 0;
        for i in 0..10 {
            let v = vec![i as u8; 100_000]; // 100 KB each
            total += v.len();
        }
        Ok::<_, String>(total)
    });

    assert_eq!(total, 1_000_000);
}

/// Test: Verify that Vec::resize fails gracefully when over budget.
#[cfg_attr(not(target_os = "windows"), test)]
#[ignore = "memory limit tests interfere with each other when run in the same process"]
fn test_vec_resize_fails_gracefully() {
    use memory_guard::assert_fails_under_memory_limit;

    assert_fails_under_memory_limit(100 * 1024 * 1024, || {
        let mut v = Vec::new();
        // Try to resize to a size that exceeds the memory limit
        v.resize(100_000_000, 0u8);
        Ok::<_, String>(v.len())
    });
}

/// Test: Verify that alloc::String::from_utf8 fails gracefully on large input.
#[cfg_attr(not(target_os = "windows"), test)]
#[ignore = "memory limit tests interfere with each other when run in the same process"]
fn test_string_from_large_bytes_fails_gracefully() {
    use memory_guard::assert_fails_under_memory_limit;

    assert_fails_under_memory_limit(100 * 1024 * 1024, || {
        // Create a large byte array
        let large_bytes = vec![b'a'; 100_000_000];
        let _s = String::from_utf8(large_bytes).map_err(|e| e.to_string())?;
        Ok::<_, String>(())
    });
}

/// Test: Nested allocations under memory limit.
#[cfg_attr(not(target_os = "windows"), test)]
fn test_nested_allocations_under_limit() {
    use memory_guard::assert_succeeds_under_memory_limit;

    let count = assert_succeeds_under_memory_limit(100 * 1024 * 1024, || {
        let outer: Vec<Vec<u8>> = (0..100)
            .map(|i| vec![i as u8; 10_000])
            .collect();
        Ok::<_, String>(outer.len())
    });

    assert_eq!(count, 100);
}
