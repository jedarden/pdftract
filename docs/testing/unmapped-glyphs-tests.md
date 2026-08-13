# Test Edge Cases and Limitations

This document describes edge cases, limitations, and special behaviors discovered during testing and assertion improvement work in the pdftract codebase.

## Table of Contents

1. [Test Timeout and Hung Test Prevention](#test-timeout-and-hung-test-prevention)
2. [Orphaned Process Management](#orphaned-process-management)
3. [Unmapped Glyph Testing Edge Cases](#unmapped-glyph-testing-edge-cases)
4. [Configuration File Handling](#configuration-file-handling)
5. [Platform-Specific Behavior](#platform-specific-behavior)
6. [Common Failure Modes and Debugging](#common-failure-modes-and-debugging)
7. [Test Configuration References](#test-configuration-references)

## Test Timeout and Hung Test Prevention

### Critical Test Hygiene Rule

**Never let a hung test stall the test loop.** On 2026-05-24, one test froze the entire marathon for ~5.5 hours when a `pdftract mcp` server subprocess was spawned with `Stdio::piped()`, never drained its stdout/stderr, and relied on bare `child.kill()` / `child.wait()` for cleanup. The `wait()` blocked indefinitely (0% CPU), which hung `cargo test`, which kept the marathon's stdout pipe open.

### Timeout Configuration

The project uses **nextest** with explicit timeout configuration in `.config/nextest.toml`:

```toml
[profile.default]
# Marathon-safety default. A test still running after 30s × 2 = 60s is KILLED
slow-timeout = { period = "30s", terminate-after = 2 }
fail-fast = false

[profile.ci]
# CI profile: JUnit output, 1 retry for flaky tests. Killed after 60s × 3 = 180s.
fail-fast = true
retries = 1
slow-timeout = { period = "60s", terminate-after = 3 }

[profile.ci-proptest]
# Property test profile: higher timeout for proptest shrinks, no retries (deterministic)
# Killed after 120s × 3 = 360s
fail-fast = true
retries = 0
slow-timeout = { period = "120s", terminate-after = 3 }
```

**Important:** `slow-timeout` alone only *warns* that a test is slow. You must use `terminate-after = N` to actually KILL a test that exceeds the timeout.

### Running Tests Safely

**Always run tests through `cargo nextest run`, NEVER bare `cargo test`.** nextest isolates each test in its own process and enforces the per-test `slow-timeout`.

If nextest is genuinely unavailable, wrap the fallback in a hard wall-clock timeout:

```bash
timeout --kill-after=30s 600s cargo test --all-targets 2>&1 | tail -80
```

**Exit code 124** — or a nextest `TIMEOUT`/`TERMINATED` line — means a test hung. Find and fix it.

### Process Spawn Edge Cases

**A test that spawns a process or binds a socket MUST clean up deterministically:**

1. **Kill the child from an RAII guard** whose `Drop` runs `kill()` + a *bounded* wait, so cleanup fires even on panic or early return
2. **Bound every wait** with the existing `wait_with_timeout` helper. A bare `child.wait()` on a server that outlives the signal blocks forever
3. **Give the child `Stdio::null()`** (or drain its pipes on a thread). A long-running server left with undrained `Stdio::piped()` blocks on a full pipe and wedges both ends
4. **Bind servers to port `:0`** and read back the chosen port, so reruns never collide on a fixed port still held by a leaked process

**Never spawn overlapping retries of a hanging command.** If `cargo nextest`/`cargo test` does not return, the runner is wedged — kill it and its whole tree before doing anything else.

## Orphaned Process Management

### Problem Statement

Tests that spawn subprocesses (especially MCP servers, test harness processes) can leave orphaned processes if:
- Tests panic before cleanup runs
- A process doesn't exit when stdin closes
- `wait()` blocks indefinitely on a hung child
- Test timeouts kill the test runner but not the spawned processes

Orphaned processes from previous runs can:
- Block new test runs (port already in use)
- Consume system resources
- Cause flaky test behavior
- Violate test isolation assumptions

### Default Process Patterns

The verification system checks for these process patterns by default:
1. `pdftract mcp` - MCP server subprocess
2. `TH-0` - Test harness process (hyphen variant)
3. `TH_0` - Test harness process (underscore variant)

### Verification Methods

**Manual shell script:**
```bash
# Basic check (exits 0 if clean, 1 if orphans found)
./scripts/check-orphaned-processes.sh

# Verbose output
./scripts/check-orphaned-processes.sh --verbose

# JSON output for parsing
./scripts/check-orphaned-processes.sh --json

# Kill any orphans found
./scripts/check-orphaned-processes.sh --kill
```

**In-test Rust helpers:**
```rust
use pdftract_core::test_helpers::process_guard::{
    verify_no_orphaned_processes,
    OrphanedProcessGuard,
};

#[test]
fn test_mcp_server_cleanup() {
    // Record initial state, verify cleanup on drop
    let _guard = OrphanedProcessGuard::new();
    
    let mut server = spawn_mcp_stdio();
    // ... test code ...
    drop(server);
    
    // Verify no orphans remain
    verify_no_orphaned_processes().unwrap();
}
```

### Common Orphan Scenarios

1. **Test Timeout Leaves Children Alive**: Test exceeds time limit, test runner killed but spawned processes survive

2. **Panic Before Cleanup**: Test code panics after spawning a process but before cleanup code runs

3. **Undrained Stdio::piped() Blocks wait()**: Long-running server with `Stdio::piped()` fills stdout/stderr buffer, process blocks, `wait()` never returns

4. **Port Already in Use from Previous Run**: New test fails with "Address already in use" error

5. **Fuzz Harness Leaves Target Processes**: Fuzzer crashes or is killed, target pdftract processes continue running

## Unmapped Glyph Testing Edge Cases

### Configuration

Unmapped glyph names are configured in `build/unmapped-glyph-names.json`:

```json
{
  "unmapped_glyph_names": [
    ".notdef",
    ".null",
    "g000",
    "g001",
    // ... more g-series glyphs
    "CustomA",
    "CustomB",
    "NotAGlyph",
    "glyph_0041",
    "custom_glyph_a",
    "custom_glyph_b",
    "custom_glyph_c"
  ],
  "description": "Glyph names that should be skipped during CMAP and ToUnicode entry creation...",
  "version": "1.1"
}
```

### Critical Edge Cases

1. **Leading Slash Handling**: The `is_unmapped_glyph_name()` function handles glyph names both with and without leading `/` (e.g., both `.notdef` and `/.notdef` are recognized)

2. **Empty Configuration Defaults**: When `unmapped_glyph_names` is not specified in the config file, it defaults to an empty `Vec` (not `None`) due to `#[serde(default)]` attribute

3. **Consecutive Name Assignments**: In /Differences arrays, names can be assigned consecutively after a code: `[code /name1 /name2 /name3]`. Tests verify unmapped glyphs in consecutive sequences are properly filtered while normal glyphs are preserved

4. **CMAP Entry Creation**: The `DifferencesOverlay::parse()` method filters unmapped glyphs at CMAP entry creation time (marked with `// MARKER: CMAP entry creation point` in the code)

### Test Categories

1. **Structural Tests**: Verify basic parsing and function existence
2. **Presence Tests**: Assert that normal glyphs ARE present in CMAP output
3. **Absence Tests**: Assert that unmapped glyphs are ABSENT from CMAP output
4. **Consecutive Sequence Tests**: Verify filtering works with consecutive name assignments
5. **Range Mapping Tests**: Test `beginbfrange...endbfrange` constructs
6. **Configuration Tests**: Verify JSON config parsing with various edge cases

### Known Limitations

1. **No Runtime Reconfiguration**: Once compiled, the unmapped glyph set is fixed at build time via `include!(concat!(env!("OUT_DIR"), "/unmapped_glyph_names.rs"))`

2. **Custom Sets Only in DifferencesOverlay**: The `DifferencesOverlay` struct supports custom unmapped glyph sets via `with_unmapped_glyph_names()`, but this is not exposed at the API level for general use

3. **No Unicode Validation**: The system checks if a glyph name is in the unmapped set but does not validate if a name has a valid Unicode mapping beyond that check

### Type 3 Font-Specific Limitations

#### Content Stream Resolution
- **Missing CharProcs**: Type 3 fonts without `/CharProcs` dictionary are treated as zero-glyph fonts
- **Indirect References**: CharProcs as indirect references are not supported (treated as zero-glyph font)
- **Direct Streams**: CharProcs entries that are direct streams (not references) are skipped with diagnostic
- **Missing Glyphs**: Glyphs named in encoding but not in `/CharProcs` emit diagnostic and fail to rasterize

#### Widths Array Validation
- **Length Mismatch**: When `/Widths` array length doesn't match `LastChar - FirstChar + 1`, the array is:
  - Truncated if too long
  - Padded with zeros if too short
  - A diagnostic `FontType3WidthsLengthMismatch` is emitted
- **Missing Widths**: When `/Widths` is missing, defaults to all-zero array
- **Indirect Widths**: Indirect references for `/Widths` are not supported (defaults to all-zero)

#### Encoding Limitations
- **Single-Byte Only**: Type 3 fonts only support single-byte character codes (0-255)
- **Multi-Byte Codes**: Multi-byte codes immediately fall through to failure at Level 2
- **Arbitrary Glyph Names**: Custom glyph names not in AGL escalate to Level 4 (shape recognition)

#### Rasterization Limitations
- **Document Context Required**: Rasterization requires document context (source, resolver, decompress counter)
- **No Resolver**: Without a stream resolver, glyph content cannot be fetched
- **Empty Glyphs**: Empty content streams produce all-white bitmaps
- **Font Bbox**: Default font bbox is [0, 0, 0, 0] if not specified; affects rasterization size

### Font Resolver Cache Limitations

#### Thread-Safety Concurrency
- **DashMap Usage**: Resolver cache uses DashMap for thread-safe concurrent access
- **Cache Key**: Combines font ID (Arc pointer cast) and character code bytes
- **Emission Tracking**: Separate DashMap tracks which (font, code) pairs have already emitted diagnostics
- **One-Time Emission**: GLYPH_UNMAPPED diagnostic emitted exactly once per (font, code) pair

#### Cache Behavior
- **Cache Hit Returns Cached Result**: Same (font, code) pair returns cached resolution
- **Cache Miss Computes Resolution**: Cache miss computes resolution through 4-level fallback chain
- **Standard 14 Fonts**: Fonts without embedded programs skip Level 3 (fingerprinting)
- **Shape DB Feature**: Level 4 shape recognition only works when `shape-db` feature is enabled

### Encoding Resolution Chain Edge Cases

#### Level 1: ToUnicode CMap
- **Empty Mapping Falls Through**: Empty CMap results or U+FFFD-only results fall through to Level 2
- **Multi-Codepoint Support**: Ligature expansion returns multiple characters (e.g., "fi" → ['f', 'i'])
- **CMap Required**: No CMap means immediate fall-through to Level 2

#### Level 2: AGL + Encoding
- **Single-Byte Only**: Level 2 only supports single-byte character codes
- **Glyph Name Required**: Must successfully map code → glyph name → AGL
- **Multi-Codepoint AGL**: Tries multi-codepoint AGL lookup first (for ligatures), falls back to single-codepoint
- **Not In AGL**: Glyph name not in Adobe Glyph List falls through to Level 3

#### Level 3: Font Fingerprint
- **Glyph ID Required**: Level 3 requires glyph ID (not character code) for lookup
- **Embedded Program Required**: Fonts without embedded programs skip Level 3 entirely
- **Cached Fingerprint**: Requires pre-populated fingerprint database entry
- **Fallback**: No glyph ID or no fingerprint database entry falls through to Level 4

#### Level 4: Shape Recognition
- **Feature-Gated**: Only available when `shape-db` feature is enabled
- **Rasterization Required**: Must rasterize glyph to 32×32 bitmap
- **pHash Computation**: Computes perceptual hash of bitmap for database lookup
- **Hamming Distance Threshold**: Match accepted only if distance ≤ 8; otherwise falls through
- **Confidence 0.7**: Successful match returns with confidence 0.7

#### Failure Mode
- **All Levels Failed**: When all four levels fail, returns U+FFFD with:
  - `chars: ['\u{FFFD}']`
  - `source: UnicodeSource::Unknown`
  - `confidence: 0.0`
- **Diagnostic Emitted**: `FontGlyphUnmapped` diagnostic emitted (once per (font, code) pair)

## Configuration File Handling

### Build-Time Code Generation

The unmapped glyph names are compiled into the binary at build time:

1. **build.rs** reads `build/unmapped-glyph-names.json`
2. Generates `$OUT_DIR/unmapped_glyph_names.rs` with a `LazyLock<HashSet<&'static str>>`
3. **src/font/unmapped.rs** includes this file via `include!(concat!(env!("OUT_DIR"), "/unmapped_glyph_names.rs"))`

### Configuration Edge Cases

1. **Missing Configuration File**: If `build/unmapped-glyph-names.json` is missing, the build will fail when build.rs attempts to read it

2. **Invalid JSON**: Malformed JSON in the configuration file will cause a build-time panic

3. **Version Mismatches**: The configuration includes a `version` field for tracking format changes, but there is no runtime version checking

4. **Empty vs. Missing Fields**: An explicit empty array `[]` is treated the same as a missing `unmapped_glyph_names` field (both produce an empty `Vec`)

### Checksum Verification

Build-time data files are verified via checksums in `build/CHECKSUMS.sha256`. If verification fails:
- A warning is emitted
- The build panics with "Checksum verification failed - aborting build"
- This prevents tampering with or accidental modification of build-time data

## Platform-Specific Behavior

### Known Platform Differences

1. **Process Management**: Process spawning and cleanup behavior may differ between Unix-like systems and Windows (though Windows support is not currently a focus)

2. **Path Handling**: File path handling in test fixtures assumes Unix-style path separators

3. **Signal Handling**: Orphaned process cleanup relies on Unix-style signals (SIGTERM, SIGKILL)

4. **Shell Script Dependencies**: The orphaned process verification scripts require `pgrep` to be installed (part of `procps` package on Debian/Ubuntu)

### NixOS Considerations

The lab box runs NixOS, which may have different package availability and paths compared to traditional Linux distributions.

## Common Failure Modes and Debugging

### Test Timeout/Hang

**Symptoms:**
- Test runner never completes
- 0% CPU usage but process still running
- `cargo nextest run` shows "TERMINATED" status

**Debugging Steps:**
1. Check for undrained pipes in process spawn code
2. Verify all `wait()` calls are bounded with timeouts
3. Look for servers spawned with `Stdio::piped()` but no reader
4. Check for missing RAII guards on child processes

**Example Fix:**
```rust
// BAD - may block forever
let child = Command::new("pdftract")
    .arg("mcp")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;
child.wait(); // This blocks forever if pipe fills

// GOOD - bounded timeout with RAII guard
let _guard = ProcessGuard::new(child);
wait_with_timeout(&mut child, 1000)?;
```

### Orphaned Processes After Test Run

**Symptoms:**
- Subsequent test runs fail with "Address already in use"
- `pgrep -af "pdftract mcp"` shows processes from previous runs
- Test suite completes but orphans remain

**Debugging Steps:**
1. Run `./scripts/check-orphaned-processes.sh --verbose` to identify orphans
2. Check process age to determine if they're from recent or old runs
3. Verify PPID is 1 (true orphan) vs. another value (legitimate parent)
4. Run tests individually to identify which test leaves orphans

**Example Fix:**
```rust
// Add RAII guard to ensure cleanup on panic/early return
let _guard = OrphanedProcessGuard::new();

let server = Command::new("pdftract")
    .arg("mcp")
    .arg("--stdio")
    .spawn()?;

// Test code here

// Cleanup happens automatically when guard drops
```

### Configuration Parse Errors

**Symptoms:**
- Build fails with "Checksum verification failed"
- Runtime panics related to unmapped glyph names
- Tests fail with "unmapped_glyph_names is empty"

**Debugging Steps:**
1. Verify `build/unmapped-glyph-names.json` exists and is valid JSON
2. Check that `build/CHECKSUMS.sha256` includes the config file
3. Regenerate checksums: `cd crates/pdftract-core/build && sha256sum unmapped-glyph-names.json >> CHECKSUMS.sha256`
4. Verify the file hasn't been modified: `sha256sum -c CHECKSUMS.sha256`

### Assertion Message Improvements

All assertions now include diagnostic context with:
- **Expected**: What the test expects
- **Found**: What was actually found
- **Why this matters**: Explanation of why this assertion matters

**Example:**
```rust
assert!(
    is_unmapped_glyph_name(".notdef"),
    ".notdef should be identified as unmapped. \
     Expected: true. \
     Found: {}. \
     Why this matters: .notdef is the standard PDF fallback glyph configured in \
     build/unmapped-glyph-names.json and must never appear in text extraction.",
    is_unmapped_glyph_name(".notdef")
);
```

This makes test failures much easier to debug by providing context about what went wrong and why it matters.

## Test Helper Functions and Edge Cases

### Overview

The `crates/pdftract-core/src/font/test_glyph_helper.rs` module provides utilities for generating Type3 font glyph data in tests. Understanding these helpers is critical for writing unmapped glyph tests.

### Glyph Generation Functions

#### Rectangle Glyphs

**`make_rect_glyph(x, y, width, height)`**
- Generates: `"{x} {y} {width} {height} re f"` (rectangle + fill operators)
- Simplest valid glyph that produces visible output
- Tests the rasterizer's handling of the `re` operator shorthand
- Example: `make_rect_glyph(0.0, 0.0, 100.0, 100.0)` → `"0 0 100 100 re f"`

**`make_rect_glyph_with_path_commands(x, y, width, height)`**
- Generates: `"{x} {y} m {x+w} {y} l {x+w} {y+h} l {x} {y+h} l h f"` (explicit path)
- Produces identical output to `make_rect_glyph()` but uses individual path operators
- Tests that the rasterizer handles both `re` shorthand and explicit `m l l l h f` sequences
- Important for code coverage: ensures both code paths are tested

#### Line Glyphs

**`make_line_glyph(x1, y1, x2, y2)`**
- Generates: `"{x1} {y1} m {x2} {y2} l h S"` (moveto, lineto, closepath, stroke)
- Tests stroked path rendering vs filled paths (`f` vs `S`)
- Example: `make_line_glyph(0.0, 0.0, 50.0, 50.0)` → `"0 0 m 50 50 l h S"`

#### Empty Glyphs

**`make_empty_glyph()`**
- Returns: Empty `Vec<u8>` (no drawing operations)
- Tests handling of glyphs with no visible content (spaces, zero-width joiners)
- Rasterizes to all-white bitmap (no pixels set)
- Important for testing that the system doesn't crash on missing glyph data

### CharProc Mapping Functions

#### `make_test_char_procs()`
Returns `HashMap<Arc<str>, ObjRef>` with standard mapping:
- `/A` → ObjRef(1, 0)
- `/B` → ObjRef(2, 0)
- `/C` → ObjRef(3, 0)
- `/D` → ObjRef(4, 0)

#### `make_custom_char_procs(mappings)`
Accepts slice of `(name, ObjRef)` tuples for custom mappings.

#### `make_custom_char_procs_from_names(glyph_names, base_id)`
Auto-generates sequential IDs starting from `base_id`.

### Resolver Function

#### `make_test_resolver(glyph_map)`
Creates a closure that maps `ObjRef` IDs to glyph content bytes:
- **ID Mapping**: ID 1 → "/A", ID 2 → "/B", etc. (ASCII-based: `(ID + 'A' - 1) as char`)
- **Returns**: `Option<Vec<u8>>` (None for non-existent IDs)
- **Usage**: Provides test fonts with glyph content without needing PDF file parsing

**Important Edge Case**: The resolver assumes ASCII character names (A-Z). For custom glyph names like "g000" or "CustomA", you must create a custom resolver function.

### Test Helper Limitations and Edge Cases

#### 1. ASCII-Only Character Name Assumption
The default `make_test_resolver()` assumes character names follow ASCII A-Z pattern:
```rust
// This will FAIL for custom names
let resolver = make_test_resolver(&glyph_map);
resolver(ObjRef::new(1, 0)); // Returns "/A"
resolver(ObjRef::new(27, 0)); // Returns NULL character, not a valid glyph name
```

**Solution**: Create custom resolver functions for non-standard glyph names.

#### 2. Sequential ID Assumption
Test helpers assume ObjRef IDs map sequentially to character names starting from ID 1. If your test uses non-sequential IDs (e.g., ObjRef(100, 0)), the default resolver won't work.

#### 3. No Compression Support
Helper functions generate uncompressed content streams. They don't support `/Filter` entries for compressed streams. For testing compressed glyphs, you must manually add compression or use actual fixture files.

#### 4. No Resource Dictionaries
The helpers don't create resource dictionaries (fonts, color spaces, etc.). Tests requiring resources (e.g., colored glyphs, pattern fills) need custom fixtures.

#### 5. No FontMatrix Transformations
Helpers use default glyph space with identity FontMatrix [1 0 0 1 0 0]. For testing custom font matrices, you need to create Type3Font instances with FontMatrix set explicitly.

#### 6. Empty Streams vs Missing Glyphs
**Important distinction**:
- **Empty glyph** (`make_empty_glyph()`): Valid glyph with no drawing operations → all-white bitmap
- **Missing glyph** (not in CharProcs): `FontType3MissingGlyph` diagnostic → failure to rasterize

This distinction is critical for unmapped glyph tests: an empty glyph should succeed but produce no visible pixels, while a missing glyph should fail with a diagnostic.

### Example: Writing a Custom Glyph Test

```rust
use pdftract_core::font::test_glyph_helper::*;
use pdftract_core::font::type3::Type3Font;
use std::collections::HashMap;

// Create custom glyph names (unmapped in AGL)
let glyph_names = &["g000", "g001", "CustomA"];
let char_procs = make_custom_char_procs_from_names(glyph_names, 1);

// Create glyph data with STRING keys (not integer IDs!)
let mut glyph_map: HashMap<String, Vec<u8>> = HashMap::new();
glyph_map.insert("/g000".to_string(), make_rect_glyph(0.0, 0.0, 50.0, 50.0));
glyph_map.insert("/g001".to_string(), make_line_glyph(0.0, 0.0, 100.0, 100.0));
glyph_map.insert("/CustomA".to_string(), make_empty_glyph());

// Create custom resolver (manual mapping for non-ASCII names)
let resolver = |ref_id: ObjRef| -> Option<Vec<u8>> {
    match ref_id.id() {
        1 => glyph_map.get("/g000").cloned(),
        2 => glyph_map.get("/g001").cloned(),
        3 => glyph_map.get("/CustomA").cloned(),
        _ => None,
    }
};

// Create font with custom CharProcs
let font = Type3Font::mock(Some(char_procs));

// Test rasterization of unmapped glyph
// (This will escalate to Level 4 shape recognition)
```

### Critical Edge Case: Test Helper Key Format

**Problem**: Early test code used integer IDs as HashMap keys instead of the expected string format.

**Wrong Pattern:**
```rust
// DON'T DO THIS - this pattern was found to be incorrect
let mut glyph_map = HashMap::new();
glyph_map.insert(10, glyph_data);  // Integer key - WRONG!
let resolver = make_test_resolver(&glyph_map);
```

**Why This Fails:**
The `make_test_resolver()` function maps ObjRef IDs to character names using the formula:
```rust
(ref_id.object as u8 + b'A' - 1) as char
```

For ObjRef ID 1, this maps to character name "/A". For ID 2, "/B", etc.
But the resolver expects the glyph_map to use these string names as keys, not integer IDs.

**Correct Pattern:**
```rust
// DO THIS - use string keys matching the expected character name format
let mut glyph_map: HashMap<String, Vec<u8>> = HashMap::new();
glyph_map.insert("/A".to_string(), glyph_data);  // String key - CORRECT!
let resolver = make_test_resolver(&glyph_map);
```

### Critical Edge Case: High ObjRef ID Mapping

**Problem**: The default resolver's character name generation formula produces invalid results for high ObjRef IDs.

**The Formula Issue:**
```rust
(ref_id.object as u8 + b'A' - 1) as char
```

- For ID 1: `(1 + 65 - 1) = 65` → 'A' ✓
- For ID 10: `(10 + 65 - 1) = 74` → 'J' ✓
- For ID 26: `(26 + 65 - 1) = 90` → 'Z' ✓
- For ID 27: `(27 + 65 - 1) = 91` → '[' ✗ (not a letter)
- For ID 100: `(100 + 65 - 1) = 164` → '¤' (currency symbol) ✗

**Solution**: For high ObjRef IDs or custom glyph names, create a custom resolver instead of using `make_test_resolver()`:

```rust
// For custom glyph names, use make_custom_char_procs_from_names
let char_procs = make_custom_char_procs_from_names(&["g1", "g2", "g3"], 1);

// And create a custom resolver that maps by name instead of ID formula
let resolver = |ref_id: ObjRef| -> Option<Vec<u8>> {
    // Map based on the actual glyph names in your char_procs
    match ref_id.id() {
        1 => Some(glyph_data_1),
        2 => Some(glyph_data_2),
        3 => Some(glyph_data_3),
        _ => None,
    }
};
```

### API Migration Notes (v0.x → v1.x)

The test helper APIs underwent significant improvements to fix edge cases:

**Changed Functions:**
1. **`make_test_char_procs()`**: Now returns `HashMap<Arc<str>, ObjRef>` instead of `HashMap<String, ObjRef>` for better performance and thread-safety
2. **`make_custom_char_procs()`**: Now accepts `&[(&str, ObjRef)]` instead of `&[(String, ObjRef)]` for more ergonomic usage
3. **New Function**: `make_custom_char_procs_from_names()` - Auto-generates sequential IDs from a list of glyph names

**Migration Example:**
```rust
// OLD (no longer compiles)
let mappings = vec![
    ("/A".to_string(), ObjRef::new(1, 0)),
    ("/B".to_string(), ObjRef::new(2, 0)),
];
let char_procs = make_custom_char_procs(&mappings);

// NEW (recommended approach)
let char_procs = make_custom_char_procs_from_names(&["A", "B"], 1);
```

## Test Configuration References

### Configuration Files

1. **`.config/nextest.toml`**: Test timeout and retry configuration
2. **`build/unmapped-glyph-names.json`**: Unmapped glyph names configuration
3. **`build/CHECKSUMS.sha256`**: Build-time data file checksums

### Test Helper Modules

1. **`pdftract_core::test_helpers::process_guard`**: Orphaned process verification
2. **`pdftract_core::font::unmapped`**: Unmapped glyph name checking
3. **`pdftract_core::font::encoding::DifferencesOverlay`**: /Differences array parsing

### Documentation

1. **`docs/test-hygiene/orphaned-process-verification.md`**: Complete guide to orphaned process verification
2. **`docs/test-hygiene/post-test-orphan-verification-integration.md`**: CI integration for post-test verification
3. **`docs/test-hygiene/troubleshooting-orphaned-processes.md`**: Troubleshooting procedures

### Scripts

1. **`scripts/check-orphaned-processes.sh`**: Shell script for orphaned process detection and cleanup
2. **`xtask/src/bin/gen_unmapped_fixtures.rs`**: Fixture generation for unmapped glyph tests

## Test Skip Conditions

### Test-Level Skips via `#[ignore]`

Certain tests are marked with `#[ignore]` and require explicit invocation:

1. **Performance Tests**: Tests marked with `#[ignore = "Performance test - run with --release"]`
   - Example: `test_encryption_performance` in `encryption_integration_tests.rs`
   - Must be run with: `cargo test --release -- --ignored`
   - Reason: Performance tests require optimized builds and take longer to run

2. **Diagnostic/Debug Tests**: Tests marked with `#[ignore = "Diagnostic test - run with cargo test -- --ignored"]`
   - Examples: `debug_list_available_fixtures` in `json_schema.rs`, `debug_ocg_default_off` in `document_model.rs`
   - Used for troubleshooting and fixture discovery
   - Not part of the normal test suite

3. **Memory Limit Tests**: Tests marked with `#[ignore = "memory limit tests interfere with each other when run in the same process"]`
   - Examples: Multiple tests in `memory_guard.rs` and `memory_guard_tests.rs`
   - Must be run individually: `cargo test test_memory_guard_alloc_failure -- --ignored`
   - Reason: Setting process-wide memory limits affects all tests in the same process

### Platform-Specific Skips

**Windows Limitations:**
- Memory guard tests use `#[cfg_attr(not(target_os = "windows"), test)]`
- Windows doesn't support per-thread memory limits via `rlimit`
- These tests are automatically skipped on Windows, not marked as failures

**From memory_guard.rs documentation:**
```rust
#[cfg_attr(not(target_os = "windows"), test)]
fn test_allocation_fails_gracefully() {
    // Test code here - skipped on Windows
}
```

### Fixture-Dependent Skips

Some tests skip execution when required fixtures are missing:

1. **Forms Integration Tests**: `tests/forms_integration.rs`
   - Skips XFA fixtures with explicit TODO messages
   - Skips when no PDF files found in fixtures directory
   - Skips when no ground truth file exists for a fixture

2. **Page Access Tests**: `crates/pdftract-core/tests/test_page_access.rs`
   - Prints "⚠ Fixture not found, skipping test" when fixture missing
   - Continues without failing

3. **Encryption Fixtures Usage**: `tests/encryption_fixtures_usage_example.rs`
   - Marked with `#[ignore = "Example test - demonstrates fixture usage"]`
   - Used as documentation for fixture usage patterns

### Running Ignored Tests

```bash
# Run all ignored tests
cargo nextest run -- --ignored

# Run a specific ignored test
cargo nextest run test_memory_guard_alloc_failure -- --ignored

# Run performance tests with optimized build
cargo test --release -- --ignored
```

## Summary

The pdftract test suite has comprehensive edge case coverage for:
- Timeout prevention and hung test detection
- Orphaned process management with automated verification
- Unmapped glyph filtering behavior
- Configuration file handling and validation
- Platform-specific considerations
- Test skip conditions and when to use them
- Debugging and troubleshooting procedures

All tests use enhanced assertion messages with diagnostic context to make failures easier to understand and fix. The test infrastructure prioritizes preventing test hangs and resource leaks through timeout enforcement and process cleanup verification.
