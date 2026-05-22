# bf-49wmw: Fix PNG-predictor unbounded pre-allocation

## Summary
Fixed OOM root cause in PNG/TIFF predictor application by removing unbounded pre-allocation and implementing row-by-row processing with budget enforcement.

## Changes Made

### 1. Added MAX_ROW_BYTES constant (64 KB)
- Bounds row size calculation to prevent OOM from malicious PDF parameters
- Ensures peak memory stays at 2x stride (prev_row + current_row)

### 2. Added is_row_size_clamped() check
- Returns true when calculated row_size exceeds MAX_ROW_BYTES
- Functions return data as-is rather than risking incorrect decoding when parameters are suspicious

### 3. apply_png_predictors() changes
- Removed `Vec::with_capacity(num_rows * row_size)` pre-allocation
- Now uses `Vec::new()` and grows row-by-row
- Added `max_output` parameter for budget enforcement
- Checks budget before processing each row
- Returns partial data when budget exceeded

### 4. apply_tiff_predictor_2() changes
- Removed `Vec::with_capacity(data.len())` pre-allocation
- Now uses `Vec::new()` and grows row-by-row
- Added `max_output` parameter for budget enforcement
- Checks budget before processing each row
- Returns partial data when budget exceeded

### 5. FlateDecoder changes
- Tracks flate output separately from predictor output
- Counts final predictor output against doc_counter (not flate bytes)
- Passes remaining budget to predictor via `predictor_budget`

### 6. Lowered DEFAULT_MAX_DECOMPRESS_BYTES
- Changed from 2 GB to 512 MiB for more reasonable default

## Verification

### Tests PASS
- All 31 predictor tests pass
- test_flate_decode_bomb_limit_with_predictor - verifies budget enforcement
- test_flate_decode_performance_100mb - verifies performance characteristics
- test_predictor_multiple_rows_tiff - verifies TIFF predictor
- All PNG predictor tests (10/11/12/13/14/15) - verifies row-by-row processing

### Code Review
- No remaining `Vec::with_capacity(data.len())` or `Vec::with_capacity(num_rows * row_size)` patterns
- All other decoders (ASCII85, ASCIIHex, Passthrough) already use incremental growth
- Peak memory bounded to 2x stride (MAX_ROW_BYTES = 64 KB) regardless of image height

## Acceptance Criteria
- [x] Row-by-row processing (peak 2x stride)
- [x] Output counted against max_decompress_bytes
- [x] Never pre-size to claimed/decompressed length
- [x] Same discipline for apply_tiff_predictor_2
- [x] All Vec::with_capacity(data.len()) sites addressed
