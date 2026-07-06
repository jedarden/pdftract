# Understanding unmapped_glyph_names Configuration

**Task:** Examine how unmapped_glyph_names is currently configured and accessed in the codebase.

**Date:** 2026-07-06

## Summary

The `unmapped_glyph_names` configuration is a compile-time generated HashSet that stores glyph names that should be skipped during CMAP and ToUnicode entry creation. These glyphs have no valid Unicode mapping and should not appear in text extraction output.

## Data Structure

**Primary Storage:**
- **Type:** `LazyLock<HashSet<&'static str>>`
- **Location:** `crates/pdftract-core/src/font/unmapped.rs`
- **Global Name:** `UNMAPPED_GLYPH_NAMES`
- **String Lifetime:** Static references (no allocation at runtime)

## Current Contents

From `build/unmapped-glyph-names.json` (version 1.0):
```json
[
  ".notdef",
  ".null",
  "g000",
  "g001",
  "g002",
  "g003",
  "g004",
  "g005",
  "g006",
  "g007",
  "g008",
  "g009"
]
```

**Description from the source:**
- `.notdef`: The standard PDF fallback glyph representing "no glyph available"
- `.null`: Null glyph
- `g000-g009`: PUA (Private Use Area) glyphs commonly used in embedded fonts with custom encodings

## Build-Time Generation

**Build Script:** `crates/pdftract-core/build.rs`
**Function:** `generate_unmapped_glyph_names()` (lines 946-1050)

The build script:
1. Reads `build/unmapped-glyph-names.json`
2. Parses the JSON to extract the `unmapped_glyph_names` array
3. Generates Rust code at build time that creates a `LazyLock<HashSet<&'static str>>`
4. Writes the generated code to `$OUT_DIR/unmapped_glyph_names.rs`
5. Includes the generated file via `include!(concat!(env!("OUT_DIR"), "/unmapped_glyph_names.rs"));`

**Fallback behavior:** If the JSON file is missing, it uses a minimal default set containing only `.notdef`.

## Access Patterns

### 1. Direct Global Access (Recommended)

```rust
use pdftract_core::font::unmapped::UNMAPPED_GLYPH_NAMES;

// Check if a glyph name is in the unmapped set
fn is_glyph_unmapped(name: &str) -> bool {
    UNMAPPED_GLYPH_NAMES.contains(name)
}
```

### 2. Helper Function (Recommended)

```rust
use pdftract_core::font::unmapped::is_unmapped_glyph_name;

// Handles leading '/' automatically
let is_unmapped = is_unmapped_glyph_name("/.notdef"); // true
let is_unmapped = is_unmapped_glyph_name(".notdef");  // true
let is_unmapped = is_unmapped_glyph_name("A");        // false
```

The `is_unmapped_glyph_name()` function:
- Accepts glyph names with or without leading `/` (PDF glyph names are often prefixed with `/`)
- Strips the leading slash if present before checking the set
- Returns `bool` directly

### 3. Through DifferencesOverlay (For CMAP Generation)

```rust
use pdftract_core::font::encoding::DifferencesOverlay;

// Create overlay with default unmapped glyph names
let overlay = DifferencesOverlay::new();

// Create overlay with custom unmapped glyph names
let custom_set: HashSet<String> = [".notdef", ".null"].iter()
    .map(|s| s.to_string()).collect();
let overlay = DifferencesOverlay::with_unmapped_glyph_names(custom_set);

// Check if a glyph is unmapped
let is_unmapped = overlay.is_unmapped_glyph_name("/.notdef");
```

The `DifferencesOverlay` stores:
- **Type:** `HashSet<String>` (owned strings, not `&'static str`)
- **Reason:** Allows runtime customization of the set
- **Default:** Initialized from `UNMAPPED_GLYPH_NAMES` via `default_unmapped_glyph_names()`

## Module Structure

**Exports from `pdftract_core::font`:**
```rust
pub mod unmapped;  // pub mod unmapped
```

**Public API from `pdftract_core::font::unmapped`:**
```rust
pub static UNMAPPED_GLYPH_NAMES: LazyLock<HashSet<&'static str>>
pub fn is_unmapped_glyph_name(name: &str) -> bool
```

## Usage Example for CMAP Skip Logic

Here's how to check if a glyph should be skipped during CMAP entry creation:

```rust
use pdftract_core::font::unmapped::is_unmapped_glyph_name;

// When creating CMAP entries
fn should_skip_glyph(glyph_name: &str) -> bool {
    is_unmapped_glyph_name(glyph_name)
}

// Example usage
assert!(should_skip_glyph(".notdef"));
assert!(should_skip_glyph("/.null"));
assert!(!should_skip_glyph("A"));
assert!(!should_skip_glyph("/space"));
```

## Key Implementation Details

1. **LazyLock for Efficiency:** The HashSet is built on first access, not at program startup
2. **Static String Literals:** No heap allocation for the glyph names themselves
3. **Leading Slash Tolerance:** The helper function automatically handles `/` prefixes
4. **Build-Time Validation:** The JSON file is checksum-verified during build (TH-06 supply-chain gate)
5. **Runtime Customization:** `DifferencesOverlay` allows custom sets without recompilation

## Acceptance Criteria Status

✅ **unmapped_glyph_names data structure identified**
   - Type: `LazyLock<HashSet<&'static str>>`
   - Global name: `UNMAPPED_GLYPH_NAMES`

✅ **Storage location documented**
   - Module: `crates/pdftract-core/src/font/unmapped.rs`
   - Generated from: `build/unmapped-glyph-names.json`

✅ **Access pattern understood**
   - Direct: `UNMAPPED_GLYPH_NAMES.contains(name)`
   - Helper: `is_unmapped_glyph_name(name)`
   - Via overlay: `DifferencesOverlay::is_unmapped_glyph_name(name)`

✅ **Code example provided**
   - See "Usage Example for CMAP Skip Logic" above

## Files Referenced

- `/home/coding/pdftract/crates/pdftract-core/src/font/unmapped.rs` - Main module
- `/home/coding/pdftract/crates/pdftract-core/build.rs` - Build script
- `/home/coding/pdftract/build/unmapped-glyph-names.json` - Source data
- `/home/coding/pdftract/crates/pdftract-core/src/font/encoding.rs` - Usage in CMAP
- `/home/coding/pdftract/crates/pdftract-core/src/font/mod.rs` - Module export

## Test Coverage

The module includes tests in `unmapped.rs`:
- `test_notdef_is_unmapped` - Verifies `.notdef` is recognized
- `test_normal_glyphs_not_unmapped` - Verifies normal glyphs are not in the set
- `test_unmapped_set_contains_expected_entries` - Verifies the set is populated

All tests verify both with and without leading `/` prefix.
