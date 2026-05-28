# Verification Note: pdftract-2k3ms - Phase 3.4 Marked Content Tracking (Coordinator)

## Bead Description

Coordinator for sub-phase 3.4: track BMC/BDC/EMC marked-content sequences and populate the `mcid: Option<u32>` field on each emitted Glyph with the innermost MCID currently in scope. Also handle Optional Content Group (OCG) /OC tags: glyphs inside an OCG whose default state is OFF are STILL emitted but flagged for downstream filtering.

## Status: COMPLETE ✅

All 3 child beads are closed, and all coordinator acceptance criteria are met.

## Children Status

| Child Bead | Title | Status | Verification Note |
|------------|-------|--------|-------------------|
| pdftract-1l6wn | BMC / BDC / EMC operator parsers + marked-content stack | ✅ CLOSED | notes/pdftract-1l6wn.md |
| pdftract-64atr | MCID propagation to Glyph.mcid via emit_glyph wrapper | ✅ CLOSED | notes/pdftract-64atr.md |
| pdftract-1q19p | OCG /OC tag tracking + default-OFF detection via /OCProperties | ✅ CLOSED | Implementation verified in code |

## Acceptance Criteria Verification

### 1. All 3 children closed ✅

All three child beads are closed:
- `pdftract-1l6wn` (BMC/BDC/EMC operators) - closed
- `pdftract-64atr` (MCID propagation) - closed
- `pdftract-1q19p` (OCG tracking) - closed

### 2. Nested BDC: innermost MCID wins for enclosed glyphs ✅

**Implementation:** `MarkedContentStack::innermost_mcid()` in `marked_content_stack.rs`:
```rust
pub fn innermost_mcid(&self) -> Option<u32> {
    self.stack.iter().rev().find_map(|frame| frame.mcid)
}
```

The method iterates from the innermost frame (`rev()`) and returns the first MCID found, ensuring the innermost MCID wins.

**Test Coverage:**
- `test_innermost_mcid_with_nested` - verifies innermost MCID wins
- `test_nested_frames` - verifies MCID visibility changes as frames are pushed/popped

### 3. EMC without matching BMC: ignored, no panic ✅

**Implementation:** `MarkedContentStack::pop_emc()` in `marked_content_stack.rs`:
```rust
pub fn pop_emc(&mut self) -> Option<MarkedContentFrame> {
    if self.stack.is_empty() {
        self.diagnostics.push(Diagnostic::with_static_no_offset(
            DiagCode::EmcWithoutBmc,
            "EMC operator without matching BMC/BDC",
        ));
        None
    } else {
        self.stack.pop()
    }
}
```

Returns `None` and emits a diagnostic; no panic occurs.

**Test Coverage:**
- `test_pop_emc_underflow` - verifies no panic and diagnostic emitted
- `test_parse_emc_underflow` - verifies BDC operator handler handles underflow

### 4. MCID 0: valid (zero is a legal MCID) ✅

**Implementation:** The `mcid` field is `Option<u32>`, which allows `Some(0)` as a valid value distinct from `None`.

**Test Coverage:**
- `test_glyph_with_mcid_zero` (in pdftract-64atr tests) - verifies MCID 0 is treated as valid
- The implementation correctly distinguishes `Some(0)` from `None`

### 5. OCG default OFF: glyphs inside emitted with `is_hidden` flag ✅

**Implementation:**
- `MarkedContentFrame` has `is_hidden: bool` field (line 29)
- `MarkedContentStack::is_hidden()` returns true if ANY frame is hidden (line 160-162)
- `Glyph` struct has `is_hidden: bool` field (glyph/mod.rs line 73)
- BDC parser checks for /OC tag and /OCG property, resolves against OFF set (marked_content_operators.rs lines 69-85)
- `emit_glyph` accepts `is_hidden` parameter and sets it on the glyph

**Test Coverage:**
- `test_parse_bdc_ocg_not_in_off_set` - OCG not in OFF → not hidden
- `test_parse_bdc_ocg_in_off_set` - OCG in OFF → hidden
- `test_parse_bdc_ocg_with_leading_slash` - /OC with leading slash works
- `test_parse_bdc_non_oc_tag_ignores_ocg` - non-OC tags ignore OCG property
- `test_stack_is_hidden_with_hidden_frame` - hidden flag propagates
- `test_stack_is_hidden_nested_outer_hidden` - outer hidden propagates to inner

## Test Results

```
# Marked-content stack tests (18 tests)
cargo test -p pdftract-core --lib parser::marked_content_stack
Result: 18 passed

# Marked-content operator tests (30 tests)
cargo test -p pdftract-core --lib parser::marked_content_operators
Result: 30 passed

# OCG tests (20 tests)
cargo test -p pdftract-core --lib parser::ocg
Result: 20 passed

# Total: 68 tests passed, 0 failed
```

## Integration Points

### Phase 3.4 → Phase 3.2 (Glyph Emission)
- `emit_glyph()` accepts `mcid: Option<u32>` and `is_hidden: bool` parameters
- MCID and hidden flags are set on every emitted glyph
- Downstream Phase 4.6 will filter hidden glyphs based on user preferences

### Phase 3.4 → Phase 7.1 (StructTree Exploitation)
- MCID links glyphs to structure elements via the StructTree
- Innermost MCID ensures correct structure-based reading order

### OCG Integration
- `/OCProperties` parsed at document level (parser/ocg.rs)
- OFF set passed to content stream executor
- BDC /OC tags check OCG visibility and set `is_hidden` flag

## Key Implementation Details

### INV: Marked Content Stack Independence
The marked-content stack is independent of the graphics state stack (q/Q operators). This is correctly implemented in `content_stream.rs` where the two stacks are managed separately.

### INV: Hidden Flag OR Semantics
Per bead pdftract-1q19p, the `is_hidden` flag is OR'd through nested frames: if any frame in the stack has `is_hidden=true`, all glyphs within are marked hidden. This is implemented in `MarkedContentStack::is_hidden()`.

### INV: Innermost MCID Semantics
The `innermost_mcid()` method scans from the innermost frame outward, returning the first MCID found. BMC frames (no MCID) are transparent—the search continues outward.

## Conclusion

All coordinator acceptance criteria are met. The marked-content tracking implementation is complete with comprehensive test coverage. The three child beads collectively implement:
- BMC/BDC/EMC operator parsing with depth limiting
- MCID propagation to emitted glyphs (innermost wins)
- OCG /OC tag tracking with default-OFF detection
- Hidden flag propagation through nested marked-content scopes

**Status: READY TO CLOSE**

## Git Commit

No new code changes were required for this coordinator bead. All implementation work was completed by the child beads. This verification note documents the integration and validates the coordinator-level acceptance criteria.
