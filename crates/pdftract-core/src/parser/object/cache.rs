//! LRU object cache with cycle detection and resolution depth limiting.
//!
//! This module provides:
//! - LRU cache for resolved PDF objects (4096 entries)
//! - Per-thread cycle detection integration
//! - Resolution depth limiting (max 256 levels)
//! - Cache statistics (hits, misses)
//!
//! # Architecture
//!
//! - Each `Document` gets its own `ObjectCache` instance
//! - The cache uses `Mutex<LruCache>` for thread safety (contention is minimal)
//! - Per-thread cycle detection via the `cycle` module prevents infinite loops
//! - Resolution depth limit catches pathological deep chains
//!
//! # Example
//!
//! ```rust,no_run
//! use pdftract_core::parser::object::{ObjRef, PdfObject, cache::ObjectCache};
//! use std::sync::Arc;
//!
//! let cache = ObjectCache::new();
//!
//! // Resolve an object with cycle detection
//! let obj_ref = ObjRef::new(42, 0);
//! if let Some(obj) = cache.get(obj_ref) {
//!     // Cache hit - use the cached object
//! } else {
//!     // Cache miss - resolve and insert
//!     let obj = resolve_object(obj_ref);
//!     cache.insert(obj_ref, Arc::new(obj));
//! }
//! ```

use super::cycle::{is_resolving, ResolutionGuard, RESOLVING};
use super::{ObjRef, PdfObject};
use crate::diagnostics::{DiagCode, Diagnostic as Diag};
use std::sync::Arc;
use std::sync::Mutex;
use std::num::NonZeroUsize;
use lru::LruCache;

/// Maximum resolution depth for object references.
///
/// Real PDFs rarely exceed 30 levels. This limit protects against
/// adversarial input that could cause stack overflow through deep chains.
const MAX_RESOLUTION_DEPTH: u16 = 256;

/// RAII guard that manages both thread-local cycle detection and depth tracking.
///
/// This guard:
/// - Holds the cycle detection guard (manages thread-local set)
/// - Holds a reference to the depth counter for cleanup on drop
///
/// When dropped, the guard:
/// - Removes the object reference from the thread-local cycle detection set
/// - Decrements the depth counter
///
/// This ensures proper cleanup even if:
/// - The resolution function returns early
/// - A panic occurs during resolution
pub struct CacheResolutionGuard {
    /// The underlying cycle detection guard (manages thread-local set)
    _guard: ResolutionGuard,
    /// Shared depth counter for cleanup on drop
    depth: Arc<Mutex<u16>>,
}

impl std::fmt::Debug for CacheResolutionGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheResolutionGuard")
            .field("obj_ref", &self._guard.obj_ref())
            .finish()
    }
}

impl CacheResolutionGuard {
    /// Get the object reference being tracked by this guard.
    #[inline]
    pub fn obj_ref(&self) -> ObjRef {
        self._guard.obj_ref()
    }
}

impl Drop for CacheResolutionGuard {
    fn drop(&mut self) {
        // Decrement the depth counter
        if let Ok(mut depth) = self.depth.lock() {
            if *depth > 0 {
                *depth -= 1;
            }
        }
        // The ResolutionGuard drop will handle removing from thread-local set
    }
}

/// Cache statistics.
///
/// Tracks hit rates for diagnostic and performance monitoring.
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
}

impl CacheStats {
    /// Calculate the cache hit ratio as a percentage.
    ///
    /// Returns None if there have been no accesses.
    #[inline]
    pub fn hit_ratio(&self) -> Option<f64> {
        let total = self.hits + self.misses;
        if total == 0 {
            None
        } else {
            Some((self.hits as f64 / total as f64) * 100.0)
        }
    }
}

/// LRU object cache with cycle detection.
///
/// This cache:
/// - Stores up to 4096 resolved objects per document
/// - Tracks per-thread resolution state for cycle detection
/// - Enforces resolution depth limits
/// - Provides cache statistics
///
/// # Thread Safety
///
/// The cache uses `Mutex<LruCache>` for thread safety. PDF document parsing
/// is single-threaded per document, and rayon parallelism happens at the
/// page level (Phase 3), not during object resolution. For inter-document
/// parallelism, each Document has its own cache instance.
pub struct ObjectCache {
    /// LRU cache of resolved objects
    cache: Mutex<LruCache<ObjRef, Arc<PdfObject>>>,
    /// Cache statistics
    stats: Mutex<CacheStats>,
    /// Shared depth counter (Arc allows guards to decrement on drop)
    depth: Arc<Mutex<u16>>,
}

impl ObjectCache {
    /// Create a new object cache with 4096 entry capacity.
    #[inline]
    pub fn new() -> Self {
        ObjectCache {
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(4096).unwrap())),
            stats: Mutex::new(CacheStats::default()),
            depth: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a new object cache with a custom capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or_else(|| NonZeroUsize::new(1).unwrap());
        ObjectCache {
            cache: Mutex::new(LruCache::new(capacity)),
            stats: Mutex::new(CacheStats::default()),
            depth: Arc::new(Mutex::new(0)),
        }
    }

    /// Get a cached object by reference.
    ///
    /// Returns `Some(Arc<PdfObject>)` if the object is cached, `None` otherwise.
    /// A cache miss increments the miss counter.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdftract_core::parser::object::{ObjRef, cache::ObjectCache};
    ///
    /// let cache = ObjectCache::new();
    /// let obj_ref = ObjRef::new(42, 0);
    ///
    /// if let Some(obj) = cache.get(obj_ref) {
    ///     // Cache hit!
    /// } else {
    ///     // Cache miss - need to resolve
    /// }
    /// ```
    #[inline]
    pub fn get(&self, obj_ref: ObjRef) -> Option<Arc<PdfObject>> {
        let mut cache = self.cache.lock().ok()?;
        let result = cache.get(&obj_ref).cloned();

        if result.is_some() {
            if let Ok(mut stats) = self.stats.lock() {
                stats.hits += 1;
            }
        } else {
            if let Ok(mut stats) = self.stats.lock() {
                stats.misses += 1;
            }
        }

        result
    }

    /// Insert a resolved object into the cache.
    ///
    /// If the cache is at capacity, the least-recently-used entry is evicted.
    /// Circular references (PdfNull from cycle detection) are NOT cached.
    ///
    /// # Parameters
    ///
    /// - `obj_ref`: The object reference to cache
    /// - `obj`: The resolved object to store
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdftract_core::parser::object::{ObjRef, PdfObject, cache::ObjectCache};
    /// use std::sync::Arc;
    ///
    /// let cache = ObjectCache::new();
    /// let obj_ref = ObjRef::new(42, 0);
    /// let obj = PdfObject::Integer(123);
    ///
    /// cache.insert(obj_ref, Arc::new(obj));
    /// ```
    #[inline]
    pub fn insert(&self, obj_ref: ObjRef, obj: Arc<PdfObject>) {
        // Critical: Do NOT cache PdfNull from cycle detection
        // Otherwise, legitimate accesses to the same object would return cached Null
        if obj.is_null() {
            return;
        }

        if let Ok(mut cache) = self.cache.lock() {
            cache.put(obj_ref, obj);
        }
    }

    /// Get the current cache statistics.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdftract_core::parser::object::cache::ObjectCache;
    ///
    /// let cache = ObjectCache::new();
    /// let stats = cache.stats();
    /// println!("Hit ratio: {:.1}%", stats.hit_ratio().unwrap_or(0.0));
    /// ```
    #[inline]
    pub fn stats(&self) -> CacheStats {
        self.stats
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    /// Reset the cache statistics.
    ///
    /// Useful for measuring hit ratios over specific operations.
    #[inline]
    pub fn reset_stats(&self) {
        if let Ok(mut stats) = self.stats.lock() {
            *stats = CacheStats::default();
        }
    }

    /// Get the current number of cached objects.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdftract_core::parser::object::cache::ObjectCache;
    ///
    /// let cache = ObjectCache::new();
    /// println!("Cached objects: {}", cache.len());
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.cache
            .lock()
            .map(|c| c.len())
            .unwrap_or(0)
    }

    /// Check if the cache is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all cached objects.
    ///
    /// This does not reset the cache statistics.
    #[inline]
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
    }

    /// Begin resolving an object with cycle and depth checking.
    ///
    /// This method:
    /// 1. Checks the per-thread cycle detection set
    /// 2. Increments the resolution depth counter
    /// 3. Returns an error if a cycle is detected or depth is exceeded
    ///
    /// On success, returns a `ResolutionGuard` that automatically cleans up
    /// when dropped (removes the object from the cycle detection set and
    /// decrements the depth counter).
    ///
    /// # Errors
    ///
    /// - Returns `STRUCT_CIRCULAR_REF` diagnostic if a cycle is detected
    /// - Returns `STRUCT_DEPTH_EXCEEDED` diagnostic if depth limit is reached
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdftract_core::parser::object::{ObjRef, cache::ObjectCache};
    ///
    /// let cache = ObjectCache::new();
    /// let obj_ref = ObjRef::new(42, 0);
    ///
    /// match cache.begin_resolution(obj_ref) {
    ///     Ok(_guard) => {
    ///         // Safe to resolve - guard cleans up on drop
    ///         // ... resolve object ...
    ///     }
    ///     Err(diag) => {
    ///         // Cycle or depth exceeded - handle error
    ///     }
    /// }
    /// ```
    pub fn begin_resolution(&self, obj_ref: ObjRef) -> Result<ResolutionGuard, Diag> {
        // Check per-thread cycle detection first
        if is_resolving(obj_ref) {
            return Err(Diag::with_dynamic_no_offset(
                DiagCode::StructCircularRef,
                format!("Circular reference detected at {}", obj_ref),
            ));
        }

        // Check depth limit
        {
            let mut depth = self.depth.lock().map_err(|_| {
                Diag::with_dynamic_no_offset(
                    DiagCode::StructDepthExceeded,
                    "Lock poisoned - depth tracking unavailable".to_string(),
                )
            })?;

            if *depth >= MAX_RESOLUTION_DEPTH {
                return Err(Diag::with_dynamic_no_offset(
                    DiagCode::StructDepthExceeded,
                    format!(
                        "Resolution depth exceeds limit of {} (obj ref: {})",
                        MAX_RESOLUTION_DEPTH, obj_ref
                    ),
                ));
            }

            *depth += 1;
        }

        // Create the resolution guard (inserts into thread-local RESOLVING set)
        let guard = ResolutionGuard::new(obj_ref);

        Ok(guard)
    }

    /// End resolution and decrement depth counter.
    ///
    /// This is called automatically by the `ResolutionGuard` drop,
    /// but can be called manually if needed.
    #[inline]
    pub fn end_resolution(&self) {
        if let Ok(mut depth) = self.depth.lock() {
            if *depth > 0 {
                *depth -= 1;
            }
        }
    }

    /// Get the least-recently-used entry for testing.
    ///
    /// This is a diagnostic method that peeks at the LRU entry without
    /// modifying its position. Used primarily for testing cache eviction.
    pub fn peek_lru(&self) -> Option<(ObjRef, Arc<PdfObject>)> {
        self.cache
            .lock()
            .ok()?
            .peek_lru()
            .map(|(k, v)| (*k, v.clone()))
    }

    /// Check if an object reference is in the LRU position.
    ///
    /// Used for testing cache eviction behavior.
    pub fn is_lru(&self, obj_ref: ObjRef) -> bool {
        self.peek_lru()
            .map(|(k, _)| k == obj_ref)
            .unwrap_or(false)
    }

    /// Get the current resolution depth for testing.
    ///
    /// Used for testing depth tracking behavior.
    pub fn depth(&self) -> u16 {
        self.depth
            .lock()
            .map(|d| *d)
            .unwrap_or(0)
    }
}

impl Default for ObjectCache {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::PdfObject;

    #[test]
    fn test_cache_hit_miss() {
        let cache = ObjectCache::new();
        let obj_ref = ObjRef::new(42, 0);

        // First access is a miss
        assert!(cache.get(obj_ref).is_none());
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);

        // Insert and access again - should hit
        let obj = Arc::new(PdfObject::Integer(123));
        cache.insert(obj_ref, obj.clone());
        assert!(cache.get(obj_ref).is_some());

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_hit_ratio() {
        let cache = ObjectCache::new();

        // Empty cache - no hit ratio
        assert_eq!(cache.stats().hit_ratio(), None);

        let obj_ref = ObjRef::new(1, 0);
        let obj = Arc::new(PdfObject::Integer(42));

        // Miss then hit = 50% ratio
        cache.get(obj_ref);
        cache.insert(obj_ref, obj.clone());
        cache.get(obj_ref);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_ratio(), Some(50.0));
    }

    #[test]
    fn test_null_not_cached() {
        let cache = ObjectCache::new();
        let obj_ref = ObjRef::new(1, 0);

        // Insert PdfNull - should not be cached
        let null_obj = Arc::new(PdfObject::Null);
        cache.insert(obj_ref, null_obj);

        // Should still miss
        assert!(cache.get(obj_ref).is_none());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = ObjectCache::with_capacity(3);

        let refs = [
            ObjRef::new(1, 0),
            ObjRef::new(2, 0),
            ObjRef::new(3, 0),
            ObjRef::new(4, 0), // This will evict obj 1
        ];

        // Insert 3 objects
        for i in 0..3 {
            cache.insert(refs[i], Arc::new(PdfObject::Integer(i as i64)));
        }

        // Access obj 2 to make it recently-used
        cache.get(refs[1]);

        // Insert 4th object - should evict obj 1 (LRU)
        cache.insert(refs[3], Arc::new(PdfObject::Integer(99)));

        // Obj 1 should be gone
        assert!(cache.get(refs[0]).is_none());

        // Others should still exist
        assert!(cache.get(refs[1]).is_some());
        assert!(cache.get(refs[2]).is_some());
        assert!(cache.get(refs[3]).is_some());
    }

    #[test]
    fn test_cache_clear() {
        let cache = ObjectCache::new();
        let obj_ref = ObjRef::new(1, 0);

        cache.insert(obj_ref, Arc::new(PdfObject::Integer(42)));
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.get(obj_ref).is_none());

        // Stats should persist after clear
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1); // From the earlier miss
    }

    #[test]
    fn test_reset_stats() {
        let cache = ObjectCache::new();
        let obj_ref = ObjRef::new(1, 0);

        // Generate some stats
        cache.get(obj_ref);
        let obj = Arc::new(PdfObject::Integer(42));
        cache.insert(obj_ref, obj.clone());
        cache.get(obj_ref);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);

        cache.reset_stats();
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cycle_detection() {
        let cache = ObjectCache::new();
        let ref_a = ObjRef::new(1, 0);

        // First resolution should succeed
        {
            let _guard = cache.begin_resolution(ref_a).unwrap();
            assert!(_guard.obj_ref() == ref_a);
        }

        // After guard drops, should be able to resolve again
        {
            let _guard = cache.begin_resolution(ref_a).unwrap();
            assert!(_guard.obj_ref() == ref_a);
        }
    }

    #[test]
    fn test_cycle_detection_fails_on_cycle() {
        let cache = ObjectCache::new();
        let ref_a = ObjRef::new(1, 0);

        // First resolution succeeds
        let guard1 = cache.begin_resolution(ref_a).unwrap();

        // Second resolution while first is active should fail (cycle)
        let result = cache.begin_resolution(ref_a);
        assert!(result.is_err());
        let diag = result.unwrap_err();
        assert_eq!(diag.code, DiagCode::StructCircularRef);

        // Clean up
        drop(guard1);
    }

    #[test]
    fn test_depth_limit() {
        let cache = ObjectCache::new();

        // Resolution depth of 256 should succeed
        let mut guards = Vec::with_capacity(256);
        for i in 0..256 {
            let obj_ref = ObjRef::new(i as u32, 0);
            let guard = cache.begin_resolution(obj_ref).unwrap();
            guards.push(guard);
        }

        // 257th resolution should fail
        let obj_ref = ObjRef::new(999, 0);
        let result = cache.begin_resolution(obj_ref);
        assert!(result.is_err());
        let diag = result.unwrap_err();
        assert_eq!(diag.code, DiagCode::StructDepthExceeded);

        // Clean up guards
        drop(guards);
    }

    #[test]
    fn test_depth_tracking_across_resolutions() {
        let cache = ObjectCache::new();
        let obj_ref = ObjRef::new(1, 0);

        // First resolution
        {
            let _guard = cache.begin_resolution(obj_ref).unwrap();
            // Depth should be 1
            assert_eq!(cache.depth(), 1);
        }

        // After guard drops, depth should be 0
        assert_eq!(cache.depth(), 0);
    }

    #[test]
    fn test_peek_lru() {
        let cache = ObjectCache::with_capacity(3);

        let refs = [
            ObjRef::new(1, 0),
            ObjRef::new(2, 0),
            ObjRef::new(3, 0),
        ];

        // Insert in order: 1, 2, 3
        for i in 0..3 {
            cache.insert(refs[i], Arc::new(PdfObject::Integer(i as i64)));
        }

        // LRU should be obj 1 (least recently used)
        let lru = cache.peek_lru();
        assert!(lru.is_some());
        let (k, _) = lru.unwrap();
        assert_eq!(k, refs[0]);

        // Access obj 2 - LRU should still be obj 1
        cache.get(refs[1]);
        let lru = cache.peek_lru();
        assert_eq!(lru.unwrap().0, refs[0]);

        // Access obj 1 - LRU should become obj 2
        cache.get(refs[0]);
        let lru = cache.peek_lru();
        assert_eq!(lru.unwrap().0, refs[1]);
    }

    #[test]
    fn test_is_lru() {
        let cache = ObjectCache::with_capacity(3);

        let refs = [
            ObjRef::new(1, 0),
            ObjRef::new(2, 0),
            ObjRef::new(3, 0),
        ];

        for i in 0..3 {
            cache.insert(refs[i], Arc::new(PdfObject::Integer(i as i64)));
        }

        // Obj 1 should be LRU
        assert!(cache.is_lru(refs[0]));
        assert!(!cache.is_lru(refs[1]));
        assert!(!cache.is_lru(refs[2]));

        // Access obj 1 - obj 2 becomes LRU
        cache.get(refs[0]);
        assert!(!cache.is_lru(refs[0]));
        assert!(cache.is_lru(refs[1]));
        assert!(!cache.is_lru(refs[2]));
    }

    #[test]
    fn test_thread_local_cycle_detection() {
        use std::thread;

        let cache = Arc::new(ObjectCache::new());
        let ref_a = ObjRef::new(1, 0);

        // Main thread resolves A
        let guard1 = cache.begin_resolution(ref_a).unwrap();

        // Spawn a thread - should have its own cycle detection
        let cache_clone = Arc::clone(&cache);
        let handle = thread::spawn(move || {
            // This thread should NOT see A as resolving (different thread-local set)
            let result = cache_clone.begin_resolution(ref_a);
            assert!(result.is_ok(), "Should succeed - different thread-local RESOLVING set");
        });

        handle.join().unwrap();

        // Main thread still has A in its resolution set
        let result = cache.begin_resolution(ref_a);
        assert!(result.is_err(), "Should fail - cycle in main thread");

        drop(guard1);
    }

    #[test]
    fn test_resolution_guard_cleanup_on_panic() {
        use std::panic;

        let cache = ObjectCache::new();
        let obj_ref = ObjRef::new(1, 0);

        // Guard should clean up even if panic occurs
        let result = panic::catch_unwind(|| {
            let _guard = cache.begin_resolution(obj_ref).unwrap();
            // Depth should be 1
            assert_eq!(cache.depth(), 1);
            panic!("intentional panic");
        });

        assert!(result.is_err());

        // After panic, depth should be back to 0
        assert_eq!(cache.depth(), 0);
    }

    #[test]
    fn test_end_resolution_manually() {
        let cache = ObjectCache::new();
        let obj_ref = ObjRef::new(1, 0);

        let _guard = cache.begin_resolution(obj_ref).unwrap();
        assert_eq!(cache.depth(), 1);

        // Manual end_resolution
        cache.end_resolution();
        assert_eq!(cache.depth(), 0);

        // Guard drop should not go negative (defensive)
        drop(_guard);
        assert_eq!(cache.depth(), 0);
    }
}
