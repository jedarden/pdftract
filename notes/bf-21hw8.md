# bf-21hw8: Bound predictor tests (PNG and TIFF)

## Summary
Added 4 new tests to verify PNG and TIFF predictor functions use row-by-row processing with bounded peak memory (2x stride = 2 × MAX_ROW_BYTES = 128 KB), never pre-allocating full output buffers inside tests.

## Changes Made

### 1. test_png_predictor_budget_enforcement_small_fixture
- **Fixture:** 200 bytes (20 rows × 10 bytes with PNG selector)
- **Budget:** 100 bytes (forces truncation)
- **Verifies:**
  - Output never exceeds max_output budget
  - Truncation occurs at row boundary
  - Peak memory stays at 2x stride (prev_row + current_row)

### 2. test_tiff_predictor_2_budget_enforcement_small_fixture
- **Fixture:** 160 bytes (20 rows × 8 bytes grayscale)
- **Budget:** 80 bytes (half of decoded size)
- **Verifies:**
  - Output never exceeds max_output budget
  - Truncation occurs at row boundary
  - Peak memory stays at 2x stride

### 3. test_png_predictor_multiple_selectors_budget_per_row
- **Fixture:** 25 bytes (5 rows with different PNG selectors: 10/11/12/13/14)
- **Budget:** 6 bytes (2 rows only)
- **Verifies:**
  - Budget is checked BEFORE processing each row (per bf-49wmw)
  - All PNG selector types respect budget
  - Early abort at row boundary

### 4. test_tiff_predictor_2_rgb_budget_enforcement
- **Fixture:** 45 bytes (5 rows × 9 bytes RGB data)
- **Budget:** 18 bytes (2 rows only)
- **Verifies:**
  - Multi-byte pixels (RGB) process row-by-row
  - Budget enforced per-row
  - Correct differencing for multi-component data

## Verification

### Tests PASS
All 4 new tests pass:
```
test_png_predictor_budget_enforcement_small_fixture ... ok
test_tiff_predictor_2_budget_enforcement_small_fixture ... ok
test_png_predictor_multiple_selectors_budget_per_row ... ok
test_tiff_predictor_2_rgb_budget_enforcement ... ok
```

### Code Review
- All fixtures are small (under 250 bytes) - no large buffer allocation
- No `Vec::with_capacity(data.len())` or similar patterns in tests
- Tests use the production `apply_predictor()` function which already implements row-by-row processing (from bf-49wmw)
- Budget assertions verify early truncation occurs

### Production Code (from bf-49wmw) Verified
- `apply_png_predictors()`: Uses `Vec::new()`, grows row-by-row, checks budget before each row
- `apply_tiff_predictor_2()`: Uses `Vec::new()`, grows row-by-row, checks budget before each row
- Peak memory bounded to 2 × MAX_ROW_BYTES (128 KB) regardless of image height

## Acceptance Criteria
- [x] PNG predictor tests use small fixtures (< 250 bytes)
- [x] TIFF predictor 2 tests use small fixtures (< 250 bytes)
- [x] Tests assert row-by-row peak memory (budget enforcement)
- [x] Tests never pre-allocate full second copy of output
- [x] Mirrors bf-49wmw row-by-row discipline

## References
- Production fix: bf-49wmw (row-by-row processing with MAX_ROW_BYTES = 64 KB)
- Test file: crates/pdftract-core/src/parser/stream.rs (predictor_tests module)
