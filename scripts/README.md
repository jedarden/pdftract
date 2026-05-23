# Scripts

This directory contains utility scripts for pdftract development and testing.

## Memory Ceiling Enforcement

### Fuzz Tests (`run-fuzz-with-limits.sh`)

Runs cargo-fuzz targets with memory limits to ensure pathological inputs fail fast:

```bash
scripts/run-fuzz-with-limits.sh [target]
```

**Memory limits:**
- Cgroup MemoryMax: 1536 MB (hard ceiling)
- Libfuzzer RSS limit: 1024 MB (per-execution)
- Libfuzzer malloc limit: 1024 MB (total)

**Environment:**
- `FUZZ_TIME_SECONDS`: Time per target (default: 60)
- `MEMORY_MAX_MB`: Cgroup limit in MB (default: 1536)
- `RSS_LIMIT_MB`: Libfuzzer RSS limit (default: 1024)

**Implementation:** Uses cgroup v2 MemoryMax (preferred) or cgroup v1 memory.limit_in_bytes with OOM killer disabled for clean failure mode.

### Property Tests (`run-proptest-with-limits.sh`)

Runs proptest modules with memory limits:

```bash
scripts/run-proptest-with-limits.sh [test_name]
```

**Memory limits:**
- Cgroup MemoryMax: 2048 MB (hard ceiling)

**Environment:**
- `PROPTEST_CASES`: Test cases per module (default: 1000)
- `MEMORY_MAX_MB`: Cgroup limit in MB (default: 2048)
- `PROPTEST_SEED`: Proptest seed (default: random)

**Proptest modules:** lexer, object_parser, xref, stream, cmap_parser

**Input size caps:** All proptest strategies are bounded:
- Lexer/object parser: up to 10 KB inputs
- Xref/stream parsers: up to 100 KB inputs
- Nested structures: depth-limited (e.g., 500 for parser depth checks)

These bounds ensure tests complete quickly while still exercising edge cases.

## Why Memory Ceilings?

Per bf-1g1fd and the Quality Targets (plan.md Phase 0.4), adversarial inputs must not OOM the host. Memory ceilings enforce:

1. **Clean failure mode** - Allocation errors instead of host OOM
2. **Fast failure** - Pathological cases abort immediately at the limit
3. **Regressions as test failures** - Memory growth is caught in CI

CI enforces these limits via cgroup MemoryMax in `.ci/argo-workflows/pdftract-ci.yaml` (proptests) and `.ci/argo-workflows/pdftract-nightly-fuzz.yaml` (fuzz).

## Other Scripts

### `generate-minimal-pdf.sh`

Generates minimal valid PDF documents for testing.

### `check-provenance.sh`

Verifies binary provenance and SBOM signatures.

### `check-secrets.sh`

Scans for accidental secrets in committed code.

### `generate_test_corpus.py`

Generates synthetic PDF test corpus.
