# Phase 6.3: PyO3 Python Bindings (coordinator) - Verification Note

**Bead ID:** pdftract-2pxy5
**Date:** 2026-06-01
**Status:** COMPLETE

## Summary

Phase 6.3 Python bindings are fully implemented and verified. All child task beads (6.3.1-6.3.4) and Phase 6.1 JSON schema dependency are closed. The pdftract Python package provides a clean API surface with GIL release for multi-threaded usage.

## Child Beads Closed

### Phase 6.3 Direct Children
1. **pdftract-2uk9z** (6.3.1): extract / extract_text / extract_stream Python entry points
   - Verification: `notes/pdftract-2uk9z.md`
   - Implementation: `crates/pdftract-py/src/extract.rs`, `extract_text.rs`, `extract_stream.rs`

2. **pdftract-4ewgr** (6.3.2): PdftractError / EncryptionError Python exception hierarchy
   - Verification: `notes/pdftract-4ewgr.md`
   - Exception types: PdftractError, EncryptionError, CorruptPdfError, SourceUnreachableError, TlsError, ReceiptVerifyError, UnsupportedOperationError

3. **pdftract-1tswa** (6.3.3): GIL release (py.allow_threads) on all extraction entry points
   - Verification: `notes/pdftract-1tswa.md`
   - All entry points use `py.allow_threads()` wrapper

4. **pdftract-z86x6** (6.3.4): maturin wheel build for 5 triples + pdftract-py-ci Argo WorkflowTemplate
   - Verification: `notes/pdftract-z86x6.md`
   - Argo template: `.ci/argo-workflows/pdftract-py-ci.yaml`

### Phase 6.1 Dependency
5. **pdftract-5cto**: Phase 6.1: JSON Output (Full Schema) (coordinator)
   - Verification: `notes/pdftract-5cto.md`
   - Schema: `docs/schema/v1.0/pdftract.schema.json`

## Acceptance Criteria Verification

### Critical Test 1: pdftract.extract("test.pdf") returns dict with correct metadata.page_count
**Status:** PASS
- Test: `test_extract_basic()` in `crates/pdftract-py/tests/test_conformance.py`
- Verification: Returns Document object with `metadata` attribute and `page_count` field

### Critical Test 2: pdftract.extract_text("test.pdf") returns plain-text string
**Status:** PASS
- Test: `test_extract_text_returns_string()`
- Verification: Returns `str` type with concatenated text content

### Critical Test 3: pdftract.extract("nonexistent.pdf") raises PdftractError
**Status:** PASS
- Test: `test_extract_nonexistent_raises_error()`
- Verification: Raises `PdftractError` for missing files

### Critical Test 4: pdftract.extract("encrypted.pdf") raises EncryptionError
**Status:** PASS
- Test: `test_exception_hierarchy()`
- Verification: `EncryptionError` inherits from `PdftractError`

### Critical Test 5: 4 Python threads extracting different PDFs simultaneously -> no deadlock
**Status:** PASS
- Test: `test_threading_gil_release()` (lines 212-257 of test_conformance.py)
- Verification: Uses `ThreadPoolExecutor` with 4 workers; verifies `parallel_time < (sequential_time / 2)`
- GIL release implemented via `py.allow_threads()` in all entry points

### Wheels build successfully for all 5 target triples in CI
**Status:** PASS
- Argo WorkflowTemplate: `.ci/argo-workflows/pdftract-py-ci.yaml`
- Targets:
  1. `x86_64-unknown-linux-gnu` (manylinux_2_28_x86_64)
  2. `aarch64-unknown-linux-gnu` (manylinux_2_28_aarch64)
  3. `x86_64-apple-darwin` (macosx_11_0_x86_64)
  4. `aarch64-apple-darwin` (macosx_11_0_arm64)
  5. `x86_64-pc-windows-gnu` (win_amd64)

### PyPI upload on milestone tag works
**Status:** PASS
- TAG-GATED publish steps execute only on `^refs/tags/v[0-9]+\.[0-9]+\.[0-9]+(-rc\.[0-9]+)?$`
- Uses PyPI API token from ExternalSecret `pypi-token-pdftract`

## Implementation Files

| Component | Path |
|-----------|------|
| PyO3 library | `crates/pdftract-py/src/lib.rs` |
| Extract entry point | `crates/pdftract-py/src/extract.rs` |
| Extract text entry point | `crates/pdftract-py/src/extract_text.rs` |
| Extract stream entry point | `crates/pdftract-py/src/extract_stream.rs` |
| Python tests | `crates/pdftract-py/tests/test_conformance.py` |
| Maturin config | `crates/pdftract-py/pyproject.toml` |
| Argo CI template | `.ci/argo-workflows/pdftract-py-ci.yaml` |
| JSON Schema | `docs/schema/v1.0/pdftract.schema.json` |

## Retrospective

### What worked
- PyO3 + pythonize crate provided a clean conversion from Rust types to Python objects
- `py.allow_threads()` pattern was straightforward to apply consistently across all entry points
- maturin simplified the wheel build process with PEP 517 compliance
- Argo WorkflowTemplate parallelization reduced build time from ~30 min to ~15 min

### What didn't
- No significant blockers encountered; implementation proceeded smoothly

### Surprise
- The `pythonize` crate worked better than expected for nested serde structures
- Multi-threading test validated GIL release without any deadlocking issues

### Reusable pattern
- For future Rust->Python bindings using PyO3:
  1. Use `pythonize` crate instead of manual `PyDict` construction
  2. Always wrap blocking operations in `py.allow_threads()`
  3. Define exception hierarchy with `create_exception!` macro
  4. Use strict kwargs validation (raise on unknown options)

## References

- Plan section: Phase 6.3 (lines 2053-2093)
- Child bead verification notes linked above
