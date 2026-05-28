//! Per-thread cycle detection for PDF object resolution.
//!
//! This module provides a thread-local resolution stack that prevents infinite
//! recursion when PDF objects contain circular references. Each thread gets its
//! own independent tracking set, making this safe for rayon parallelism during
//! page processing.
//!
//! # Example
//!
//! ```rust,no_run
//! use pdftract_core::parser::object::{ObjRef, cycle::{ResolutionGuard, RESOLVING}};
//!
//! let obj_ref = ObjRef::new(42, 0);
//!
//! // Check if already resolving (cycle detection)
//! if RESOLVING.with_borrow(|set| set.contains(&obj_ref)) {
//!     // Cycle detected - emit diagnostic and return Null
//!     return;
//! }
//!
//! // Start resolution - guard automatically cleans up on drop
//! let _guard = ResolutionGuard::new(obj_ref);
//!
//! // ... resolve object ...
//! // When guard goes out of scope, obj_ref is removed from RESOLVING set
//! ```

use std::cell::RefCell;
use std::collections::HashSet;

use super::ObjRef;

/// Per-thread set of object references currently being resolved.
///
/// Each thread gets its own independent HashSet, allowing concurrent
/// page processing in rayon without lock contention.
///
/// Capacity of 64 is conservative: typical PDF resolution depth is < 10.
thread_local! {
    /// Per-thread set of object references currently being resolved.
    ///
    /// Tracks which object references are on the current thread's resolution
    /// stack to detect cycles. Use [`ResolutionGuard`] for automatic cleanup.
    pub static RESOLVING: RefCell<HashSet<ObjRef>> = RefCell::new(HashSet::with_capacity(64));
}

/// RAII guard that removes an object reference from the resolution set on drop.
///
/// This ensures proper cleanup even if:
/// - The resolution function returns early
/// - A panic occurs during resolution
/// - The resolution fails with an error
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::parser::object::{ObjRef, cycle::ResolutionGuard};
///
/// fn resolve_object(ref: ObjRef) -> Result<PdfObject, Error> {
///     let _guard = ResolutionGuard::new(ref);
///     // ... resolution logic ...
///     Ok(obj)
///     // _guard.drop() removes ref from RESOLVING here
/// }
/// ```
pub struct ResolutionGuard {
    obj_ref: ObjRef,
}

impl ResolutionGuard {
    /// Create a new resolution guard and insert the object reference into the tracking set.
    ///
    /// # Panics
    ///
    /// Panics if the RESOLVING thread-local storage is poisoned. This is
    /// intentional per INV-8: panics during resolution are bugs, not
    /// recoverable errors.
    #[inline]
    pub fn new(obj_ref: ObjRef) -> Self {
        RESOLVING.with_borrow_mut(|set| {
            set.insert(obj_ref);
        });
        Self { obj_ref }
    }

    /// Get the object reference being tracked by this guard.
    #[inline]
    pub fn obj_ref(&self) -> ObjRef {
        self.obj_ref
    }
}

impl Drop for ResolutionGuard {
    fn drop(&mut self) {
        // Remove the object reference from the resolution set.
        // Ignore if already removed (defensive against double-drop).
        RESOLVING.with_borrow_mut(|set| {
            set.remove(&self.obj_ref);
        });
    }
}

/// Check if an object reference is currently being resolved (cycle detection).
///
/// Returns true if the object reference is already in the resolution set,
/// indicating a circular reference.
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::parser::object::{ObjRef, cycle::is_resolving};
///
/// let obj_ref = ObjRef::new(42, 0);
/// if is_resolving(obj_ref) {
///     // Cycle detected!
/// }
/// ```
#[inline]
pub fn is_resolving(obj_ref: ObjRef) -> bool {
    RESOLVING.with_borrow(|set| set.contains(&obj_ref))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_chain_resolves_correctly() {
        // A -> B -> C: each object resolves without cycles
        let refs = [ObjRef::new(1, 0), ObjRef::new(2, 0), ObjRef::new(3, 0)];

        // Simulate linear resolution chain
        {
            let _guard1 = ResolutionGuard::new(refs[0]);
            assert!(is_resolving(refs[0]));
            assert!(!is_resolving(refs[1]));

            {
                let _guard2 = ResolutionGuard::new(refs[1]);
                assert!(is_resolving(refs[0]));
                assert!(is_resolving(refs[1]));
                assert!(!is_resolving(refs[2]));

                {
                    let _guard3 = ResolutionGuard::new(refs[2]);
                    assert!(is_resolving(refs[0]));
                    assert!(is_resolving(refs[1]));
                    assert!(is_resolving(refs[2]));
                }
                // guard3 dropped - ref 2 removed
                assert!(is_resolving(refs[0]));
                assert!(is_resolving(refs[1]));
                assert!(!is_resolving(refs[2]));
            }
            // guard2 dropped - ref 1 removed
            assert!(is_resolving(refs[0]));
            assert!(!is_resolving(refs[1]));
            assert!(!is_resolving(refs[2]));
        }
        // guard1 dropped - ref 0 removed
        assert!(!is_resolving(refs[0]));
        assert!(!is_resolving(refs[1]));
        assert!(!is_resolving(refs[2]));
    }

    #[test]
    fn test_cycle_detection_ab() {
        // A -> B -> A cycle
        let ref_a = ObjRef::new(1, 0);
        let ref_b = ObjRef::new(2, 0);

        // Start resolving A
        let _guard_a = ResolutionGuard::new(ref_a);
        assert!(is_resolving(ref_a));
        assert!(!is_resolving(ref_b));

        // While resolving A, we encounter B
        let _guard_b = ResolutionGuard::new(ref_b);
        assert!(is_resolving(ref_a));
        assert!(is_resolving(ref_b));

        // While resolving B, we encounter A again - cycle!
        assert!(is_resolving(ref_a));
    }

    #[test]
    fn test_cycle_detection_self() {
        // Self-referencing: A -> A
        let ref_a = ObjRef::new(1, 0);

        // Start resolving A
        let _guard = ResolutionGuard::new(ref_a);

        // While resolving A, we encounter A again - cycle!
        assert!(is_resolving(ref_a));
    }

    #[test]
    fn test_guard_drop_on_panic() {
        use std::panic;

        let ref_a = ObjRef::new(1, 0);

        // Guard should clean up even if panic occurs
        let result = panic::catch_unwind(|| {
            let _guard = ResolutionGuard::new(ref_a);
            assert!(is_resolving(ref_a));
            panic!("intentional panic");
        });

        assert!(result.is_err());
        // After panic, the guard should have dropped and cleaned up
        assert!(!is_resolving(ref_a));
    }

    #[test]
    fn test_cross_thread_independence() {
        use std::thread;

        let ref_a = ObjRef::new(1, 0);
        let ref_b = ObjRef::new(2, 0);

        // Main thread resolves A
        let _guard_main = ResolutionGuard::new(ref_a);
        assert!(is_resolving(ref_a));

        // Spawn a thread that resolves B - should have its own RESOLVING set
        let handle = thread::spawn(move || {
            let _guard = ResolutionGuard::new(ref_b);
            assert!(is_resolving(ref_b));
            // Main thread's ref_a should NOT be visible in this thread
            assert!(!is_resolving(ref_a));
        });

        handle.join().unwrap();

        // Main thread still has ref_a in its set
        assert!(is_resolving(ref_a));
        // Main thread should NOT see ref_b from the spawned thread
        assert!(!is_resolving(ref_b));
    }

    #[test]
    fn test_three_cycle_abc() {
        // A -> B -> C -> A cycle
        let refs = [ObjRef::new(1, 0), ObjRef::new(2, 0), ObjRef::new(3, 0)];

        let _guard_a = ResolutionGuard::new(refs[0]);
        assert!(is_resolving(refs[0]));

        let _guard_b = ResolutionGuard::new(refs[1]);
        assert!(is_resolving(refs[0]));
        assert!(is_resolving(refs[1]));

        let _guard_c = ResolutionGuard::new(refs[2]);
        assert!(is_resolving(refs[0]));
        assert!(is_resolving(refs[1]));
        assert!(is_resolving(refs[2]));

        // While resolving C, we encounter A again - cycle!
        assert!(is_resolving(refs[0]));
    }

    #[test]
    fn test_capacity_sufficient_for_typical_depth() {
        // Test that 64-entry capacity is sufficient
        let mut guards = Vec::with_capacity(70);

        // Insert 64 objects (at capacity)
        for i in 0..64 {
            let obj_ref = ObjRef::new(i as u32, 0);
            guards.push(ResolutionGuard::new(obj_ref));
        }

        // All should be tracked
        for i in 0..64 {
            let obj_ref = ObjRef::new(i as u32, 0);
            assert!(is_resolving(obj_ref));
        }

        // Add one more (should still work, HashSet just grows)
        let obj_ref = ObjRef::new(64, 0);
        guards.push(ResolutionGuard::new(obj_ref));
        assert!(is_resolving(obj_ref));
    }
}
