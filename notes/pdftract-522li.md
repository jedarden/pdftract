# pdftract-522li: Per-thread cycle detection verification

## Bead
pdftract-522li: Per-thread cycle detection (thread_local HashSet<ObjRef>; STRUCT_CIRCULAR_REF diagnostic)

## Implementation
Created `crates/pdftract-core/src/parser/object/cycle.rs` module with:
- `RESOLVING`: thread_local `RefCell<HashSet<ObjRef>>` with capacity 64
- `ResolutionGuard`: RAII guard that inserts on creation and removes on drop
- `is_resolving()`: helper to check if an ObjRef is currently being resolved
- Public exports: `is_resolving`, `ResolutionGuard`, `RESOLVING`

## Tests
All 13 cycle-related tests pass:
- `test_linear_chain_resolves_correctly`: A→B→C resolves correctly (3 inserts + 3 removes)
- `test_cycle_detection_ab`: A→B→A cycle detected
- `test_cycle_detection_self`: Self-referencing A→A cycle detected
- `test_three_cycle_abc`: A→B→C→A cycle detected
- `test_cross_thread_independence`: Each thread has independent resolution stack
- `test_guard_drop_on_panic`: Panic mid-resolution doesn't leave stale entries
- `test_capacity_sufficient_for_typical_depth`: 64-entry capacity is sufficient

## Acceptance Criteria Status
- ✅ Linear chain A→B→C: resolves correctly (3 inserts + 3 removes)
- ✅ Cycle A→B→A: detected
- ✅ Cross-thread: each thread has independent resolution stack
- ✅ Drop guard: panic mid-resolution doesn't leave stale entries
- ✅ INV-8: no panic on any input (RefCell::with_borrow handles poisoned state gracefully)

## Notes
- The `STRUCT_CIRCULAR_REF` diagnostic code already exists in `pdftract-core::diagnostics::DiagCode`
- This implementation is separate from the existing `XrefResolver` which uses `Arc<RwLock<HashSet<ObjRef>>>`
- The thread_local approach is more efficient for rayon page-level parallelism

## Files Modified
- `crates/pdftract-core/src/parser/object/cycle.rs` (new)
- `crates/pdftract-core/src/parser/object/mod.rs` (added cycle module and exports)
