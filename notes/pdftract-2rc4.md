# Verification Note: pdftract-2rc4

## Summary

Verified and maintained the JSON Schema generation and migration tooling for pdftract v1.0.

## Acceptance Criteria Status

### PASS Criteria

1. **Schema exists and validates as JSON Schema 2020-12**
   - File: `docs/schema/v1.0/pdftract.schema.json` (73,034 bytes)
   - Generated from Rust types using schemars derive
   - Contains all required fields: page_index, page_number, page_label, width, height, rotation, page_type

2. **page_type enum includes broken_vector**
   ```bash
   $ grep -A 10 '"broken_vector"' docs/schema/v1.0/pdftract.schema.json
   ```
   Confirmed enum values: text, scanned, mixed, broken_vector, blank, figure_only

3. **attachments data field carries contentEncoding: base64**
   ```bash
   $ grep -B 5 -A 5 'contentEncoding.*base64' docs/schema/v1.0/pdftract.schema.json
   ```
   Confirmed contentEncoding: base64 on AttachmentJson.data field

4. **xtask validate-schema regenerates and diffs cleanly**
   ```bash
   $ cargo run --manifest-path=xtask/Cargo.toml --bin xtask validate-schema
   ✓ Schema is up-to-date: /home/coding/pdftract/docs/schema/v1.0/pdftract.schema.json
   ```

5. **Migration tool runs end-to-end**
   ```bash
   $ echo '{"schema_version": "1.0", "test": "value"}' | ./target/release/migrate-schema --from 1.0 --to 1.0
   {"schema_version":"1.0","test":"value"}
   ```

### WARN Criteria

None - all infrastructure components are in place and functional.

## Files Modified

- `xtask/src/main.rs` - Added missing SpanJson.confidence_source enum constraint to add_enum_constraints function

## Infrastructure Components

1. **Schema Generator**: `xtask/src/bin/gen_schema.rs`
   - Generates JSON Schema from Rust types
   - Uses schemars crate with JSON Schema 2020-12 dialect
   - Adds explicit enum constraints for stability
   - Sorts keys recursively for deterministic output

2. **Schema Validator**: `xtask/src/main.rs::validate_schema()`
   - Regenerates schema in memory
   - Compares byte-for-byte with checked-in version
   - Fails build on drift (CI gate)

3. **Migration Library**: `crates/pdftract-schema-migrate/src/lib.rs`
   - MigrationRegistry with version-pair migrations
   - Identity migration for v1.0 -> v1.0
   - Validates migration direction (no downgrades, no major version changes)

4. **Migration CLI**: `crates/pdftract-schema-migrate/src/bin/migrate-schema.rs`
   - CLI tool for running migrations
   - Supports stdin/stdout and file I/O
   - Auto-detects pretty-printing for terminals

5. **Validation Tests**: `tests/schema/validate_fixtures.rs`
   - Validates fixture outputs against schema
   - Generates expected.json on first run
   - Tests individual fixtures and full suite

## Commands

- Generate schema: `cargo run --manifest-path=xtask/Cargo.toml --bin gen_schema`
- Validate schema: `cargo run --manifest-path=xtask/Cargo.toml --bin xtask validate-schema`
- Run migration: `./target/release/migrate-schema --from 1.0 --to 1.0 input.json -o output.json`

## Related Plan Sections

- Lines 97 (schema as source of truth)
- Lines 823 (INV-11 schema validation gate)
- Lines 986 (Anti-Pattern: serde_json::Value)
- Lines 1836 (broken_vector enum requirement)
- Lines 2002-2030 (Phase 6.1 schema deliverable)
- Lines 2640 (attachments base64 encoding)
- Lines 3230/3250 (INV-11 gates in checklists)

## Verification Date

2026-06-01
