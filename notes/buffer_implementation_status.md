# OutOfOrderBuffer Implementation Status

## Summary

The `OutOfOrderBuffer` implementation in `crates/pdftract-core/src/output/ndjson/buffer.rs` is partially complete. The basic functionality works correctly, but some complex synchronization tests are not passing.

## Passing Tests

The following basic tests pass:
- `test_in_order_push_pop` - Push and pop pages in order
- `test_out_of_order_push_pop` - Push and pop pages out of order
- `test_duplicate_detection` - Detect and reject duplicate page indices
- `test_gap_in_sequence` - Handle gaps in the page sequence
- `test_completion_detection` - Detect when all pages have been emitted
- `test_buffer_size_tracking` - Track buffer size correctly

## Failing Tests

The following tests with complex synchronization are not passing:
- `test_backpressure_blocks_when_full` - Tests backpressure when buffer is full
- `test_bead_sequence` - Tests the specific bead sequence from the requirements
- `test_concurrency_stress` - Tests concurrent access from multiple threads

## Issues

The main issue is with the backpressure logic. The test expects that when the buffer has 8 pages (the window size), the 9th push should block. However, this leads to a deadlock scenario:
1. If the buffer has pages 1-8 (missing page 0)
2. The 9th push (page 9) blocks because the buffer is full
3. Pushing page 0 also blocks because the buffer is full
4. Deadlock - neither thread can proceed

## Implementation Details

The current implementation uses:
- `BinaryHeap` for ordering pages by page_index
- `HashSet` for O(1) duplicate detection
- `Mutex` for protecting the internal state
- `Condvar` for signaling between producer and consumer threads

The backpressure condition is:
```rust
while inner.heap.len() > NDJSON_OUT_OF_ORDER_WINDOW_PAGES {
    inner = self.not_full.wait(inner).unwrap();
}
```

This allows the buffer to grow to WINDOW_SIZE + 1 pages before blocking, which prevents the deadlock but doesn't match the test's expectations.

## Next Steps

To fix the failing tests, we need to:
1. Redesign the backpressure logic to handle edge cases correctly
2. Ensure that critical pages (like page 0) can always be added even when the buffer is full
3. Add proper synchronization to prevent deadlocks

This requires more time to design and implement correctly.
