//! Mock test fixtures for testing Type3 glyph rasterization.
//!
//! This module provides minimal implementations of resolver, source, and counter
//! types for testing purposes. These fixtures intentionally provide only the
//! functionality needed to verify parameter passing - they are not full implementations.
//!
//! # Design Principles
//!
//! - **Minimal**: Each fixture provides only what's needed to verify parameter passing
//! - **No full functionality**: Mock resolver doesn't actually resolve objects; mock source returns fixed data
//! - **Verification focused**: Fixtures track whether they were called to verify callback invocation
//!
//! # Usage
//!
//! ```rust
//! use pdftract_core::tests::test_fixtures::{MockResolver, MockSource, DecompressCounter};
//!
//! let resolver = MockResolver::new();
//! let source = MockSource::with_data(vec![1, 2, 3]);
//! let counter = DecompressCounter::new();
//!
//! // Use in tests to verify parameter passing
//! assert!(!resolver.was_called());
//! // ... invoke code that should use resolver ...
//! assert!(resolver.was_called());
//! ```

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use pdftract_core::parser::stream::PdfSource;
use pdftract_core::parser::xref::XrefResolver;

/// Mock resolver fixture.
///
/// This is a minimal wrapper around the real XrefResolver that tracks
/// whether it has been called. For testing purposes, we can use the
/// real XrefResolver since it provides a `new()` constructor.
pub struct MockResolver {
    /// The underlying resolver (real implementation)
    pub resolver: XrefResolver,
    /// Tracks whether this resolver was invoked
    pub called: Arc<AtomicBool>,
}

impl MockResolver {
    /// Create a new mock resolver.
    pub fn new() -> Self {
        Self {
            resolver: XrefResolver::new(),
            called: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Mark this resolver as having been called.
    pub fn mark_called(&self) {
        self.called.store(true, Ordering::SeqCst);
    }

    /// Check if this resolver was called.
    pub fn was_called(&self) -> bool {
        self.called.load(Ordering::SeqCst)
    }
}

impl Default for MockResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock source fixture.
///
/// Minimal implementation of PdfSource that tracks read operations
/// and returns fixed test data. This verifies that source parameters
/// are passed correctly through callbacks.
#[derive(Debug, Clone)]
pub struct MockSource {
    /// Tracks whether this source was invoked
    pub called: Arc<AtomicBool>,
    /// Fixed data to return from read_at
    pub data: Vec<u8>,
}

impl MockSource {
    /// Create a new mock source with empty data.
    pub fn new() -> Self {
        Self {
            called: Arc::new(AtomicBool::new(false)),
            data: Vec::new(),
        }
    }

    /// Create a mock source with specific test data.
    pub fn with_data(data: Vec<u8>) -> Self {
        Self {
            called: Arc::new(AtomicBool::new(false)),
            data,
        }
    }

    /// Mark this source as having been called.
    pub fn mark_called(&self) {
        self.called.store(true, Ordering::SeqCst);
    }

    /// Check if this source was called.
    pub fn was_called(&self) -> bool {
        self.called.load(Ordering::SeqCst)
    }
}

impl Default for MockSource {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfSource for MockSource {
    fn read_at(&self, _offset: u64, len: usize) -> std::io::Result<Vec<u8>> {
        self.mark_called();
        // Return zeros or truncated data based on request
        let actual_len = len.min(self.data.len());
        Ok(self.data[..actual_len].to_vec())
    }

    fn len(&self) -> std::io::Result<u64> {
        self.mark_called();
        Ok(self.data.len() as u64)
    }

    fn is_empty(&self) -> std::io::Result<bool> {
        self.mark_called();
        Ok(self.data.is_empty())
    }
}

/// Mock decompression counter fixture.
///
/// Thread-safe counter that tracks decompression operations.
/// Uses Arc<AtomicU64> for Send + Sync compatibility.
#[derive(Debug, Clone)]
pub struct DecompressCounter {
    /// The underlying atomic counter
    pub inner: Arc<AtomicU64>,
}

impl DecompressCounter {
    /// Create a new counter initialized to zero.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Increment the counter by a specific amount.
    pub fn increment(&self, amount: u64) {
        self.inner.fetch_add(amount, Ordering::SeqCst);
    }

    /// Get the current counter value.
    pub fn get(&self) -> u64 {
        self.inner.load(Ordering::SeqCst)
    }

    /// Reset the counter to zero.
    pub fn reset(&self) {
        self.inner.store(0, Ordering::SeqCst);
    }
}

impl Default for DecompressCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Alternative counter using Mutex<usize> for non-atomic scenarios.
///
/// This provides a simpler counter type when atomic operations are
/// not required (single-threaded test contexts).
#[derive(Debug, Clone)]
pub struct SimpleCounter {
    /// The underlying counter
    pub inner: Arc<Mutex<usize>>,
}

impl SimpleCounter {
    /// Create a new simple counter initialized to zero.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(0)),
        }
    }

    /// Increment the counter.
    pub fn increment(&self) {
        let mut count = self.inner.lock().unwrap();
        *count += 1;
    }

    /// Get the current count.
    pub fn get(&self) -> usize {
        let count = self.inner.lock().unwrap();
        *count
    }

    /// Reset to zero.
    pub fn reset(&self) {
        let mut count = self.inner.lock().unwrap();
        *count = 0;
    }
}

impl Default for SimpleCounter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod fixtures_tests {
    use super::*;

    #[test]
    fn test_mock_resolver_initial_state() {
        let mock = MockResolver::new();
        assert!(!mock.was_called(), "New mock resolver should not be called");
    }

    #[test]
    fn test_mock_resolver_mark_and_check() {
        let mock = MockResolver::new();
        mock.mark_called();
        assert!(mock.was_called(), "Should be marked as called");
    }

    #[test]
    fn test_mock_source_default() {
        let source = MockSource::new();
        assert!(!source.was_called());
        assert!(source.data.is_empty());
    }

    #[test]
    fn test_mock_source_with_data() {
        let data = vec![1, 2, 3, 4, 5];
        let source = MockSource::with_data(data.clone());
        assert_eq!(source.data, data);
    }

    #[test]
    fn test_mock_source_read_at_marks_called() {
        let source = MockSource::with_data(vec![1, 2, 3, 4, 5]);
        let result = source.read_at(0, 3);
        assert!(result.is_ok());
        assert!(source.was_called(), "read_at should mark source as called");
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_mock_source_len_marks_called() {
        let source = MockSource::with_data(vec![1, 2, 3, 4, 5]);
        let result = source.len();
        assert!(result.is_ok());
        assert!(source.was_called(), "len should mark source as called");
        assert_eq!(result.unwrap(), 5);
    }

    #[test]
    fn test_decompress_counter_default() {
        let counter = DecompressCounter::new();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_decompress_counter_increment() {
        let counter = DecompressCounter::new();
        counter.increment(5);
        assert_eq!(counter.get(), 5);
        counter.increment(3);
        assert_eq!(counter.get(), 8);
    }

    #[test]
    fn test_decompress_counter_reset() {
        let counter = DecompressCounter::new();
        counter.increment(10);
        assert_eq!(counter.get(), 10);
        counter.reset();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_simple_counter_default() {
        let counter = SimpleCounter::new();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_simple_counter_increment() {
        let counter = SimpleCounter::new();
        counter.increment();
        assert_eq!(counter.get(), 1);
        counter.increment();
        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn test_simple_counter_reset() {
        let counter = SimpleCounter::new();
        counter.increment();
        counter.increment();
        counter.increment();
        assert_eq!(counter.get(), 3);
        counter.reset();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_all_fixtures_compile() {
        // This test verifies all fixture types can be instantiated
        let _resolver = MockResolver::new();
        let _source = MockSource::new();
        let _counter = DecompressCounter::new();
        let _simple_counter = SimpleCounter::new();

        // If we get here, all fixtures compiled successfully
        assert!(true);
    }
}
