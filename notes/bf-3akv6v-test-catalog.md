# Negative Fraction Tests Catalog

## Overview
This document catalogs all negative_fraction test functions in the pdftract codebase, established as the baseline catalog for bead bf-3akv6v.

**Search Command:** `grep -r "fn test.*negative_fraction" --include="*.rs"`

**Total Test Count:** 5 tests

**File Location:** `crates/pdftract-core/src/font/type3_rasterizer.rs`

**Module Path:** `pdftract_core::font::type3_rasterizer::tests`

## Test Inventory

### 1. test_intersection_x_negative_fraction
- **Line:** 4427
- **Full Name:** `test_intersection_x_negative_fraction`
- **Module:** `pdftract_core::font::type3_rasterizer::tests::test_intersection_x_negative_fraction`
- **Purpose:** Tests Edge intersection_x() behavior with negative fractions
- **Test Case:** x = -2.3 → -2
- **Verification:**
  - Verifies negative fractions round toward zero (nearest integer)
  - Confirms intersection_x uses round_x() internally
  - Validates Edge struct stores x as i32 correctly
- **Related Bead:** bf-2an1s2

### 2. test_round_x_negative_fractions_round_down
- **Line:** 4877
- **Full Name:** `test_round_x_negative_fractions_round_down`
- **Module:** `pdftract_core::font::type3_rasterizer::tests::test_round_x_negative_fractions_round_down`
- **Purpose:** Tests round_x() with negative fractions that round AWAY from zero
- **Test Cases:**
  - -0.5 → -1 (exact boundary case)
  - -0.6, -0.7, -0.8, -0.9, -0.99 → -1
  - -1.6 → -2
  - -2.7 → -3
  - -3.8 → -4
  - -5.9 → -6
- **Verification:**
  - Confirms negative fractions round AWAY from zero (toward larger magnitude)
  - Validates the -0.5 boundary rounds correctly
  - Tests range of negative fraction values
- **Related Bead:** bf-hh2ek5

### 3. test_round_x_negative_fraction_rounds_down
- **Line:** 5064
- **Full Name:** `test_round_x_negative_fraction_rounds_down`
- **Module:** `pdftract_core::font::type3_rasterizer::tests::test_round_x_negative_fraction_rounds_down`
- **Purpose:** Tests basic negative fraction rounding
- **Test Case:** x = -2.3 → -2
- **Verification:**
  - Confirms round_x() correctly rounds negative fractions toward zero (truncation)
  - Validates basic case with fractional part

### 4. test_round_x_small_negative_fraction_rounds_down
- **Line:** 5072
- **Full Name:** `test_round_x_small_negative_fraction_rounds_down`
- **Module:** `pdftract_core::font::type3_rasterizer::tests::test_round_x_small_negative_fraction_rounds_down`
- **Purpose:** Tests small negative fraction boundary at -0.5
- **Test Case:** x = -0.5 → -1
- **Verification:**
  - Confirms -0.5 rounds away from zero (toward -1)
  - Validates Rust's round() "round half away from zero" behavior
  - Tests critical boundary case

### 5. test_round_x_very_small_negative_fraction_rounds_down
- **Line:** 5081
- **Full Name:** `test_round_x_very_small_negative_fraction_rounds_down`
- **Module:** `pdftract_core::font::type3_rasterizer::tests::test_round_x_very_small_negative_fraction_rounds_down`
- **Purpose:** Tests very small negative fraction rounding
- **Test Case:** x = -0.1 → 0
- **Verification:**
  - Confirms small negative fractions round toward zero
  - Validates truncation behavior for values with minimal fractional part

## Test Context

All five tests are part of the Type3 font rasterizer test suite and verify the behavior of the `round_x()` function and `Edge::intersection_x()` method when handling negative fractional values. These tests are critical for correct scanline polygon fill algorithm behavior in glyph rasterization.

**Key Functions Under Test:**
- `round_x(x: f64) -> i32` (line 674)
- `Edge::intersection_x(&self) -> i32` (line 701)

**Rounding Behavior Verified:**
- Negative fractions with magnitude < 0.5 round toward zero (e.g., -0.1 → 0)
- Negative fractions at exact 0.5 boundary round away from zero (e.g., -0.5 → -1)
- Negative fractions with magnitude > 0.5 round away from zero (e.g., -0.6 → -1, -2.3 → -2)

## File Structure

```
crates/pdftract-core/src/font/type3_rasterizer.rs
├── Main module (lines 1-1748)
│   ├── pub fn round_x(x: f64) -> i32 (line 674)
│   └── pub struct Edge (line 683)
│       └── pub(crate) fn intersection_x(&self) -> i32 (line 701)
└── Test module #[cfg(test)] (lines 1749-end)
    ├── test_intersection_x_negative_fraction (line 4427)
    ├── test_round_x_negative_fractions_round_down (line 4877)
    ├── test_round_x_negative_fraction_rounds_down (line 5064)
    ├── test_round_x_small_negative_fraction_rounds_down (line 5072)
    └── test_round_x_very_small_negative_fraction_rounds_down (line 5081)
```

## Verification Notes

- **Total test count confirmed:** 5 tests
- **All tests located in:** Single file `crates/pdftract-core/src/font/type3_rasterizer.rs`
- **Test module location:** Lines 1749+ (#[cfg(test)] module)
- **No other negative_fraction tests found:** Search complete across all .rs files

## Metadata

**Catalog Created:** 2026-08-10
**Bead ID:** bf-3akv6v
**Parent Bead:** bf-1djtvm
**Purpose:** Baseline test catalog for negative fraction test isolation
