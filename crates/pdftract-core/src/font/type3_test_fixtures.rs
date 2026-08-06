//! Mock test fixtures for Type3 rasterizer tests.
//!
//! This module provides minimal mock implementations of resolver, source,
//! and counter types for testing parameter passing in callbacks.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// Mock resolver tracking flag.
///
/// Minimal fixture to verify resolver parameter was passed to a callback.
/// Uses `Arc<AtomicBool>` so it can be shared and cloned across threads.
///
/// # Example
///
/// ```rust
/// let resolver_called = Arc::new(AtomicBool::new(false));
/// let resolver_clone = resolver_called.clone();
/// let callback = move |obj_ref| {
///     resolver_clone.store(true, Ordering::SeqCst);
///     Some(b"test content".to_vec())
/// };
/// callback(ObjRef::new(1, 0));
/// assert!(resolver_called.load(Ordering::SeqCst));
/// ```
pub type MockResolver = Arc<AtomicBool>;

/// Create a new mock resolver flag initialized to false.
///
/// # Returns
///
/// A `MockResolver` (Arc<AtomicBool>) set to false.
pub fn mock_resolver() -> MockResolver {
    Arc::new(AtomicBool::new(false))
}

/// Mock source tracking flag.
///
/// Minimal fixture to verify source parameter was passed to a callback.
/// Uses `Arc<AtomicBool>` so it can be shared and cloned across threads.
///
/// # Example
///
/// ```rust
/// let source_used = Arc::new(AtomicBool::new(false));
/// let source_clone = source_used.clone();
/// let callback = move |obj_ref| {
///     source_clone.store(true, Ordering::SeqCst);
///     Some(b"test content".to_vec())
/// };
/// callback(ObjRef::new(1, 0));
/// assert!(source_used.load(Ordering::SeqCst));
/// ```
pub type MockSource = Arc<AtomicBool>;

/// Create a new mock source flag initialized to false.
///
/// # Returns
///
/// A `MockSource` (Arc<AtomicBool>) set to false.
pub fn mock_source() -> MockSource {
    Arc::new(AtomicBool::new(false))
}

/// Mock counter for tracking callback invocations.
///
/// Minimal fixture using `Arc<AtomicU64>` to track how many times
/// a callback was invoked or how many operations were performed.
///
/// # Example
///
/// ```rust
/// let counter = Arc::new(AtomicU64::new(0));
/// let counter_clone = counter.clone();
/// let callback = move |obj_ref| {
///     counter_clone.fetch_add(1, Ordering::SeqCst);
///     Some(b"test content".to_vec())
/// };
/// callback(ObjRef::new(1, 0));
/// callback(ObjRef::new(2, 0));
/// assert_eq!(counter.load(Ordering::SeqCst), 2);
/// ```
pub type MockCounter = Arc<AtomicU64>;

/// Create a new mock counter initialized to zero.
///
/// # Returns
///
/// A `MockCounter` (Arc<AtomicU64>) set to 0.
pub fn mock_counter() -> MockCounter {
    Arc::new(AtomicU64::new(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::types::ObjRef;

    #[test]
    fn test_mock_resolver_flag() {
        let resolver = mock_resolver();
        assert!(!resolver.load(Ordering::SeqCst));

        resolver.store(true, Ordering::SeqCst);
        assert!(resolver.load(Ordering::SeqCst));
    }

    #[test]
    fn test_mock_source_flag() {
        let source = mock_source();
        assert!(!source.load(Ordering::SeqCst));

        source.store(true, Ordering::SeqCst);
        assert!(source.load(Ordering::SeqCst));
    }

    #[test]
    fn test_mock_counter_increment() {
        let counter = mock_counter();
        assert_eq!(counter.load(Ordering::SeqCst), 0);

        counter.fetch_add(1, Ordering::SeqCst);
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        counter.fetch_add(1, Ordering::SeqCst);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_callback_captures_all_parameters() {
        let resolver = mock_resolver();
        let source = mock_source();
        let counter = mock_counter();

        let resolver_clone = resolver.clone();
        let source_clone = source.clone();
        let counter_clone = counter.clone();

        // Callback that uses all three parameters
        let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
            resolver_clone.store(true, Ordering::SeqCst);
            source_clone.store(true, Ordering::SeqCst);
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Some(b"test".to_vec())
        };

        // Invoke callback
        callback(ObjRef::new(1, 0));

        // Verify all parameters were captured/used
        assert!(resolver.load(Ordering::SeqCst), "resolver flag should be set");
        assert!(source.load(Ordering::SeqCst), "source flag should be set");
        assert_eq!(counter.load(Ordering::SeqCst), 1, "counter should be 1");
    }

    #[test]
    fn test_cloning_creates_independent_references() {
        let resolver1 = mock_resolver();
        let resolver2 = resolver1.clone();

        resolver1.store(true, Ordering::SeqCst);
        assert!(resolver2.load(Ordering::SeqCst), "clone should see the same value");

        resolver2.store(false, Ordering::SeqCst);
        assert!(!resolver1.load(Ordering::SeqCst), "changes are reflected in both");
    }
}
