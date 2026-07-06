# CMAP Test Structure and Fixture Exploration — bf-3tzsn

## Overview

This note documents the CMAP (character map) test infrastructure, including existing test patterns, fixture structures, and configuration mechanisms for unmapped glyph handling.

## Acceptance Criteria: PASS ✅

- [x] Located existing CMAP test file(s)
- [x] Identified test fixture patterns
- [x] Found examples of unmapped_glyph_names config
- [x] Documented the test structure pattern in this note
- [x] Ready to create test fixture based on this understanding

---

## Test Files Structure

### 1. Unit Tests: `crates/pdftract-core/tests/unmapped_glyph_names_config.rs`

Tests the `UnmappedGlyphNamesConfig` structure that reads from `build/unmapped-glyph-names.json`:

```rust
struct UnmappedGlyphNamesConfig {
    #[serde(default)]
    unmapped_glyph_names: Vec<String>,
    
    #[serde(default)]
    description: Option<String>,
    
    #[serde(default)]
    version: Option<String>,
}
```

**Test patterns observed:**
- `test_unmapped_glyph_names_defaults_to_empty()` - Config without field defaults to empty Vec
- `test_unmapped_glyph_names_specified()` - Config with field parses correctly
- `test_unmapped_glyph_names_empty_array()` - Explicit `[]` parses correctly
- `test_unmapped_glyph_names_minimal_config()` - Empty `{}` defaults all fields

### 2. Integration Tests: `crates/pdftract-core/tests/encoding_recovery.rs`

Tests Unicode recovery from PDFs without ToUnicode CMaps using the `EncodingFixture` pattern:

```rust
struct EncodingFixture {
    name: &'static str,
    pdf_path: &'static str,
    truth_path: &'static str,
    description: &'static str,
}
```

**Test fixtures in corpus:**
- `no-mapping` - PDF with no ToUnicode, custom encoding (worst case)
- `agl-only` - AGL glyph names only (Level 2 recovery)
- `fingerprint-match` - Embedded font for fingerprinting (Level 3)
- `shape-match` - Subset font for shape recognition (Level 4)

**Test patterns observed:**
- `test_no_mapping_fixture()` - Verifies CER ≥ 0.0 and recovery_rate ≤ 1.0
- `test_agl_only_fixture()` - Perfect match assertions (CER = 0.0, rate = 1.0)
- `test_corpus_recovery_rate()` - Calculates weighted average across corpus

---

## Fixture Structure

### Directory Layout

```
tests/fixtures/
├── encoding/           # Encoding/Unicode recovery fixtures
│   ├── no-mapping.pdf
│   ├── no-mapping.txt
│   ├── no-mapping.md
│   ├── agl-only.pdf
│   ├── agl-only.txt
│   ├── fingerprint-match.pdf
│   ├── fingerprint-match.txt
│   ├── shape-match.pdf
│   ├── shape-match.txt
│   ├── generate_unmapped_glyphs.py
│   └── create_unmapped_comprehensive.py
└── forms/              # Form fixtures (with generator pattern)
    └── generate_form_fixtures.rs
```

### Fixture Documentation Pattern

Each fixture has a corresponding `.md` file with this structure:

**Example:** `tests/fixtures/encoding/no-mapping.md`

```markdown
# no-mapping.pdf Fixture

## Purpose
Level 4 Unicode recovery test fixture - worst case scenario...

## Structure
### PDF Properties
- PDF Version: 1.4
- Pages: 1
- Page Size: 612 x 792 pts (Letter)

### Font Details
Name: CustomNoMap
Type: Type 1
Encoding: Custom (Differences array)
Embedded: No
ToUnicode CMap: No

## Inspection Commands
### Basic PDF info:
pdfinfo tests/fixtures/encoding/no-mapping.pdf

### Font details:
pdffonts tests/fixtures/encoding/no-mapping.pdf

## Verification
# Check SHA256 matches expected value
sha256sum tests/fixtures/encoding/no-mapping.pdf

# Verify extraction produces U+FFFD characters
./target/release/pdftract extract tests/fixtures/encoding/no-mapping.pdf \
  --format text -o /tmp/no-mapping-output.txt

## Test References
- tests/debug_encoding_pdf.rs
- tests/debug_encoding_fixtures.rs
- tests/encoding_recovery.rs
```

---

## Unmapped Glyph Names Configuration

### Configuration File: `build/unmapped-glyph-names.json`

```json
{
  "unmapped_glyph_names": [
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
  ],
  "description": "Glyph names that should be skipped during CMAP and ToUnicode entry creation...",
  "version": "1.0"
}
```

### Build-Time Processing: `crates/pdftract-core/build.rs`

The `generate_unmapped_glyph_names()` function:

1. Reads `build/unmapped-glyph-names.json`
2. Emits build warning if not found (uses default `.notdef` only)
3. Generates `UNMAPPED_GLYPH_NAMES` LazyLock<HashSet<&'static str>> in `$OUT_DIR`
4. Include path: `include!(concat!(env!("OUT_DIR"), "/unmapped_glyph_names.rs"));`

### Runtime Usage: `crates/pdftract-core/src/font/encoding.rs`

The `DifferencesOverlay` struct uses the unmapped set:

```rust
pub struct DifferencesOverlay {
    entries: Vec<(u8, Arc<str>)>,
    unmapped_glyph_names: std::collections::HashSet<String>,
}

impl DifferencesOverlay {
    fn default_unmapped_glyph_names() -> std::collections::HashSet<String> {
        crate::font::unmapped::UNMAPPED_GLYPH_NAMES
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
    
    fn is_unmapped_glyph_name(&self, name: &str) -> bool {
        self.unmapped_glyph_names.contains(name)
    }
}
```

**CMAP entry creation marker (line 221-228 in encoding.rs):**
```rust
// MARKER: CMAP entry creation point - Type1 font encoding differences.
// See notes/bf-e4uvb-child-1.md for documentation.
if !overlay.is_unmapped_glyph_name(&name) {
    overlay.entries.push((cursor as u8, Arc::clone(name)));
}
```

---

## Generator Patterns

### Python Generator: `tests/fixtures/encoding/generate_unmapped_glyphs.py`

**Purpose:** Generate PDF fixtures with custom glyph names and encodings

**Usage:**
```bash
python3 generate_unmapped_glyphs.py [OPTIONS]
  --output PATH       Output PDF path (default: unmapped-glyphs.pdf)
  --ground-truth PATH  Ground truth .txt path (default: unmapped-glyphs.txt)
  --title NAME         Font base name (default: UnmappedTestFont)
  --glyphs JSON       Glyph mappings as JSON string
```

**Default glyph set (10 character codes):**
- Codes 0-2: `/g001`, `/g002`, `/g003` (PUA unmapped)
- Codes 3-6: `/CustomA`, `/CustomB`, `/NotAGlyph`, `/glyph_0041` (unmapped)
- Codes 7-9: `/A`, `/B`, `/space` (AGL mapped)

### Rust Generator: `tests/fixtures/forms/generate_form_fixtures.rs`

**Pattern for creating PDF fixtures programmatically:**

```rust
use lopdf::dictionary;
use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};

fn create_simple_page(content: &[u8], doc: &mut Document) -> ObjectId {
    let mut page_dict = Dictionary::new();
    page_dict.set("Type", "Page");
    page_dict.set("MediaBox", Object::Array(vec![
        Object::Real(0.0), Object::Real(0.0),
        Object::Real(612.0), Object::Real(792.0)
    ]));
    // ... add resources and content stream
    doc.add_object(page_dict)
}
```

---

## CMAP Source Code Structure

### CMap Parser: `crates/pdftract-core/src/font/cmap.rs`

**Purpose:** Parse `/ToUnicode` streams from PDF fonts as PostScript CMap programs

**Supported CMap syntax:**
- `beginbfchar` / `endbfchar`: Single-character mappings
- `beginbfrange` / `endbfrange`: Range mappings (contiguous and explicit array)
- `usecmap`: Inheritance from named CMaps (stub - emits diagnostic)
- Comments: `%` to end of line (stripped by lexer)

**CMAP entry creation marker (line 83-85 in cmap.rs):**
```rust
/// MARKER: CMAP entry creation point - this is where individual ToUnicode
/// mappings are stored. Called by parse_beginbfchar() and parse_beginbfrange().
/// See notes/bf-e4uvb-child-1.md for documentation.
pub fn add_mapping(&mut self, src: Vec<u8>, dst: Vec<char>) {
    self.mappings.insert(src, dst);
}
```

---

## Predefined CMaps

**Location:** `crates/pdftract-core/build/predefined-cmaps/`

**Files:**
- `adobe-gb1.json` - Adobe-GB1-UCS2 (Simplified Chinese)
- `adobe-cns1.json` - Adobe-CNS1-UCS2 (Traditional Chinese)
- `adobe-japan1.json` - Adobe-Japan1-UCS2 (Japanese)
- `adobe-korea1.json` - Adobe-Korea1-UCS2 (Korean)

**Example structure (adobe-japan1.json):**
```json
{
  "236": "あ",
  "237": "い",
  "238": "う"
}
```

---

## Test Creation Pattern Summary

### To create a new CMAP test:

1. **Create fixture files:**
   - `<name>.pdf` - Test PDF
   - `<name>.txt` - Ground truth expected output
   - `<name>.md` - Documentation following the pattern above

2. **Add fixture to corpus:**
   - Add `EncodingFixture` entry to `get_fixtures()` in `encoding_recovery.rs`
   - Include paths relative to `crates/pdftract-core/tests/`

3. **Write test function:**
   ```rust
   #[test]
   fn test_<name>_fixture() {
       let fixture = &get_fixtures()[index];
       let result = test_encoding_fixture(fixture).unwrap();
       // Assertions based on fixture expectations
   }
   ```

4. **Update unmapped_glyph_names if needed:**
   - Edit `build/unmapped-glyph-names.json`
   - Add new glyph names to `unmapped_glyph_names` array
   - Rebuild to regenerate code

5. **Regenerate CHECKSUMS.sha256:**
   ```bash
   cd crates/pdftract-core/build
   sha256sum unmapped-glyph-names.json >> CHECKSUMS.sha256
   ```

---

## Build-Time Configuration

**Build.rs watches these files:**
```rust
println!("cargo:rerun-if-changed=build/unmapped-glyph-names.json");
println!("cargo:rerun-if-changed=build/std14-metrics.json");
println!("cargo:rerun-if-changed=build/named-encodings.json");
println!("cargo:rerun-if-changed=build/agl.json");
println!("cargo:rerun-if-changed=build/font-fingerprints.json");
println!("cargo:rerun-if-changed=build/predefined-cmaps/");
println!("cargo:rerun-if-changed=build/glyph-shapes.json");
```

**Checksum verification (TH-06 supply-chain gate):**
- All build data files are checksummed in `CHECKSUMS.sha256`
- Build fails if checksums don't match
- To regenerate: `sha256sum <files> > CHECKSUMS.sha256`

---

## Key Learnings

1. **Configuration is build-time, not runtime** - `unmapped-glyph-names.json` is read at compile time and generates Rust code
2. **CMAP tests use two patterns** - Unit tests for config parsing, integration tests for extraction behavior
3. **Fixtures are documented in Markdown** - Each fixture has a corresponding `.md` file with structure, verification, and references
4. **Unmapped glyphs are filtered at entry creation** - The `is_unmapped_glyph_name()` check prevents unmapped glyphs from being added to CMAP entries
5. **Python generators for encoding fixtures** - `generate_unmapped_glyphs.py` creates PDFs with custom encodings
6. **Rust generators for form fixtures** - `generate_form_fixtures.rs` uses `lopdf` library for programmatic PDF creation

---

## References

- `crates/pdftract-core/tests/unmapped_glyph_names_config.rs` - Config parsing tests
- `crates/pdftract-core/tests/encoding_recovery.rs` - Integration tests
- `crates/pdftract-core/src/font/cmap.rs` - ToUnicode CMap parser
- `crates/pdftract-core/src/font/encoding.rs` - Named encodings and unmapped glyph handling
- `crates/pdftract-core/build.rs` - Build-time code generation
- `build/unmapped-glyph-names.json` - Unmapped glyph names configuration
- `tests/fixtures/encoding/no-mapping.md` - Example fixture documentation

---

## Status

✅ **Task Complete** - All acceptance criteria met. Ready to create test fixtures based on this understanding.

**Date:** 2026-07-06
**Bead:** bf-3tzsn
