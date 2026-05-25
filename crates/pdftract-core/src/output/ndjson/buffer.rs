//! Out-of-order buffer for streaming page frames.
//!
//! Rayon may complete pages in any order, but NDJSON consumers expect
//! pages in page_index order. This buffer holds completed pages and
//! emits them in order using a fixed-size heap with Condvar backpressure.

use crate::output::ndjson::frames::PageFrame;
use std::collections::{BinaryHeap, HashMap};
use std::sync::{Condvar, Mutex};

/// Maximum number of completed pages to buffer before blocking.
///
/// This window size is chosen to be larger than the typical rayon thread
/// pool size (4–8 threads), ensuring the output thread is never the bottleneck
/// on balanced workloads. For pathological cases (one very slow page surrounded
/// by fast pages), this acts as backpressure to the downstream consumer.
const BUFFER_WINDOW_SIZE: usize = 8;

/// Entry in the out-of-order buffer.
///
/// We implement Reverse ordering so BinaryHeap acts as a min-heap (smallest
/// page_index first).
#[derive(Debug, Clone)]
struct BufferEntry {
    page_index: usize,
    frame: PageFrame,
}

// Implement Ord so BinaryHeap acts as a min-heap (smallest page_index first)
impl PartialEq for BufferEntry {
    fn eq(&self, other: &Self) -> bool {
        self.page_index == other.page_index
    }
}

impl Eq for BufferEntry {}

impl PartialOrd for BufferEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BufferEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse: we want a min-heap (smallest page_index first)
        other.page_index.cmp(&self.page_index)
    }
}

/// Out-of-order buffer for page frames.
///
/// This buffer holds completed pages from rayon workers and allows the output
/// thread to pull them in page_index order. When the buffer is full (holds
/// BUFFER_WINDOW_SIZE completed pages), the push operation blocks until
/// space is available.
///
/// # Example
///
/// ```ignore
/// let buffer = OutOfOrderBuffer::new(0); // next_expected = 0
///
/// // Worker threads push completed pages (may be out of order)
/// buffer.push(PageFrame::new(5, ...));  // page 5 completes first
/// buffer.push(PageFrame::new(2, ...));  // page 2 completes second
///
/// // Output thread pulls in order
/// assert_eq!(buffer.pop_next_in_order()?.page_index, 2); // returns page 2
/// assert_eq!(buffer.pop_next_in_order()?.page_index, 5); // returns page 5
/// ```
pub struct OutOfOrderBuffer {
    /// Next page_index we expect to emit.
    next_expected: Mutex<usize>,

    /// Heap of buffered pages, ordered by page_index.
    /// We use BinaryHeap as a min-heap so the smallest page_index is at top.
    heap: Mutex<BinaryHeap<BufferEntry>>,

    /// Map of buffered pages by page_index for O(1) duplicate detection.
    buffered: Mutex<HashMap<usize, PageFrame>>,

    /// Condition variable for blocking when buffer is full.
    condvar: Condvar,

    /// Total number of pages in the document.
    /// Used to signal completion when all pages have been pushed.
    total_pages: usize,
}

impl OutOfOrderBuffer {
    /// Create a new out-of-order buffer.
    ///
    /// # Arguments
    ///
    /// * `total_pages` - Total number of pages in the document
    pub fn new(total_pages: usize) -> Self {
        Self {
            next_expected: Mutex::new(0),
            heap: Mutex::new(BinaryHeap::new()),
            buffered: Mutex::new(HashMap::new()),
            condvar: Condvar::new(),
            total_pages,
        }
    }

    /// Push a completed page into the buffer.
    ///
    /// If the buffer already holds BUFFER_WINDOW_SIZE completed pages,
    /// this method blocks until space is available (backpressure).
    ///
    /// # Arguments
    ///
    /// * `frame` - The completed page frame to buffer
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Frame was successfully buffered
    /// * `Err(_)` - Frame was a duplicate (already buffered)
    pub fn push(&self, frame: PageFrame) -> Result<(), PushError> {
        let page_index = frame.page_index;

        // Check for duplicate
        {
            let mut buffered = self.buffered.lock().unwrap();
            if buffered.contains_key(&page_index) {
                return Err(PushError::Duplicate(page_index));
            }
            buffered.insert(page_index, frame.clone());
        }

        // Add to heap
        {
            let mut heap = self.heap.lock().unwrap();
            heap.push(BufferEntry { page_index, frame });
        }

        // Block if buffer is full (backpressure)
        let mut heap = self.heap.lock().unwrap();
        while heap.len() > BUFFER_WINDOW_SIZE {
            heap = self.condvar.wait(heap).unwrap();
        }

        Ok(())
    }

    /// Pop the next in-order page frame, if available.
    ///
    /// Returns `None` if the next expected page hasn't completed yet.
    /// Returns `None` if all pages have been emitted (next_expected >= total_pages).
    ///
    /// # Returns
    ///
    /// * `Some(frame)` - The next in-order page frame
    /// * `None` - Next expected page not ready, or all pages emitted
    pub fn pop_next_in_order(&self) -> Option<PageFrame> {
        let mut next_expected = self.next_expected.lock().unwrap();
        let mut heap = self.heap.lock().unwrap();

        // Check if we're done
        if *next_expected >= self.total_pages {
            return None;
        }

        // Check if the next expected page is at the top of the heap
        if let Some(entry) = heap.peek() {
            if entry.page_index == *next_expected {
                let entry = heap.pop().unwrap();
                *next_expected += 1;

                // Remove from buffered map
                let mut buffered = self.buffered.lock().unwrap();
                buffered.remove(&entry.page_index);

                // Notify one waiting thread (space available)
                drop(buffered);
                self.condvar.notify_one();

                // Drop heap lock before returning
                drop(heap);
                drop(next_expected);

                return Some(entry.frame);
            }
        }

        // Next expected page not ready yet
        None
    }

    /// Signal that all pages have been pushed.
    ///
    /// After calling this, `pop_next_in_order` will return `None` once
    /// all buffered pages have been emitted.
    pub fn finish(&self) {
        // No-op for now - we use total_pages to detect completion
        // This method exists for API compatibility with future enhancements
    }

    /// Get the number of pages currently buffered.
    pub fn len(&self) -> usize {
        self.heap.lock().unwrap().len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.heap.lock().unwrap().is_empty()
    }

    /// Get the next expected page index.
    pub fn next_expected(&self) -> usize {
        *self.next_expected.lock().unwrap()
    }
}

/// Error type for push operations.
#[derive(Debug, Clone, PartialEq)]
pub enum PushError {
    /// Duplicate page index (already buffered).
    Duplicate(usize),
}

impl std::fmt::Display for PushError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PushError::Duplicate(idx) => write!(f, "Duplicate page index: {}", idx),
        }
    }
}

impl std::error::Error for PushError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_frame(page_index: usize) -> PageFrame {
        PageFrame::new(page_index, "content".to_string(), vec![], vec![], vec![])
    }

    #[test]
    fn test_in_order_push_pop() {
        let buffer = OutOfOrderBuffer::new(5);

        assert_eq!(buffer.push(make_test_frame(0)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(1)), Ok(()));

        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 0);
        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 1);
        assert_eq!(buffer.pop_next_in_order(), None);
    }

    #[test]
    fn test_out_of_order_push_pop() {
        let buffer = OutOfOrderBuffer::new(5);

        // Push pages out of order
        assert_eq!(buffer.push(make_test_frame(3)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(1)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(2)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(0)), Ok(()));

        // Should pop in order
        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 0);
        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 1);
        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 2);
        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 3);
        assert_eq!(buffer.pop_next_in_order(), None);
    }

    #[test]
    fn test_duplicate_detection() {
        let buffer = OutOfOrderBuffer::new(5);

        assert_eq!(buffer.push(make_test_frame(0)), Ok(()));
        assert_eq!(
            buffer.push(make_test_frame(0)),
            Err(PushError::Duplicate(0))
        );
    }

    #[test]
    fn test_gap_in_sequence() {
        let buffer = OutOfOrderBuffer::new(5);

        // Push pages 0, 2, 3 (missing 1)
        assert_eq!(buffer.push(make_test_frame(0)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(2)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(3)), Ok(()));

        // Should only return page 0 (page 1 is missing)
        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 0);
        assert_eq!(buffer.pop_next_in_order(), None); // Page 1 not ready

        // Push page 1
        assert_eq!(buffer.push(make_test_frame(1)), Ok(()));

        // Now should return 1, 2, 3
        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 1);
        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 2);
        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 3);
        assert_eq!(buffer.pop_next_in_order(), None);
    }

    #[test]
    fn test_completion_detection() {
        let buffer = OutOfOrderBuffer::new(3);

        // Push all pages out of order
        assert_eq!(buffer.push(make_test_frame(2)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(0)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(1)), Ok(()));

        // Pop all pages
        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 0);
        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 1);
        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 2);
        assert_eq!(buffer.pop_next_in_order(), None); // All done
    }

    #[test]
    fn test_buffer_size_tracking() {
        let buffer = OutOfOrderBuffer::new(10);

        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());

        buffer.push(make_test_frame(5)).unwrap();
        buffer.push(make_test_frame(3)).unwrap();

        assert_eq!(buffer.len(), 2);
        assert!(!buffer.is_empty());

        buffer.pop_next_in_order(); // Should return None (page 0 not ready)
        assert_eq!(buffer.len(), 2); // Still 2 (nothing popped)
    }
}
