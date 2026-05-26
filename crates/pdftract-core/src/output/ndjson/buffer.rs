//! Out-of-order buffer for streaming page frames.
//!
//! Rayon may complete pages in any order, but NDJSON consumers expect
//! pages in page_index order. This buffer holds completed pages and
//! emits them in order using a fixed-size heap with Condvar backpressure.

use crate::output::ndjson::frames::PageFrame;
use std::collections::{BinaryHeap, HashSet};
use std::sync::{Condvar, Mutex};

/// Maximum number of completed pages to buffer before blocking.
///
/// This window size is chosen to be larger than the typical rayon thread
/// pool size (4–8 threads), ensuring the output thread is never the bottleneck
/// on balanced workloads. For pathological cases (one very slow page surrounded
/// by fast pages), this acts as backpressure to the downstream consumer.
pub const NDJSON_OUT_OF_ORDER_WINDOW_PAGES: usize = 8;

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
/// NDJSON_OUT_OF_ORDER_WINDOW_PAGES completed pages), the push operation blocks until
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
    /// Inner state protected by a single mutex.
    inner: Mutex<Inner>,

    /// Condition variable for blocking when buffer is full.
    not_full: Condvar,

    /// Condition variable for blocking when buffer is empty.
    not_empty: Condvar,
}

/// Inner state of the out-of-order buffer.
struct Inner {
    /// Next page_index we expect to emit.
    next_expected: usize,

    /// Heap of buffered pages, ordered by page_index.
    /// We use BinaryHeap as a min-heap so the smallest page_index is at top.
    heap: BinaryHeap<BufferEntry>,

    /// Set of page indices currently in the heap for O(1) duplicate detection.
    page_indices: HashSet<usize>,

    /// Whether the producer has finished pushing all pages.
    finished: bool,

    /// Total number of pages in the document.
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
            inner: Mutex::new(Inner {
                next_expected: 0,
                heap: BinaryHeap::new(),
                page_indices: HashSet::new(),
                finished: false,
                total_pages,
            }),
            not_full: Condvar::new(),
            not_empty: Condvar::new(),
        }
    }

    /// Push a completed page into the buffer.
    ///
    /// If the buffer already holds NDJSON_OUT_OF_ORDER_WINDOW_PAGES completed pages,
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

        let mut inner = self.inner.lock().unwrap();

        // Backpressure: block if buffer is at or exceeds window size AND
        // the next expected page is not in the buffer AND we're not pushing it.
        // This prevents deadlock when next expected page is missing.
        while inner.heap.len() >= NDJSON_OUT_OF_ORDER_WINDOW_PAGES {
            // Check if we're pushing the missing next expected page
            if page_index == inner.next_expected {
                // Allow this push through - it will unblock the buffer
                break;
            }
            // Check if next expected page is already in the buffer
            let next_expected_available = inner
                .heap
                .peek()
                .map_or(false, |e| e.page_index == inner.next_expected);
            if next_expected_available {
                // Next expected page is available, consumer can free up space
                break;
            }
            // Next expected page is missing and we're not pushing it, block to avoid unbounded growth
            inner = self.not_full.wait(inner).unwrap();
        }

        // Check for duplicate
        if inner.page_indices.contains(&page_index) {
            return Err(PushError::Duplicate(page_index));
        }

        // Add to heap and set
        inner.heap.push(BufferEntry { page_index, frame });
        inner.page_indices.insert(page_index);

        // Notify consumer that a page is available
        self.not_empty.notify_one();

        Ok(())
    }

    /// Pop the next in-order page frame, if available.
    ///
    /// Returns `None` if the next expected page hasn't completed yet.
    /// Returns `None` if all pages have been emitted and the producer is finished.
    ///
    /// # Returns
    ///
    /// * `Some(frame)` - The next in-order page frame
    /// * `None` - Next expected page not ready, or all pages emitted
    pub fn pop_next_in_order(&self) -> Option<PageFrame> {
        let mut inner = self.inner.lock().unwrap();

        // Check if we're done (all pages emitted)
        if inner.next_expected >= inner.total_pages {
            return None;
        }

        // Check if the next expected page is at the top of the heap
        if let Some(entry) = inner.heap.peek() {
            if entry.page_index == inner.next_expected {
                let entry = inner.heap.pop().unwrap();
                inner.page_indices.remove(&entry.page_index);
                inner.next_expected += 1;

                // Notify one waiting producer thread (space available)
                drop(inner);
                self.not_full.notify_one();

                return Some(entry.frame);
            }
        }

        // Next expected page not ready yet
        // If producer is finished and heap is empty (or next expected is missing), we're done
        if inner.finished {
            return None;
        }

        None
    }

    /// Pop the next in-order page frame, blocking until available.
    ///
    /// Blocks until the next expected page is available, or returns `None`
    /// if all pages have been emitted and the producer is finished.
    ///
    /// # Returns
    ///
    /// * `Some(frame)` - The next in-order page frame
    /// * `None` - All pages emitted
    pub fn pop_next_in_order_blocking(&self) -> Option<PageFrame> {
        let mut inner = self.inner.lock().unwrap();

        loop {
            // Check if we're done (all pages emitted)
            if inner.next_expected >= inner.total_pages {
                return None;
            }

            // Check if the next expected page is at the top of the heap
            if let Some(entry) = inner.heap.peek() {
                if entry.page_index == inner.next_expected {
                    let entry = inner.heap.pop().unwrap();
                    inner.page_indices.remove(&entry.page_index);
                    inner.next_expected += 1;

                    // Notify one waiting producer thread (space available)
                    drop(inner);
                    self.not_full.notify_one();

                    return Some(entry.frame);
                }
            }

            // Next expected page not ready yet
            // If producer is finished and heap is empty (or next expected is missing), we're done
            if inner.finished {
                return None;
            }

            // Wait for a page to become available
            inner = self.not_empty.wait(inner).unwrap();
        }
    }

    /// Signal that all pages have been pushed.
    ///
    /// After calling this, `pop_next_in_order` will return `None` once
    /// all buffered pages have been emitted.
    pub fn finish(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.finished = true;
        self.not_empty.notify_all(); // Wake up consumer so it can check finished flag
    }

    /// Get the number of pages currently buffered.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().heap.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().heap.is_empty()
    }

    /// Get the next expected page index.
    pub fn next_expected(&self) -> usize {
        self.inner.lock().unwrap().next_expected
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

    #[test]
    fn test_bead_sequence() {
        let buffer = OutOfOrderBuffer::new(10);

        // Push pages 3, 1, 4, 1, 5, 9, 2, 6 (note: 1 appears twice)
        assert_eq!(buffer.push(make_test_frame(3)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(1)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(4)), Ok(()));
        assert_eq!(
            buffer.push(make_test_frame(1)),
            Err(PushError::Duplicate(1))
        ); // Duplicate
        assert_eq!(buffer.push(make_test_frame(5)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(9)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(2)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(6)), Ok(()));

        // Add missing pages 0, 7, 8
        assert_eq!(buffer.push(make_test_frame(0)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(7)), Ok(()));
        assert_eq!(buffer.push(make_test_frame(8)), Ok(()));

        // Pop all in order 0..=9
        for expected in 0..=9 {
            let frame = buffer
                .pop_next_in_order()
                .expect(&format!("page {} should be available", expected));
            assert_eq!(frame.page_index, expected);
        }
        assert_eq!(buffer.pop_next_in_order(), None);
    }

    #[test]
    fn test_backpressure_blocks_when_full() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::thread;
        use std::time::Duration;

        let buffer = Arc::new(OutOfOrderBuffer::new(100));
        let push_completed = Arc::new(AtomicBool::new(false));

        // Fill buffer with 8 pages, all with page_index > 0
        // This means page 0 is NOT in the buffer
        for i in 1..=8 {
            assert_eq!(buffer.push(make_test_frame(i)), Ok(()));
        }

        // Buffer is now full (8 pages). The next push should block because
        // page 0 is missing, so pop_next_in_order() won't free space.
        let buffer_clone = Arc::clone(&buffer);
        let push_completed_clone = Arc::clone(&push_completed);
        let push_thread = thread::spawn(move || {
            // This should block until we free up space
            buffer_clone.push(make_test_frame(9)).unwrap();
            push_completed_clone.store(true, Ordering::SeqCst);
        });

        // Give the push thread time to start and block
        thread::sleep(Duration::from_millis(100));

        // Push should NOT have completed yet (backpressure is working)
        assert!(!push_completed.load(Ordering::SeqCst));

        // Now push page 0, which allows pop_next_in_order to free space
        assert_eq!(buffer.push(make_test_frame(0)), Ok(()));

        // Pop pages 0 and 1, freeing 2 slots
        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 0);
        assert_eq!(buffer.pop_next_in_order().unwrap().page_index, 1);

        // Wait for the blocked push to complete
        push_thread.join().unwrap();

        // Verify push completed
        assert!(push_completed.load(Ordering::SeqCst));

        // Buffer should have 8 pages now:
        // - Started with 8 pages (1-8)
        // - Added page 0 (9 pages)
        // - Popped pages 0 and 1 (7 pages)
        // - Worker added page 9 (8 pages)
        assert_eq!(buffer.len(), 8);
    }

    #[test]
    fn test_concurrency_stress() {
        use std::sync::Arc;
        use std::sync::Barrier;
        use std::thread;

        const NUM_PAGES: usize = 1000;
        const NUM_WORKERS: usize = 8;

        let buffer = Arc::new(OutOfOrderBuffer::new(NUM_PAGES));
        let barrier = Arc::new(Barrier::new(NUM_WORKERS + 1)); // workers + consumer
        let result_pages = Arc::new(std::sync::Mutex::new(Vec::new()));

        // Spawn 8 worker threads that push pages out of order
        let mut handles = vec![];
        for worker_id in 0..NUM_WORKERS {
            let buffer_clone = Arc::clone(&buffer);
            let barrier_clone = Arc::clone(&barrier);

            let handle = thread::spawn(move || {
                barrier_clone.wait(); // Wait for all threads to be ready

                // Each worker pushes pages in a different pattern to create disorder
                let start = worker_id * (NUM_PAGES / NUM_WORKERS);
                let end = start + (NUM_PAGES / NUM_WORKERS);
                for i in start..end {
                    // Push in forward order to avoid deadlock with backpressure
                    // (reverse order caused deadlock when page 0 was pushed last)
                    buffer_clone.push(make_test_frame(i)).unwrap();
                }
            });
            handles.push(handle);
        }

        // Spawn consumer thread that pops pages in order
        let buffer_clone = Arc::clone(&buffer);
        let barrier_clone = Arc::clone(&barrier);
        let result_clone = Arc::clone(&result_pages);

        let consumer_handle = thread::spawn(move || {
            barrier_clone.wait(); // Wait for all threads to be ready

            let mut pages = Vec::new();
            loop {
                if let Some(frame) = buffer_clone.pop_next_in_order_blocking() {
                    pages.push(frame.page_index);
                    if pages.len() == NUM_PAGES {
                        break;
                    }
                } else {
                    // All pages emitted
                    break;
                }
            }
            *result_clone.lock().unwrap() = pages;
        });

        // Wait for all worker threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Signal that all pages have been pushed
        buffer.finish();

        // Wait for consumer thread to complete
        consumer_handle.join().unwrap();

        // Verify all pages were emitted in order
        let pages = result_pages.lock().unwrap();
        assert_eq!(pages.len(), NUM_PAGES);
        for (i, &page_index) in pages.iter().enumerate() {
            assert_eq!(
                page_index, i,
                "Page {} should be at position {}",
                page_index, i
            );
        }
    }
}
