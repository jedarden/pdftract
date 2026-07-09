# pdftract xtask

This directory contains [xtask](https://github.com/matklad/cargo-xtask) utilities for the pdftract project. The `xtask` pattern is a Cargo convention for project-specific tasks that don't belong in the main crate.

## Overview

The `xtask` crate provides convenient commands for:
- Generating JSON schemas from Rust types
- Generating CLI reference documentation
- Creating PDF test fixtures
- Running memory ceiling tests
- Measuring rustdoc coverage
- Managing profile documentation

## Running xtask Commands

All xtask commands are run via Cargo from the workspace root:

```bash
# From the workspace root (where Cargo.toml with [workspace] lives)
cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- <command>

# Or use the shorthand if you're in the xtask directory
cargo run --bin xtask -- <command>
```

## Available Commands

### Schema Generation

#### `gen-schema`
Generate the canonical JSON Schema for pdftract extraction output from Rust types.

**Purpose:** Creates `docs/schema/v1.0/pdftract.schema.json` by deriving a JSON Schema from `pdftract_core::schema::Output` using `schemars`.

**Output:** `docs/schema/v1.0/pdftract.schema.json`

**When to run:** After modifying the `Output` type structure or adding new fields. The schema is checked into the repository and should be regenerated whenever output types change.

**Usage:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- gen-schema
```

**What it does:**
- Uses `schemars::schema_for!(Output)` to generate schema from Rust types
- Adds explicit enum constraints for specific fields (severity, page_type, confidence_source)
- Sets stable `$id`, `title`, and `description`
- Sorts all keys recursively for deterministic output
- Writes to `docs/schema/v1.0/pdftract.schema.json`

---

#### `validate-schema`
Validate that the checked-in JSON Schema matches the generated schema.

**Purpose:** CI gate to ensure schema drift is detected and committed. Fails if the checked-in schema differs from the generated schema.

**When to run:** In CI to detect schema drift. Developers should run this before committing changes to output types.

**Usage:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- validate-schema
```

**What it does:**
- Regenerates schema in memory
- Compares with checked-in `docs/schema/v1.0/pdftract.schema.json`
- Fails with diff preview if drift detected
- Prints helpful message explaining how to regenerate

---

### Documentation Generation

#### `gen-cli-reference` or via xtask: (not exposed as main xtask command)
Generate CLI reference documentation from clap command tree.

**Purpose:** Creates `docs/user-docs/src/cli-reference.md` with auto-generated CLI documentation.

**Output:** `docs/user-docs/src/cli-reference.md`

**When to run:** After modifying CLI arguments, commands, or help text.

**Usage:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin gen_cli_reference
cargo run --manifest-path=xtask/Cargo.toml --bin gen_cli_reference -- --output docs/custom-cli.md
```

**What it does:**
- Uses `clap-markdown` to generate markdown from clap command definition
- Preserves hand-curated content after `<!-- AUTOGEN END -->` marker
- Adds default usage examples if file doesn't exist

---

#### `rustdoc_coverage`
Calculate rustdoc coverage for pdftract-core public API.

**Purpose:** Measure documentation coverage by counting public items (modules, functions, structs, enums, traits, type aliases, constants) and those with worked examples (```rust blocks).

**When to run:** Before release or to track documentation progress. Target: 80% coverage.

**Usage:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin rustdoc_coverage
```

**Output:** Console report showing coverage by category and overall percentage.

**What it does:**
- Walks `crates/pdftract-core/src/` directory
- Parses each `.rs` file for `pub` declarations
- Checks for preceding doc comments with ```rust examples
- Reports coverage percentage with PASS/FAIL against 80% target

---

#### `doc-profile <profile-name>`
Generate a profile README skeleton from `profile.yaml`.

**Purpose:** Creates a template README for builtin profiles with extracted field information.

**Output:** `profiles/builtin/<profile-name>/README.md`

**When to run:** When creating a new profile or updating profile structure.

**Usage:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- doc-profile invoice
```

**What it does:**
- Reads `profiles/builtin/<profile-name>/profile.yaml`
- Extracts match criteria (text patterns, structural signals)
- Documents extracted fields with types and sources
- Creates template README with placeholders for match criteria and known limitations

---

#### `doc-profiles`
Generate README skeletons for all builtin profiles.

**Purpose:** Bulk-update all profile READMEs from their YAML definitions.

**When to run:** After bulk profile changes or during profile refactoring.

**Usage:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- doc-profiles
```

---

### PDF Fixture Generators

#### `generate-tagged-fixtures`
Generate tagged PDF fixtures for Phase 7.1 StructTree testing.

**Purpose:** Create PDF/UA and PDF/A tagged fixtures for structure tree parsing.

**Fixtures Generated:**
- `tests/fixtures/tagged/tagged-ua-simple.pdf` - Minimal PDF/UA-1 with heading + paragraph
- `tests/fixtures/tagged/tagged-ua-table.pdf` - PDF/UA with tagged table (TR/TD structure)
- `tests/fixtures/tagged/tagged-a-2a.pdf` - PDF/A-2a document with StructTree
- `tests/fixtures/tagged/tagged-mcid-ordering.pdf` - MCID-to-structure-element mapping test

**When to run:** After modifying StructTree parsing or when adding new tagged fixture tests.

**Usage:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- generate-tagged-fixtures
```

---

#### `generate-stress-pdfs`
Generate stress-test PDFs for memory ceiling validation.

**Purpose:** Create large-page-count PDFs to test memory targets.

**Fixtures Generated:**
- `tests/fixtures/perf/100-page-vector.pdf` (100 pages, buffered mode, target: <512 MB)
- `tests/fixtures/perf/10k-page.pdf` (10,000 pages, streaming mode, target: <256 MB)

**When to run:** Before memory ceiling tests or when updating memory targets.

**Usage:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- generate-stress-pdfs
```

**What it does:**
- Uses `lopdf` to create multi-page PDFs with minimal content per page
- Reports file sizes after generation
- Creates fixtures for both buffered (100-page) and streaming (10k-page) modes

---

#### `generate-page-class-fixtures`
Generate page classification test fixtures.

**Purpose:** Create fixtures for testing page type classification (Vector, Scanned, Hybrid, BrokenVector).

**Fixtures Generated:**
- `tests/fixtures/page_class/vector_pure/source.pdf` - Pure text PDF (born-digital)
- `tests/fixtures/page_class/scanned_single/source.pdf` - Image-only PDF
- `tests/fixtures/page_class/brokenvector_pdfa/source.pdf` - Invisible text + image
- `tests/fixtures/page_class/hybrid_header_body/source.pdf` - Text header + scanned body

**When to run:** When adding page classification tests or modifying classification heuristics.

**Usage:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- generate-page-class-fixtures
```

**What it does:**
- Creates 4 fixture types testing different page characteristics
- Generates `expected.json` for each fixture with classification expectations
- Uses `lopdf` for direct PDF construction

---

#### `generate-brokenvector-fixtures`
Generate BrokenVector OCR test fixtures for assisted-OCR testing.

**Purpose:** Create PDFs with invisible text layers at known offsets to test assisted OCR performance.

**Fixtures Generated:**
- `tests/fixtures/ocr/brokenvector_aligned/source.pdf` - Text layer at correct positions (assisted OCR should outperform blind OCR)
- `tests/fixtures/ocr/brokenvector_misaligned/source.pdf` - Text layer offset by (10pt, 5pt) (assisted OCR should not regress)

**When to run:** When working on assisted OCR or BrokenVector page handling.

**Usage:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- generate-brokenvector-fixtures
```

**What it does:**
- Creates two fixtures: aligned (offset 0,0) and misaligned (offset 10pt, 5pt)
- Includes ground truth text files
- Uses invisible text (Tr=3) over scan image

---

#### `generate-sensitive-fixture`
Generate password-protected PDF for TH-08 log audit testing.

**Purpose:** Create a PDF with unique markers to verify that sensitive content (passwords, body text) does NOT leak into log output.

**Fixtures Generated:**
- `tests/fixtures/security/sensitive.pdf` - PDF with unique markers
- `tests/fixtures/security/sensitive.pdf.provenance.md` - Provenance documentation

**When to run:** When modifying TH-08 test or updating logging infrastructure.

**Usage:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- generate-sensitive-fixture
```

**What it does:**
- Creates PDF with body text: "UNIQUE-MARKER-IN-BODY-TEXT-7f9a"
- Creates PDF with password: "UNIQUE-PASSWORD-FOR-TH08-7f9a"
- These markers are designed to be unlikely in normal logs
- Test verifies RUST_LOG=trace does NOT leak sensitive data

---

#### `gen-shape-db <fonts-dir> [output-path]`
Generate glyph shape database from font files.

**Purpose:** Create perceptual hash database for glyph shape recognition (Level 4 Unicode recovery).

**Output:** `build/glyph-shapes.json` (default) or custom path.

**When to run:** When adding new fonts to the training set or updating shape recognition.

**Usage:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- gen-shape-db /path/to/fonts build/glyph-shapes.json
```

**What it does:**
- Walks font directory for .ttf/.otf files
- Rasterizes each glyph at 32x32 using `fontdue`
- Computes pHash (perceptual hash) for each glyph
- Resolves collisions by keeping higher-frequency character
- Outputs JSON array of `{phash_hex, char, source_font, frequency_rank}` entries

**Requirements:** Font directory with TrueType/OpenType fonts.

---

### Testing & Quality

#### `memory-ceiling`
Run memory ceiling tests against perf and malformed corpora.

**Purpose:** Enforce Tier-1 memory targets from the plan:
- Peak RSS, 100-page vector PDF (buffered mode) < 512 MB
- Peak RSS, streaming/NDJSON mode < 256 MB  
- Peak RSS, adversarial fixtures < 1 GB hard ceiling

**Output:** `memory-report.json` (CI artifact for historical tracking)

**When to run:** In CI before merge, or when optimizing memory usage.

**Usage:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- memory-ceiling
```

**What it does:**
1. Builds `pdftract` binary in release mode
2. Runs extraction on perf corpus (buffered mode, 512 MB budget)
3. Runs extraction on perf corpus (streaming mode, 256 MB budget)
4. Runs extraction on malformed corpus (adversarial, 1 GB budget)
5. Samples RSS via `/proc/[pid]/status` (Linux-specific)
6. Generates `memory-report.json` with commit SHA, budgets, and results
7. Fails if any document exceeds its budget

**Note:** Analogous to `cargo-bloat` for memory usage - fails the build if budgets are exceeded.

---

## Binary-Specific Generators

Some generators are invoked directly as binaries rather than through the main `xtask` command:

### `gen_encoding_fixtures`
Generate encoding test fixtures for Unicode recovery Levels 2-4.

**Binary:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin gen_encoding_fixtures
```

**Fixtures Generated:**
- `tests/fixtures/encoding/agl-only.pdf` - Standard 14 font, Level 2 (AGL) recovery
- `tests/fixtures/encoding/no-mapping.pdf` - Font with no encoding or ToUnicode
- `tests/fixtures/encoding/fingerprint-match.pdf` - Font embedded for fingerprint matching (Level 3)
- `tests/fixtures/encoding/shape-match.pdf` - Font for shape-based recognition (Level 4)

---

### `gen_form_fixtures`
Generate AcroForm and XFA PDF test fixtures.

**Binary:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin gen_form_fixtures
```

**Fixtures Generated:**
- `tests/fixtures/forms/acroform-text-fields.pdf` - Text, checkbox, radio, dropdown fields
- `tests/fixtures/forms/acroform-readonly.pdf` - Pre-filled read-only fields
- `tests/fixtures/forms/acroform-submit.pdf` - Form with submit button
- `tests/fixtures/forms/xfa-dynamic.pdf` - XFA dynamic form placeholder

---

### `gen_scanned_fixtures`
Generate scanned image-based PDF fixtures.

**Binary:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin gen_scanned_fixtures
```

**Fixtures Generated:** OCR test fixtures with 300 DPI scanned pages.

---

### `gen_lzw_fixtures`
Generate LZW-compression test fixtures.

**Binary:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin gen_lzw_fixtures
```

**Fixtures Generated:** PDFs with LZW-compressed content streams for decompression testing.

---

### `gen_unmapped_fixtures`
Generate unmapped Unicode test fixtures.

**Binary:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin gen_unmapped_fixtures
```

**Fixtures Generated:** Fixtures for testing unmapped character codepoints.

---

### `generate_document_json`
Generate document JSON fixtures for schema validation testing.

**Binary:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin generate_document_json
```

---

### `migrate_schema`
Migrate schema versions during schema evolution.

**Binary:**
```bash
cargo run --manifest-path=xtask/Cargo.toml --bin migrate_schema
```

**Purpose:** Transform JSON documents between schema versions when the output schema changes.

---

## Development Workflow

### Typical Development Cycle

1. **Modify output types:**
   ```bash
   cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- gen-schema
   cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- validate-schema
   ```

2. **Update CLI arguments:**
   ```bash
   cargo run --manifest-path=xtask/Cargo.toml --bin gen_cli_reference
   ```

3. **Add new fixture tests:**
   ```bash
   # For tagged PDFs
   cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- generate-tagged-fixtures
   
   # For encoding tests
   cargo run --manifest-path=xtask/Cargo.toml --bin gen_encoding_fixtures
   
   # For page classification
   cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- generate-page-class-fixtures
   ```

4. **Memory optimization:**
   ```bash
   cargo run --manifest-path=xtask/Cargo.toml --bin xtask -- memory-ceiling
   # Review memory-report.json, optimize, rerun
   ```

5. **Documentation tracking:**
   ```bash
   cargo run --manifest-path=xtask/Cargo.toml --bin rustdoc_coverage
   ```

---

## Adding a New xtask Command

To add a new command to the main xtask binary:

1. Add the command name to the usage text in `src/main.rs`
2. Add a match arm in the `match args[1].as_str()` block
3. Implement the command function (can be in `main.rs` or a separate module)
4. Update this README with the new command

Example:
```rust
// In main.rs match block
"my-new-command" => {
    my_new_command()?;
    Ok(())
}

// Implement the function
fn my_new_command() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running my new command...");
    // ... implementation ...
    Ok(())
}
```

---

## Related Documentation

- [Test Fixtures Structure](../tests/fixtures/STRUCTURE.md) - Complete fixtures directory organization
- [Fixture Provenance Log](../tests/fixtures/PROVENANCE.md) - Generation history for each fixture
- [tools/README.md](../tools/README.md) - Python and shell generator scripts
- [Schema Documentation](../docs/schema/v1.0/) - JSON Schema for extraction output
- [User Documentation](../docs/user-docs/) - CLI reference and user guides

---

## Maintenance Notes

- **Schema versioning:** When updating the schema, increment the version in `docs/schema/v1.0/` and update all references.
- **Fixture idempotency:** All fixture generators should be idempotent - running multiple times should produce identical output.
- **Cross-platform:** Most xtask commands work on all platforms. `memory-ceiling` RSS sampling is Linux-specific.
- **CI integration:** Several xtask commands are integrated into Argo Workflows CI (see `.ci/argo-workflows/`).

---

## Conventions

- **Output paths:** All generators write to `tests/fixtures/` unless otherwise specified.
- **Error handling:** Use `Result<(), Box<dyn std::error::Error>>` for command functions.
- **Workspace root:** Use the `find_workspace_root()` helper to locate the workspace from any directory.
- **Progress feedback:** Print progress messages so users know what's being generated.
