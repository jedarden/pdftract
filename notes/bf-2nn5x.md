# Verification: unmapped_glyph_names Config Wired to CMAP Entry Creation

## Task
Wire unmapped_glyph_names config to CMAP entry creation point.

## Status: PASS

The unmapped_glyph_names configuration was already wired to the CMAP entry creation point in a previous commit (7660cfc8: "feat(bf-4zgpr): add unmapped_glyph_names config field to CMAP generator"). This bead verifies the wiring is complete and functional.

## Implementation

### CMAP Entry Creation Point
**Location:** `crates/pdftract-core/src/font/encoding.rs:220-230`

The CMAP entry creation for Type1 font encoding differences occurs in `DifferencesOverlay::parse()`. This is where individual (code, glyph_name) pairs are added to the overlay.

### Configuration Storage
**Location:** `crates/pdftract-core/src/font/encoding.rs:129-146`

The `DifferencesOverlay` struct stores the `unmapped_glyph_names` configuration:

```rust
pub struct DifferencesOverlay {
    entries: Vec<(u8, Arc<str>)>,
    unmapped_glyph_names: std::collections::HashSet<String>,
}
```

### Configuration Initialization
**Location:** `crates/pdftract-core/src/font/encoding.rs:140-168`

The configuration is initialized with defaults from the global `UNMAPPED_GLYPH_NAMES` set:

```rust
pub fn new() -> Self {
    Self {
        entries: Vec::new(),
        unmapped_glyph_names: Self::default_unmapped_glyph_names(),
    }
}

fn default_unmapped_glyph_names() -> std::collections::HashSet<String> {
    crate::font::unmapped::UNMAPPED_GLYPH_NAMES
        .iter()
        .map(|s| s.to_string())
        .collect()
}
```

### Skip Check at Entry Creation
**Location:** `crates/pdftract-core/src/font/encoding.rs:223-230`

The skip check prevents unmapped glyph names from being added as CMAP entries:

```rust
if cursor <= 255 {
    // Skip unmapped glyph names (e.g., .notdef) to prevent them from
    // appearing in text extraction output. These glyphs have no valid
    // Unicode mapping and should emit GLYPH_UNMAPPED diagnostics instead.
    if !overlay.is_unmapped_glyph_name(&name) {
        overlay.entries.push((cursor as u8, Arc::clone(name)));
    }
}
```

### Accessor Methods
**Locations:** `crates/pdftract-core/src/font/encoding.rs:148-158, 265-296`

The configuration provides methods for customization and access:

```rust
// Custom constructor
pub fn with_unmapped_glyph_names(unmapped_glyph_names: std::collections::HashSet<String>) -> Self

// Membership check
fn is_unmapped_glyph_name(&self, name: &str) -> bool

// Getter
pub fn unmapped_glyph_names(&self) -> &std::collections::HashSet<String>

// Setter
pub fn set_unmapped_glyph_names(&mut self, unmapped_glyph_names: std::collections::HashSet<String>)
```

## Call Chain

Configuration flows from global constants to entry creation:

1. **Global Definition:** `UNMAPPED_GLYPH_NAMES` in `unmapped.rs` (auto-generated from AGL)
2. **Default Initialization:** `DifferencesOverlay::new()` loads global set
3. **Parse Entry:** `DifferencesOverlay::parse()` creates overlay with defaults
4. **Skip Check:** `is_unmapped_glyph_name(&name)` checks membership before adding
5. **Entry Creation:** `overlay.entries.push()` executes only for mapped glyphs

## Custom Configuration Path

For custom configurations, users can:
1. Create a `HashSet<String>` with desired glyph names
2. Use `DifferencesOverlay::with_unmapped_glyph_names(custom_set)`
3. Or use `set_unmapped_glyph_names()` on an existing overlay

## Acceptance Criteria Status

- ✅ CMAP entry creation point accepts unmapped_glyph_names (via `self` field)
- ✅ Configuration is correctly passed from defaults through to entry creation
- ✅ Code compiles without errors (`cargo check --package pdftract-core` passed)

## Why This Design

The `unmapped_glyph_names` configuration is stored as instance state in `DifferencesOverlay` rather than passed as a parameter to `add_mapping()` because:

1. **Type1 Encoding Context:** The CMAP entry creation for Type1 fonts works with glyph names (e.g., ".notdef"), not byte codes
2. **Semantic Fit:** The configuration belongs to the overlay that creates entries, not each individual entry
3. **Flexibility:** Instance state allows custom configuration via constructor/setter without changing method signatures

## Note on ToUnicode CMap

The ToUnicode CMap parser (`cmap.rs`) does not use this configuration because:
- ToUnicode CMaps map byte sequences → Unicode characters directly
- There are no glyph names involved in ToUnicode mappings
- The skip logic is specific to Type1 font encoding differences

## Related Files

- `crates/pdftract-core/src/font/encoding.rs` - CMAP entry creation and config
- `crates/pdftract-core/src/font/unmapped.rs` - Global UNMAPPED_GLYPH_NAMES set
- `crates/pdftract-core/src/font/cmap.rs` - ToUnicode CMap parser (different code path)

## Related Beads

- bf-4zgpr: Added unmapped_glyph_names config field to DifferencesOverlay
- bf-6cvmz: Documented unmapped_glyph_names configuration structure
- bf-o4nrq: Documented CMAP entry creation point
