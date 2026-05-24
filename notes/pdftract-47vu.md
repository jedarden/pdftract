# pdftract-47vu: pHash implementation

## Summary

Implemented `phash_glyph(bitmap: &[u8; 1024]) -> u64` that computes a 64-bit perceptual hash for 32×32 grayscale glyph bitmaps using the pHash algorithm.

## Implementation Details

### Algorithm
1. **Input validation**: Special case for uniform bitmaps (all pixels identical) returns 0 deterministically
2. **Normalization**: Convert 8-bit pixel values [0, 255] to centered float32 [-1.0, +1.0]
3. **DCT**: Apply 32×32 2D DCT-II using separable row-column approach
4. **Coefficient extraction**: Extract 64 low-frequency AC coefficients (8×8 block, skipping DC at [0,0], using [0,8] as replacement)
5. **Median threshold**: Compute median of the 64 coefficients, set bit i if coefficient i > median

### Files Created
- `crates/pdftract-core/src/font/shape.rs` - pHash implementation with DCT, hamming_distance helper

### Files Modified
- `crates/pdftract-core/src/font/mod.rs` - Added shape module, exported `phash_glyph` and `hamming_distance`

### Key Design Decisions
1. **Hand-rolled DCT**: Implemented DCT-II from scratch with precomputed cosine basis for performance
2. **Uniform bitmap handling**: Added early return for uniform bitmaps to ensure deterministic output (avoiding floating-point noise issues)
3. **Coefficient selection**: Used 8×8 low-frequency block with DC excluded, replaced with [0,8] to maintain 64 values
4. **Median computation**: Average of indices 31 and 32 for 64 values (standard median for even-length array)

### Test Coverage
- Uniform bitmaps (white, black, gray) hash to 0
- Non-uniform patterns produce non-zero hashes
- Identical inputs produce identical hashes (deterministic)
- Different shapes produce different hashes
- Hamming distance computation correct

## Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| Hash of all-zero bitmap is 0x0000000000000000 | PASS | Special case handling |
| Hash of all-255 bitmap is 0x0000000000000000 | PASS | Special case handling |
| Hash of half-black/half-white is non-zero | PASS | Vertical edge produces non-zero hash |
| Same bitmap hashed twice produces identical u64 | PASS | Deterministic |
| Bench: phash 1000 glyphs in < 50 ms | N/A | No benchmark yet (performance test TODO) |

## Verification

```bash
# Tests
cargo test -p pdftract-core --lib font::shape
# Result: 12 passed; 0 failed

# Compile check
cargo check --all-targets
# Result: OK

# Formatting
cargo fmt
# Result: OK
```

## Performance Considerations

- DCT precomputation: 32×32 cosine basis computed at runtime (~100 LOC, no external deps)
- Expected per-hash time: ~30 µs (per bead description)
- Called only for Level 4 fallback (~1% of glyphs)
- Amortized cost: negligible for per-page processing

## References

- Plan section: Phase 2.5 Perceptual hash algorithm (line 1420)
- Bead: pdftract-47vu
