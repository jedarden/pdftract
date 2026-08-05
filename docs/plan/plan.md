# pdftract Implementation Plan

**Version:** 1.3
**Status:** Active
**Repo:** jedarden/pdftract
**Last updated:** 2026-07-20

---

### Revision History

| Version | Date | Material Changes |
|---|---|---|
| 1.0 | 2026-05-16 | Initial plan: Phases 0–7, vector + OCR extraction, JSON/NDJSON/text output, PyO3 bindings, HTTP serve, StructTree, tables, forms, signatures, attachments. |
| 1.1 | 2026-05-16 | Brilliant-ideas integration round: added MCP server (Phase 6.7), Markdown output (6.5), multi-output emission (6.6), visual citation receipts (6.8), content-addressed cache (6.9), folder grep (7.8), inspector web viewer (7.9), document profiles (7.10), structural fingerprint (1.7), remote HTTP range source (1.8), document type classification (5.6). Plus pre-flight categories 1–4: Non-Goals, Glossary, ADRs, Open Questions, Proof Obligations, Acceptance Scenarios, Edge Case Catalog, Failure Mode Taxonomy, Diagnostic Code Catalog, Cross-Cutting Concerns, Anti-Patterns Catalog, Invariants. |
| 1.2 | 2026-07-05 | Workspace reconciliation: updated File and Module Layout to reflect shipped artifacts missing from v1.1 tree — added C ABI library (pdftract-libpdftract), CER diff tool (pdftract-cer-diff), schema migration tool (pdftract-schema-migrate), xtask build utilities, SDK skeleton templates, language SDK bindings (Node, Java, .NET, Go, Ruby, PHP, Swift), Python subprocess SDK, E2E phone smoke tests, built-in profiles at repo root, CI gate scripts, and tools directory. Cross-referenced SDK strategy to docs/notes/sdk-architecture.md. No semantic changes to phase specifications. |
| 1.3 | 2026-07-20 | Deployed-artifact improvement pass (fleet-wide audit, separate from the 2026-07-19 plan-vs-code gap audit): added ADR-010 (`pdftract-wasm` sibling crate + client-side browser demo) to unblock a zero-install way to experience pdftract ahead of the still-pending first crates.io/PyPI/Docker Hub release. No changes to Phase 1–7 requirements. |

Future revisions MUST append a new row before any material change lands in subsequent sections. The revision history is the single source of truth for "what changed when" — section-level edits MUST NOT silently mutate already-shipped semantics.

---

## Primary Objectives

pdftract must be the **most accurate, fastest, and lightest-weight** PDF text extraction tool available. These are not aspirational — they are acceptance criteria. Every architectural and dependency decision is evaluated against all three in priority order.

### Accuracy targets (acceptance criteria — CI-gated)

| Metric | Target | Measurement |
|---|---|---|
| Character error rate, clean vector PDFs | < 0.5% | Against ground-truth corpus, `tests/fixtures/vector/` |
| Word error rate, clean OCR (300 DPI scans) | < 3% | Against ground-truth corpus, `tests/fixtures/scanned/` |
| Reading order correctness, multi-column | > 95% | Left column entirely before right column in all fixtures |
| Unicode recovery rate (no ToUnicode) | > 90% | Font fingerprint + AGL levels 2–4 on `tests/fixtures/encoding/` |
| Regression gate, real-world corpus | < 0.5% CER delta vs. golden | 500-PDF private corpus on every PR |
| Text readability score | > 0.85 | Proprietary composite of printable ratio, dict word ratio, ligature repair |

### Speed targets (acceptance criteria — CI-gated)

| Metric | Target | Measurement |
|---|---|---|
| 100-page vector PDF, 4-core CI | < 3 seconds | `cargo bench`, `tests/fixtures/perf/` |
| 10-page scanned PDF (OCR path), 4-core CI | < 30 seconds | includes Tesseract |
| Single-page extraction latency (serve mode) | < 150 ms p99 | wrk benchmark against `/extract` |
| Throughput vs. pdfminer.six (Python) | ≥ 10× faster | Benchmarked on identical hardware |
| Throughput vs. pypdf (Python) | ≥ 5× faster | Same benchmark suite |

### Weight targets (acceptance criteria)

| Metric | Target |
|---|---|
| Binary size, default features (no OCR, no serve) | < 4 MB stripped |
| Binary size, `--features ocr,serve` | < 12 MB stripped |
| Binary size, `--features full` (everything except `full-render`) | < 14 MB stripped |
| Default dependency count (`cargo tree -d`) | < 30 unique crates (direct, verified against `cargo tree --depth 1 -e normal --features default`). Transitive dependency count is not gated — only direct crates are tracked. The < 30 direct crate limit is verified as a CI check on the first passing build. |
| Shared library dependencies (ldd) | Zero beyond libc + libm |
| Docker image, CLI only | < 20 MB (distroless base) |
| Docker image, with OCR (`tesseract-ocr` system pkg) | < 120 MB |
| Docker image, `pdftract:full` (`--features ocr,serve,mcp,inspect,grep,profiles,cache,receipts,remote`) | < 140 MB |
| Fingerprint reproducibility (Phase 1.7) | Byte-identical hash across runs and platforms for the same input |
| Multi-output overhead (Phase 6.6) | Emitting JSON + Markdown + plain text simultaneously completes in ≤ 1.1× the single-format extraction time |
| Cache-hit latency (Phase 6.9) | < 20 ms p99 for a 100-page PDF |
| `pdftract grep` throughput (Phase 7.8) | ≥ 50 MB/s on 1000-PDF corpus, 4-core CI |
| Remote-source bytes downloaded (Phase 1.8, partial extraction) | < 5 MB for a single-page extract from a 500-page PDF |

Decisions that violate any target require explicit justification and a waiver comment in the relevant section below.

### Memory targets (acceptance criteria — CI-gated)

The fourth leg of "lightest-weight" is **runtime memory, not just binary size**. Binding invariant: pdftract MUST process any single document — including adversarial inputs — within a bounded peak-RSS ceiling that does **not** scale with input size, page count, or attack payload. A PDF that is small on disk must never be able to force multi-GB residency. This is a deployment-scalability requirement: hosts and serverless/worker runtimes budget on the order of a few hundred MB to ~1–2 GB per worker, so any single document needing > ~1 GB is a defect and > 4 GB is a release blocker.

| Metric | Target | Measurement |
|---|---|---|
| Peak RSS, 100-page vector PDF (buffered mode) | < 512 MB | `tests/fixtures/perf/`; RSS sampled at 10 ms by the memory-ceiling harness |
| Peak RSS, streaming/NDJSON mode (any page count, incl. 10,000-page EC-03) | < 256 MB, **constant in page count** | `tests/fixtures/perf/10k-page.pdf`; RSS must stay flat as page count grows |
| Peak RSS, any adversarial fixture (bomb, deep nesting, huge xref, predictor abuse) | < 1 GB hard ceiling; must not scale with payload | `tests/security/` + `tests/fixtures/malformed/`, run under a cgroup `MemoryMax` cap in CI |
| `ExtractionOptions.max_decompress_bytes` default (document-cumulative) | **512 MB** (was 2 GB) | Per `docs/research/adversarial-inputs-and-parser-security.md`; enforced incrementally in Phase 1.5 |
| Buffer pre-allocation discipline | No buffer pre-sized to a claimed or decompressed length before bytes are read | Clippy lint + review; predictor/filter stages bounded to 2 × stride, row-by-row (per `image-and-figure-extraction.md`) |
| Concurrency budget (rayon page parallelism) | Document-wide peak ≤ the ceiling above; per-page budget = ceiling ÷ max in-flight pages | The page-parallel scheduler caps simultaneously-resident pages so the ceiling holds regardless of core count |
| Serve mode (Phase 6.4) per-request residency | Bounded per request; one pathological document cannot exhaust the host | Per-request `max_decompress_bytes` + worker isolation; OOM of one request returns 5xx, never crashes the host |

CI memory-ceiling gate (analogous to the `cargo bloat` size gate): a harness samples peak RSS while extracting the perf and malformed corpora and fails the build if any document exceeds its budget. The full test and fuzz suites run under a cgroup `MemoryMax` cap so a memory regression surfaces as a clean test failure, never an OOM that takes down the runner.

> **Supersedes legacy default.** The 512 MB `max_decompress_bytes` default above supersedes the 2 GB value previously referenced in the Edge Case Catalog (EC-10), Failure Mode Taxonomy, Threat Model (TH-01), and Anti-Patterns (now reconciled to 512 MB). The 2 GB default was the root cause of an observed multi-GB OOM: a 2 GB decompress plus a full second copy in the PNG-predictor stage (`apply_png_predictors` pre-allocates `num_rows * row_size` and is outside the `max_bytes` budget), multiplied across rayon page parallelism.

### Adoption Targets (informational, not CI-gated)

The targets below are tracked publicly to gauge real-world traction. They are NOT CI-gated and missing them does not block any release; they exist to inform planning for subsequent versions and to surface positioning gaps early.

| Metric | 6-month target | 12-month target | Source |
|---|---|---|---|
| GitHub stars on `jedarden/pdftract` | 500 | 2,000 | GitHub API |
| PyPI weekly downloads (`pdftract`) | 1,000 | 10,000 | PyPI stats / `pepy.tech` |
| Docker pulls per month (`ronaldraygun/pdftract*` tags) | 500 | 5,000 | Docker Hub stats |
| Shipped MCP integrations | 2 (Claude Desktop, Cursor) | 4 (+ Continue, + custom) | Counted via published config snippets in `docs/integrations/` |
| Community-contributed profiles in `profiles/community/` | 5 | 25 | Merged PRs |
| External-contributor corpus PDFs in regression suite | 50 | 500 | Merged PRs |

Adoption metrics are reviewed quarterly. A material miss against the 12-month target on any row triggers a positioning retrospective recorded in the project's notes directory, not a plan-level rework.

### Ambition Calibration

Not every target above carries the same weight. The Accuracy / Speed / Weight tables above present binding numerical commitments; the table below classifies them by what failure means at release time. Calibration exists so reviewers can distinguish between a target whose miss blocks the milestone and a target whose miss triggers a planning discussion.

| Tier | Definition | Targets in this tier | Failure consequence |
|---|---|---|---|
| **Tier 1 — HARD GATES** (block release) | Numerical commitments whose miss would compromise the product's stated core promise. CI failure = release blocked. | Accuracy: CER < 0.5% on vector; reading order > 95%; Unicode recovery > 90%; regression Δ < 0.5%; readability > 0.85. Speed: 100-page vector < 3 s; OCR speed target (10-page in < 30 s) from v0.2.0 onward. Weight: < 4 MB default binary; < 14 MB `full`; INV-11 schema validity. Memory: adversarial-input peak RSS < 1 GB hard ceiling (OOM safety). | Release blocked at the failing milestone; no override available. |
| **Tier 2 — SHOULD HIT** (block release after one warning) | Numerical commitments where a one-time miss is tolerable provided the trend is corrected by the next minor release. | Speed: grep ≥ 50 MB/s; serve p99 < 150 ms; cache-hit < 20 ms p99. Weight: multi-output overhead ≤ 1.1×; cache-hit latency; remote bytes < 5 MB single-page; benchmark ratios ≥ 10× pdfminer.six and ≥ 5× pypdf. | First miss: stderr warning at build time + tracked deviation in `benches/results/`. Subsequent miss: release blocked. |
| **Tier 3 — ASPIRATIONAL** (track but never block) | Targets that depend on factors outside the engineering team's control (competitor evolution, user adoption, ecosystem maturity). | All Adoption Targets above; "≥ 10× pdfminer.six" if pdfminer.six materially improves before v1.0; community-contributed profile count; external-contributor corpus PDFs. | Recorded in quarterly review. Material miss triggers a planning retrospective; never a release block. |

The classification of every existing target is recorded above; new targets are placed into a tier as they are added. Moving a target from Tier 3 to Tier 2 (or Tier 2 to Tier 1) is a SHOULD-be-announced policy change recorded in the Revision History; the reverse — relaxing a Tier 1 target into Tier 2 — is a MAJOR-version event and requires a Proof Obligations Ledger fallback entry.

---

## Overview

pdftract is a Rust PDF text extraction library and CLI. It extracts Unicode text from PDF files — including scanned pages via OCR — and emits structured JSON, NDJSON, Markdown, or plain text output. The output schema is defined in `docs/research/extraction-output-schema.md` and is stable at schema version 1.0.

The binary exposes the following subcommands, each of which is documented in detail in its respective phase:

| Subcommand | Phase | Purpose |
|---|---|---|
| `pdftract extract` | 1–6 | Single-document extraction with one or more simultaneous output formats |
| `pdftract serve` | 6.4 | Long-running HTTP service for multi-tenant extraction |
| `pdftract mcp` | 6.7 | Model Context Protocol server (stdio or HTTP transport, never both at once) |
| `pdftract hash` | 1.7 | Compute the reproducible structural fingerprint of a PDF |
| `pdftract verify-receipt` | 6.8 | Verify a citation receipt against the source PDF |
| `pdftract cache` | 6.9 | Inspect and manage the content-addressed extraction cache |
| `pdftract grep` | 7.8 | Folder-scale regex search across PDFs with page+bbox results |
| `pdftract inspect` | 7.9 | Launch the web debug viewer for a PDF (local-only by default) |
| `pdftract classify` | 5.6 | Print the detected document type without running extraction |
| `pdftract profiles` | 7.10 | List, show, export, install, and validate document profiles |

A PyO3 Python binding (`pip install pdftract`) exposes the extraction API to Python code.

The implementation is organized into eight phases. Phase 0 establishes CI infrastructure (prerequisite). Phases 1–4 deliver a working vector-extraction CLI. Phase 5 adds OCR and document-type classification. Phase 6 adds the full API surface (PyO3, HTTP, MCP, Markdown, multi-output, receipts, cache). Phase 7 adds advanced features that require the Phase 1–6 foundation (StructTree, tables, signatures, forms, attachments, hyperlinks, article threads, `grep`, `inspect`, profiles).

### Key architectural decisions (baked in from the start)

- **File I/O:** `memmap2` for zero-copy random access; `madvise(MADV_SEQUENTIAL)` on content streams.
- **Object cache:** LRU with 4096-entry capacity (`lru` crate); object streams decompressed once and cached as `Arc<[u8]>`.
- **Parallelism:** `rayon` for page-level parallelism; per-page work is embarrassingly parallel after Phases 1–2 (parser and font pipeline) complete.
- **Serialization:** `serde` + `serde_json`; `BufWriter` wrapping `io::Stdout` for NDJSON streaming.
- **Error model:** All parse errors are recoverable and produce diagnostic entries in the `errors` array; no `panic!` in library code.
- **Crate layout:** `pdftract-core` (lib), `pdftract-cli` (binary), `pdftract-py` (PyO3, optional feature).

### Normative Language

This plan uses the keywords MUST, MUST NOT, SHOULD, SHOULD NOT, MAY, REQUIRED, RECOMMENDED, OPTIONAL with the precise meaning defined in RFC 2119 and clarified in RFC 8174 (only when shown in ALL CAPS).

- **MUST / REQUIRED / SHALL** — the requirement is mandatory; a non-compliant implementation is non-conformant.
- **MUST NOT / SHALL NOT** — the prohibition is absolute; a violating implementation is non-conformant.
- **SHOULD / RECOMMENDED** — the requirement is strong; deviations require a documented justification in the relevant section.
- **SHOULD NOT / NOT RECOMMENDED** — the prohibition is strong; deviations require a documented justification.
- **MAY / OPTIONAL** — the implementation choice is free; no compliance impact either way.

Where these words appear in lowercase, they are used in their ordinary English sense and carry no normative weight. Behavioral statements outside these keywords are descriptive of intent, not contractual requirements.

### File and Module Layout

The workspace is organised so that the library (`pdftract-core`) is the only crate that other consumers depend on directly. The CLI, Python bindings, and inspector UI are siblings that compose `pdftract-core` behind their respective surfaces.

```
pdftract/
├── Cargo.toml                                (workspace root)
├── crates/
│   ├── pdftract-core/
│   │   ├── Cargo.toml
│   │   ├── build.rs                          (phf_codegen for AGL, wordlist, fingerprints, glyph shapes)
│   │   ├── src/
│   │   │   ├── lib.rs                        (public API surface)
│   │   │   ├── parser/
│   │   │   │   ├── lexer.rs                  (Phase 1.1)
│   │   │   │   ├── object.rs                 (Phase 1.2)
│   │   │   │   ├── xref.rs                   (Phase 1.3)
│   │   │   │   ├── document.rs               (Phase 1.4)
│   │   │   │   ├── stream.rs                 (Phase 1.5)
│   │   │   │   ├── error.rs                  (Phase 1.6 diagnostics)
│   │   │   │   ├── fingerprint.rs            (Phase 1.7)
│   │   │   │   └── source.rs                 (Phase 1.8 PdfSource trait + impls)
│   │   │   ├── font/
│   │   │   │   ├── detect.rs                 (Phase 2.1)
│   │   │   │   ├── encoding.rs               (Phase 2.2 Levels 1–2)
│   │   │   │   ├── cjk.rs                    (Phase 2.3)
│   │   │   │   ├── type3.rs                  (Phase 2.4)
│   │   │   │   └── shape_db.rs               (Phase 2.5 Level 4)
│   │   │   ├── content/
│   │   │   │   ├── gstate.rs                 (Phase 3.1)
│   │   │   │   ├── text_ops.rs               (Phase 3.2)
│   │   │   │   ├── xobject.rs                (Phase 3.3)
│   │   │   │   ├── marked_content.rs         (Phase 3.4)
│   │   │   │   └── inline_image.rs           (Phase 3.5)
│   │   │   ├── layout/
│   │   │   │   ├── span.rs                   (Phase 4.1)
│   │   │   │   ├── line.rs                   (Phase 4.2)
│   │   │   │   ├── column.rs                 (Phase 4.3)
│   │   │   │   ├── block.rs                  (Phase 4.4)
│   │   │   │   ├── reading_order.rs          (Phase 4.5)
│   │   │   │   └── readability.rs            (Phase 4.7)
│   │   │   ├── ocr/
│   │   │   │   ├── classify.rs               (Phase 5.1)
│   │   │   │   ├── extract_image.rs          (Phase 5.2)
│   │   │   │   ├── preprocess.rs             (Phase 5.3)
│   │   │   │   ├── tesseract.rs              (Phase 5.4)
│   │   │   │   ├── assisted.rs               (Phase 5.5)
│   │   │   │   └── document_type.rs          (Phase 5.6)
│   │   │   ├── output/
│   │   │   │   ├── sink.rs                   (Phase 6.6 OutputSink trait)
│   │   │   │   ├── json.rs                   (Phase 6.1)
│   │   │   │   ├── ndjson.rs                 (Phase 6.2)
│   │   │   │   ├── markdown.rs               (Phase 6.5)
│   │   │   │   ├── text.rs                   (Phase 4.6)
│   │   │   │   └── receipt.rs                (Phase 6.8)
│   │   │   ├── cache/                        (Phase 6.9)
│   │   │   ├── profiles/                     (Phase 7.10 evaluator + built-in profile bundle)
│   │   │   └── advanced/
│   │   │       ├── struct_tree.rs            (Phase 7.1)
│   │   │       ├── table.rs                  (Phase 7.2)
│   │   │       ├── signature.rs              (Phase 7.3)
│   │   │       ├── form.rs                   (Phase 7.4)
│   │   │       ├── attachment.rs             (Phase 7.5)
│   │   │       ├── hyperlink.rs              (Phase 7.6)
│   │   │       └── thread.rs                 (Phase 7.7)
│   │   └── tests/                            (Tier 2 integration tests; see Test Infrastructure)
│   ├── pdftract-cli/
│   │   └── src/
│   │       ├── main.rs                       (subcommand dispatch)
│   │       ├── extract.rs                    (Phases 1–6 driver)
│   │       ├── grep.rs                       (Phase 7.8)
│   │       ├── inspect.rs                    (Phase 7.9)
│   │       ├── hash.rs                       (Phase 1.7)
│   │       ├── classify.rs                   (Phase 5.6 CLI)
│   │       ├── profiles.rs                   (Phase 7.10 CLI)
│   │       ├── cache.rs                      (Phase 6.9 CLI)
│   │       ├── serve.rs                      (Phase 6.4)
│   │       ├── mcp.rs                        (Phase 6.7)
│   │       └── verify_receipt.rs             (Phase 6.8)
│   ├── pdftract-py/
│   │   └── src/lib.rs                        (PyO3 bindings, Phase 6.3)
│   ├── pdftract-libpdftract/
│   │   └── src/lib.rs                        (C ABI library; FFI layer for C/C++ integrations)
│   ├── pdftract-cer-diff/
│   │   └── src/main.rs                       (Character error rate diff tool; compares extractions)
│   ├── pdftract-schema-migrate/
│   │   └── src/main.rs                       (Schema migration tool; validates/translates schema versions)
│   └── pdftract-inspector-ui/
│       └── ...                               (HTML/CSS/JS bundled via include_bytes!, Phase 7.9)
├── xtask/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                           (Build automation tasks: codegen, dist, schema generation)
│       ├── migrate/                          (Schema migration utilities)
│       └── bin/                              (Generated executables)
├── sdk/
│   ├── php/                                  (PHP subprocess SDK)
│   └── python-subprocess/                    (Python subprocess SDK)
├── templates/
│   └── sdk-skeleton/                        (SDK template for new language bindings)
├── Language SDK repositories (sibling repos under jedarden/):
│   ├── pdftract-node/                       (Node.js/TypeScript SDK)
│   ├── pdftract-java/                       (Java SDK)
│   ├── pdftract-dotnet/                      (.NET SDK)
│   ├── pdftract-go/                          (Go SDK)
│   ├── pdftract-ruby/                        (Ruby SDK)
│   ├── pdftract-php/                         (PHP SDK)
│   ├── pdftract-swift/                       (Swift SDK)
│   └── swift-sdk/                            (Swift native SDK)
├── e2e/
│   └── phone-smoke/                          (End-to-end phone smoke tests)
├── profiles/
│   ├── builtin/                              (Built-in document profiles; Phase 7.10)
│   └── community/                            (Community-contributed profiles)
├── benches/
│   └── competitors/
│       ├── requirements.txt                  (pdfminer.six, pypdf, pdfplumber pins)
│       └── run_all.py                        (Tier 4 benchmark runner)
├── build/
│   ├── font-fingerprints.json                (Phase 2.2 Level 3 source data)
│   └── glyph-shapes.json                     (Phase 2.5 shape DB source data)
├── ci/
│   ├── rustdoc-gate.sh                       (Documentation coverage gate)
│   ├── schema-gate.sh                         (Schema validation gate)
│   └── wer-gate.sh                           (Word error rate gate)
├── tools/
│   ├── README.md                             (Tools documentation)
│   ├── convert_pdf_to_scanned.sh             (PDF to scanned image converter)
│   ├── count_docs.py, count_docs.sh          (Documentation counters)
│   ├── count_public_api.py                   (Public API surface counter)
│   ├── generate_encoding_fixtures.py, .rs   (Encoding test fixture generator)
│   ├── generate_encrypted_pdf_fixtures.py, .rs (Encrypted PDF fixture generator)
│   ├── generate_invoice_fixture.rs, .py      (Invoice test fixture generator)
│   ├── generate_stress_pdf.py                (Stress test PDF generator)
│   ├── build-objstm-fixture/                 (Object stream fixture builder)
│   ├── build-xref-fixture/                   (XRef fixture builder)
│   ├── debug-fingerprint/                    (Font fingerprint debugging utilities)
│   └── debug-fingerprint-diff/              (Fingerprint diff utilities)
├── docs/
│   ├── plan/plan.md                          (this document)
│   ├── research/                             (per-feature deep dives referenced from phases)
│   ├── schema/v1.0/pdftract.schema.json      (Phase 6.1 deliverable)
│   ├── integrations/                         (MCP config snippets, IDE setup; populated post-v1)
│   ├── user-docs/                            (User-facing documentation)
│   ├── notes/
│   │   ├── sdk-architecture.md               (SDK strategy and architecture — SEE: SDK cross-reference)
│   │   ├── sdk-conformance-runner.md         (SDK conformance test runner)
│   │   ├── sdk-contract.md                   (SDK contract specification)
│   │   ├── sdk-invocation.md                 (SDK invocation patterns)
│   │   └── ...                               (Additional project notes)
│   ├── adr/                                  (Architecture Decision Records)
│   ├── operations/                           (Operational procedures)
│   ├── security/                             (Security documentation)
│   ├── conformance/                          (Conformance test specifications)
│   └── research-index.md                     (Research document index)
└── tests/
    └── fixtures/
        ├── vector/                           (clean LaTeX/Word/InDesign PDFs)
        ├── scanned/                          (physical scans; OCR path)
        ├── cjk/                              (Chinese, Japanese, Korean)
        ├── malformed/                        (truncated, corrupt xref, circular)
        ├── encrypted/                        (AES-128, AES-256, RC4)
        ├── forms/                            (AcroForm, XFA)
        ├── tagged/                           (PDF/UA, PDF/A-a)
        ├── encoding/                         (no-ToUnicode fonts; Levels 2–4 recovery)
        ├── perf/                             (≥100-page vector PDFs)
        ├── grep-corpus/                      (1000-PDF Phase 7.8 benchmark corpus)
        └── profiles/                         (per-profile fixture sets, Phase 7.10)
```

**SDK cross-reference:** The SDK strategy and architecture is documented in [`docs/notes/sdk-architecture.md`](docs/notes/sdk-architecture.md). Language SDK repositories are sibling repositories under the `jedarden/` organization; each SDK implements the contract specified in the SDK architecture document.

The layout is normative: phase-specific code MUST land in the file indicated for its phase. New top-level modules added in future revisions MUST be reflected here in the same plan revision that introduces them.

---

## Dependency Matrix

Feature flags control the binary footprint. The default build (`cargo build`) includes only the core extraction path. Heavy optional capabilities are behind named features.

**Feature flags:**
- `default` = `["cli", "decrypt", "markdown"]` — strips to core + CLI + encryption + Markdown output; no OCR, no HTTP, no Python
- `decrypt` — RC4 and AES-128/256 decryption (RustCrypto crates; part of the default feature set because encryption handling is core, not optional)
- `markdown` — Markdown output formatter (Phase 6.5); pure string formatting on top of Phase 4 blocks. No external crates. In default features because the cost is negligible and Markdown is a primary output format.
- `ocr` — adds Tesseract + Leptonica (system libraries required)
- `serve` — adds axum + tokio (HTTP server)
- `mcp` — adds the MCP server subcommand (Phase 6.7). Depends on `serve`; both transports share the HTTP infrastructure. No additional external crates (JSON-RPC framing is hand-written).
- `inspect` — adds the inspector web debug viewer subcommand (Phase 7.9). Depends on `serve`. Bundles a ~80 KB static HTML/CSS/JS frontend via `include_bytes!`. No new external crates.
- `cache` — adds the content-addressed extraction-result cache (Phase 6.9). Adds `zstd` (~50 KB). Implicitly enabled when `serve` is enabled (the serve mode is the primary cache consumer; users who want caching without HTTP can enable `cache` standalone).
- `receipts` — adds visual citation receipts (Phase 6.8). No new external crates (reuses `sha2` and `ttf-parser` from default).
- `remote` — adds the HTTP range-read source adapter (Phase 1.8). Adds `ureq` (~500 KB).
- `grep` — adds the `pdftract grep` folder-search subcommand (Phase 7.8). Adds `regex`, `walkdir`, `indicatif` (total ~600 KB).
- `profiles` — adds configurable document profiles (Phase 7.10). Adds `serde_yaml` (~200 KB). Requires `regex` (auto-enabled if not already pulled in by `grep`).
- `python` — adds PyO3 (maturin build)
- `full-render` — adds pdfium-render (large native binary; improves scanned-page rasterization)
- `full` = `["ocr", "serve", "mcp", "inspect", "python", "remote", "grep", "profiles", "cache", "receipts", "markdown"]` — the "everything except `full-render`" superset. Used for the `pdftract:full` Docker image and the GitHub Releases `pdftract-full` binaries.
- `wordlist-bloom` — replaces the default phf::Set English word list with a Bloom filter; enable if the binary-size CI check (`cargo bloat`) reports the word list exceeds 250 KB.

| Crate | Version | Feature | Purpose |
|---|---|---|---|
| `memmap2` | 0.9 | default | Memory-mapped file access |
| `flate2` | 1 | default | FlateDecode / zlib decompression |
| `lzw` | 0.10 | default | LZWDecode |
| `ttf-parser` | 0.21 | default | TrueType/OpenType glyph metrics and cmap lookup |
| `owned_ttf_parser` | 0.21 | default | Arc-safe wrapper for ttf-parser |
| `fontdue` | 0.9 | default | TrueType/OpenType glyph rasterization for shape-based Unicode recognition (Level 4). Estimated binary contribution ~60 KB. |
| `lru` | 0.12 | default | Object cache eviction |
| `rayon` | 1 | default | Page-level parallelism |
| `serde` | 1 | default | Serialization derive macros |
| `serde_json` | 1 | default | JSON output |
| `indexmap` | 2 | default | Ordered dictionaries (PDF dict key order matters for CMap parsing) |
| `unicode-normalization` | 0.1 | default | NFC normalization |
| `sha2` | 0.10 | default | SHA-256 hashing for font program fingerprinting (Level 3 Unicode recovery) |
| `encoding_rs` | 0.8 | default | CJK encoding decoding (Shift-JIS, GB18030, Big5, EUC-KR) |
| `phf` | 0.11 | default | Compile-time AGL hash map (zero runtime allocation) |
| `clap` | 4 | cli | CLI argument parsing |
| `thiserror` | 1 | default | Error type derivation |
| `log` | 0.4 | default | Logging facade |
| `env_logger` | 0.4 | default | Logging implementation (stderr, RUST_LOG env var) |
| `image` | 0.25 | ocr | Raster image decoding and DPI-scaled rendering (TIFF/CCITT support requires system libtiff; documented trade-off) |
| `tesseract` | 0.14 | ocr | Tesseract OCR FFI bindings |
| `leptonica-plumbing` | 0.4 | ocr | Leptonica image preprocessing (Sauvola, deskew) |
| `quick-xml` | 0.36 | default | XMP conformance detection (default build); HOCR parsing and XFA parsing (enabled when ocr/python features are active) |
| `pdfium-render` | 0.8 | full-render | High-fidelity rasterization via PDFium (large native binary — ~20 MB) |
| `pyo3` | 0.21 | python | Python bindings |
| `maturin` | build | python | PyO3 wheel packaging |
| `axum` | 0.7 | serve | HTTP serve mode |
| `tokio` | 1 | serve | Async runtime for axum |
| `tower-http` | 0.5 | serve | Request size limiting and tracing |
| `multer` | 3 | serve | Multipart form parsing |
| `bytes` | 1 | serve | Zero-copy byte sharing in HTTP path |
| `aes` | 0.8 | decrypt | AES-128 and AES-256 decryption (RustCrypto, ~50 KB) |
| `rc4` | 0.1 | decrypt | RC4 decryption (RustCrypto, ~10 KB) |
| `bloomfilter` | 0.2 | wordlist-bloom (optional) | An alternative to the default phf::Set word list. Enable with `--features wordlist-bloom` to replace the phf word list with a Bloom filter if the binary-size CI check fails. Not a default dep — it is a manual authoring decision. ~25 KB for 20k words at 0.1% false-positive rate |
| `unicode-bidi` | 0.3 | default | Unicode bidi character category lookup for RTL line detection |
| `strsim` | 0.11 | default | String similarity metrics (Levenshtein) for header/footer cross-page deduplication |
| `ureq` | 0.10 | remote | Synchronous HTTP client with rustls backend; supports `Range:` requests for Phase 1.8 partial PDF extraction. Chosen over `reqwest` for binary size (no async runtime, no tokio coupling). |
| `regex` | 1.10 | grep, profiles | Regex engine for `pdftract grep` and profile field/match patterns. Used for any feature that needs runtime regex compilation. |
| `walkdir` | 2 | grep | Recursive directory walking for `pdftract grep` |
| `indicatif` | 0.17 | grep | Terminal progress bars and ETA for folder-scale searches |
| `zstd` | 0.13 | cache | Compression for cached extraction results in Phase 6.9 (~3× compression on JSON output) |
| `serde_yaml` | 0.9 | profiles | YAML deserialization for user-authored document profile files (Phase 7.10) |

**Build dependencies (Cargo.toml `[build-dependencies]`):**

| Crate | Version | Purpose |
|---|---|---|
| `phf_codegen` | 0.11 | Generates compile-time phf maps (AGL, word list, font fingerprints, glyph shapes) from `build.rs` |
| `serde_json` | 1 | Parses `build/font-fingerprints.json` and `build/glyph-shapes.json` in `build.rs` |

**Removed vs. first draft:** `jpeg-decoder` dropped — DCTDecode is passthrough; SOI/EOI marker validation is a 4-byte check with no external dependency. `whichlang` dropped — language detection is not on the critical accuracy path; BCP-47 lang tags come from PDF `/Lang` attributes and StructTree `/Lang`, not inference.

---

## Glossary

Definitions of recurring terms. Each entry is the precise sense intended throughout this plan; conflicting interpretations from external sources are explicitly NOT in scope here. Each entry references the phase that introduces the term.

| Term | Definition |
|---|---|
| **anchor** | An HTML comment line emitted alongside a Markdown block carrying its `page`, `block`, `bbox`, and `kind` so the Markdown output can be deterministically mapped back to the source PDF coordinates. Introduced in Phase 6.5. |
| **AGL** | Adobe Glyph List. The ~4,400-entry static map from PostScript glyph names (e.g. `aacute`) to Unicode scalar values, applied as the Level 2 fallback when no `/ToUnicode` CMap is present. Introduced in Phase 2.2. |
| **bead** | A single rectangular region (bbox + page reference) within a PDF article thread. Beads chain via `/N` links to form a thread. Introduced in Phase 7.7. (Note: distinct from the `br`/beads CLI used for project task tracking — that meaning is project-management context and does not appear in pdftract output.) |
| **block** | A grouping of one or more lines representing a logical unit of content (paragraph, heading, list, table, caption, figure, code, header, footer, watermark, formula, quote). Introduced in Phase 4.4. |
| **BrokenVector** | A page that nominally contains vector text operators but produces text below the readability threshold (typically PDF/A with a degenerate or scrambled text layer over a scan). Routed to the assisted-OCR path in Phase 5.5. Introduced in Phase 5.1. |
| **codepoint** | A Unicode scalar value (`char` in Rust). Distinct from "glyph", which is a renderable shape; a single codepoint MAY be rendered by multiple glyphs (e.g. `fi` ligature) and a single glyph MAY decode to multiple codepoints. |
| **codespace** | A range of byte sequences declared valid by a CMap's `begincodespacerange`/`endcodespacerange`. Defines how the byte stream of a `Tj` operand is split into character codes. Introduced in Phase 2.3. |
| **confidence_source** | Enum tagging the provenance of a span's Unicode resolution: `native` (ToUnicode/AGL/fingerprint), `heuristic` (shape match, correction, or U+FFFD), or `ocr` (Tesseract). Introduced in Phase 4.1. |
| **content stream** | The byte stream of PDF drawing operators on a page, decoded via Phase 1.5 and executed by Phase 3. |
| **fingerprint** | The 256-bit `pdftract-v1:<hex>` Merkle-style hash identifying a PDF's semantic content independent of metadata churn. Introduced in Phase 1.7. |
| **form XObject** | A reusable PDF graphics object containing its own content stream and resource dictionary, invoked from a page via the `Do` operator. Introduced in Phase 3.3. |
| **frame** | One newline-delimited JSON object in NDJSON streaming output, tagged `frame: "header" \| "page" \| "footer"`. Introduced in Phase 6.2. |
| **Hybrid** | A page containing both vector text and scanned image regions (e.g. a scanned form with a vector header). Detected by Phase 5.1 grid analysis; output type `mixed`. |
| **kind** | The classification of a block — one of `heading`, `paragraph`, `list`, `table`, `caption`, `figure`, `code`, `header`, `footer`, `watermark`, `formula`, `quote`. Introduced in Phase 4.4. |
| **marked content sequence** | A `BMC`/`BDC` … `EMC` operator span in a content stream, optionally carrying an MCID and properties dict. Used to associate glyphs with structure-tree elements. Introduced in Phase 3.4. |
| **MCID** | Marked Content Identifier. A non-negative integer assigned via `BDC /Tag << /MCID N >>` linking glyphs to their owning structure element (Phase 7.1). |
| **mojibake** | Text corrupted by an encoding mismatch — typically Latin-1 bytes interpreted as UTF-8, producing sequences like `Ã©` for `é`. Detected and repaired in Phase 4.7. |
| **page_index** | Zero-based integer, canonical for all programmatic references (errors, NDJSON ordering, cache keys, fingerprint). Introduced in Phase 6.1. |
| **page_number** | One-based integer, equal to `page_index + 1`. Emitted alongside `page_index` as a convenience for human display only. Introduced in Phase 6.1. |
| **profile** | A user-editable YAML document declaring matching predicates and extraction tuning for a document type (invoice, receipt, contract, etc.). Drives Phase 5.6 classification and Phase 7.10 field extraction. |
| **receipt** | A portable proof-of-provenance object binding extracted text to a PDF region. `lite` mode carries fingerprint + bbox + content hash; `svg` mode adds an inline self-contained glyph rendering. Introduced in Phase 6.8. |
| **span** | A run of contiguous glyphs sharing the same font, size, color, rendering mode, and word-boundary state, carrying a single bbox. The smallest text unit with a single bbox. Introduced in Phase 4.1. |
| **structure tree** | The `/StructTreeRoot` tree of logical elements (paragraphs, headings, table cells) in a tagged PDF, used as the authoritative reading order when present. Introduced in Phase 7.1. |
| **thread** | A PDF article thread — an ordered chain of beads forming a logical reading flow across pages and columns. Introduced in Phase 7.7. |
| **ToUnicode** | A CMap stream in a font's `/ToUnicode` entry mapping character codes to Unicode scalar values. The Level 1 (highest-confidence) source for glyph-to-codepoint resolution. Introduced in Phase 2.2. |

---

## Non-Goals

pdftract is deliberately scoped. Features outside this scope are NOT in the plan, NOT in v1.0.0, and NOT subject to feature requests until the v1.1+ planning horizon. Each non-goal is paired with the reason it is out of scope.

### What pdftract is NOT

| Non-goal | Why out of scope |
|---|---|
| **PDF authoring or writing** | pdftract is a read-only extractor. Building a writer requires a complete object-emit layer, encryption-on-write, font-embedding pipeline, and signature-on-write infrastructure — each comparable in size to the read path. Conflating read and write doubles the binary footprint and the attack surface. Use `lopdf`, `pdfium-render`, or `printpdf` for authoring. |
| **Full PDF rendering / printing** | High-fidelity page rendering (correct anti-aliased glyph outlines, transparency blends, shading patterns, soft masks, halftone, color management) is a multi-megabyte native dependency (PDFium ~20 MB, MuPDF ~10 MB). pdftract's optional `full-render` feature embeds PDFium for OCR rasterization only; it is NOT a rendering API. |
| **Cryptographic signature validation** | Validating PKCS#7/CAdES signatures requires the full certificate chain, OCSP/CRL retrieval, and trust-store management — none of which fit the < 14 MB binary or the no-network-by-default posture. Phase 7.3 extracts signature *metadata* only and reports `validation_status: "not_checked"`. Users who need validation should pair pdftract's metadata output with `openssl smime` or a dedicated PKI library. |
| **Translation of extracted text** | Machine translation is a model-shipping decision (gigabytes of weights or external API dependency) orthogonal to extraction. pdftract emits Unicode text with detected `lang` tags; downstream tools (LibreTranslate, DeepL, Argos) consume those. |
| **Summarization of extracted text** | Summarization is an LLM concern. pdftract's MCP server (Phase 6.7) is the integration point: an agent calls `extract` to get text, then summarises in the model's context. Embedding a summariser in pdftract would couple the binary to a specific model family. |
| **OCR engine training** | Tesseract training is a distinct workflow with its own tooling (`tesstrain`). pdftract bundles Tesseract as a runtime dependency; it does not retrain or fine-tune. |
| **Non-Latin handwritten OCR** | Tesseract has poor accuracy on handwritten text in any script. Handwritten OCR requires specialised models (e.g. CRNN-based engines). Out of scope until a viable embeddable engine emerges; for v1, pdftract emits the Tesseract output as-is with whatever confidence Tesseract reports. |
| **Filling out PDF forms** | Phase 7.4 extracts AcroForm and XFA field values for reading. Writing back (filling fields, generating an output PDF with new values) requires the authoring pipeline that is itself a non-goal — see "PDF authoring or writing" above. |
| **Watermark removal** | pdftract DETECTS watermark blocks (Phase 7) and excludes them from `--text` and Markdown output by default, but does NOT modify the source PDF to physically remove them. Modification requires the authoring pipeline. |
| **Password cracking on encrypted PDFs** | Bruteforce attacks on RC4/AES-encrypted PDFs are out of scope for ethical and scope reasons. pdftract attempts the empty password and any user-supplied password from `--password` once; failure emits `ENCRYPTION_UNSUPPORTED` and the process exits 3. Users who need password recovery should use dedicated tools (`pdfcrack`, `john`). |

### Scope Lock Doctrine

The scope above is fixed for the v1.0.0 release. The following rules govern any scope change:

1. **Scope cannot expand mid-flight.** Once a phase enters implementation (a PR opens against its module), no new requirements may be added to that phase without first updating this plan. Concretely: PR reviews block on "did the plan change to authorise this?" — silent feature creep is rejected at code review.
2. **Plan amendment precedes implementation.** Any new feature, even one motivated by user feedback during a phase, lands in this `plan.md` first (via a new Revision History entry, scoped to a future version), and only then in code. The single source of truth for v1.0.0 scope is the latest revision of this file.
3. **The 14 pre-flight categories are the only pre-Phase-1 deltas.** The current plan-review report identified 14 missing/partial pattern categories. Sections drafted to address them are the ONLY scope changes that land before Phase 1 begins. New feature ideas that surface during the pre-flight review window are tagged "v1.1+" and recorded in Open Questions, not in any phase's requirements.
4. **Post-Phase-1 feature requests are deferred.** Once Phase 1 PRs land, all new feature ideas — however compelling — are deferred to v1.1+. The release branch (v1.0.0) accepts bug fixes and clarifications only; new features go to `main` for the next minor release.
5. **Section renumbering is forbidden mid-release.** Stable phase numbers (1.1, 1.2, … 7.10) are referenced by external documents and downstream issues. Renumbering invalidates those references; only additive insertion (e.g. a new 7.11) is permitted.

Scope changes that violate any of these rules are recorded as a process failure in the project notes and rolled back.

---

## Architecture Decision Records

The following ADRs capture the load-bearing design decisions that are most likely to attract future "why didn't you use X?" challenges. Each ADR is immutable once accepted; reversing a decision requires a new ADR superseding it (e.g. `ADR-001a Supersedes ADR-001`). The "Invalidation trigger" field is the explicit, observable condition under which the decision MUST be reopened.

### ADR-001: Use `ureq` (not `reqwest`) for the remote source adapter

- **Decision:** Phase 1.8's `HttpRangeSource` uses `ureq` with the `rustls` backend.
- **Context:** The `remote` feature must download partial PDFs via HTTP Range requests. Two mainstream Rust HTTP clients exist: `reqwest` (async, tokio-coupled, broad TLS-backend support) and `ureq` (synchronous, no async runtime, rustls-only).
- **Rationale:** Binary size and dependency surface dominate the decision. `reqwest` pulls in tokio plus a TLS abstraction layer for ~3–4 MB of binary contribution; `ureq` is ~500 KB and has no async runtime. Phase 1.8 lives behind a `remote` feature flag in a binary whose total size budget is 14 MB; a 3 MB allocation to HTTP transport is disproportionate. The synchronous API integrates naturally with rayon (which is already the parallelism primitive) and avoids the rayon ↔ tokio bridging complexity that the Phase 6.4 serve mode requires via `spawn_blocking`.
- **Consequences:** `pdftract grep https://...` and `pdftract extract https://...` run synchronously, one request per page-fetch. This is acceptable because per-page latency is dominated by extraction CPU, not HTTP round-trips. The `serve` mode (Phase 6.4) still uses `axum`/`tokio` for incoming requests; the bridge to `ureq` for outgoing fetches goes via `spawn_blocking`.
- **Rejected alternative:** `reqwest`. Rejected on binary-size grounds.
- **Invalidation trigger:** If pdftract begins making concurrent outgoing fetches to multiple distinct hosts within a single extraction (currently NOT planned), the lack of an async client becomes a throughput bottleneck and `reqwest` becomes worth reconsidering. Concretely: if a future feature requires fetching > 4 hosts concurrently for one extraction, reopen.

### ADR-002: Use `phf::Set` (not Bloom filter) for the English word list

- **Decision:** Phase 4.7's English wordlist ships as a compile-time `phf::Set<&'static str>` containing ~20,000 entries.
- **Context:** The readability scorer needs O(1) dictionary-word lookup. Two options: a perfect-hash `phf::Set` (exact membership, ~200 KB compile-time data) or a Bloom filter (probabilistic membership with tunable false-positive rate, ~25 KB at 0.1% FPR for 20k words).
- **Rationale:** Accuracy is the top-priority Primary Objective. A Bloom filter at 0.1% FPR will spuriously raise the dictionary-coverage signal for ~0.1% of non-word inputs — a small but real accuracy hit on a signal weighted 30% in the composite. The 175 KB delta is within the 4 MB default-feature budget (the wordlist consumes ~5% of it). Exact lookup also makes the signal trivially debuggable; Bloom-filter false positives are non-reproducible noise.
- **Consequences:** ~200 KB of compiled-in static data. CI verifies the actual contribution via `cargo bloat --release --crates | grep pdftract_wordlist ≤ 250 KB`.
- **Rejected alternative:** Bloom filter via the `bloomfilter` crate. Retained as an escape hatch under `--features wordlist-bloom` if the CI bloat check ever fails.
- **Invalidation trigger:** If the bloat check exceeds 250 KB on a future build (e.g. wordlist expanded for multilingual support), switch to the Bloom-filter path under the `wordlist-bloom` feature.

### ADR-003: Make `pdfium-render` opt-in via `full-render`, not default

- **Decision:** PDFium-based page rendering is gated behind `--features full-render`. The default build uses direct image XObject compositing in Phase 5.2.
- **Context:** Some scanned PDFs render correctly only via a full PDF rasteriser — those with overlapping image XObjects, soft masks, image masks, or JBIG2/JPX content. PDFium is the highest-fidelity option, but it's a ~20 MB native binary.
- **Rationale:** > 90% of scanned PDFs use a single full-page image per page and composite correctly without PDFium. Defaulting to PDFium would push the `pdftract:ocr` Docker image from ~120 MB to ~140 MB — a 17% size increase to handle a minority case. The 10% of users whose PDFs need PDFium can opt in via the `pdftract:full` image tag.
- **Consequences:** Default builds emit `OCR_JBIG2_UNSUPPORTED`, `OCR_JPX_UNSUPPORTED`, and `OCR_CCITT_UNSUPPORTED` diagnostics on the rare PDFs that need those decoders. Users see a clear "enable `--features full-render` to handle this" message.
- **Rejected alternative:** Make PDFium the default. Rejected on binary-size grounds.
- **Invalidation trigger:** If the < 90% direct-compositing success rate drops below 75% on the regression corpus (i.e. > 25% of scanned PDFs now need full-render), reopen.

### ADR-004: Bridge `rayon` (page parallelism) and `tokio` (HTTP) via `spawn_blocking`

- **Decision:** Phase 6.4's `serve` mode uses `axum`/`tokio` for the HTTP layer and calls into the synchronous extraction pipeline via `tokio::task::spawn_blocking`. Per-document page parallelism inside extraction is `rayon`, which runs on its own pool. No `tokio::spawn` is used for page-level work.
- **Context:** Two parallelism primitives coexist: `rayon` for embarrassingly-parallel page CPU work (the right tool for that), and `tokio` for async HTTP (the right tool for accepting many concurrent client requests). The bridge between them must not deadlock or starve.
- **Rationale:** `spawn_blocking` is the canonical bridge documented by both projects. It runs the synchronous extraction on tokio's blocking thread pool (separate from the async executor), inside which rayon's own thread pool runs page-level parallelism. The async executor is never blocked; the blocking pool sizes scale with concurrent requests; rayon scales within each request.
- **Consequences:** Two thread pools exist at runtime in `serve` mode. The total OS thread count is bounded by `tokio_blocking_threads + rayon_threads`, which on a typical 8-core host is ~16 threads — well within normal limits. The extraction call site is the same in CLI and serve mode (a synchronous `extract(...)`) — there are no parallel async/sync code paths to maintain.
- **Rejected alternative 1:** Rewrite extraction as async (`tokio::spawn` per page). Rejected: extraction is CPU-bound, not I/O-bound, and would gain nothing from async while losing rayon's work-stealing.
- **Rejected alternative 2:** Use `rayon` exclusively (no tokio; `axum` replaced with a synchronous HTTP server). Rejected: `axum`'s ecosystem (middleware, tracing, multipart) is the standard for production HTTP services.
- **Invalidation trigger:** If `spawn_blocking` overhead is measurably ≥ 5% of total per-request time in benchmarks, reopen and consider a custom dispatch.

### ADR-005: Use a filesystem-backed cache (no SQLite, sled, or RocksDB)

- **Decision:** Phase 6.9's cache stores entries as individual `.json.zst` files in a sharded directory layout. No embedded database is used.
- **Context:** Cache implementations span a spectrum: plain files (zero deps, OS-managed) → SQLite (~1 MB native lib) → sled (~2 MB pure Rust) → RocksDB (~5 MB native lib). Each adds capability (transactions, queries) but also size and operational complexity.
- **Rationale:** The cache's access pattern is single-key get/put with LRU eviction. Filesystems do this natively (the OS page cache backs reads; rename-on-write provides atomicity). SQLite/sled/RocksDB add transaction guarantees pdftract doesn't need (multiple writers tolerate duplicated work per ADR-005's eviction policy) at substantial binary cost. Operators can `rm -rf` the cache dir to clear it — no `cache clear` command is strictly required (one is provided for convenience).
- **Consequences:** The `cache` feature adds only `zstd` (~50 KB) to the binary. Cache directories can be inspected with standard `ls`, `du`, `find` tools. Backup/restore is `tar`. Cache corruption is bounded to individual files (a corrupt entry is treated as a miss and deleted, per Phase 6.9's critical tests).
- **Rejected alternative:** SQLite-backed cache (sled or RocksDB even less competitive on binary size). Rejected on binary size and operational simplicity.
- **Invalidation trigger:** If cache write throughput becomes the bottleneck under > 10,000 req/s sustained load (currently a non-goal), an LSM-tree store like sled becomes worth reconsidering.

### ADR-006: MCP stdio and HTTP transports are mutually exclusive per process

- **Decision:** A single `pdftract mcp` invocation listens on exactly one transport — stdio OR HTTP, never both. Operators who need both run two processes.
- **Context:** The MCP spec defines two transports (stdio over the host process's stdin/stdout, HTTP+SSE over a network socket). A single process could theoretically serve both.
- **Rationale:** Stdio mode treats stdout as the JSON-RPC sink — nothing else may write to it (logs go to stderr). HTTP mode treats stdout as a log channel — JSON-RPC goes over the socket. The two contracts cannot coexist on the same stdout file descriptor without one transport's framing leaking into the other's payload. Forbidding the combination at the CLI flag layer makes the contract unambiguous.
- **Consequences:** A user wanting a single binary to serve a local Claude Desktop AND a remote agent runs `pdftract mcp --stdio` and `pdftract mcp --bind 0.0.0.0:8080` in two processes. This is a normal Unix idiom; the operational overhead is negligible.
- **Rejected alternative:** Dual-transport mode with logs routed to a file in stdio mode and to stderr in HTTP mode. Rejected: the dual contract is a footgun (a single misconfigured log statement leaks the wrong sink), and the binary-size cost of the runtime branching is non-trivial.
- **Invalidation trigger:** If MCP-spec evolution standardises a multi-transport mode with a defined isolation boundary, reopen.

### ADR-007: Use YAML (not TOML or JSON) for profile templates

- **Decision:** Phase 7.10 document profiles are authored in YAML.
- **Context:** Profile files are user-authored configuration with rich nested structure (combinator trees, per-field localisation hints, extraction tuning). Three configuration formats are mainstream in the Rust ecosystem: YAML (`serde_yaml`), TOML (`toml`), JSON (built into `serde_json`).
- **Rationale:** YAML's combinator nesting is the cleanest (the example invoice profile reads as English: `all:`, `any:`, `none:`); TOML's flat-table-with-nested-tables idiom is awkward for the `any`/`all`/`none` combinators; JSON requires quoting every key and rejects comments (essential for user-authored config). Operators are likely to copy-paste-edit profile YAMLs, and YAML's comment support is critical for documentation in place.
- **Consequences:** The `profiles` feature adds `serde_yaml` (~200 KB). YAML's footguns (significant whitespace, type coercion of `yes`/`no`/`on`/`off`) are documented in `docs/research/profile-authoring.md` and the `pdftract profiles validate` command catches the common mistakes at validation time.
- **Rejected alternative 1:** TOML. Rejected for the combinator-nesting reason above.
- **Rejected alternative 2:** JSON. Rejected for the no-comments reason.
- **Invalidation trigger:** If a YAML parser security advisory (RustSec) affects `serde_yaml` and a fix is not forthcoming within 30 days, switch to TOML and rewrite the example profiles.

### ADR-008: Structural fingerprint excludes `/Producer`, `/CreationDate`, XMP metadata, `/ID`

- **Decision:** The Phase 1.7 fingerprint is computed over decoded content streams, resolved resource dicts, page geometry, structure tree, and catalog feature flags. It explicitly EXCLUDES `/Producer`, `/Creator`, `/CreationDate`, `/ModDate`, `/Author`, `/Title`, `/Subject`, `/Keywords`, the XMP `/Metadata` stream, the `/ID` trailer array, xref byte layout, and object number assignment.
- **Context:** The fingerprint is the cache key (Phase 6.9) and the receipt binding identity (Phase 6.8). Its stability across producer-tool re-saves is the load-bearing property. Two extreme designs are possible: hash the raw file bytes (trivial; immediately breaks on any save) or hash only the rendered output (perfect stability; prohibitively expensive).
- **Rationale:** The chosen field set is the smallest set that distinguishes content edits from cosmetic re-saves. `/Producer`, `/CreationDate`, etc. are tool-stamps that change on every save in Acrobat, pdftk, QPDF — including saves that touch no content. The XMP `/Metadata` stream similarly carries producer-side history. The `/ID` array is per-save random. xref layout and object numbering are byte-layout artefacts. Excluding all of these means a content-identical re-save produces an identical fingerprint, which is the requirement.
- **Consequences:** Acceptance criteria: same PDF re-saved by Acrobat/pdftk/QPDF → identical fingerprint (validated by Phase 1.7 critical tests). Cache hits work correctly across re-saves. Receipts survive re-saves.
- **Rejected alternative:** Include metadata and `/ID` in the fingerprint. Rejected: every re-save would invalidate caches and receipts, defeating both features.
- **Invalidation trigger:** If a real-world workflow surfaces where two semantically distinct PDFs collide on the fingerprint (false positive), reopen to add a discriminating field. If a content-only edit fails to change the fingerprint (false negative), reopen to fix the hash inputs. Both cases require a new fingerprint algorithm version (`pdftract-v2:`) — the version prefix exists for this reason.

### ADR-009: Argo Workflows on `iad-ci` is the only CI runner

- **Decision:** All CI — tests, lints, benchmarks, cross-compiles, fuzz runs, regression-corpus checks, and the entire release pipeline — runs as Argo WorkflowTemplates on the `iad-ci` Rackspace Spot cluster. GitHub Actions, Travis, CircleCI, GitLab CI, and any other hosted CI are EXPLICITLY FORBIDDEN. Secrets (PyPI token, crates.io token, GHCR PAT, NuGet/Maven/RubyGems/npm credentials, cosign keyless OIDC config) live in OpenBao and reach workflows via ESO-synced Kubernetes Secrets.
- **Context:** The project ecosystem runs on a private Kubernetes-native CI fleet documented in the parent `CLAUDE.md`. The fleet is already wired for cross-cluster credential management, image registries, Cloudflare Pages deploys, and Tailscale-only access. Adding GitHub Actions would fork the CI configuration across two systems and require duplicating secret management, with no operational gain.
- **Rationale:** Argo on `iad-ci` already produces the binaries, images, and PyPI wheels for several sibling projects (`kalshi-tape`, `kalshi-weather`, `news-trader`, `botburrow-agents`). Reusing the same patterns reduces operational surface, keeps credentials in one vault, and reuses existing observability. The cost (forks cannot trigger CI from a button click) is acceptable: a maintainer re-runs `pdftract-ci` against a PR branch in seconds.
- **Consequences:** macOS and Windows binaries are *built* via `cross` on Linux but never *executed* in CI — runtime tests for those platforms become a manual quarterly smoke test (tracked as KU-12). PyPI Trusted Publishing (OIDC) does not apply (it's GitHub-Actions-only); the PyPI token is stored in OpenBao instead. External contributors cannot self-serve CI; the contributor workflow (see Release Engineering and Distribution) documents this explicitly.
- **Rejected alternative:** GitHub Actions as the public-facing CI with Argo as a backend mirror. Rejected because: (a) parent `CLAUDE.md` forbids GitHub Actions across all repos, (b) two CI systems = two failure modes = doubled operational load, (c) credential surface area doubles.
- **Invalidation trigger:** If `iad-ci` is decommissioned, OR if the project moves out of the `ardenone-cluster` operational sphere, OR if upstream MCP/PyPI/crates.io introduce CI requirements that Argo on private infrastructure cannot satisfy. None are currently anticipated.

### ADR-010: 2026-07-20 — Ship a `pdftract-wasm` sibling crate + client-side browser demo ahead of the first registry release

- **Status:** Accepted
- **Decision:** Add a new workspace member, `crates/pdftract-wasm`, that compiles a deliberately narrowed slice of the extraction pipeline — parsing, font-encoding recovery, and reading-order segmentation for **vector-text pages only** — to `wasm32-unknown-unknown` via `wasm-bindgen`. Pair it with a small static HTML/JS page (built and deployed through the existing, already-created `pdftract-docs-build` Argo WorkflowTemplate, the same Cloudflare Pages pattern used for `jedarden.com` per this workspace's CLAUDE.md) where a visitor drags in a PDF and sees pdftract's structured JSON output — entirely client-side, no upload, no server round-trip. OCR/scanned/mixed pages are explicitly out of scope for the WASM build and the demo shows a clear "vector-only in-browser; use the CLI or `serve` for scanned documents" message rather than silently degrading.
- **Context:** As of this review, pdftract has no live surface anywhere. There is no git tag (workspace version is still `0.1.0`), and every install channel in `README.md` (crates.io, PyPI, Docker Hub, Homebrew) is marked "🚧 coming soon" — confirmed still true (`bf-10qd4` was closed by amending the README to say so, not by publishing). The README's one link presented as *already live* — "**User guide:** [pdftract.com](https://pdftract.com)" — does not resolve at all (`getaddrinfo ENOTFOUND pdftract.com`, checked live during this review). Meanwhile the mdbook user docs are already built and sitting in the repo at `docs/user-docs/build/user-docs/`, and a `pdftract-docs-build` WorkflowTemplate (`.ci/argo-workflows/pdftract-docs-build.yaml`, added under `bf-5o8cg`) already exists to build+deploy them — it has just never been pointed at a live destination. Getting the full 10-registry release cascade (`pdftract-release-cascade`) to actually run is tracked separately and is a long pole; it should not have to gate a visitor being able to experience pdftract's actual differentiator — correct multi-column reading order and font-encoding recovery on documents that break other extractors (see the comparison table in `README.md`) — today, with nothing installed. This ADR exercises a path the plan already anticipated: Risk Register row **R11** states "WASM build later requested despite explicit Non-Goal ... can be revisited as a v2.0 sibling crate (`pdftract-wasm`) without modifying `pdftract-core`" — this decision is that reconsideration, made explicit and scheduled.
- **Alternatives Considered:**
  - **Do nothing until the full registry release cascade completes.** Rejected: leaves a ~3,900-line plan and 1,100+ closed implementation beads with zero live surface indefinitely; today, anyone who lands on the GitHub repo sees dead "coming-soon" badges and a domain that doesn't resolve.
  - **Server-side hosted demo** (upload a PDF to a `pdftract serve` instance running on `iad-ci` or similar). Rejected: requires operating a public-facing upload endpoint with rate limiting and abuse controls before v1.0 hardening is complete, and the README's own target documents — legal filings, financial reports, medical/scientific PDFs — are exactly the class of document a privacy-conscious visitor will not upload to a stranger's server. Client-side WASM sidesteps this entirely and is itself a trust signal worth stating on the demo page.
  - **Compile the full pipeline, including OCR, to WASM** (e.g. a WASM-targeted Tesseract build). Rejected: no maintained Tesseract-to-WASM path meets pdftract's own binary-weight discipline (the native `ocr` feature already fights to stay under its 12 MB budget per the Weight Targets table); scoping the WASM crate to vector-text-only is consistent with ADR-003's existing precedent of gating heavier capability behind opt-in tiers rather than shipping one monolithic build.
  - **Make `pdftract-core` itself WASM-portable** (`#[cfg(target_arch = "wasm32")]` branching inside the core) instead of a sibling crate. Rejected: this is precisely the alternative R11 already ruled out, to keep `#[cfg]` branching out of the accuracy-critical core; a sibling crate leaves `pdftract-core`'s dependency and feature-flag surface completely untouched.
- **Consequences:** `crates/pdftract-wasm` and a small static demo site become new release artifacts needing their own bundle-size budget (analogous to the existing Weight Targets and to the `pdftract-inspector-ui` 80 KB budget under R12), not yet defined here. The `pdftract-docs-build` pipeline needs to actually be wired to a live Cloudflare Pages project + DNS record, which as a side effect finally resolves the dead `pdftract.com` reference in `README.md`. This is purely additive — no change to `pdftract-core`'s public API, feature flags, or existing Weight/Speed/Accuracy targets. The first concrete step is a small spike (tracked as a bead, not this ADR) that proves `pdftract-core`'s vector-extraction path actually builds under `wasm32-unknown-unknown` before any demo UI is built.
- **Invalidation trigger:** If the wasm32 build spike shows that `pdftract-core`'s vector path pulls in a dependency that cannot target `wasm32-unknown-unknown` at all — not just OCR, but e.g. the `memmap2`-based zero-copy file I/O named in "Key architectural decisions," which has no `wasm32` story — reopen and re-scope to a smaller demo (single-page extraction against an in-memory `Uint8Array`, or a curated set of fixtures pre-rendered server-side) rather than a general-purpose client-side extractor.

---

## Open Questions

Questions that the current plan does not yet resolve. Each question is tagged with the phase by which it must be resolved; unresolved questions block that phase's PR merge. Questions tagged "v1.1+" are explicitly deferred and do NOT block v1.0.0.

| ID | Question | Resolve before | Owner / forum |
|---|---|---|---|
| OQ-01 | When does the 500-PDF private regression corpus become available, and what is its licensing for CI use? | Phase 0 sign-off | Project lead; recorded in `docs/notes/corpus-licensing.md` |
| OQ-02 | Who owns the font-fingerprint database curation pipeline (`build/font-fingerprints.json`) — is it a maintainer task, a community contribution, or an automated harvest from Google Fonts / Adobe? | Phase 2.2 implementation | Maintainer; documented in `docs/research/font-fingerprinting.md` |
| OQ-03 | What is the Tesseract version pinning policy — pin to a specific 5.x patch release, or follow latest stable? Pinning gives reproducibility; following stable gets bug fixes faster. | Phase 5.4 implementation | CI maintainer; recorded in `Dockerfile` comment |
| OQ-04 | How are OCR language packs distributed? Bundled in the Docker image (size cost), downloaded on first use (network dependency), or required as an out-of-band install? | **RESOLVED** 2026-05-23 by bead pdftract-32x4 | Bundled in Docker images with tiered strategy (`:ocr` ~150 MB, `:full` ~600 MB). Documented in `docs/notes/ocr-language-packs.md` |
| OQ-05 | What is the realistic coverage gap of the 5,000-entry glyph-shape DB on real-world subsetted fonts? Is 70% Latin-only coverage acceptable for v1.0.0, or must Cyrillic/Greek hit the same bar? | Phase 2.5 sign-off | Accuracy lead; benchmarked against `tests/fixtures/encoding/` |
| OQ-06 | Does the Phase 7.10 profile field-extraction DSL need user-defined parsers (custom JavaScript / Lua / WASM hooks)? Built-in `decimal`/`date`/`int`/`bool` may be insufficient for niche document types. | v1.1+ | Deferred — solicit user feedback after v1.0.0 |
| OQ-07 | How is the MCP server discovered by Claude Desktop / Cursor — manual config edit, a "pdftract setup-mcp" subcommand that writes the config, or both? Config file locations differ across OSes. | Phase 6.7 sign-off | MCP integration lead; documented in `docs/integrations/mcp-clients.md` |
| OQ-08 | Should a `pdftract serve` Docker image be published as a SaaS-ready turnkey container with TLS termination, request logging, and rate limiting baked in? Currently `pdftract serve` is "deploy behind a proxy". | v1.1+ | Deferred — assess after v1.0.0 deployment patterns |
| OQ-09 | Does the cache need a cross-process advisory lock to prevent the rare two-writer race? Currently last-write-wins is tolerated (per ADR-005). | Phase 6.9 sign-off (or defer) | Cache lead; benchmarked under contention |
| OQ-10 | What is the v1.0.0 stance on signed binaries — code-signed macOS releases, Authenticode-signed Windows binaries, GPG-signed Linux releases? Each adds CI complexity. | Phase 0 sign-off (decide what ships at v1.0.0) | Release lead; documented in `docs/notes/release-signing.md` |

The list is non-exhaustive; any concern surfaced during phase implementation that cannot be resolved within the phase is appended to this table.

---

## Proof Obligations Ledger

Every quantitative claim in this plan is a proof obligation. The table below lists the load-bearing claims, what must be true for each to hold, the observable signal that would invalidate the claim, and the planned fallback. A claim that fails its proof in CI blocks the milestone release until either the claim is met, the plan is revised, or the fallback is engaged.

| Claim | What Must Be True | Invalidation Signal | Fallback |
|---|---|---|---|
| pdftract is ≥ 10× faster than `pdfminer.six` on vector PDFs (Primary Objectives) | The default-feature binary completes 100-page vector extraction in < 3 s on 4-core CI; `pdfminer.six` on the same fixture takes ≥ 30 s | Tier 4 benchmark suite reports a ratio < 10× | Profile the slowest fixture, optimise the regressing path; if optimisation cannot close the gap, downgrade the claim to "≥ 5×" with a Revision History entry and a public note. |
| pdftract is ≥ 5× faster than `pypdf` on vector PDFs (Primary Objectives) | Same as above, against `pypdf==4.2.0` | Tier 4 benchmark suite reports a ratio < 5× | Same fallback plan as above. |
| Default binary < 4 MB stripped (Weight Targets) | `cargo build --release --features default && strip` produces a binary ≤ 4 MB on `x86_64-unknown-linux-musl` | CI bloat check reports > 4 MB | First-line: identify the largest crate via `cargo bloat`; consider migrating wordlist to Bloom filter (per ADR-002 escape hatch); consider gating `markdown` behind a feature. If still over budget, raise the limit with a documented justification in a new Revision History entry. |
| Glyph shape DB (~5,000 entries) covers common Latin/Greek/Cyrillic at 0.7 confidence (Phase 2.5) | On the `tests/fixtures/encoding/` corpus, ≥ 90% of glyphs in Latin/Greek/Cyrillic scripts that lack ToUnicode/AGL resolution are recovered to the correct Unicode by Phase 2.5 with confidence ≥ 0.7 | Encoding-corpus integration test reports < 90% Level-4 recovery rate | Expand the DB by re-running the offline hash pipeline on additional open-source fonts; if coverage still falls short, downgrade the Primary Objectives "Unicode recovery rate > 90%" claim to a more conservative value in a Revision History entry. |
| Rule-based document classifier achieves ≥ 90% accuracy on a 200-doc corpus (Phase 5.6) | The Phase 5.6 critical-tests fixture corpus (50 invoices, 50 papers, 50 contracts, 50 misc) produces ≥ 180 correct classifications | Phase 5.6 acceptance test fails | Tighten the matching predicates of the underperforming profile; expand its built-in `text_contains` / `heading_matches` lists. If 90% remains unreachable, deferr the document-type metadata to a non-CI-gated "best effort" status in a Revision History entry. |
| `ureq` contributes < 500 KB to binary size (Dependency Matrix, ADR-001) | `cargo bloat --release --features remote --crates` shows `ureq` and its transitive deps contributing < 500 KB to the stripped binary | Bloat check exceeds 500 KB | Reopen ADR-001 if the delta consistently exceeds 1 MB. Investigate disabling `ureq` features (e.g. native-tls) to shed transitive weight. |
| Tesseract WER < 3% on clean 300-DPI scans (Primary Objectives) | The `tests/fixtures/scanned/` corpus produces a measured word error rate < 3% on extractions using Tesseract 5.x with default language pack | Phase 5.4 integration test reports WER ≥ 3% | First-line: tune the Phase 5.3 preprocessing pipeline (deskew threshold, Sauvola window). If still failing, restrict the claim to specific document subtypes in a Revision History entry. |
| MCP stdio + HTTP mode mutual exclusion suffices for all known deployment patterns (ADR-006) | No reported MCP deployment requires a single process to serve both transports concurrently | A user-reported deployment surfaces that genuinely cannot be solved by running two processes | Reopen ADR-006 and design a dual-transport mode with explicit log-channel routing. Will likely require an `--mcp-log-file` flag and refactoring of all logging call sites. |
| Multi-output emission completes within 1.1× single-format time (Phase 6.6) | Producing JSON+Markdown+text concurrently from one extraction takes ≤ 1.1× the time of producing JSON only | Phase 6.6 acceptance test fails | Identify the slowest sink; defer its `close` work to a background thread (rayon `spawn_blocking` for sinks would suffice). If the gap remains > 10%, document the realistic ratio in the acceptance criterion. |
| Cache-hit latency < 20 ms p99 for a 100-page PDF (Phase 6.9) | Cache reads complete in < 20 ms at the 99th percentile on commodity SSD | Phase 6.9 acceptance test fails | Profile the read path (decompression, JSON parse); consider partial-result caching (return header frame immediately, hydrate pages on demand). |
| Folder grep throughput ≥ 50 MB/s on 1000-PDF corpus, 4-core CI (Phase 7.8) | Searching "the" across `tests/fixtures/grep-corpus/` completes at ≥ 50 MB/s aggregate input throughput | `pdftract-grep-1000` benchmark target reports < 50 MB/s | Tune rayon thread count for the workload; profile per-file overhead (mmap setup, parser init); consider a pre-warmed extraction pool. |

Failure of any claim is a process trigger: the responsible phase owner files an issue, the failure is logged in `benches/results/<commit-sha>.json` with the deviation, and a Revision History entry is added if the claim is permanently downgraded.

---

## Risk Register

The risks below are the named threats to project delivery. Each carries a likelihood, an impact, and a mitigation plan whose status is tracked against the phase that owns the risk. A risk's promotion from `Open` to `Mitigated` requires the named mitigation to be observably in place; closure (`Closed`) requires that the conditions for re-emergence are documented.

| R# | Risk | Likelihood (H/M/L) | Impact (H/M/L) | Mitigation | Owner |
|---|---|---|---|---|---|
| R1 | 10× pdfminer.six perf claim missed at Phase 4 exit | M | H | Tier 4 benchmark gate enforced from Phase 3 onward; Phase 4 exit blocks if missed; Plan B: re-frame claim against `pypdf` (5× target) if `pdfminer.six` materially improves before v1.0 | Perf lead (Phase 4 owner) |
| R2 | < 4 MB default-binary budget blown by font-fingerprint DB or wordlist | M | H | `cargo bloat` check in CI on every PR; ADR-002 escape hatch (wordlist-bloom) ready behind a feature flag; Plan B: `markdown` moves behind a feature if needed | Weight lead (Phase 2 + Phase 0 owners) |
| R3 | Tesseract WER > 3% on clean 300-DPI scans | M | H | Pre-Phase-5 spike to verify on `tests/fixtures/scanned/`; Phase 5.3 preprocessing tuning before locking the target; Plan B: revise target to 5% with a documented methodology footnote in Revision History | Accuracy lead (Phase 5 owner) |
| R4 | `pdfium-render` binary blows `full-render` budget | L | M | Opt-in `full-render` feature only (ADR-003); excluded from `--features default` and `--features full` Weight Target rows; Plan B: stay opt-in, no Plan B required for default users | Phase 5 / 7 owner |
| R5 | `ureq` vs `reqwest` TLS edge cases break `remote` fetch | L | M | Integration test suite against real HTTPS endpoints in CI (`tests/integration/remote/`); covers TLS 1.2, TLS 1.3, ALPN, SNI; Plan B: ship `reqwest` as alt feature gated behind `remote-reqwest` | Phase 1.8 owner |
| R6 | 500-PDF private regression corpus not assembled before v0.1.0 | H | H | Phase 0 deliverable; project lead recruits sourcing partners at kickoff; OQ-01 tracks licensing; Plan B: minimum viable corpus of 50 documents gates v0.1.0, full 500 gates v1.0.0 | Project lead (Phase 0 owner) |
| R7 | Glyph-shape DB (~5,000 entries) insufficient for real-world subsetted fonts | M | M | Level 4 fallback already accepts 0.7 confidence (Phase 2.5); coverage tracked as a CI metric; DB expandable PR-by-PR; Plan B: bundle PaddleOCR or doctr as opt-in `--alt-ocr` feature in v1.1 if WER target remains stuck | Accuracy lead (Phase 2 owner) |
| R8 | Supply-chain compromise via typosquatted crate or upstream yanking | L | H | `cargo audit` + `cargo deny` + Cargo.lock pinned for binaries; quarterly `cargo vendor` mirrors; new direct deps require ADR or written PR justification (Supply Chain Considerations) | Release lead (Phase 0 owner) |
| R9 | MCP spec change breaks the server before v1.0 | M | M | Pin to a specific MCP spec version explicitly in `crates/pdftract-cli/src/mcp.rs`; bump support window aligned with MCP minor releases; Plan B: maintain a compatibility shim for the prior minor for ≥ 1 minor release | MCP lead (Phase 6.7 owner) |
| R10 | PDF 2.0 features (PAdES-LTV signatures, AES-256 enhancements, `/Encryption v5`) not covered | M | M | Phase 7.3 already documents "no crypto validation" as a non-goal; document `/Encryption v5` limitation in `docs/pdf-2-coverage.md`; Plan B: support PDF 2.0 incrementally; defer to v1.1 if user demand emerges | Phase 7 lead |
| R11 | WASM build later requested despite explicit Non-Goal | L | L | Non-goal documented (Non-Goals section); can be revisited as a v2.0 sibling crate (`pdftract-wasm`) without modifying `pdftract-core`; Plan B: none required at v1.0.0 | Project lead |
| R12 | Inspector frontend bundle exceeds 80 KB budget | L | L | CI gate `cargo run --bin inspector-bundle-check`; minify required (`esbuild --minify` in build); Plan B: inspector frontend moves to a separate npm package fallback if budget cannot be met | Phase 7.9 owner |
| R13 | Argo Workflows in `iad-ci` cluster degraded or unavailable for a prolonged window | L | H | Tagged releases reproducible from `git` via `cargo build --release`; manual release procedure documented in `docs/operations/manual-release.md`; Plan B: short-term fall back to local builds; long-term: secondary CI runner registered in declarative-config | Release lead |
| R14 | Adoption (PyPI / GitHub stars) falls below 12-month targets | M | M | Adoption Targets are Tier 3 (Ambition Calibration); informational, not gating; planning retrospective triggered; Plan B: invest in `docs/integrations/` example bank and conference talks | Project lead |

A risk's mitigation MUST be operational (passing test, deployed gate, etc.) before the phase that depends on the mitigation can be marked complete. Risk status is reviewed at every milestone tag; new risks discovered during implementation are appended to this table.

### Plan B Strategies

The mitigation column above frequently names a fallback. This subsection consolidates the named Plan Bs for the risk register, each tied back to the originating R#. A Plan B activates when the primary mitigation has been observed to fail; activation is a planning event recorded in the Revision History.

| PB# | Tied to | Plan B |
|---|---|---|
| PB-1 | R1 | If `pdfminer.six` benchmark slips (the 10× ratio narrows because `pdfminer.six` materially improves before v1.0), re-frame the perf claim against `pypdf` (≥ 5× ratio is more stable). Revision History entry MUST document the change; the 10× claim remains in Aspirational tier as a stretch goal. |
| PB-2 | R2 | Switch wordlist storage to a Bloom filter (per ADR-002 escape hatch). The feature flag `wordlist-bloom` toggles the storage backend at compile time; default-feature build picks whichever fits the < 4 MB budget on the target triple. |
| PB-3 | R3 | Accept WER 5% on clean 300-DPI scans with a methodology footnote tying the number to the Tesseract version pinned in Dockerfile (per OQ-03). Document the per-fixture WER table in `docs/notes/ocr-accuracy.md`. |
| PB-5 | R5 | Ship `reqwest` as an alt feature gated behind `remote-reqwest`; the default `remote` continues to use `ureq` (per ADR-001). Documentation explains the trade-off; users opt into `reqwest` only if they hit a `ureq` edge case. |
| PB-7 | R7 | Bundle PaddleOCR or doctr as an opt-in `--alt-ocr` feature in v1.1 if WER target stuck. The integration is gated behind `alt-ocr` feature; binary size impact is documented and excluded from the default-binary Weight Target. |
| PB-10 | R10 | Support PDF 2.0 features incrementally; ship an explicit compatibility matrix in `docs/pdf-2-coverage.md`. The first PDF 2.0 feature shipped MAY be additive (no breaking change); breaking changes (e.g. changing the crypto surface) wait for the next major bump. |
| PB-12 | R12 | Inspector frontend moves to a separate npm package (`@pdftract/inspector-ui`) loaded by URL at runtime; the binary embeds only a 4 KB bootstrap stub. Trade-off: requires internet access at runtime for the inspect UI, documented in the inspector's launch banner. |
| PB-13 | R13 | Manual release procedure (`docs/operations/manual-release.md`) reproduces the milestone release locally; release lead executes the steps; CHANGELOG and Revision History note the manual release. Resume Argo-driven releases on the next milestone. |

A Plan B that activates MUST update the Proof Obligations Ledger entry whose claim it relaxes, MUST update the Revision History with the activation, and SHOULD trigger a Risk Register review to recalibrate the original risk's likelihood after the Plan B is in place.

---

## Known Unknowns

The list below catalogs the items that are not yet known at plan time and whose resolution is tied to a specific phase deliverable. Some overlap with Open Questions is intentional; this section is specifically about *uncertainties whose answer will materially shape phase implementation*, whereas Open Questions covers any unresolved decision (including process / staffing items). Each KU is tied to a resolution strategy; resolution status is reviewed at every phase exit gate.

| KU# | Unknown | Resolution strategy | Phase |
|---|---|---|---|
| KU-1 | Glyph-shape DB coverage gap on real-world subsetted fonts | Spike of 100 random PDFs from `tests/fixtures/perf/` measured against the DB; coverage ratio recorded; if < 80% Latin/Greek/Cyrillic, the DB is expanded before Phase 2.5 sign-off | Phase 2.5 |
| KU-2 | Tesseract behaviour on Hybrid pages with overlapping vector + scan content | Phase 5.5 fixture suite (`tests/fixtures/hybrid/`) targets 10 known-tricky hybrid cases; classifier decision rules are tuned to ensure neither path is starved | Phase 5.5 |
| KU-3 | Actual binary contribution of `regex` after dead-code elimination | `cargo bloat --features default --crates` in Phase 0 CI records the per-crate size; if `regex` contributes > 1 MB, switch to `regex-lite` for the cold path | Phase 0 |
| KU-4 | rayon+tokio bridge produces thread-pool starvation under realistic load | Phase 6.4 load test with concurrent extractions (`wrk -c 32 -d 60s`); rayon pool utilization gauge added per Monitoring & Alerting; remediation: tune `spawn_blocking` permit count | Phase 6.4 |
| KU-5 | Claude Desktop / Cursor / Continue successfully discover and connect to `pdftract mcp --stdio` | Manual smoke test before v0.3.0; results recorded in `docs/integrations/mcp-clients.md`; per-client config snippet shipped in the same doc | Phase 6.7 |
| KU-6 | Cache filesystem layout scales to ~1M entries on ext4 | Phase 6.9 load test with synthetic fingerprints; verify `lookup` latency stays < 20 ms; verify `purge` doesn't take > 30 s; remediation: shard cache by fingerprint prefix into 256 subdirectories | Phase 6.9 |
| KU-7 | Structural fingerprint correctly identifies a PDF re-saved with linearization toggled | Phase 1.7 critical test: take a fixture, linearize it via `qpdf --linearize`, verify the fingerprint matches the non-linearized version (per ADR-008) | Phase 1.7 |
| KU-8 | Binary contribution of `serde_yaml` on stripped release | `cargo bloat` in Phase 7.10; if > 200 KB, evaluate `yaml-rust2` as a drop-in replacement | Phase 7.10 |
| KU-9 | Whether IBKR-style proprietary PDFs (financial statements) match the document-type classifier accuracy target | Phase 5.6 sign-off includes a 50-doc "finance" subcorpus; if accuracy < 80%, add a domain-specific profile in `profiles/community/` and document the gap | Phase 5.6 |
| KU-10 | Whether the `--receipts=svg` mode produces deterministic SVG bytes across platforms | Phase 6.8 critical test: produce SVG on Linux + macOS + Windows runners; assert byte-identical output (INV-3 family) | Phase 6.8 |
| KU-11 | Whether profile YAML reload (`--profile-hot-reload`) survives `inotify` instance exhaustion on Linux | Phase 7.10 critical test: spawn `serve` with `--profile-hot-reload`, then exhaust `inotify` via `fs.inotify.max_user_instances`; verify graceful degradation to polling | Phase 7.10 |
| KU-12 | Whether macOS and Windows binaries (built via `cross` on Linux but never runtime-tested in CI per ADR-009) work correctly on real hardware | Manual quarterly smoke-test runbook in `docs/operations/manual-platform-smoke.md`; release lead executes against at least one physical macOS machine and one Windows VM before each milestone tag; failures block the milestone | Pre-milestone (every release) |
| KU-13 | Whether the SDK conformance suite (`tests/sdk-conformance/cases.json`) is comprehensive enough to detect schema regressions before SDKs ship | Phase 6 sign-off includes a 30+ scenario corpus; review at every milestone; gaps surfaced by SDK users add new cases and trigger a patch SDK release | Phase 6 (initial), ongoing |

A KU that cannot be resolved within its assigned phase escalates: either the assigned phase blocks until the unknown is resolved, OR an Open Question is added with explicit deferral to v1.1+, OR the assumption is recorded as an accepted risk in the Risk Register. New Known Unknowns identified during phase implementation are appended to this table.

---

## Acceptance Scenarios

End-to-end user scenarios in the Setup / Action / Expected / Pass / Fail format. These are the named acceptance criteria for the v1.0.0 release; the Tier 4 benchmark suite is the implementation of automated checks for the speed-related ones, and the per-phase critical tests cover the rest. A scenario that cannot be made to pass blocks the corresponding milestone.

### Scenario AS-01: Extract a clean academic paper to JSON

- **Setup:** A 12-page LaTeX-produced academic paper at `tests/fixtures/vector/academic-paper.pdf`. pdftract CLI binary built with `--features default` on `x86_64-unknown-linux-musl`.
- **Action:** `pdftract extract tests/fixtures/vector/academic-paper.pdf --json out.json`
- **Expected:** `out.json` is created. Content includes: `schema_version = "1.0"`; `metadata.page_count = 12`; `metadata.pdf_fingerprint` is a 64-char hex string with the `pdftract-v1:` prefix; `extraction_quality.overall_quality` is `"high"`; each page has a non-empty `spans` array; reading order places the abstract before the introduction.
- **Pass criteria:** Exit code 0; `out.json` validates against `docs/schema/v1.0/pdftract.schema.json`; character error rate against the ground-truth text < 0.5%.
- **Fail criteria:** Any of: non-zero exit code, schema validation failure, CER ≥ 0.5%, abstract serialized after introduction in reading order, missing `pdf_fingerprint`.

### Scenario AS-02: Extract a scanned receipt via OCR

- **Setup:** A single-page scanned receipt at `tests/fixtures/scanned/receipt-300dpi.pdf` (physical scan, English text, 300 DPI). pdftract built with `--features ocr` and `tesseract` system library installed.
- **Action:** `pdftract extract tests/fixtures/scanned/receipt-300dpi.pdf --ocr --text`
- **Expected:** Plain-text output to stdout containing the merchant name, line items, subtotal, tax, and total. Span confidences in the corresponding JSON output range 0.4–0.95 depending on print quality. `metadata.extraction_quality.overall_quality` is `"medium"` or `"high"`.
- **Pass criteria:** Exit code 0; word error rate vs. ground-truth transcript < 3%; total currency amount parses as a decimal matching the ground truth.
- **Fail criteria:** WER ≥ 3%; missing total line; OCR latency > 30 s on 4-core CI; `Tesseract` not found error message indicating misconfigured environment (process must abort cleanly with a clear diagnostic, not silently produce empty output).

### Scenario AS-03: Search a folder of 500 contracts for a regex

- **Setup:** A folder `tests/fixtures/grep-corpus/contracts/` containing 500 contract PDFs. pdftract built with `--features grep`.
- **Action:** `pdftract grep -E 'Termination(\s+for)?\s+Cause' tests/fixtures/grep-corpus/contracts/ --json --progress-json 2> progress.log`
- **Expected:** JSON-Lines output on stdout, one match per line, including file path, page index, bbox, matched text, and PDF fingerprint. Progress events on stderr (`file_start`, `file_progress`, `file_done`) emitted at least every 500 ms during processing. Total wall-clock time ≤ 20 s on 4-core CI.
- **Pass criteria:** Exit code 0 if any match found; all matches present in `--highlight DIR` output as Highlight annotations on the same pages; first match printed within 100 ms of process start; throughput ≥ 50 MB/s aggregate input.
- **Fail criteria:** Missing matches that ground-truth scan finds; throughput < 50 MB/s; progress events absent for any single 1-second window; binary exits before processing all files; encrypted PDFs in the folder cause a fatal error instead of a per-file skip diagnostic.

### Scenario AS-04: Claude Desktop invokes pdftract via MCP to summarise a PDF

- **Setup:** pdftract built with `--features ocr,serve,mcp`. Claude Desktop configured with a single MCP server entry in `~/Library/Application Support/Claude/claude_desktop_config.json` (or platform equivalent) pointing to `pdftract mcp --stdio`. A test PDF at `~/Documents/test-paper.pdf`.
- **Action:** In a Claude Desktop session, the user types: "Summarise the document at ~/Documents/test-paper.pdf." Claude invokes the `extract` tool via MCP.
- **Expected:** `pdftract mcp --stdio` accepts the JSON-RPC `tools/call` request with method `extract` and `path` argument. Process responds with a JSON-RPC reply carrying the extracted document JSON. Total stdio round-trip time for a 10-page PDF: < 1 second. Claude Desktop receives the document text and produces a summary in its response.
- **Pass criteria:** Tool call succeeds; response is valid JSON-RPC 2.0; Claude can quote text from the PDF in its summary verifying actual content reached the model; no `LATIN1`/`UTF-8` corruption in the round trip.
- **Fail criteria:** Tool-list call hangs; stdout contains anything that is not valid JSON-RPC framing (would crash Claude Desktop's MCP client); response time > 5 s for a 10-page PDF; bytes from stderr leak into the JSON-RPC channel.

### Scenario AS-05: Cache-hit on a resubmitted PDF returns in < 20 ms

- **Setup:** pdftract built with `--features serve,cache`. `pdftract serve --port 8080 --cache-dir /tmp/pdftract-cache --cache-size 1GiB` running in the background. A test PDF `test.pdf` (100 pages, ~5 MB).
- **Action:** First request: `curl -F file=@test.pdf http://localhost:8080/extract -o first.json -w '%{time_total}\n'`. Note the timing and verify `X-Pdftract-Cache: miss` header. Second request: same command, output to `second.json`. Note the timing and verify `X-Pdftract-Cache: hit` header.
- **Expected:** First request takes the baseline extraction time (~2 s for 100 pages). Second request completes in < 20 ms total response time (cache lookup + decompress + JSON serialization). `first.json` and `second.json` are byte-identical.
- **Pass criteria:** Cache-hit response time < 20 ms p99 across 100 repeat requests; byte-identical JSON between miss and hit; `metadata.cache_status: "hit"` and `metadata.cache_age_seconds: > 0` in the second response; metadata.pdf_fingerprint identical between miss and hit.
- **Fail criteria:** Cache-hit response time ≥ 20 ms p99; JSON differs between miss and hit; cache miss reported on second identical request; metadata.pdf_fingerprint differs between two extractions of the same byte-identical input.

### Scenario AS-06: Encrypted PDF with no password fails gracefully via the Python API

- **Setup:** pdftract built with `--features python,decrypt`, wheel installed via `pip install pdftract`. A test PDF `encrypted.pdf` protected by a non-empty user password.
- **Action:** Run the following Python code:
  ```python
  import pdftract
  try:
      pdftract.extract("encrypted.pdf")
  except pdftract.EncryptionError as e:
      print(f"Caught: {e}")
  ```
- **Expected:** `EncryptionError` raised (NOT a generic `PdftractError`, NOT a Python `Exception`, NOT a `RuntimeError`). The error message identifies that the file is encrypted and that no password was supplied or the supplied password failed. No partial extraction output. Process exits cleanly with no traceback noise from FFI.
- **Pass criteria:** `EncryptionError` raised with a clear human-readable message; subsequent call `pdftract.extract("encrypted.pdf", password="correctpw")` succeeds and returns the document JSON.
- **Fail criteria:** A non-specific exception is raised; Python crashes with a SIGSEGV from the FFI layer; partial output is returned; subsequent password-supplied call also fails despite the password being correct.

---

## Edge Case Catalog

The following 26 edge cases are exercised by integration tests in `tests/fixtures/`. Each has a unique identifier (EC-NN) for cross-reference from per-phase critical tests and from the Failure Mode Taxonomy below. The Resolution column describes the expected behaviour, NOT the actual implementation (which lives in the cited phase).

| ID | Name | Description | Resolution |
|---|---|---|---|
| EC-01 | Empty PDF | A 0-byte file or a syntactically valid PDF with zero pages | Phase 1.4 emits diagnostic `STRUCT_MISSING_KEY`; output is a valid document with `page_count: 0`, empty `spans/blocks/pages` |
| EC-02 | Single-page PDF | The minimum valid PDF — 1 page, 1 paragraph | Baseline path; output validates against schema |
| EC-03 | 10,000-page PDF | Synthetic stress PDF | Phase 6.2 streaming mode handles without exceeding memory budget; non-streaming mode buffers the document model (~20 MB per 500 pages × 200 spans/page; ~400 MB peak — within target for streaming workflows) |
| EC-04 | Encrypted (RC4) | RC4-encrypted PDF, user password "test" | Phase 1.4 with `--password test` decrypts successfully via the `rc4` crate (default feature `decrypt`) |
| EC-05 | Encrypted (AES-128) | AES-128 with the same handler | Phase 1.4 decrypts via `aes` crate; same flow as EC-04 |
| EC-06 | Encrypted (AES-256) | AES-256 (PDF 2.0) | Phase 1.4 decrypts via `aes` crate; same flow |
| EC-07 | Corrupt xref | xref offset off by one (common real-world corruption) | Phase 1.3 strategy 4 (forward scan fallback) recovers; `XREF_REPAIRED` diagnostic emitted |
| EC-08 | Circular object references | Object A → B → A | Phase 1.2 per-thread resolution stack detects; `STRUCT_CIRCULAR_REF` diagnostic; PdfNull returned for the cycle |
| EC-09 | Missing `/MediaBox` | Page with no MediaBox and no inherited MediaBox | Phase 1.4 substitutes US Letter (612×792); `STRUCT_MISSING_KEY` diagnostic per page |
| EC-10 | FlateDecode bomb | A small compressed stream that expands to > 2 GB | Phase 1.5 enforces `max_decompress_bytes` (512 MB default); emits `STREAM_BOMB`; returns partial bytes |
| EC-11 | JBIG2 without `full-render` | JBIG2-encoded image needing OCR | Phase 5.2 emits `OCR_JBIG2_UNSUPPORTED`; page skipped from OCR |
| EC-12 | JPX without `full-render` | JPEG 2000-encoded image needing OCR | Phase 5.2 emits `OCR_JPX_UNSUPPORTED`; page skipped from OCR |
| EC-13 | CCITT without libtiff or `full-render` | CCITT fax-encoded image needing OCR | Phase 5.2 emits `OCR_CCITT_UNSUPPORTED`; page skipped from OCR |
| EC-14 | Type 3 font with arbitrary glyph names | Custom Type 3 font, no ToUnicode | Phase 2.4 falls through to Level 4 shape recognition; confidence 0.7 |
| EC-15 | Type 0 CJK with Shift-JIS | Japanese composite font using Shift-JIS codespace | Phase 2.3 decodes via `encoding_rs::SHIFT_JIS`; multi-byte codes parsed via codespace ranges |
| EC-16 | OCG with default OFF state | Optional content group set to OFF by default | Phase 1.4 reads `/OCProperties /D /BaseState`; Phase 3 suppresses glyphs inside `OC` BDC blocks whose group is OFF |
| EC-17 | `/ActualText` override | Tagged PDF with `/ActualText` on a ligature span | Phase 7.1 uses ActualText value, not glyph-decoded text |
| EC-18 | `/Artifact` marked content | Tagged PDF with decorative content marked as Artifact | Phase 7.1 suppresses Artifact glyphs from output |
| EC-19 | RTL Arabic page | Right-to-left script | Phase 4.2 detects via `unicode-bidi`; spans sorted right-to-left; `direction: "rtl"` on line |
| EC-20 | Two-column with sidebar | Magazine-style layout | Phase 4.5 XY-cut produces main-column and sidebar regions; sidebar follows main flow |
| EC-21 | `/Rotate 90/180/270` | Page rotated by content-stream metadata | Phase 3.1 applies inverse rotation to all glyph bboxes; output page width/height reflect rotated dimensions |
| EC-22 | Font subset without `/ToUnicode` | Subset font `ABCDEF+Helvetica` with no ToUnicode | Phase 2.2 strips prefix; falls through Levels 2–4 |
| EC-23 | Missing `/Encoding` | Type 1 font with no Encoding and no ToUnicode | Phase 2.2 falls through to Level 3 (fingerprint) or Level 4 (shape) |
| EC-24 | Hyphenated word at line break | "compre-\nhensive" with the hyphen at column end | Phase 4.7 strips the hyphen and joins; output: "comprehensive" |
| EC-25 | Ligature split as U+FFFD + glyph | A `fi` ligature where the first half decoded as U+FFFD | Phase 4.7 reconstructs from shape-matched component glyphs |
| EC-26 | OCR-degraded text with low confidence | Tesseract emits text with confidence 0.3 on a noisy region | Phase 5.4 emits the text with `confidence: 0.3`; downstream consumers can filter on confidence |
| EC-27 | Oversized form XObject cycle | A invokes B, B invokes A, depth 20 reached | Phase 3.3 cycle detection at second A; `STRUCT_XOBJECT_CYCLE` diagnostic; extraction continues |
| EC-28 | Soft-hyphen U+00AD | Page contains soft-hyphens U+00AD inserted by typesetter | Phase 4.7 strips U+00AD from output text |
| EC-29 | Mojibake `Ã©` | Latin-1 bytes interpreted as UTF-8 in a content stream | Phase 4.7 re-decodes via `encoding_rs`; accepted if readability improves |
| EC-30 | Blank page | Page with no content stream operators | Phase 5.1 classifies as `blank`; `spans: []`, `blocks: []` |
| EC-31 | Figure-only page | Page with only image XObjects, no text | Phase 5.1 classifies as `figure_only`; `blocks: []` (or single `figure` block if Phase 7 figure detection is enabled) |

Each row references the originating phase. PRs adding new edge cases append to this table with a new EC-NN and add a fixture under `tests/fixtures/`.

---

## Failure Mode Taxonomy

Failure modes that may occur at runtime, categorised by source. Each entry pairs the failure with its detection signal (how pdftract knows the failure happened), the recovery strategy (what pdftract does next), and the test fixture that exercises the case (where the fixture is named).

| Category | Failure Mode | Detection Signal | Recovery Strategy | Test Fixture |
|---|---|---|---|---|
| **Network** | `REMOTE_FETCH_INTERRUPTED` | TCP connection drops mid-fetch; `ureq` returns an `io::Error` with `kind = ConnectionReset` or `BrokenPipe` | Emit diagnostic; yield partial result (pages already buffered); CLI exit code 5 | Mock HTTP server in Phase 1.8 critical tests; closes connection after first 50 KB |
| **Network** | `REMOTE_NO_RANGE_SUPPORT` | `HEAD` response lacks `Accept-Ranges: bytes`, or a `Range` request returns 200 instead of 206 | Fall back to streaming the entire response body into a temp file, then `MmapSource` over that | Mock HTTP server with `Accept-Ranges` header stripped |
| **Network** | TLS handshake failure | `ureq` returns `rustls::Error` from connect | Emit diagnostic with the certificate chain reason; CLI exit code 6 | Mock HTTPS server with expired or self-signed cert |
| **Network** | DNS resolution failure | `ureq` returns `io::Error` with `kind = NotFound` from connect | Emit diagnostic; CLI exit code 4 | Hostname `pdftract.invalid` |
| **Disk** | Cache write failure (ENOSPC) | `std::fs::write` returns `io::Error` `kind = StorageFull` | Emit diagnostic to stderr; complete extraction; cache write is skipped | Synthetic small tmpfs filled to capacity |
| **Disk** | Output write failure | `std::fs::write` to the `--json out.json` path fails | Emit diagnostic; non-zero exit; temp file removed (no partial output) | Output path inside a read-only directory |
| **Input** | Corrupt xref | `startxref` offset points outside file, or xref table malformed | Phase 1.3 strategy 4: forward scan fallback; `XREF_REPAIRED` diagnostic | `tests/fixtures/malformed/corrupt-xref.pdf` |
| **Input** | Stream-decode error | FlateDecode produces an invalid zlib stream mid-decompression | Return bytes decoded so far; `STREAM_DECODE_ERROR` diagnostic; page continues | `tests/fixtures/malformed/truncated-flate.pdf` |
| **Input** | Encryption-unsupported | `/Encrypt` dict identifies an unknown handler (e.g. an Adobe LiveCycle policy server) | Emit `ENCRYPTION_UNSUPPORTED` diagnostic; CLI exit code 3 | `tests/fixtures/encrypted/livecycle.pdf` |
| **Input** | Glyph unmapped (Level 4 miss) | No ToUnicode, no AGL match, no fingerprint hit, no shape-DB hit within Hamming threshold | Emit U+FFFD; `confidence: 0.0`; `unicode_source: "unknown"`; `GLYPH_UNMAPPED` diagnostic | `tests/fixtures/encoding/no-mapping.pdf` |
| **Input** | Stream bomb | Single stream or document-cumulative decompressed size > `max_decompress_bytes` | Return bytes decoded so far; `STREAM_BOMB` diagnostic | `tests/fixtures/malformed/compression-bomb.pdf` |
| **Input** | JBIG2/JPX/CCITT decode unsupported | Image filter not available in current build | `OCR_JBIG2_UNSUPPORTED` / `OCR_JPX_UNSUPPORTED` / `OCR_CCITT_UNSUPPORTED` diagnostic; page skipped from OCR | EC-11, EC-12, EC-13 fixtures |
| **Dependency** | Tesseract not found | `tesseract` system library fails to load at startup with `--features ocr` | Emit clear error to stderr referencing the install command for the OS; exit code 4 | Docker image with `tesseract-ocr` removed |
| **Dependency** | libtiff missing | `image` crate's TIFF/CCITT decode fails | `OCR_CCITT_UNSUPPORTED` diagnostic; page skipped from OCR | Docker image with `libtiff` removed |
| **Dependency** | PDFium missing | `--features full-render` requested but `libpdfium.so` unavailable at runtime | Emit clear error to stderr at first use; fall back to direct compositing path | Docker image with `pdfium` symlink broken |
| **Internal logic** | Graphics state stack overflow | `q` operator nests beyond 64 levels deep | Emit `GSTATE_STACK_OVERFLOW`; discard the push (safe failure); continue parsing | `tests/fixtures/malformed/deep-gsave.pdf` |
| **Internal logic** | Form XObject cycle | Same object number appears twice in the form-XObject execution stack | `STRUCT_XOBJECT_CYCLE` diagnostic; abort that sub-tree; extraction continues | EC-27 fixture |
| **Internal logic** | Page out of range | `--pages 200-` requested on a 100-page PDF | `PAGE_OUT_OF_RANGE` diagnostic for each missing index; processing continues for the in-range pages | `tests/fixtures/vector/100-pages.pdf` with `--pages 99-200` |
| **Resource** | Decompression cap exceeded | Cumulative decompressed bytes > `max_decompress_bytes` | `STREAM_BOMB` diagnostic; return bytes decoded so far; CLI exits 0 with partial result | Same as "Stream bomb" above |
| **Resource** | Request body too large (serve mode) | HTTP request body exceeds `--max-upload-mb` | HTTP 413 with JSON body `{"error":"REQUEST_TOO_LARGE",...}` | Phase 6.4 critical-test fixture |

Each row is exercised by at least one fixture under `tests/fixtures/` and one Tier 2 integration test. New failure modes added in future revisions append to this table.

---

## Diagnostic Code Catalog

Stable identifiers for every diagnostic emitted by pdftract. Codes are part of the public API surface — downstream consumers MAY pattern-match on them. Code renaming requires a Revision History entry and a deprecation window.

Severity values: `info` (informational, does not affect output validity), `warn` (output usable but degraded), `error` (output for this region/page invalid; other regions OK), `fatal` (extraction aborted).

| Code | Category | Severity | Recoverable? | Suggested User Action | Phase Origin |
|---|---|---|---|---|---|
| `STRUCT_MISSING_KEY` | Structural | warn | yes | Inspect the source PDF; missing keys are typically substituted with safe defaults | Phase 1.4 |
| `STRUCT_INVALID_NAME` | Structural | warn | yes | None — the offending name was truncated to 127 bytes per spec | Phase 1.1 |
| `STRUCT_CIRCULAR_REF` | Structural | warn | yes | None — cycle broken at the second visit; affected object returned as null | Phase 1.2 |
| `XREF_REPAIRED` | Structural | info | yes | None — the xref was reconstructed via forward scan; output may be incomplete on truncated files | Phase 1.3 |
| `STRUCT_XOBJECT_CYCLE` | Structural | warn | yes | Investigate the source PDF for a producer bug; cycle is broken at depth 20 | Phase 3.3 |
| `GSTATE_STACK_OVERFLOW` | Structural | warn | yes | Investigate the source PDF for a malformed content stream | Phase 3.1 |
| `STREAM_DECODE_ERROR` | Stream | warn | yes | Partial output returned for this stream; consider re-saving the PDF through a normalising tool | Phase 1.5 |
| `STREAM_BOMB` | Stream | error | yes | Increase `--max-decompress-gb` if the PDF is trusted; otherwise treat as a hostile file | Phase 1.5 |
| `ENCRYPTION_UNSUPPORTED` | Encryption | fatal | no | Supply the correct password via `--password`, or use an Adobe-side decryption tool first | Phase 1.4 |
| `GLYPH_UNMAPPED` | Font | warn | yes | The glyph could not be resolved by any of the four levels; output contains U+FFFD | Phase 2.2 |
| `OCR_JBIG2_UNSUPPORTED` | OCR | warn | yes | Build with `--features full-render` to enable JBIG2 decoding via PDFium | Phase 1.5 / Phase 5.2 |
| `OCR_JPX_UNSUPPORTED` | OCR | warn | yes | Build with `--features full-render`, or install `libopenjp2` system library | Phase 1.5 / Phase 5.2 |
| `OCR_CCITT_UNSUPPORTED` | OCR | warn | yes | Install `libtiff` system library, or build with `--features full-render` | Phase 1.5 / Phase 5.2 |
| `REMOTE_FETCH_INTERRUPTED` | Remote | error | yes | Retry the request; check network connectivity | Phase 1.8 |
| `REMOTE_NO_RANGE_SUPPORT` | Remote | warn | yes | None — pdftract falls back to whole-file download; consider hosting on a Range-supporting server | Phase 1.8 |
| `PAGE_OUT_OF_RANGE` | Resource | warn | yes | Adjust the `--pages` argument to the actual document page count | Phase 1.8 |
| `BROKENVECTOR_OCR_UNAVAILABLE` | OCR | warn | yes | Build with `--features ocr` to enable OCR recovery on broken-vector pages | Phase 4.7 |
| `TAGGED_PDF_STRUCT_TREE_DEFERRED` | Layout | info | yes | None — Phase 7.1 will replace this fallback in v1.0.0 | Phase 4.5 |
| `MCP_TOOL_INVALID_PARAMS` | MCP | error | yes | Adjust the tool-call arguments to match the schema in `tools/list` | Phase 6.7 |
| `MCP_PATH_TRAVERSAL` | MCP | error | yes | The requested path escapes `--root`; either fix the path or restart the server without `--root` | Phase 6.7 |
| `CACHE_ENTRY_CORRUPT` | Cache | warn | yes | None — the entry was deleted and extraction re-ran | Phase 6.9 |

### Exit code mapping (CLI)

| Code | Meaning |
|---|---|
| 0 | Success (including success with non-fatal diagnostics) |
| 1 | Generic runtime error (unrecoverable, not in this table) |
| 2 | Corrupt file (parser could not recover any pages) |
| 3 | Encrypted, no password / wrong password (`ENCRYPTION_UNSUPPORTED` fatal) |
| 4 | Unreadable source (file not found, permission denied, DNS failure, missing OCR dependency) |
| 5 | Network fetch interrupted (`REMOTE_FETCH_INTERRUPTED`) |
| 6 | TLS handshake failure |
| 10 | Receipt verification failed: fingerprint mismatch (`pdftract verify-receipt`) |
| 11 | Receipt verification failed: bbox overlap < 90% (`pdftract verify-receipt`) |
| 12 | Receipt verification failed: content hash mismatch (`pdftract verify-receipt`) |

Exit codes are part of the public API surface. Renumbering requires a Revision History entry and the previous code remains valid through one minor version for compatibility.

---

## Cross-Cutting Concerns

The following concerns apply across all phases. They are documented here rather than inline in any single phase because they shape every phase's contract.

### Rollback and binary downgrade

pdftract releases follow semver. Downgrading to a previous version is supported via the same install mechanisms used to upgrade:

- **Cargo:** `cargo install pdftract --version 1.0.0` reverts to a specific version.
- **PyPI:** `pip install pdftract==1.0.0` reverts the Python wheel.
- **Docker:** Pin to a specific tag (`ronaldraygun/pdftract:1.0.0` or `ronaldraygun/pdftract:full-1.0.0`) — the `latest` tag floats. Operators are RECOMMENDED to pin in production.

Outputs are forward-compatible within a minor version: a JSON document produced by v1.0.0 is readable by v1.0.5 (additive schema changes only). A document produced by v1.0.5 MAY contain fields absent in v1.0.0; v1.0.0 consumers ignore unknown fields per the JSON Schema (`additionalProperties: true` is the v1.x policy).

Outputs are NOT guaranteed forward-compatible across major versions. v2.x consumers MAY require migration; the Revision History MUST flag any schema breaking change.

### State capture for diagnostics

`pdftract extract --capture-diagnostics OUT.tar` produces a tar archive containing:
- The input PDF (with byte-identical SHA-256 to the original)
- A JSON dump of the full `ExtractionOptions` used
- The full JSON extraction output, including all `errors[]` entries
- A copy of the pdftract version banner (`pdftract --version` output)
- A copy of the relevant environment variables (`RUST_LOG`, `PDFTRACT_*`)

The archive is the canonical artifact attached to bug reports — maintainers can reproduce any reported issue by running `pdftract extract` on the captured PDF with the captured options. Sensitive information (passwords supplied via `--password`) is redacted in the captured options.

### Invariants

Named testable properties that hold across all phases. Each invariant is the predicate; the "Enforced by" line names the test or check that asserts it. A violation of any invariant is a P0 bug.

| ID | Invariant | Enforced by |
|---|---|---|
| INV-1 | For every span where `font_size > 0`, the bbox is non-degenerate: `bbox[2] > bbox[0] AND bbox[3] > bbox[1]` | `tests/integration/invariants/non_degenerate_bbox.rs` |
| INV-2 | `page_index` is monotone in the page list: page 0 first, page 1 second, …, page N−1 last; no gaps, no duplicates | `tests/integration/invariants/page_index_monotone.rs` |
| INV-3 | `pdf_fingerprint` is byte-stable across runs for the same input on the same algorithm version | Phase 1.7 critical test: 10 invocations produce identical fingerprint |
| INV-4 | `confidence_source` is non-null for every span with non-empty `text` | `tests/integration/invariants/confidence_source_present.rs` |
| INV-5 | Extraction with `--receipts=lite` followed by `pdftract verify-receipt` succeeds (round-trip) | Phase 6.8 critical test |
| INV-6 | A cache hit returns byte-identical JSON to a fresh extraction with the same options | Phase 6.9 critical test |
| INV-7 | Multi-output emission produces byte-identical per-format output regardless of which other formats are concurrently active | Phase 6.6 acceptance criterion: same JSON whether `--json` alone or `--json --md --text` |
| INV-8 | No `panic!` reaches the public boundary of `pdftract-core`; all errors are emitted as `errors[]` entries in the output | `cargo test --features default,decrypt -- --include-ignored` plus a clippy lint denying `unwrap_used` and `expect_used` in lib code |
| INV-9 | In MCP stdio mode (Phase 6.7), stdout MUST contain only JSON-RPC frames; logs MUST go to stderr | Phase 6.7 critical test: pipes stdout to a JSON-RPC parser; any non-JSON-RPC byte fails the test |
| INV-10 | In `serve` and `mcp --bind` modes, the HTTP API MUST NOT accept file-path parameters; all PDFs arrive via multipart upload (`serve`) or `https://` URLs (`mcp`) | Phase 6.4 / 6.7 critical tests inspect each endpoint's parameter list |
| INV-11 | The JSON output validates against `docs/schema/v1.0/pdftract.schema.json` for every page in every fixture | Tier 2 schema validation step in CI |
| INV-12 | `extraction_version` in receipts is a valid semver and matches the binary version | Phase 6.8 acceptance test |
| INV-13 | The fingerprint version prefix (`pdftract-v1:`) is present on every fingerprint emission | Phase 1.7 acceptance test (regex match) |

New invariants added in future revisions append to this table with a new test fixture. Invariants are immutable: weakening an invariant requires a Revision History entry and a new minor version.

---

## Threat Model

pdftract is exposed to untrusted input across multiple surfaces. This section enumerates attacker profiles, attack surfaces, and per-threat mitigations. Every threat MUST have at least one corresponding test fixture; new threats SHALL be added to this section before the mitigating code is merged.

### Attacker Profiles

| Profile | Capability | Realistic vector |
|---|---|---|
| A1: Untrusted PDF author | Crafts a malicious PDF byte sequence | User extracts a PDF from email/web; SaaS user uploads attacker-supplied PDF to `pdftract serve` |
| A2: Malicious HTTP client of `serve` | Sends crafted multipart uploads, oversized bodies, malformed headers to the `pdftract serve` endpoint | Public-facing or multi-tenant `serve` deployment |
| A3: Malicious MCP client | Sends crafted JSON-RPC requests, oversized parameters, malicious URLs to a `pdftract mcp --bind` instance | LLM agent operates against a shared MCP server; co-tenant agent on a multi-tenant deployment |
| A4: Supply-chain attacker | Publishes a typosquatted crate, yanks a dep, ships a backdoored point release | Upstream registry compromise; dependency confusion |
| A5: Operator misconfig | Operator binds `mcp --bind 0.0.0.0:PORT` without `--auth-token`; ships profiles containing credentials; runs `--debug` in production | Misread documentation; copy-pasted insecure example |

### Attack Surfaces

| Surface | Phase | Exposure |
|---|---|---|
| PDF lexer / object parser | 1.1, 1.2 | Every extraction; attacker A1 |
| Stream decoder (FlateDecode, LZWDecode, ASCII85Decode, CCITT, DCT, JBIG2) | 1.5 | Every extraction; attacker A1 |
| Cross-reference resolver and forward-scan fallback | 1.3 | Every extraction; attacker A1 |
| Font program parser (Type 1 charstring, TrueType / CFF tables) | 2.1, 2.4 | Every extraction; attacker A1 |
| Content stream interpreter (graphics state machine, text operators) | 3.1, 3.2 | Every extraction; attacker A1 |
| Remote source HTTP fetcher (`ureq`) | 1.8 | `remote` feature; attackers A1 + A3 (via MCP `url` parameter) |
| Tesseract subprocess / OCR pipeline | 5.4 | `ocr` feature; attacker A1 |
| `serve` HTTP listener (axum) | 6.4 | `serve` feature; attacker A2 |
| MCP server (stdio + HTTP transports) | 6.7 | `mcp` feature; attacker A3 |
| Profile YAML loader (`serde_yaml`) | 7.10 | `profiles` feature; attackers A1, A5 |
| Cache filesystem layout | 6.9 | `cache` feature; attacker with local FS write access (e.g. shared host) |
| Output sink atomic write (`tempfile` + `persist`) | 6.6 | Every extraction; symlink-race attacker with local FS write access |
| Inspector mode web frontend (HTML + SVG) | 7.9 | `inspect` feature; attacker A1 (XSS via crafted PDF content rendered into the UI) |
| Argo Workflows CI runners (Phase 0) | 0 | Attacker A4 (supply-chain compromise propagated through CI) |

Impact classes referenced in the Per-Threat Security Matrix: **DoS** (denial of service, memory or CPU exhaustion), **InfoDisc** (information disclosure beyond intended scope), **Tamper** (data tampering with cached or persisted artifacts), **RCE** (remote code execution in the pdftract host process), **Supply** (supply-chain compromise of build or release artifacts).

### Per-Threat Security Matrix

The matrix below lists the threats covered by mitigations in this plan. Every row is linked to a test fixture; the test name follows the convention `tests/security/<TH-id>-<short-name>.rs`.

| Threat ID | Attacker | Vector | Mitigation | Test |
|---|---|---|---|---|
| TH-01 | A1 | Decompression bomb: 10 KB FlateDecode stream expands to multi-GB | `ExtractionOptions.max_decompress_bytes` (default 512 MB); Phase 1.5 enforces the cap; abort emits `STREAM_BOMB` diagnostic per Diagnostic Code Catalog | `tests/security/TH-01-stream-bomb.rs` against `tests/fixtures/malformed/bomb-10k-2g.pdf` |
| TH-02 | A3 | Path traversal: MCP client requests `../../etc/passwd` via a tool that accepts a path parameter | `pdftract mcp` MUST NOT accept file-path parameters (per INV-10); `--root DIR` (when introduced) canonicalises and rejects paths outside `DIR` with `PATH_OUTSIDE_ROOT` diagnostic | `tests/security/TH-02-path-traversal.rs` exercising 10 traversal payloads |
| TH-03 | A5 | Unauthenticated MCP bind on a public interface | `pdftract mcp --bind` MUST require `--auth-token` (or `PDFTRACT_MCP_TOKEN`) unless the bind address resolves to `127.0.0.1`/`::1`; startup aborts otherwise with exit code 78 | `tests/security/TH-03-mcp-no-auth.rs`: spawn `mcp --bind 0.0.0.0:0` with no token, assert startup failure |
| TH-04 | A1 | JavaScript embedded in `/AA`, `/OpenAction`, or `/JS` entries triggers execution | pdftract NEVER executes embedded JavaScript; presence is flagged as a `JAVASCRIPT_PRESENT` diagnostic (info-level) and surfaced in the JSON output as `metadata.javascript_actions[]` for downstream review | `tests/security/TH-04-js-presence.rs` against `tests/fixtures/security/embedded-js.pdf` |
| TH-05 | A3 | SSRF: MCP `extract` tool fetches an attacker-supplied URL targeting an internal service (e.g. `http://169.254.169.254/`, `http://10.0.0.1/`) | URL schemes restricted to `https://`; localhost / private-IP / link-local / loopback ranges refused unless `--allow-private-networks` is set; refusal emits `URL_PRIVATE_NETWORK` diagnostic and HTTP 400 in serve mode | `tests/security/TH-05-ssrf-block.rs` with payloads covering RFC 1918 ranges, IPv6 ULAs, `localhost`, and metadata endpoints |
| TH-06 | A4 | Supply-chain compromise via typosquatted or yanked crate | `Cargo.lock` checked in for binary crates; `cargo audit` runs in Phase 0 CI on every PR (severity ≥ medium blocks merge); `cargo deny` enforces license + ban lists; checksum pin on `build/font-fingerprints.json` and `build/glyph-shapes.json` | Phase 0 CI gate: `cargo audit` + `cargo deny check`; nightly cron re-runs both |
| TH-07 | A5 | PDF password disclosed via process arg list (`ps aux`) | Passwords accepted only via env var (`PDFTRACT_PASSWORD`), `--password-stdin`, Python `password=`, MCP `password` body, or serve `password` form field. `--password VALUE` plain-text flag is REJECTED unless `PDFTRACT_INSECURE_CLI_PASSWORD=1` is set with a warning | `tests/security/TH-07-ps-leak.rs`: spawn extract with `--password foo`, assert exit 64 with hint |
| TH-08 | A5 | PDF content disclosed via debug logs | Logging policy (see Audit Logging below): NEVER log PDF bytes, password values, bearer tokens, or extracted text content at any level. Audit-log lines reference fingerprint, not path | `tests/security/TH-08-log-audit.rs`: run extract with `--debug` over `tests/fixtures/security/sensitive.pdf`, grep the log for known content strings; any match fails the test |
| TH-09 | A1 | XSS in inspector frontend: crafted PDF embeds `<script>` in a text span which the inspector renders as HTML | Inspector renders extracted text as `<text>` SVG content (not `innerHTML`); the frontend SHALL never use `innerHTML`/`outerHTML` with extraction output; CSP header `default-src 'self'; script-src 'self'` set on every inspector response | `tests/security/TH-09-inspector-xss.rs` against `tests/fixtures/security/xss-payload.pdf`; assert no script execution via headless browser |
| TH-10 | Local-FS attacker | Cache poisoning: malicious co-tenant writes a bogus cache entry whose key collides with a legitimate fingerprint | Each cache entry MUST store an integrity hash (HMAC-SHA-256 over `fingerprint || extraction_options || output_blob` with a per-cache random key created on `cache init`); reads verify the HMAC and treat mismatch as a miss with `CACHE_INTEGRITY_FAIL` diagnostic | `tests/security/TH-10-cache-poison.rs`: write a forged entry; assert it is rejected and the real extraction re-runs |

### Supply Chain Considerations

| Concern | Policy |
|---|---|
| `Cargo.lock` | Checked in for binary crates (`pdftract-cli`, `pdftract-py`). SHOULD be `.gitignore`d for the `pdftract-core` library crate so downstream consumers can resolve their own versions. |
| `cargo audit` | Runs in Phase 0 CI on every PR. Advisories of severity ≥ medium block merge. Severity-low advisories file a tracking issue but do not block. Daily cron re-runs against `main` and opens an issue on any new advisory. |
| `cargo deny` — licenses | Permitted licenses for default features: MIT, Apache-2.0 (with or without LLVM exception), BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-DFS-2016, MPL-2.0 (file-level only). GPL / AGPL / LGPL are FORBIDDEN in default features; an `agpl-tools` feature MAY surface AGPL-licensed optional code provided the binary built with that feature is shipped as a separate artifact. |
| `cargo deny` — bans | Forbidden: `openssl-sys`, `native-tls`, `git2`, `libgit2-sys` (we use `rustls`; no git CLI dependency). Minimum versions: `ring >= 0.17.5`, `rustls >= 0.23`. Duplicate-version policy: a duplicated major version produces a warning; a duplicated major across direct deps produces an error. |
| Build-time data files | `build/font-fingerprints.json` and `build/glyph-shapes.json` have SHA-256 checksums committed in `build/CHECKSUMS.sha256`. `build.rs` verifies checksums on every build; a mismatch aborts the build with a clear error pointing to the regeneration script. |
| Dependency update policy | Renovate runs monthly. Patch-level updates auto-merged after CI green. Minor-level updates require maintainer review. Major-level updates require an ADR. New direct deps (any version) require a written justification in the PR and a Dependency Matrix entry. |
| Vendored deps | NONE. Everything via crates.io. NO git deps in published crates. Pre-release deps (`-alpha`, `-beta`, `-rc`) are FORBIDDEN in default features. |
| Backup mirror | Quarterly `cargo vendor` snapshots are committed to `ardenone/declarative-config` under `build-mirrors/pdftract/<quarter>/`. These exist purely for incident recovery (registry outage, mass-yank event); they are NOT used in the normal build path. |
| Release artifact signing | GitHub Releases include `pdftract.<triple>.sha256` and a `provenance.intoto.jsonl` SLSA Level 2 attestation generated by the Argo runner. Code-signing for macOS/Windows binaries is tracked in OQ-10. |

### Secrets Handling

The following secrets pass through pdftract at runtime: **PDF passwords**, **MCP bearer tokens**, **inspector tokens**, and (transitively) **HTTP basic-auth headers attached to `remote` fetches**. Each has a defined ingress channel, a no-leak guarantee, and a rotation procedure.

**PDF password.** Accepted via:
- `--password-stdin` flag (CLI; read one line from stdin)
- `PDFTRACT_PASSWORD` env var
- Python `password=` kwarg
- MCP `password` parameter (in the request body, NOT URL)
- `pdftract serve` `password` form field (multipart body)
- `--password VALUE` plain CLI arg is REJECTED unless `PDFTRACT_INSECURE_CLI_PASSWORD=1` is set, in which case a stderr warning is emitted and the bare value is masked in any internal echo. See TH-07.

PDF passwords MUST be redacted in:
- `--capture-diagnostics` archive
- `--progress-json` event stream (`{"event":"password_received"}` — never the value)
- Audit logs (`password=<redacted>`)
- Stack traces and panic messages (the password value is never embedded in error strings)

**MCP bearer token.** Accepted via:
- `--auth-token-file PATH` (PATH contains only the token, terminating newline stripped) — RECOMMENDED
- `PDFTRACT_MCP_TOKEN` env var
- `--auth-token VALUE` plain CLI arg is REJECTED unless `PDFTRACT_INSECURE_CLI_TOKEN=1` is set
- Public-bind without a token aborts startup (see TH-03)

Tokens never appear in `ps`, audit logs, request logs, or stack traces. The token value is held in a `secrecy::SecretString` to prevent accidental `Debug` print.

**Inspector token.** Same channels and same redaction rules as the MCP bearer token. The `inspect` subcommand auto-generates a single-use token on launch and prints it to stderr along with the launch URL; the token is not persisted.

**HTTP basic auth on remote fetches.** Embedded credentials in URLs (`https://user:pass@host/...`) are accepted but the password component MUST be stripped from any log line and any diagnostic emission. The full URL is preserved in memory for the duration of the fetch only.

**Profile YAML files.** Profile loaders MUST reject any YAML containing top-level `password:`, `token:`, `secret:`, or `api_key:` keys with `PROFILE_SECRETS_FORBIDDEN`. Profiles are checked into git in the `profiles/community/` directory; secrets in them would be a public disclosure incident.

**Rotation.** Tokens are rotated by stopping the server, regenerating the token, and restarting. There is no in-process rotation API. Rotation cadence is recommended at 90 days, enforced by deployment tooling (out of pdftract scope).

### Audit Logging

pdftract uses the standard `log` crate facade with `env_logger` as the default backend. Levels follow `env_logger` semantics: `error` < `warn` < `info` < `debug` < `trace`. The `RUST_LOG` env var controls verbosity; default is `pdftract=info`.

**Always logged at `info`:**
- Subcommand invocation (subcommand name, version, feature set — NOT arguments)
- `serve` / `mcp --bind` startup with bind address and chosen transport
- Cache hits and misses (fingerprint, decision)
- Profile resolution decisions (matched profile name, priority)
- Significant configuration choices (e.g. `cache enabled at DIR`, `OCR fallback armed`)

**Logged at `debug` (only when `RUST_LOG=pdftract=debug` is set):**
- Per-phase timing breakdown
- Resolved `ExtractionOptions` (with passwords redacted, paths preserved)
- Per-page glyph and span counts
- Cache key derivation steps (without the resulting key bytes)

**NEVER logged at any level:**
- Password values (PDF, MCP, inspector)
- Bearer-token values
- PDF byte contents (not even at `trace`)
- Full extracted text (only span counts, page counts, and fingerprints)
- Profile file contents when the profile references secrets (the loader rejects such profiles per `PROFILE_SECRETS_FORBIDDEN`)
- `Cookie`, `Authorization`, or `Proxy-Authorization` HTTP headers

**Logged ONLY when `--audit-log FILE` is set:** Per-request audit lines in newline-delimited JSON. Each line carries:
```
{"ts":"2026-05-16T12:34:56Z","client_ip":"10.0.0.1","tool":"extract","fingerprint":"pdftract-v1:abcd…","duration_ms":1234,"status":200,"diagnostics":["XREF_REPAIRED"]}
```
The `client_ip` field is the HTTP peer for `serve` / `mcp --bind`; absent for stdio MCP. `fingerprint` is logged instead of the path or URL.

**Rotation.** pdftract does not rotate logs. Operators MUST configure `logrotate` (or equivalent) on the audit-log file. The `--audit-log` flag accepts `-` for stdout; in that case rotation is the responsibility of the supervisor.

**Test fixture.** `tests/security/TH-08-log-audit.rs` (per the security matrix) runs an extraction over a sensitive fixture with `RUST_LOG=pdftract=trace` and asserts that no known-sensitive substring appears in the captured log buffer.

---

## Anti-Patterns

The following patterns are NEVER acceptable in pdftract code. PR reviews block on them; clippy lints catch the ones that can be lint-detected. The Why column explains the failure mode — each anti-pattern has caused a real-world bug in similar projects.

| Anti-pattern | Why it fails | Correct approach |
|---|---|---|
| `panic!` / `unwrap()` / `expect()` in `pdftract-core` (library code) | A library panic propagates through the FFI/PyO3 boundary as an abort or a `RuntimeError`, killing the host process. Per INV-8, all errors are recoverable diagnostic emissions. | Emit a diagnostic via the Phase 1.6 error model; return `PdfNull` or a default value; let the caller decide how to react. Test code (`#[cfg(test)]`) MAY use `unwrap()` — production lib code MUST NOT. |
| Blocking the rayon thread pool with I/O | Rayon's thread pool is sized for CPU work. A page worker that blocks on a remote fetch stalls the pool and reduces throughput proportionally. | Use `spawn_blocking` to bridge to tokio (Phase 6.4) or do I/O outside the rayon job. For Phase 1.8 remote source, the prefetch hint allows the I/O to overlap with CPU work. |
| Holding the Python GIL across rayon work | Acquiring the GIL inside a rayon job serialises all parallel work behind the GIL, defeating rayon entirely. | Phase 6.3 releases the GIL via `py.allow_threads(...)` before the rayon-driven extraction starts; reacquires only to construct the Python return value. |
| Loading the whole PDF into memory when memmap2 / range-read would suffice | A 5 GB PDF should NOT consume 5 GB of RSS. mmap relies on the OS page cache for on-demand paging; HTTP range reads fetch only what the extraction touches. | All file I/O goes through the Phase 1.8 `PdfSource` trait. Code that does `fs::read(path)?` of an unbounded file is rejected at code review. |
| Re-initialising the Tesseract `TessBaseAPI` per page | Tesseract initialisation is ~200 ms (parses language data, loads neural-net weights). Doing this per page adds 100× more startup cost than the OCR itself. | One `TessBaseAPI` per worker thread, stored in `thread_local!`. The Phase 5.4 spec mandates this. |
| Inflating an unbounded zlib stream without `max_decompress_bytes` | A 10 KB zlib stream can expand to multi-GB (compression bomb). Unbounded decompression is a DoS vector for any service accepting PDF uploads. | Phase 1.5 enforces `ExtractionOptions.max_decompress_bytes` (default 512 MB). New decoder paths MUST check this limit. |
| Following `/Prev` xref chains without cycle detection | A malicious or corrupt PDF can craft an xref `/Prev` cycle that loops forever. | Phase 1.3 tracks visited xref offsets; the second visit terminates the chain with an `XREF_REPAIRED` diagnostic. |
| Calling out to external commands without `--no-interactive` / non-interactive bypass | A subprocess that prompts for input (passwords, "are you sure?") hangs the extraction. | pdftract does not shell out for extraction work. The only subprocess is the OS browser launcher in Phase 7.9, which is opt-out via `--no-open`. |
| Writing to stdout from a `serve` handler | The serve handler returns HTTP responses; stdout is a server-process log channel. Writes to stdout interleave with axum's response writes if the framework is configured to log there. | All operational messages go through the `log` macros, which route to stderr. The HTTP response is the sole stdout consumer in non-MCP modes; in MCP stdio mode, JSON-RPC frames are the sole consumer. |
| Logging password values or PDF byte contents | Passwords appear in `--password` flags and `password` form fields. PDF bytes can contain personally identifiable information. Either in a log file is a data-breach incident. | Passwords are redacted in `--capture-diagnostics` and never logged. PDF bytes are not logged at any level; only the SHA-256 of the input (= fingerprint) is permitted in logs. |
| Mixing JSON-RPC and human prose on stdout in MCP stdio mode | A stray `println!()` or `eprintln!()` mis-routed to stdout corrupts the JSON-RPC stream. The client typically disconnects with a parse error and the user sees "MCP server crashed". | Phase 6.7 stdio mode uses an internal stdout-routing guard: all `log` output goes to stderr; only the JSON-RPC framer writes to stdout. A clippy lint denies `println!()` in `crates/pdftract-cli/src/mcp.rs`. |
| Re-using a `TessBaseAPI` across threads | `TessBaseAPI` is NOT `Send`. Sharing it across threads via `Arc` produces undefined behaviour (the Tesseract C++ object has thread-affine state). | One `TessBaseAPI` per worker thread, in `thread_local!`. Type-system enforced: `TessBaseAPI` is `!Send`. |
| Using `serde_json::Value` as the public output type | `Value` is dynamically typed; consumers need to guess the schema. Adding a field becomes silent breakage. | Phase 6.1 uses concrete `serde`-derived structs with named fields. The JSON Schema at `docs/schema/v1.0/pdftract.schema.json` is the source of truth. |
| Silent default for `--cache-dir` (e.g. always-on cache without explicit opt-in) | Hidden state on the filesystem creates surprise: the user gets stale results after an upgrade, with no clue why. | Cache is opt-in: `--cache-dir DIR` is required. `serve` mode requires the operator to pass `--cache-dir` explicitly. |
| Hard-coding paths assuming Linux (e.g. `/var/data`) | The binary targets musl Linux, macOS, and Windows. Hard-coded paths break on the latter two. | Use `directories` crate idioms (`$XDG_CONFIG_HOME`, `~/Library/Application Support/...`, `%APPDATA%\...`). Phase 7.10 profile search path is the worked example. |

---

## Phase 0: CI Infrastructure (Prerequisite)

**Goal:** Establish the Argo Workflows CI pipeline required by all subsequent phases. Binary releases and Python wheel builds are automated from day one; no milestone can ship without this.
**Complexity:** Medium
**Estimate:** 3–5 days
**Delivers:** `pdftract-ci` and `pdftract-py-ci` WorkflowTemplates active in `iad-ci`; milestone tags trigger automated releases to GitHub Releases and PyPI.

Create Argo WorkflowTemplate `pdftract-ci` in `jedarden/declarative-config → k8s/iad-ci/argo-workflows/`. The template must:

1. Build the Rust binary for five targets using `cross` (Docker-based cross-compilation):
   - `x86_64-unknown-linux-musl`
   - `aarch64-unknown-linux-musl`
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`
   - `x86_64-pc-windows-gnu`
2. Run `cargo test --features default,serve,decrypt` (excludes `ocr` and `python`) on `x86_64-unknown-linux-musl`. Run `cargo test --all-features` on `x86_64-unknown-linux-gnu` using the standard Debian-based Docker image with `apt-get install -y tesseract-ocr libleptonica-dev libtesseract-dev`. This ensures musl cross-compilation is tested for the production binary feature set, while the full test suite (including OCR integration tests) runs on glibc where system libraries are available.
3. Publish binaries to GitHub Releases on milestone tags via `gh release upload`.
4. Build the PyO3 wheel via the `pdftract-py-ci` template (separate template, uses a `ghcr.io/rust-cross/manylinux` base image for Linux wheels; `osxcross` toolchain for macOS targets; `cross` with `x86_64-pc-windows-gnu` for the Windows `.whl`). All five triples ship to PyPI on milestone tags.

The `pdftract-py-ci` WorkflowTemplate YAML is created in Phase 0 as a stub with placeholder steps (exit 0) to establish the CI infrastructure. Actual wheel-build logic is filled in during Phase 6.3 implementation.

**Phase 0 must be complete before Phase 1 code review begins.**

---

## Phase 1: Core PDF Parser (Foundation)

**Goal:** Parse any PDF object, resolve xref tables, decode streams. No text extraction yet.  
**Complexity:** Complex  
**Estimate:** 3–4 weeks  
**Delivers:** `pdftract-core::parser` module usable in unit tests.

### 1.1 Lexer

Tokenize the raw byte slice into PDF tokens. This is the lowest layer; all higher-level parsers call into it.

**Tokens to produce:**
- Boolean (`true`, `false`)
- Integer (`123`, `-7`)
- Real (`3.14`, `-.5`)
- String literals: literal strings `(...)` with all escape sequences (`\n`, `\r`, `\t`, `\\`, `\(`, `\)`, `\ddd` octal, line-continuation `\<newline>`), and hex strings `<...>` (odd-length padded with trailing zero nibble)
- Name objects: `/Name`, with `#XX` hex escape expansion, NUL-byte rejection, and length limit (127 bytes per spec)
- Array delimiters: `[`, `]`
- Dictionary delimiters: `<<`, `>>`
- Stream keyword: `stream` (validated against following `\n` or `\r\n`)
- End-stream keyword: `endstream`
- Indirect object markers: `obj`, `endobj`, `R`
- Comments: `%` to end of line (discarded)
- Whitespace: consumed between tokens (0x00, 0x09, 0x0A, 0x0C, 0x0D, 0x20)

**Crates:** none (hand-written; `nom` is an option but PDF's grammar is simple enough to avoid the dependency)

**Critical tests:**
- String with nested balanced parentheses: `(foo (bar) baz)` → `foo (bar) baz`
- String with octal escape at end of string: `(abc\101)` → `abcA`
- Hex string with odd length: `<4>` → `\x40`
- Name with `#20` → space character
- Name with `#00` → rejected (NUL in name is invalid per spec; emit diagnostic)
- Name object length limit: 127 bytes, applied to the raw byte count in the file before `#XX` hex escape expansion, matching PDF spec section 7.3.5; if exceeded, truncate the name at 127 bytes and emit `STRUCT_INVALID_NAME` diagnostic
- Whitespace-only file → empty token stream, no panic

### 1.2 Object Parser

Parse the token stream into the PDF object model.

**Types:**
- `PdfNull`
- `PdfBool(bool)`
- `PdfInt(i64)`
- `PdfReal(f64)`
- `PdfString(Vec<u8>)` — raw bytes before any encoding interpretation
- `PdfName(Arc<str>)`
- `PdfArray(Vec<PdfObject>)`
- `PdfDict(IndexMap<Arc<str>, PdfObject>)` — preserves insertion order
- `PdfRef(u32, u16)` — object number, generation number
- `PdfStream { dict: PdfDict, offset: u64 }` — offset into mmap; data decoded lazily
- `PdfIndirect { id: ObjRef, obj: Box<PdfObject> }`

**Key behaviors:**
- Indirect object parsing: `N G obj ... endobj` wrapper
- Object streams (`/ObjStm`): decompress once, parse all embedded objects, cache them under their object numbers
- Circular reference guard: track in-resolution set per thread; emit `STRUCT_CIRCULAR_REF` diagnostic and return `PdfNull` on cycle

**Crates:** `indexmap` (dict), std `Arc<[u8]>` (object stream caching — no external crate needed)

**Critical tests:**
- Nested dict: `<< /A << /B 1 >> >>` — correct inner dict
- Array of mixed types: `[1 true (str) /Name null]`
- Object stream: decompress, parse all N objects, verify all ObjRefs resolve
- Self-referencing object (circular): returns PdfNull with diagnostic, no stack overflow

### 1.3 Cross-Reference Resolution

Build the complete object → byte-offset map from the file's xref structure.

**Strategies (attempted in order on failure):**
1. **Traditional xref table:** parse from `startxref` offset; 20-byte fixed-width entries; handle `\r\n` and ` \n` line endings; merge multi-subsection tables
2. **Xref streams (PDF 1.5+):** parse `/W` field widths; decompress body with FlateDecode; parse `/Index` subsections; handle type-0/1/2 entries
3. **Hybrid files:** merge traditional table (priority) with xref stream (`/XRefStm` pointer); type-2 entries from stream fill gaps not covered by traditional table
4. **Forward scan fallback:** sequential scan for `N G obj` patterns; slower but handles severely truncated or overwritten files; emit `XREF_REPAIRED` diagnostic

**Incremental updates:** When `/Prev` is present in a trailer, recursively load the previous xref revision; later revisions override earlier entries for the same object number. This handles incremental saves, linearized files, and comment-editing workflows.

**Linearized PDF detection:** Check for a `/Linearized` dictionary in the first object of the file (object at byte offset 0 or nearby). If found: (1) parse the partial xref at the beginning of the file (the 'first-page xref'), (2) parse the complete xref at the end of the file (the 'full xref'), (3) merge them with the full xref taking precedence for any object number present in both. The hint stream (`/H` entry in the Linearized dict) is parsed for page offset hints to accelerate random-access page loading but is not required for correctness. The forward scan fallback is disabled for linearized files (it would find the partial leading xref and stop).

**Crates:** `flate2` (xref stream decompression)

**Critical tests:**
- PDF with `/Prev` chain of 3 revisions: latest value of each object number wins
- Type-2 xref entry: object resolved through `/ObjStm` correctly
- Hybrid file: traditional entries override stream entries for same object numbers
- File truncated after xref: forward scan finds all objects before truncation point
- `startxref` offset off by one (common real-world corruption): forward scan triggered, `XREF_REPAIRED` diagnostic emitted

### 1.4 Document Model

Build the in-memory document model over the xref-resolved object graph.

**Structures to build:**
- **Document catalog** from `/Root`: record `/Pages`, `/Outlines`, `/MarkInfo`, `/StructTreeRoot`, `/AcroForm`, `/Names`, `/Metadata`, `/PageLabels`, `/OCProperties`
- **Page tree** (`/Pages` subtree): flatten into a `Vec<PageDict>` with inherited attributes resolved (MediaBox, CropBox, BleedBox, TrimBox, ArtBox, Resources, Rotate). Inheritance walk: page dict overrides parent dict; root `/Pages` is the ultimate fallback. If a page's `/Contents` is an array of stream references, all streams are decoded and concatenated in order before Phase 3 content stream processing begins. Graphics state is NOT reset between concatenated streams — they are treated as a single logical stream.
- **Resource dictionary inheritance:** each page gets a fully resolved `ResourceDict` merging all ancestor `/Resources` dicts (font, XObject, ExtGState, ColorSpace, Shading, Pattern, Properties namespaces). Per-key last-write-wins at the page level.
- **Encryption dictionary** detection: if `/Encrypt` present in trailer, identify handler (`/Standard` vs. custom), extract `/V`, `/R`, `/KeyLength`, `/CF`/`/StmF`/`/StrF` entries. RC4 and AES-128/256 decryption implemented via the `aes` and `rc4` crates (RustCrypto; both gated behind the `decrypt` feature, which is on by default — see Dependency Matrix). Password attempt: empty string first, then user-supplied via `ExtractionOptions.password: Option<String>` (CLI: `--password <PASSWORD>`; Python keyword arg: `password=None`; HTTP form field: `password`). On failure: emit `ENCRYPTION_UNSUPPORTED` and abort.

**Optional Content Groups (OCGs):** If `/OCProperties` is present in the catalog, read default visibility from `/OCProperties /D /BaseState` (name value `ON` or `OFF`; defaults to `ON` if absent). Each individual OCG's membership in the default ON or OFF list is given by the arrays `/OCProperties /D /ON` (array of OCG object refs that are ON by default) and `/D /OFF` (OFF by default). An OCG present in neither array inherits `BaseState`. During content stream processing (Phase 3), track the `OC` marked content tag: if a `BDC` block carries `/OC /OCGRef`, check the referenced OCG's default state. If `OFF`, suppress all glyphs within the marked content block (they are not extracted). If `ON` or no OCG present, extract normally. Emit `ocg_present: true` in document metadata. Full OCG toggle support (programmatic state changes) is deferred to Phase 7.

**JavaScript detection:** Record `contains_javascript = true` if any of the following are present: (1) `/OpenAction` value is a JavaScript action dict (`/S /JavaScript`), (2) `/AA` (Additional Actions) at document or page level contains a JavaScript action, (3) any AcroForm field's `/AA` dict contains a JavaScript action, (4) any annotation's `/A` or `/AA` dict contains a JavaScript action. JavaScript is never executed — only its presence is flagged. This check runs during document model construction and costs one dict key scan per object.

**`conformance` detection:** Parse the `/Metadata` stream (if present) as XMP XML using `quick-xml`. Extract the `pdfaid:part` and `pdfaid:conformance` elements to construct values like `PDF/A-1b`, `PDF/A-2u`. If no XMP metadata or no `pdfaid:` namespace tags are present, `conformance = null`. **`quick-xml` feature gate:** Move `quick-xml` from the `ocr` feature to `default` since conformance detection runs for all documents. **`contains_xfa` detection:** Check for the presence of `/AcroForm /XFA` key during document model construction; if present and non-null, `contains_xfa = true`.

**Crates:** `aes`, `rc4` (both via `decrypt` feature), `quick-xml` (moved to `default` feature for conformance detection)

**Outline traversal:** Walk the `/Outlines` linked list: start at `/Root /Outlines /First`; recurse by following each node's `/First` (first child) and `/Next` (next sibling) pointers until null. For each node: (1) decode `/Title` — if the string starts with the UTF-16BE BOM (0xFE 0xFF), decode as UTF-16BE; otherwise decode as PDFDocEncoding (Latin-1 with named character overrides per Table D.2 of the spec); (2) extract `/Dest` (explicit destination array: `[page_ref /XYZ left top zoom]` etc.) or `/A /GoTo /Dest` (action-based destination), recording the page index and anchor type; (3) record `/Count` (positive = expanded, negative = collapsed). Serialize as a recursive `outline` array in the document-level JSON output. A critical test: PDF with 3-level bookmark hierarchy — all levels, titles, and page destinations extracted correctly.

**Critical tests:**
- Page inheriting MediaBox from grandparent `/Pages` node
- Page overriding `/Resources /Font` partially (merged, not replaced)
- `PageLabels` number tree: pages with roman-numeral labels followed by arabic labels
- Encrypted file with empty owner password: decrypts successfully
- Encrypted file with unknown handler: `ENCRYPTION_UNSUPPORTED` error, no crash

### 1.5 Stream Decoder

Decode stream data through its filter pipeline. Called lazily when stream content is first accessed.

**Filters to implement (in priority order):**

| Filter | Implementation | Notes |
|---|---|---|
| `FlateDecode` | `flate2::read::ZlibDecoder` | Apply predictor post-inflate: TIFF predictor 2, PNG predictors 10–15 (per-row byte selects predictor for value 15) |
| `LZWDecode` | `lzw` crate | `/EarlyChange` parameter: 1 = early (default), 0 = late; same predictor support as FlateDecode |
| `ASCII85Decode` | hand-written | `z` shortcut, partial final group, `~>` terminator, embedded whitespace ignored |
| `ASCIIHexDecode` | hand-written | Digit pairs, whitespace ignored, `>` terminator |
| `RunLengthDecode` | hand-written | Length byte: 0–127 = copy next N+1 bytes literally; 129–255 = repeat next byte 257-N times; 128 = EOD |
| `DCTDecode` | passthrough | Pass raw JPEG bytes to consumer; validate SOI/EOI markers; log `/ColorTransform` for consumer |
| `JBIG2Decode` | passthrough | Pass raw JBIG2 bytes; log global stream reference. For OCR path: requires `full-render` feature (pdfium-render decodes JBIG2 internally). Without `full-render`, emit `OCR_JBIG2_UNSUPPORTED` diagnostic and skip those image regions; JBIG2 is rare in modern PDFs. |
| `JPXDecode` | passthrough | Pass raw JPEG 2000 bytes. For OCR path: requires `full-render` feature (pdfium-render decodes JPEG 2000 internally) or system `libopenjp2`. Without either, emit `OCR_JPX_UNSUPPORTED` diagnostic and skip the page. |
| `CCITTFaxDecode` | passthrough | Pass raw CCITT bytes. For OCR path: `image` with `tiff` feature decodes Group 3/4 CCITT; this requires `libtiff` system library. Alternatively, require `full-render` feature. Emit `OCR_CCITT_UNSUPPORTED` if neither is available. |
| `Crypt` | identity only | `/Name /Identity` handled; custom crypt filters emit `ENCRYPTION_UNSUPPORTED` |

**Filter pipeline:** `/Filter` is a name or array; `/DecodeParms` is aligned or absent. Apply decoders in order. Mismatched lengths: apply defaults, log diagnostic.

**Error recovery:** zlib decompression error mid-stream: return bytes decoded so far, emit `STREAM_DECODE_ERROR` diagnostic. Never abort the page. **Decompression limit:** The stream decoder enforces `ExtractionOptions.max_decompress_bytes` (default: `512 * 1024^2` = 512 MB per document; see Memory targets). Any single stream or cumulative document total that exceeds this limit triggers a `STREAM_BOMB` diagnostic and returns the bytes decoded so far. This limit applies to all modes (CLI, Python, HTTP serve).

**Crates:** `flate2`, `lzw`, `image` (JPX/CCITT raster decode for OCR path) — DCTDecode SOI/EOI marker validation is a 4-byte inline check; no external crate needed

**Critical tests:**
- FlateDecode with PNG predictor 15 (per-row): all six predictor types appear in one stream, all decoded correctly
- LZWDecode with EarlyChange=0: verify against known reference output
- ASCII85 with `z` shortcut and odd final group
- Filter array `[/ASCII85Decode /FlateDecode]`: decoded in order
- FlateDecode with truncated zlib stream: partial output returned, diagnostic emitted
- DCTDecode: raw bytes passed through unchanged; SOI marker present

### 1.6 Error Recovery

Cross-cutting concerns for malformed files.

**Strategies:**
- **Truncated file at EOF:** forward xref scan; any `endobj` before truncation point is valid
- **Corrupt xref entry (bad offset):** attempt to parse at listed offset; if first bytes are not `N G obj`, skip entry with diagnostic; do not remove from xref map (other objects may be valid)
- **Missing required dict key:** return `PdfNull`, emit `STRUCT_MISSING_KEY` diagnostic with object number; caller must handle null gracefully
- **Integer overflow in object dimensions:** clamp to `i32::MAX` and log; do not panic
- **Circular object reference:** detected via per-thread resolution stack; return `PdfNull` with diagnostic

**Critical tests:**
- File where 30% of xref entries point to wrong offsets: remaining 70% extracted correctly
- Missing `/MediaBox` on every page: default letter size (612×792) used, diagnostic emitted per page
- Object with `endobj` missing: parser reads to next `N G obj` pattern and continues

### 1.7 PDF Structural Fingerprint

Compute a reproducible 256-bit content hash that identifies the *semantic* content of a PDF independent of metadata churn, byte ordering, and producer-tool re-saves. The fingerprint is exposed in JSON output (`metadata.pdf_fingerprint`), via the `pdftract hash` subcommand, and is the cache key for Phase 6.9 and the binding identity in Phase 6.8 receipts.

**Inputs to the hash (Merkle-style, deterministic order):**
1. Page count (u32, big-endian)
2. Per page in `page_index` order:
   - SHA-256 of each decoded content stream (Phase 1.5 output), concatenated in stream-array order
   - SHA-256 of the resolved resource dict (font fingerprints from Phase 2 Level 3 + XObject stream fingerprints + ExtGState entries that affect rendering)
   - Page geometry: MediaBox, CropBox, Rotate — canonicalized to 4-decimal-place fixed-point integers (× 10⁴, rounded half-to-even) to eliminate float-representation noise
3. SHA-256 of the structure tree if `is_tagged`; the all-zero hash otherwise
4. Catalog feature flag byte: `is_encrypted | contains_javascript << 1 | contains_xfa << 2 | ocg_present << 3`

**Inputs deliberately excluded:**
- `/Producer`, `/Creator`, `/CreationDate`, `/ModDate`, `/Author`, `/Title`, `/Subject`, `/Keywords` — metadata, not content
- `/ID` array (varies per save even for byte-identical content)
- XMP metadata stream (`/Metadata`) — orthogonal to content
- xref byte layout and object number assignment — objects are addressed by their content hash, not by file position
- Inline whitespace in content streams (lexer-normalized to a single 0x20 between tokens before hashing)

**Output format:** Lowercase hex SHA-256 prefixed with the algorithm version, e.g. `pdftract-v1:a7f3...`. The version prefix means a future fingerprint algorithm change cannot silently produce mismatches against historical fingerprints.

**CLI:** `pdftract hash FILE.pdf` prints `pdftract-v1:<hex>\n` to stdout. Exit code 0 on success; 2 if the file is corrupt; 3 if the file is encrypted and no password was supplied; 4 if the path or URL cannot be read.

**Acceptance criteria (CI-gated):**
- Reproducibility: identical input produces byte-identical fingerprint across 100 invocations, across glibc/musl, across `x86_64` and `aarch64`
- Same PDF re-saved by Acrobat, pdftk, or QPDF with no content edit: identical fingerprint
- Same PDF with `/Title`, `/Producer`, or `/CreationDate` changed (and nothing else): identical fingerprint
- One paragraph edited in a 100-page PDF: fingerprint differs

**Crates:** `sha2` (already in default deps); no new dependencies

**Critical tests:**
- Same PDF saved by Acrobat and pdftk side-by-side: same fingerprint
- PDF with `/CreationDate` differing only: same fingerprint
- PDF with one glyph removed: different fingerprint
- 10 invocations on the same file: identical fingerprint each time
- Linearized PDF and its non-linearized equivalent: same fingerprint (linearization is byte-layout-only, not a content change)

### 1.8 Remote Source Adapter (HTTP Range Reads)

Extract from PDFs hosted in cloud storage without downloading the full file. Cuts bandwidth by 95%+ for partial-page extractions from large documents. Enables `pdftract extract https://...` and `pdftract grep https://...`.

**Architecture change to Phase 1:** Replace the implicit `memmap2`-only I/O assumption with a `PdfSource` trait abstracting random access:

```rust
trait PdfSource: Read + Seek + Send + Sync {
    fn len(&self) -> u64;
    fn read_range(&self, offset: u64, length: usize) -> io::Result<Bytes>;
    /// Hint that the given range will be needed soon. No-op for local files.
    fn prefetch(&self, offset: u64, length: usize) { let _ = (offset, length); }
}
```

**Implementations:**
- `MmapSource`: memory-mapped local file — default, behavior unchanged from the original Phase 1 spec
- `FileSource`: plain `Read + Seek` over `File` — fallback when mmap fails (e.g., FUSE mounts, `/proc`, named pipes)
- `HttpRangeSource`: HTTP `Range:` request reader with a bounded LRU page-cache of 64 × 64 KB blocks (4 MB total per document)

**HTTP fetch sequence:**
1. `HEAD` request → record content-length and verify `Accept-Ranges: bytes`
2. Initial `Range: bytes=-16384` (tail) → parse `startxref`, trailer, and the root xref subsection
3. As objects are dereferenced, fetch the byte range `[/Contents stream offset, offset + length)` per page on demand
4. Resources (fonts, XObjects) fetched lazily on first reference and cached for the document lifetime
5. Cross-reference streams (PDF 1.5+) fetched when traditional xref is unavailable; the forward-scan fallback (Phase 1.3 strategy 4) is disabled for remote sources because it would require downloading the entire file

**Server requirements and fallback:** If `Accept-Ranges` is absent, or if a `Range` request returns 200 instead of 206, emit `REMOTE_NO_RANGE_SUPPORT` and fall back to streaming the entire response body into a temp file, then memory-map that. This preserves correctness at the cost of bandwidth on non-compliant servers.

**Authentication:**
- HTTPS basic via URL credentials (`https://user:pass@host/path`)
- Custom headers via `--header 'Authorization: Bearer ...'` (repeatable flag)
- S3 (SigV4) is deferred to a future `s3` feature; users today can use a presigned URL or a proxy

**CLI:**
- `pdftract extract https://example.com/doc.pdf` — auto-detects HTTPS scheme; transparently uses `HttpRangeSource`
- `pdftract extract --pages 47-52 https://example.com/huge.pdf` — partial extraction
- `pdftract extract --header 'Authorization: Bearer T0K3N' https://api.example.com/file.pdf`
- `pdftract grep "invoice" https://example.com/doc.pdf` — works identically over the network

**New CLI flag (cross-cuts Phase 6.1):** `--pages RANGE` accepts comma-separated, 1-based page ranges (e.g. `1-5,7,12-`). Default: all pages. Applies in all transport modes (local and remote) and all output formats. Out-of-range page numbers emit `PAGE_OUT_OF_RANGE` diagnostics and are skipped.

**Acceptance criteria (`remote` feature):**
- 500-page PDF on a remote server: extract pages 47–52 only with total downloaded < 5 MB
- Server without Range support: fall back to temp-file download, emit warning, complete extraction successfully
- Network failure mid-extraction: partial result with `REMOTE_FETCH_INTERRUPTED` diagnostic; no panic; exit code 5
- TLS-handshake failure: clear error message with the certificate-chain reason; exit code 6

**Feature flag:** `remote` (default OFF; adds `ureq` ~500 KB). `ureq` chosen over `reqwest` for binary size: no async runtime, rustls backend, no native TLS dependency. The CLI's default-feature binary does not include `remote`; the `pdftract:full` Docker image does.

**Crates:** `ureq` (0.10, rustls feature) — `remote` feature only.

**Critical tests:**
- Mock HTTP server with Range support: extract page 5 of a 100-page PDF, < 100 KB transferred
- Mock server without Range: fallback to full download with documented warning
- Mock server returning 416 (Range Not Satisfiable): emit diagnostic; retry without Range
- Document with a linearized hint stream: page-offset hints utilized to predict and prefetch
- Connection drop after the trailer is fetched: extraction emits `REMOTE_FETCH_INTERRUPTED`; pages already buffered are still emitted; subsequent pages are absent

---

## Phase 2: Font and Encoding Pipeline

**Goal:** For any character code from a content stream, resolve a Unicode scalar value and a confidence score.  
**Complexity:** Complex  
**Estimate:** 3–4 weeks  
**Depends on:** Phase 1 complete  
**Delivers:** `pdftract-core::font` module

### 2.1 Font Type Detection

Load and classify the font from the resource dictionary.

**Font types and loading strategy:**

| Subtype | Font Program Location | Metric Source |
|---|---|---|
| `Type1` | `/FontFile` in FontDescriptor | `/Widths` array |
| `Type1` (Standard 14) | No font program; synthesized | Known metrics table (hardcoded) |
| `TrueType` | `/FontFile2` | `/Widths` array; `hmtx` for verification |
| `Type0` (composite) | Descendant CIDFont | `/DW`, `/W` array in CIDFont dict |
| `CIDFontType0` | `/FontFile3` (`/CIDFontType0C`) | `/DW`, `/W` |
| `CIDFontType2` | `/FontFile2` or `/FontFile3` (`/OpenType`) | `/DW`, `/W` — `/CIDToGIDMap` may be the name `/Identity` (GID==CID) or a stream (decoded as 2-byte big-endian GID array) |
| `Type3` | `/CharProcs` content streams | `/Widths` |
| OpenType (CFF) | `/FontFile3` (`/OpenType`) | `hhea`/`hmtx` via `ttf-parser` |

**Font subset detection:** Many embedded fonts are subsets with name prefix like `ABCDEF+Helvetica`. Strip the six-uppercase-letter prefix before looking up Standard 14 or glyph name tables.

**Crates:** `ttf-parser`, `owned_ttf_parser`

**Critical tests:**
- Standard 14 font (no embedding): correct metrics returned without font file
- Subset font `ABCDEF+Times-Roman`: stripped to `Times-Roman`, standard metrics used
- CIDFontType2 with `/CIDToGIDMap /Identity`: GID == CID for all lookups
- CIDFontType2 with `/CIDToGIDMap` as a stream: decode the stream (FlateDecode), interpret as a flat array of 2-byte big-endian GID values indexed by CID (`CIDToGIDMap[CID*2 .. CID*2+2]` → GID); array length is 2 × (max CID + 1)
- OpenType CFF font: metrics via `ttf-parser`'s CFF support

### 2.2 Encoding Resolution

Map character codes → Unicode. Four-level fallback chain with `unicode_source` tag on each result.

**Level 1: ToUnicode CMap**

Parse the `/ToUnicode` stream as a CMap program. CMap syntax to implement:
- `beginbfchar` / `endbfchar`: `<srcCode> <dstHex>` pairs; `<dstHex>` may be a UTF-16BE multi-codepoint sequence for ligature expansion
- `beginbfrange` / `endbfrange`: `<lo> <hi> <dst>` (contiguous single-codepoint range) or `<lo> <hi> [<d0> <d1> ...]` (explicit array for non-contiguous targets)
- `usecmap` directive: inherit from named CMap (e.g., `Adobe-Japan1-UCS2`)
- Comment lines (`%`) stripped

Successful lookup: set `unicode_source = "to_unicode"`, `confidence = 1.0`.  
Result is U+FFFD or empty: fall through to Level 2.

**Level 2: Encoding vector + AGL**

Map character code → glyph name via the font's `/Encoding`:
- Named encodings: `WinAnsiEncoding`, `MacRomanEncoding`, `MacExpertEncoding`, `StandardEncoding`, `SymbolEncoding`, `ZapfDingbatsEncoding` — hardcoded tables
- `/Differences` array: sparse overlay on top of base encoding; format `[n /GlyphName1 /GlyphName2 ...]` (n is starting code)

Map glyph name → Unicode via Adobe Glyph List (AGL 1.4, ~4400 entries, compiled in as a static phf::Map). Also support AGLFN (friendly names).

Set `unicode_source = "agl"`, `confidence = 0.9`.

**Level 3: Font fingerprint cache**

Hash the embedded font program (SHA-256 of the raw font program stream bytes, computed via the `sha2` crate). Look up in a bundled database of known font checksums → per-glyph Unicode mapping tables. Initially populated with the most common 200 commercial fonts.

**Database spec:** The database is a compile-time `phf::Map<[u8; 32], &'static [(u16, char)]>` where the key is the 32-byte SHA-256 digest of the raw font program stream (the bytes of the `/FontFile`, `/FontFile2`, or `/FontFile3` stream after filter decoding, before any interpretation) and the value is a slice of `(glyph_id, unicode_char)` pairs covering every mapped glyph in that font. The map is generated at build time from a JSON source file (`build/font-fingerprints.json`) by a `build.rs` script that emits the `phf_codegen` output. **Estimated binary footprint:** ~500 KB added to the stripped binary, within the 4 MB default-feature budget (documented here as an approved allocation). **Source:** Initially curated from open-source font metric data — Adobe's publicly available font databases and Google Fonts `cmap` metric exports. The JSON source file is the authoritative artifact; PRs that add new fonts add entries to `build/font-fingerprints.json`. The database is not user-extensible at runtime.

If the font has no embedded program (Standard-14 fonts or fonts with no `/FontFile`, `/FontFile2`, or `/FontFile3`), skip Level 3 and proceed directly to Level 4. Standard-14 fonts are guaranteed to have AGL-compatible glyph names, so Level 3 is normally unreachable for them; this is a defensive guard.

Set `unicode_source = "fingerprint"`, `confidence = 0.85`.

**Level 4: Glyph shape recognition**

Render the glyph to a 32×32 grayscale bitmap rendered via `fontdue`'s rasterizer (for TrueType/OpenType glyphs) or the Type 3 content stream renderer (for Type 3 glyphs). Hash the bitmap with a perceptual hash. Look up in a bundled shape→Unicode database (see `docs/research/glyph-recognition-and-unicode-recovery.md` and Phase 2.5).

Set `unicode_source = "shape_match"`, `confidence = 0.7`.

**Failure:** Emit U+FFFD, `unicode_source = "unknown"`, `confidence = 0.0`, log `GLYPH_UNMAPPED` diagnostic.

**Crates:** `fontdue` (glyph rasterization for shape hash), `phf` (compile-time AGL hash map)

**Critical tests:**
- `ToUnicode` with multi-codepoint bfchar (`fi` ligature → `fi`): expanded to two characters
- `beginbfrange` with explicit array: non-contiguous targets resolved correctly
- `WinAnsiEncoding` code 0x92: maps to U+2019 RIGHT SINGLE QUOTATION MARK (not U+0092)
- MacRoman code 0xD2 / 0xD3: left/right double quotation marks
- Unknown glyph name not in AGL: falls through to Level 3 or 4
- Type1 font with no `/Encoding` and no `/ToUnicode`: Level 3/4 fallback triggered

### 2.3 CJK Encoding

Handle multi-byte CJK character sets for Type 0 composite fonts.

**Predefined CMaps to implement (or reference via bundled data):**
- `Identity-H` / `Identity-V`: CID == character code (passthrough)
- `UniJIS-UTF16-H`, `UniJIS-UTF16-V`: Japanese JIS → Unicode
- `UniGB-UTF16-H`, `UniGB-UTF16-V`: GB2312 → Unicode
- `UniCNS-UTF16-H`, `UniCNS-UTF16-V`: Big5/CNS → Unicode
- `UniKS-UTF16-H`, `UniKS-UTF16-V`: KS → Unicode

**Encoding decoding for raw byte sequences:**
- Shift-JIS: `encoding_rs::SHIFT_JIS`
- GB18030: `encoding_rs::GB18030`
- Big5: `encoding_rs::BIG5`
- EUC-KR: `encoding_rs::EUC_KR`

**Multi-byte code parsing:** Type 0 font's `/Encoding` CMap defines the codespace ranges (`begincodespacerange`/`endcodespacerange`). Parse the CMap to determine 1- vs. 2-byte code boundaries, then tokenize the content stream byte sequence accordingly.

**Crates:** `encoding_rs`

**Critical tests:**
- Identity-H Type 0 font with ToUnicode: CID passthrough, Unicode from ToUnicode
- Embedded Shift-JIS ToUnicode CMap: all 6879 JIS X 0208 characters resolve correctly
- Two-byte code boundary in codespace: first byte in 0x81–0xFE range triggers two-byte read; 0x00–0x7F is single-byte
- Mixed single/double-byte codes in same TJ string: all boundaries parsed correctly

### 2.4 Type 3 Font Handling

Type 3 fonts define each glyph as a content stream in `/CharProcs`. No standard Unicode mapping exists unless `/ToUnicode` is provided.

**Pipeline:**
1. Check `/ToUnicode` first (same Level 1 logic as above)
2. If absent, attempt `/Encoding` glyph name lookup (Level 2)
3. If glyph name is non-standard (arbitrary user name), rasterize the content stream to a 32×32 bitmap and apply shape recognition (Level 4)
4. Track the content stream rendering state: Type 3 glyphs can invoke other PDF operators including form XObjects; apply the same graphics state machine as Phase 3

**Metrics:** Use `/Widths`, `/FirstChar`, `/LastChar`, `/FontMatrix` to compute advance widths. `/FontMatrix` default is `[1 0 0 1 0 0]` for Type 3 (glyph units == text units); apply it to convert glyph-space advance to text space.

**Critical tests:**
- Type 3 font with meaningful `/ToUnicode`: resolved correctly
- Type 3 font with arbitrary glyph names and no ToUnicode: shape recognition fallback, `confidence = 0.7`
- Type 3 glyph stream that invokes a form XObject: recursive processing without stack overflow
- `/FontMatrix [0.001 0 0 0.001 0 0]`: advances scaled to 1/1000 of text units (matches Type 1)

### 2.5 Glyph Shape Database

The glyph shape database backs Level 4 shape recognition in Phase 2.2 and the Type 3 shape fallback in Phase 2.4. Full methodology is documented in `docs/research/glyph-recognition-and-unicode-recovery.md`.

**Perceptual hash algorithm:** Each glyph outline is rasterized to a 32×32 grayscale bitmap using `fontdue`'s rasterizer (for TrueType/OpenType glyphs) or the Type 3 content stream renderer (for Type 3 glyphs). The bitmap is then hashed using pHash (perceptual hash): apply a 32×32 DCT, retain the top-left 8×8 AC coefficients (64 values), threshold against the median of those 64 values to produce a 64-bit integer. This yields a scale-invariant hash robust to minor rendering differences.

**Database format:** A compile-time `&'static [(u64, char)]` — a sorted slice of `(pHash, char)` pairs sorted by pHash ascending. Generated at build time from a JSON source file (`build/glyph-shapes.json`) via `build.rs` (emitted as a `static` array, no `phf_codegen` needed for this structure). An exact `phf::Map<u64, char>` cannot be used here because the collision-handling requirement needs a nearest-neighbor scan over Hamming distance, not exact key lookup.

**Query algorithm:** Linear scan over all ~5,000 entries computing `(query_hash XOR entry_hash).count_ones()` for each entry. Collect all entries with Hamming distance ≤ 8; select the entry with the smallest distance. Ties broken by the Unicode frequency rank stored in the source JSON's `frequency` field (precomputed into a companion `&'static [(u64, u32)]` frequency table sorted by pHash, queried in the same pass). **Performance:** 5,000 entries × ~8 ns per XOR+popcount ≈ 40 µs worst-case scan — well within the per-page time budget. The winning character is returned with `confidence = 0.7`; if no entry falls within the 8-bit Hamming threshold, fall through to failure (U+FFFD).

**Estimated binary footprint:** ~300 KB for approximately 5,000 common glyphs (covering Latin, Greek, Cyrillic, common symbols, and extended Latin). Within the 4 MB default-feature budget.

**Source:** Glyph bitmaps are rendered from open-source fonts (Google Fonts corpus, SIL Open Font License fonts) and hashed offline. The JSON source file is the authoritative artifact; new glyphs are added by re-running the offline hash pipeline and updating `build/glyph-shapes.json`.

---

## Phase 3: Content Stream Processing

**Goal:** Execute PDF content stream operators to produce a raw glyph list with positions.  
**Complexity:** Complex  
**Estimate:** 3–4 weeks  
**Depends on:** Phase 2 complete  
**Delivers:** `pdftract-core::content` module; raw `Vec<Glyph>` per page

### 3.1 Graphics State Machine

Maintain the full graphics state stack as the content stream is executed.

**State struct fields:**
```
ctm: Matrix3x3           -- current transformation matrix
text_matrix: Matrix3x3   -- Tm (set by Tm/Td/TD/T*)
text_line_matrix: Matrix3x3  -- Tlm (reset by Td/TD/T*)
font: Option<Arc<Font>>
font_size: f64
char_spacing: f64        -- Tc
word_spacing: f64        -- Tw
horiz_scaling: f64       -- Tz (percentage, default 100)
leading: f64             -- TL
text_rise: f64           -- Ts
text_rendering_mode: u8  -- Tr (0–7)
fill_color: Color
stroke_color: Color
```

**`Color` type definition:** The `fill_color` and `stroke_color` fields above use the following enum, which covers all PDF color spaces relevant to text extraction:

```rust
enum Color {
    DeviceGray(f32),           // 0.0–1.0
    DeviceRGB([f32; 3]),       // 0.0–1.0 each
    DeviceCMYK([f32; 4]),      // 0.0–1.0 each
    Spot(Arc<str>, f32),       // (colorant name, tint 0.0–1.0)
    Other,                     // CalRGB, ICCBased, Pattern — treated as transparent
}
```

CSS hex conversion rule for the `color` field in the Span output: `DeviceRGB → #rrggbb`; `DeviceGray(v) → DeviceRGB([v,v,v]) → #rrggbb`; `DeviceCMYK([c,m,y,k]) → approximate RGB via standard formula → #rrggbb`; `Spot` and `Other → null` in the JSON output (not serialized as a color string).

**Stack operators:** `q` pushes a clone of the current state; `Q` pops. Stack depth limit: 64 (per spec); deeper push emits `GSTATE_STACK_OVERFLOW` diagnostic and discards the push (safe failure).

**Text state operators:**

| Operator | Effect |
|---|---|
| `BT` | Reset `text_matrix = identity`, `text_line_matrix = identity` |
| `ET` | End text object; discard current text matrix |
| `Tc n` | `char_spacing = n` |
| `Tw n` | `word_spacing = n` |
| `Tz n` | `horiz_scaling = n` |
| `TL n` | `leading = n` |
| `Tf name size` | Load font by resource name, set `font_size` |
| `Tr n` | `text_rendering_mode = n` |
| `Ts n` | `text_rise = n` |
| `Td tx ty` | `text_line_matrix = translate(tx, ty) * text_line_matrix`; copy to `text_matrix` |
| `TD tx ty` | Same as `Td`; also `leading = -ty` |
| `Tm a b c d e f` | Set both matrices directly |
| `T*` | Equivalent to `Td 0 -leading` |

**CTM operators:** `cm a b c d e f` — multiply CTM by the given matrix.

**Page rotation:** After all glyph bboxes for a page are computed, if the page's `/Rotate` entry is 90, 180, or 270, apply the corresponding inverse rotation matrix to all glyph bboxes so that downstream phases (baseline clustering, column detection, reading order) always operate in an un-rotated coordinate system. The page `width` and `height` in the output schema reflect the rotated page dimensions (as the viewer sees them).

**Crates:** none (hand-written matrix arithmetic; 3x3 f64 matrices, no external linear algebra dependency needed)

**Critical tests:**
- `q`/`Q` nesting 64 levels deep: succeeds; level 65 emits diagnostic
- `Td` chain: verify accumulated text_line_matrix matches manual calculation
- `Tm` followed by `Td`: Td is relative to previous text_line_matrix, not Tm
- `Tr 3` (invisible): glyph produced with `rendering_mode = 3`
- Color operators `rg`, `RG`, `k`, `K`, `cs`, `scn`: fill/stroke color tracked correctly

### 3.2 Text Operator Processing

Parse text-showing operators and produce `Glyph` structs.

**Text-showing operators:**

| Operator | Argument | Behavior |
|---|---|---|
| `Tj` | `(string)` | Show string; advance text position |
| `TJ` | `[...]` array | Alternate strings and numeric kerning adjustments |
| `'` | `(string)` | `T*` then `Tj` |
| `"` | `aw ac (string)` | Set word_spacing=aw, char_spacing=ac, then `'` |

**Per-glyph processing:**
1. Decode character code(s) from the string bytes using the current font's codespace
2. Resolve Unicode via Phase 2 font pipeline
3. Compute glyph advance width from font metrics (accounting for Tc, Tw if space glyph, Tz)
4. Compute device-space bounding box: apply text_matrix * CTM to the glyph bbox
5. Detect word boundary: if actual next-glyph x-position > expected by more than threshold → inject synthetic space
6. Advance text_matrix by advance width

**Word boundary threshold (adaptive):** Initial threshold = 0.25 * font_size. After processing 20 glyphs, compute the median actual inter-glyph gap and adjust the threshold to 1.5× that median. This adapts to per-document spacing norms. See `docs/research/word-boundary-reconstruction.md` for full formula including Tc, Tw, Tz corrections.

Three implementation requirements:
- **(a) Comparison space:** The threshold comparison is performed in **text space** (before applying the CTM). Use the glyph's advance width and gap as computed from the text matrix only; do not transform to device space before comparing.
- **(b) Recalibration window scope:** The 20-glyph recalibration window is **reset on every font switch** (`Tf` operator). Each new font starts fresh with zero samples and the fixed initial threshold.
- **(c) Bootstrap behavior:** For the **first 20 glyphs** after a font switch (or at stream start), use the fixed initial threshold of `0.25 × font_size` with no recalibration. Recalibration begins only after the 21st glyph in the current font has been processed.

**TJ kerning:** Numeric elements in a TJ array adjust the text position by `-n/1000 * font_size * Tz/100` (negative n = kern closer, positive = move apart). Large positive values (> 0.2 * font_size) produce word boundaries.

**Glyph struct:**
```rust
struct Glyph {
    codepoint: char,         // resolved Unicode or U+FFFD
    unicode_source: UnicodeSource,
    confidence: f32,
    bbox: [f32; 4],          // [x0, y0, x1, y1] in PDF user space (lower-left origin)
    font_name: Arc<str>,
    font_size: f32,
    rendering_mode: u8,
    fill_color: Color,
    is_word_boundary: bool,  // synthetic space injected before this glyph
    mcid: Option<u32>,       // MCID of innermost enclosing marked content sequence; populated during Phase 3.4 marked content tracking
}
```

**Critical tests:**
- TeX-generated PDF with no space characters: word boundaries injected at correct positions
- TJ array with large positive kerning value (word gap): space injected
- Negative TJ kern (kern tighter): no space injected
- Glyph at Tr=3: present in output with rendering_mode=3
- Font size 0 (degenerate): glyph bbox degenerates to point; no panic

### 3.3 Resource Context and Form XObject Recursion

Handle nested resource scopes introduced by form XObjects (Do operator).

**ResourceStack:** Each page starts with its resolved resource dictionary (from Phase 1.4). When a form XObject is invoked via `Do`, push a new resource scope merging the form's own `/Resources` with the current scope (form resources shadow parent resources). Pop on return.

**Form XObject execution:** Retrieve the form XObject stream, decode it, and execute it as a nested content stream. The form's `/Matrix` entry is applied to the CTM before execution; the form's `/BBox` is applied as a clipping boundary. After execution, restore the pre-form CTM.

**Cycle detection:** Track the set of form XObject object numbers currently in the execution stack. If the same object number appears twice, emit `STRUCT_XOBJECT_CYCLE` diagnostic and return without executing. Stack depth limit: 20 levels.

**Critical tests:**
- Form XObject with its own `/Resources /Font`: inner font resolved from form resources, not page resources
- Form XObject with `/Matrix [2 0 0 2 0 0]`: all glyph bboxes in form space scaled by 2
- Form XObject cycle (A invokes B invokes A): cycle detected at second A; diagnostic emitted; extraction continues
- Form XObject with empty content stream: no crash, no glyphs produced

### 3.4 Marked Content Tracking

Track BDC/BMC/EMC marked content sequences for MCID association (used in Phase 7 StructTree exploitation).

**Operators:**
- `BMC /Tag` and `BDC /Tag << props >>` or `BDC /Tag /PropName`: push tag frame with tag name and optional MCID from properties dict (`/MCID` key)
- `EMC`: pop tag frame

**Output:** Each `Glyph` carries an optional `mcid: Option<u32>` — the MCID of the innermost marked content sequence enclosing it, if any.

**Critical tests:**
- Nested BDC: innermost MCID wins for enclosed glyphs
- EMC without matching BMC (malformed): ignored; no stack underflow panic
- MCID 0: valid (zero is a legal MCID)

### 3.5 Inline Images

Detect and skip inline image data (BI/ID/EI operator sequence) without confusing the parser.

**Parsing:** `BI` signals start of inline image dict; consume key-value pairs until `ID`; then scan raw bytes for the `EI` terminator (two-byte sequence `\nEI` where the preceding byte is not a continuation of image data — the spec requires the EI to be preceded by whitespace). Extract image bytes for passthrough.

**Critical tests:**
- Inline image immediately followed by text operators: text operators parsed correctly after EI
- Inline image data containing the byte sequence `EI` in the middle: not treated as terminator (must be preceded by whitespace)

---

## Phase 4: Text Assembly and Layout

**Goal:** Transform raw `Vec<Glyph>` → structured blocks in reading order.  
**Complexity:** Complex  
**Estimate:** 3–4 weeks  
**Depends on:** Phase 3 complete  
**Delivers:** Per-page `Vec<Block>` with `Vec<Span>` in reading order; plain text output mode works

### 4.1 Glyph → Span Merging

Group consecutive glyphs into spans. A new span begins when any of the following change:
- `font_name`
- `font_size` (delta > 0.5pt)
- `rendering_mode`
- `fill_color` (normalized to RGB; spot colors treated as distinct)
- `is_word_boundary` (inject a synthetic space span or embed space in current span text)

**Span struct:**
```rust
struct Span {
    text: String,
    bbox: [f32; 4],          // union of member glyph bboxes
    font: Arc<str>,
    size: f32,
    color: Option<CssHexColor>,
    rendering_mode: u8,
    confidence: f32,         // minimum glyph confidence
    confidence_source: ConfidenceSource,
    lang: Option<Arc<str>>,  // filled in Phase 7 normalization
    flags: u8,               // SpanFlags bitmask: bit 0=bold, 1=italic, 2=smallcaps, 3=subscript, 4=superscript
}
```

**`ConfidenceSource` enum → output schema string mapping:**
```
ConfidenceSource enum → schema string:
  unicode_source "to_unicode" | "agl"          → confidence_source = "native"
  unicode_source "fingerprint"                  → confidence_source = "native"
  unicode_source "shape_match"                  → confidence_source = "heuristic"
  unicode_source "unknown" (U+FFFD)             → confidence_source = "heuristic"
  OCR path (Phase 5.4 HOCR)                    → confidence_source = "ocr"
  Phase 4.7 correction applied                  → confidence_source = "heuristic"
```

**Flag detection:**
- Bold: font name contains "Bold" or FontDescriptor `/Flags` bit 18 set or `/StemV` > 120
- Italic: font name contains "Italic"/"Oblique" or `/ItalicAngle` != 0
- Smallcaps: font name contains "SC"/"SmallCaps" or `/Flags` bit 3 set
- Subscript: `text_rise` < -0.1 * font_size
- Superscript: `text_rise` > 0.1 * font_size

**Critical tests:**
- Mixed bold/regular in one text object: span break at font change
- Word boundary between two same-font glyphs: either space appended to previous span or new space span created (implementation choice; must round-trip to correct plain text)
- Subscript with `Ts -3`: SuperScript flag NOT set, Subscript flag set

### 4.2 Line Formation

Group spans into lines by baseline proximity.

**Algorithm:**
1. Compute baseline y-coordinate for each span: `y0 + (bbox_height * 0.2)` (approximation; exact value requires font descender metrics)
2. Cluster spans with baseline within `0.5 * median_font_size` of each other → same line
3. Within a line, sort spans by x0 (left-to-right for LTR scripts)
4. **RTL detection:** If the majority of characters in a line have Unicode bidi category R or AL (right-to-left), sort spans by x1 descending and set `direction = "rtl"` on the resulting line struct

**Crates:** `unicode-bidi` (bidi character category lookup for RTL detection); clustering is otherwise a simple sort + gap scan

**Critical tests:**
- Two-column layout: columns not merged into one line (column gap exceeds threshold)
- Superscript span at higher y than baseline text: not treated as a separate line
- Arabic text: bidi R characters detected, spans sorted right-to-left

### 4.3 Column Detection

Identify column boundaries in multi-column layouts.

**Algorithm:** Collect the x0 and x1 coordinates of all spans on the page. Compute a histogram of x0 values at 1pt resolution. Gaps wider than `0.03 * page_width` with zero span coverage are column boundary candidates. Require at least 3 lines to start in each candidate column before promoting it to a confirmed column.

Apply column labels to each span. This gates the XY-cut reading order algorithm in Phase 4.5.

**Critical tests:**
- Three-column academic paper: three distinct columns detected
- Full-width heading above two-column body: heading spans all columns; body spans within columns
- Single-column page: no false column splits

### 4.4 Block Formation

Group lines into blocks (paragraphs, headings, etc.).

**Heuristics (applied in order):**
1. **Vertical gap:** gap between consecutive lines > `1.5 * line_height` → new block
2. **Indent change:** first line x0 differs from subsequent lines by > `0.03 * column_width` → paragraph indent signal; may indicate block boundary above
3. **Font size change:** median font size of next line differs from current block by > 1pt → new block
4. **Rendering mode change:** invisible (Tr=3) text separated from visible text
5. **Column boundary:** span in different column from previous span → mandatory block break

**Block kind assignment (heuristic):**
- `heading`: font size > 1.2× body median AND line count == 1 (or short)
- `header`/`footer`: block y0 in top/bottom 7% of page height AND appears on 3+ consecutive pages with identical or near-identical text. **Sequencing note:** Header/footer detection is a sequential post-processing pass executed after all pages are assembled by rayon. The pass iterates over the sorted page list, maintaining a sliding window of the last 4 pages. Blocks in the top/bottom 7% of the page that appear in ≥ 3 consecutive pages with Levenshtein distance ≤ 5% of the text length are classified `header` or `footer`. This pass runs in O(pages × blocks_per_page) and is negligible compared to per-page extraction time. Crate: `strsim` (`strsim::levenshtein` applied at the Unicode char level, not byte level).
- `paragraph`: default
- `figure`: bbox contains only image XObjects, no text glyphs
- `list`: line starts with bullet/numbered pattern (regex: `^\s*[•‣◦\-\*]\s` or `^\s*\d+[\.\)]\s`)
- `caption`: small font, follows a `figure` block within 2 lines
- `code`: all spans in the block use a monospace font (font name contains 'Mono', 'Courier', 'Code', 'Fixed', or `FontDescriptor /Flags` bit 0 set) AND the block is indented ≥ 2em relative to the surrounding body text baseline. Deferred to Phase 7 for full detection; Phase 4 emits `paragraph` for code blocks and upgrades to `code` in a post-processing pass if the monospace heuristic fires.
- `formula`: detected in Phase 7 via OpenType Math table presence (see `docs/research/opentype-math-and-formula-extraction.md`). Phase 4 emits `paragraph` for formula blocks.

**Critical tests:**
- Indented first line of paragraph: not split into two blocks
- Header text appearing on pages 1–10: classified `header` and deduplicated
- Bullet list with mixed font sizes: all items in same `list` block

### 4.5 Reading Order

Determine the reading order of blocks within the page.

**Fast path (tagged PDF):** If `is_tagged = true`, defer to Phase 7 StructTree traversal. Set `reading_order_algorithm = "struct_tree"`. Until Phase 7 is implemented (v0.1.0–v0.3.0), `is_tagged = true` pages fall through to XY-cut; `reading_order_algorithm` is set to `'xy_cut'` and a `TAGGED_PDF_STRUCT_TREE_DEFERRED` informational diagnostic is emitted. Phase 7.1 replaces this path.

**XY-cut algorithm (untagged, rectilinear layouts):**
1. Find the widest vertical whitespace gap dividing the page's text bbox into left and right halves → split into two regions
2. For each region, find the widest horizontal gap → split into top and bottom sub-regions
3. Recurse until regions contain a single column of text
4. Reading order: left region before right; top before bottom within each region

**Docstrum fallback (when XY-cut produces > 10 regions with < 3 blocks each):** Compute nearest-neighbor pairs between text blocks. Build a graph of adjacency edges weighted by distance and angle. Traverse the connected components in estimated reading order (sort root nodes by page position, follow edges within each component).

**Parameters:** k=5 nearest neighbors per block (standard Docstrum value); distance metric: Euclidean center-to-center in PDF user space; within-line adjacency angle: ±30° from horizontal; between-line adjacency angle: ±30° from vertical (blocks not meeting either constraint are not connected). **Root node definition:** A block with no incoming edges from blocks whose center-y is greater than this block's center-y (i.e., no block above it in the page is connected to it). Root nodes are sorted by (x_column_index, y descending) to establish the traversal start order.

Set `reading_order_algorithm = "xy_cut"` or `"docstrum"` in page output.

**Crates:** None (graph is a simple `Vec<Edge>`)

**Critical tests:**
- Two-column academic paper: all left-column blocks before all right-column blocks
- Magazine layout with sidebar: main text flow separated from sidebar
- Single-column text: XY-cut produces single region, no spurious splits
- Rotated page (Rotate=90): coordinate system rotated before applying algorithm

### 4.6 Output Serialization (Plain Text Mode)

Implement `--text` output as a projection of the block list.

**Rules:**
- Blocks serialized in reading order
- Paragraphs separated by `\n\n`
- Page breaks: `\f` (form feed, 0x0C)
- Headers and footers excluded by default; `--include-headers-footers` flag re-enables
- Invisible text (Tr=3) excluded unless `--include-invisible-text` flag set
- Watermark blocks excluded (Phase 7 watermark detection — see `docs/research/watermark-and-background-separation.md`). Prior to Phase 7, watermarks are not excluded from `--text` output; `kind: 'watermark'` blocks are not emitted.

**Critical tests:**
- 10-page document: 9 form-feed characters in output
- Header block: excluded from `--text` output by default
- Invisible text span: excluded from `--text` output

### 4.7 Text Readability Validation and Correction

**This phase is a primary accuracy differentiator.** Existing extractors emit raw glyph sequences regardless of whether the output text is human-readable. pdftract validates every span and repairs or discards unreadable output, ensuring extracted text can be used directly without downstream cleanup.

**Readability scoring (per-span):**

| Signal | Weight | Threshold |
|---|---|---|
| Printable Unicode fraction (non-U+FFFD, non-control) | 0.35 | > 0.95 → good |
| Dictionary word coverage (English; fast trie lookup) | 0.30 | > 0.60 → good |
| Whitespace distribution (not all one word, not all spaces) | 0.15 | ratio in [0.05, 0.40] → good |
| Ligature integrity (no split ligatures: fi, fl, ffi, ffl) | 0.10 | 0 split ligatures → good |
| Glyph confidence floor (from Phase 2) | 0.10 | min confidence > 0.6 → good |

Composite score [0.0, 1.0]. Spans below `readability_threshold` (default 0.5, configurable) are flagged `readability: "low"`.

**Correction pipeline (applied before flagging):**

1. **Ligature repair:** If `fi`, `fl`, `ffi`, `ffl`, `ff` appear as adjacent U+FFFD + glyph (Phase 2 glyph level missed the ligature but position data shows adjacency < 0.1pt gap), reconstruct the ligature string from shape-matched component glyphs.
2. **Hyphenation repair:** End-of-line hyphen (`-\n` at right edge of column) joined with start of next line's first word. Strip the hyphen; concatenate. Applies only within the same block; do not join across block boundaries.
3. **Mojibake detection:** If the span contains sequences characteristic of Latin-1 interpreted as UTF-8 (e.g., `Ã©` for `é`), attempt re-decoding via `encoding_rs` and accept if readability score improves.
4. **Soft-hyphen removal:** U+00AD (soft hyphen) stripped from output text; it is a formatting hint, not content.
5. **Word-break normalization:** U+200B (zero-width space), U+FEFF (BOM mid-stream), U+200C/200D (non-joiner/joiner used incorrectly) stripped unless the script requires them (Arabic, Indic).

**Per-page readability score:** Median of span scores, weighted by span character count. Stored in `page.extraction_quality.readability`. If page score < 0.5 and page is `Vector` class, escalate to `BrokenVector` and re-route to assisted OCR path (Phase 5.5). Prior to Phase 5 availability (v0.1.0 builds compiled without the `ocr` feature), pages escalated to BrokenVector are emitted with `page_type: 'broken_vector'`, `extraction_quality.readability` set to the computed score, and a `BROKENVECTOR_OCR_UNAVAILABLE` diagnostic. No re-extraction is attempted. The OCR escalation path is compiled conditionally via `#[cfg(feature = 'ocr')]`.

**Crates:** `unicode-normalization` (already in default deps)

**Word list:** Embed a minimal 20,000-word English frequency list as a compile-time `phf::Set` (adds ~200 KB to binary; acceptable). Binary size is verified by a CI check: `cargo bloat --release --crates | grep pdftract_wordlist` must report ≤ 250 KB. If the actual size exceeds this, replace the phf::Set with a Bloom filter (`bloomfilter` crate, ~25 KB for 20k words at 0.1% false-positive rate) and accept that ~0.1% of non-words will score as words — negligible impact on readability scoring accuracy. Non-English documents: score only on printable fraction, whitespace distribution, and glyph confidence (skip dict lookup if `lang` attribute indicates non-English). The `lang` used here is the document-level language from the catalog `/Lang` entry (available from Phase 1.4), not the per-span `lang` field (which is populated in Phase 7). If `/Lang` is absent or non-English (not matching `en*`), the dictionary word signal is set to 1.0 (disabled) for all spans in the document.

**Critical tests:**
- Span with split ligature `U+FFFD U+0069` adjacent to `f`: repaired to `fi`
- Hyphenated word spanning line break: joined correctly, hyphen stripped
- Latin-1 mojibake `Ã©` → corrected to `é` when re-decode raises readability score
- Page readability < 0.5 on vector page: page re-classified to BrokenVector, OCR invoked
- Non-English page (Chinese): dict-word signal disabled; score driven by printable fraction + confidence
- 20,000-word phf::Set lookup: < 100 ns per word (benchmark assertion)

---

## Phase 5: OCR Integration

**Goal:** Extract text from scanned pages and improve broken-vector pages via Tesseract.  
**Complexity:** Complex  
**Estimate:** 3–4 weeks  
**Depends on:** Phase 4 complete (OCR output feeds back into Phase 4 assembly)  
**Delivers:** Full extraction for scanned PDFs; `pdftract extract --ocr` flag active

### 5.1 Page Classification

Classify each page to select the extraction path before any expensive work.

**Signals (computed in order, short-circuit when confident):**

| Signal | Vector | Scanned | BrokenVector |
|---|---|---|---|
| No text operators in content stream | — | Strong | — |
| All text Tr=3 + full-page image | — | — | Definitive |
| Image coverage fraction > 0.85 | — | Strong | — |
| Character validity rate < 0.4 | — | — | Strong |
| Character validity rate > 0.85 | Strong | — | — |
| Character density ratio < 0.03 | — | Moderate | — |

**PageClass output:** `Vector | Scanned | Hybrid | BrokenVector` with `confidence: f32`.

**PageClass → page_type mapping** (internal enum value → JSON output string):

| PageClass (internal) | page_type (JSON output string) |
|---|---|
| `Vector` | `"text"` |
| `Scanned` | `"scanned"` |
| `Hybrid` | `"mixed"` |
| `BrokenVector` (pre-OCR; `ocr` feature absent) | `"broken_vector"` |
| `BrokenVector` (post-OCR; OCR processed successfully) | `"scanned"` |
| Page with no text and no images | `"blank"` |
| Page with only image XObjects, no text | `"figure_only"` |

> **Note:** `broken_vector` is a valid `page_type` output value and must be included in `docs/schema/v1.0/pdftract.schema.json`.

**Hybrid detection:** Compute per-region classification: divide page into 8×8 grid cells. Cells with text operators and high validity → vector; cells with image coverage and no text → scanned. If both types present in significant fractions — defined as ≥ 15% each (≥ 10 of 64 grid cells classified as vector AND ≥ 10 classified as scanned) — → `Hybrid`.

**Critical tests:**
- Pure text PDF: all pages `Vector` with confidence > 0.95
- Scanned single-page PDF (image only): `Scanned`
- PDF/A with invisible text layer over scanned image: `BrokenVector`
- Hybrid page with text header and scanned body: `Hybrid`, correct region split

### 5.2 Image Extraction for Raster Pages

For `Scanned` and `Hybrid` pages, produce a raster for Tesseract.

**Rendering approach — two-tier:**

**Default (no `full-render` feature):** Direct image compositing. Collect all image XObjects on the page, decode each (Phase 1.5 stream decoder), and composite them onto a blank canvas using each XObject's placement matrix (CTM from `cm` and `Do` operators). This path has zero additional binary cost and handles > 90% of scanned PDFs correctly (those where the scan is a single full-page image).

**`full-render` feature:** `pdfium-render` (wraps Chromium's PDFium). Use when the page has complex rendering geometry — multiple overlapping images, image masks, soft masks — where compositing gets the wrong result. Binary cost: ~20 MB native library (tracked against the weight target; document in PR if this feature is enabled in the default Docker image). Enable with `--features full-render` at compile time or set `ExtractionOptions.full_render = true` at runtime (feature must be compiled in).

**Release Docker images:** The standard `pdftract:latest` and `pdftract:ocr` images are built with `--features ocr,serve` only (no `full-render`). A separate `pdftract:full` image tag is built with `--features ocr,serve,full-render` and has a higher size budget (~140 MB). The weight target table's 120 MB limit applies to `pdftract:ocr` only; `pdftract:full` is documented as a heavyweight variant.

**DPI selection:**
- Standard body text (font_size > 8pt equivalent): 300 DPI
- Fine print or small text: 400 DPI
- Line art / JBIG2 pages: 200 DPI (already binary; higher DPI doesn't help) (JBIG2 decoding for OCR requires `full-render` feature; see Phase 1.5 filter notes)

**Hybrid page handling:** For Hybrid pages, Phase 3 content stream extraction runs first on the entire page to capture vector text. OCR runs only on the grid cells with image coverage fraction > 0.80 (identified during Phase 5.1 classification). Results are merged by bounding box: where a vector span's bbox overlaps an OCR span's bbox by > 50%, the vector span is used (higher confidence); non-overlapping regions use whichever source produced text in that area.

**Output:** Grayscale `image::GrayImage` for each page region needing OCR.

**Crates:** `image` (default `ocr` feature), `pdfium-render` (`full-render` feature only)

### 5.3 Image Preprocessing

Apply the preprocessing pipeline before Tesseract invocation.

**Pipeline (in order):**
1. **Deskew:** Hough line transform on grayscale input via `leptonica-plumbing`'s `pixDeskew`; no pre-binarization required for skew detection. Compute dominant angle; rotate by negative angle. Skip if detected angle < 0.3° (no meaningful skew).
2. **Contrast normalization:** Histogram stretch to [0, 255]. Applied before binarization to improve threshold quality on unevenly-lit scans. Skip for JBIG2 (already binary).
3. **Binarization:** Sauvola local adaptive thresholding for physical scans; Otsu global for digital-origin scans. Detect origin via image XObject filter: DCTDecode → Sauvola; JBIG2Decode → already binary, skip.
4. **Denoising:** 3×3 median filter for salt-and-pepper noise. Skip for JBIG2 (already clean binary).
5. **Border padding:** Add 10px white border on all sides (Tesseract accuracy improves with padding).

**Crates:** `leptonica-plumbing` (Sauvola, deskew via `pixDeskew`), `image` (Otsu, median filter)

**Critical tests:**
- 2° skewed scan: deskewed to within 0.1° before OCR
- Page with uneven lighting (shadow from binding): Sauvola thresholding produces clean binary
- Already-binary JBIG2 image: binarization step skipped, no quality degradation

### 5.4 Tesseract Integration

Invoke Tesseract on preprocessed raster images and parse HOCR output.

**Configuration:**
- Language: from `ExtractionOptions.ocr_language` (default `["eng"]`)
- Page segmentation mode: `PSM_AUTO` (Tesseract decides)
- Output format: HOCR XML (provides per-word bounding boxes and confidence scores)
- Tesseract init: one `TessBaseAPI` per thread (stored in `thread_local!`); avoid re-initialization cost

**HOCR parsing:**
- Parse `ocrx_word` elements: extract `title` attribute for `bbox x0 y0 x1 y1` and `x_wconf NNN` (confidence 0–100 → 0.0–1.0)
- Convert HOCR pixel coordinates to PDF user-space coordinates using the DPI and page geometry
- Each HOCR word → one Span with `confidence_source = "ocr"`

**Crates:** `tesseract` (0.14; wraps `libtesseract` FFI), `quick-xml` (HOCR parsing)

**Critical tests:**
- Clean black-on-white scan of Lorem Ipsum: word error rate < 2%
- Multi-language page (English and French): both language packs loaded; correct characters extracted
- Tesseract confidence < 30 on a region: `confidence = 0.3` in span output
- HOCR bbox coordinates correctly converted to PDF space after DPI scaling

### 5.5 Assisted OCR (BrokenVector Path)

For `BrokenVector` pages, use vector glyph position data to validate Tesseract output rather than as segmentation pre-seeds.

**Pipeline:**
1. Run Phase 3 content stream processing in position-hint mode: collect glyph bboxes but discard Unicode values (treat all as U+FFFD)
2. Run Tesseract in `PSM_SPARSE_TEXT` mode (page segmentation mode 11), which allows Tesseract to find text in arbitrary positions without requiring a dominant text block — appropriate for BrokenVector pages where the visible text layer may be fragmented or partially occluded
3. After OCR completes, validate each Tesseract word result against the nearest vector glyph bbox: if the Tesseract word's center falls within 5pt of a vector glyph bbox center, the word is accepted with its OCR confidence; otherwise it is flagged low-confidence (confidence capped at 0.4)
4. Parse HOCR output as in Phase 5.4, applying per-word confidence adjustments from step 3
5. If OCR confidence > 0.7 for a region: use OCR text; if OCR confidence < 0.3: re-attempt without the validation filter (pure OCR fallback)

**Critical tests:**
- PDF/A with invisible text layer at correct positions: OCR output better than blind OCR (validate WER delta)
- PDF/A with incorrect text layer positions (misaligned): validation filter rejects misaligned words; fallback to unaided OCR confidence scores

### 5.6 Document Type Classification

Classify each document into one of the recognized profile types so that Phase 7.10 profiles can apply type-specific extraction tuning. This pass runs after Phase 5 page classification and Phase 4 text assembly, but before final output serialization. Lightweight (rule-based), reproducible (no model weights), and user-extensible (every type's matching criteria are exposed as YAML in Phase 7.10).

**Built-in profile types:** `invoice`, `receipt`, `contract`, `scientific_paper`, `slide_deck`, `form`, `bank_statement`, `legal_filing`, `book_chapter`, `unknown`.

**Classifier design — a rule-based scorer:** Each profile (see Phase 7.10) defines matching predicates (text patterns, structural signals, page-count ranges, font signals). The classifier evaluates every loaded profile against the extracted document and selects the highest-scoring profile above a 0.6 confidence threshold. Below threshold → `unknown`.

The classifier is intentionally **NOT a trained ML model**:
- Reproducibility (no model weights to ship; output is a deterministic function of inputs + ruleset)
- Transparency (`metadata.document_type_reasons` shows exactly why a profile matched)
- User-extensibility (profiles are user-editable YAML — see Phase 7.10)
- Binary size (zero additional crates beyond `regex`, which is already pulled in by `grep` or `profiles`)

**Feature signals (computed once during Phase 4 assembly, reused across all profile evaluations):**
- Text pattern hit counts per page (currency symbols, ISO-style dates, "INVOICE", "WHEREAS", "Abstract", "References", etc.)
- Page-count distribution
- Table density (fraction of blocks with `kind: "table"`)
- Heading hierarchy depth
- Font diversity (count of distinct font names across the document)
- Average glyph density per page
- Presence flags: signature field, form field, math operators, bullet lists, page-number footers

**Output:** Document-level fields added to `metadata`:

```json
"metadata": {
  "document_type": "invoice",
  "document_type_confidence": 0.87,
  "document_type_reasons": [
    "text_contains matched 'Invoice #'",
    "structural.has_table = true",
    "page_count = 2 within range [1,5]"
  ]
}
```

When `--auto` is passed, the matching profile's extraction options also override defaults — see Phase 7.10 for the override semantics.

**CLI:**
- `pdftract extract --auto file.pdf` — classify and apply the matching profile automatically
- `pdftract extract --profile invoice file.pdf` — force a specific profile (skips classification)
- `pdftract classify file.pdf` — print the detected type only (no extraction):
  ```json
  {"document_type":"invoice","confidence":0.87,"reasons":["..."],"runner_up":"receipt","runner_up_confidence":0.42}
  ```

**Acceptance criteria:**
- On a labelled corpus of 200 documents (50 invoices, 50 papers, 50 contracts, 50 misc), classification accuracy ≥ 90%
- Per-document classification overhead < 5% of total extraction time
- All built-in profiles' selection rationale reported in `document_type_reasons`
- Reproducibility: classifying the same document twice produces identical output

**Crates:** `regex` (already added in `grep` and `profiles` features; auto-pulled-in when this phase runs as part of `--auto` or `--profile`)

**Feature flag:** The classifier is in default features (the rule evaluator is ~50 LOC of vanilla Rust), but the built-in profile bundle that drives it lives behind the `profiles` feature. Without `profiles`, classification always yields `unknown` and `document_type_confidence: 0.0`.

**Critical tests:**
- Acrobat sample invoice: classified as `invoice` with confidence > 0.8
- arXiv paper PDF: classified as `scientific_paper`
- IRS Form 1040: classified as `form`
- Scanned receipt: classified as `receipt`
- 100-page novel: classified as `book_chapter` or `unknown` (either accepted)
- 200-doc labelled corpus: per-class precision and recall ≥ 0.85; macro-F1 ≥ 0.88

---

## Phase 6: Output and API

**Goal:** Deliver the full output schema, PyO3 bindings, and HTTP serve mode.  
**Complexity:** Medium  
**Estimate:** 3–4 weeks  
**Depends on:** Phase 5 complete  
**Delivers:** Shippable CLI, Python package, HTTP service

### 6.1 JSON Output (Full Schema)

Implement the complete output schema from `docs/research/extraction-output-schema.md`.

**Document-level fields:**
- `schema_version: "1.0"`
- `metadata`: title, author, subject, keywords, creator, producer, creation_date, modification_date, page_count, pdf_version, is_tagged, is_encrypted, conformance, contains_javascript, contains_xfa, ocg_present, generator
- `outline`: recursive bookmark tree with title, destination, level
- `threads`: article thread chains (Phase 7 feature; empty array in Phase 6)
- `attachments`: from `/EmbeddedFiles` name tree (Phase 7; empty array in Phase 6)
- `signatures`: digital signature metadata (Phase 7; empty array in Phase 6)
- `form_fields`: AcroForm fields with values (Phase 7; empty array in Phase 6)
- `links`: document-scoped URI and internal destination links (Phase 7 feature; empty array in Phase 6)
- `extraction_quality`: aggregate across all pages
- `errors`: all diagnostics emitted during extraction

**Page-level fields (full schema):**
- `page_index` (0-based integer, canonical for programmatic use), `page_number` (integer, 1-based, = `page_index + 1`; **Phase 6.1 deliverable:** add this field to `docs/research/extraction-output-schema.md` and to `docs/schema/v1.0/pdftract.schema.json`), `page_label` (string from PDF `/PageLabels` number tree, e.g. `"iv"` or `"A-3"`; absent if the PDF defines no page labels), `width`, `height`, `rotation`, `page_type`

  > **Naming convention:** `page_index` is the stable, zero-based identifier used in all internal references (e.g., error diagnostics, NDJSON frame ordering). `page_number` is emitted alongside it as a convenience for human-facing display. Both fields are always present. SDK code and downstream tools MUST key on `page_index` for programmatic access; `page_number` is informational only.
- `spans`: full Span array per schema
- `blocks`: full Block array per schema
- `annotations`: highlights, stamps, notes, links from `/Annots` (Phase 7 feature; empty array in Phase 6)
- `tables`: parallel table structure objects for `kind: table` blocks (Phase 7)

**Crates:** `serde`, `serde_json`

**JSON Schema deliverable:** A machine-readable JSON Schema is generated from the extraction output schema and stored at `docs/schema/v1.0/pdftract.schema.json`. This file is generated once and checked into the repo. The Phase 6.1 critical test uses `jsonschema` (Python) or `jsonschema-valid` (Rust) to validate test output against this file. Creating this JSON Schema is a Phase 6.1 deliverable alongside the Rust implementation.

**Critical tests:**
- Schema validator: produce output from a known-good PDF, validate against `docs/schema/v1.0/pdftract.schema.json`
- Page with no text: `spans: []`, `blocks: []`, `page_type: "blank"` or `"figure_only"`
- Error entries: each emitted diagnostic has stable `code`, `severity`, and `page_index`

### 6.2 NDJSON Streaming Mode

Implement `--stream` / `ExtractionOptions.streaming = true`.

**Frame sequence:**
1. Header frame: `{"frame":"header","schema_version":"1.0","metadata":{...},"outline":[...],"total_pages":N}`
2. Per-page frames (emitted as each page completes via rayon): `{"frame":"page","page_index":N,...}`  
   Note: rayon may complete pages out of order; buffer completed pages and emit in page_index order with a window of 8 pages maximum. When the out-of-order buffer holds 8 completed pages and the next in-order page has not yet completed, the output thread blocks on a `Condvar` until that page's rayon task signals completion. The window size of 8 is chosen to be larger than the typical rayon thread pool size (4–8 threads), ensuring the output thread is never the bottleneck on balanced workloads. For pathological cases (one very slow page surrounded by fast pages), the window is effectively a backpressure signal to the downstream consumer.
3. Footer frame: `{"frame":"footer","extraction_quality":{...},"errors":[...],"threads":[],"attachments":[],"signatures":[],"form_fields":[],"links":[]}`

**Header/footer detection in streaming mode:** The cross-page header/footer deduplication pass (Phase 4.4) cannot run before individual page frames are emitted. In streaming mode, header and footer blocks are emitted as `kind: 'header'` / `kind: 'footer'` only if they can be identified from the trailing window of up to 4 already-emitted pages. For the first 3 pages, header/footer detection is deferred: those blocks are emitted as `kind: 'paragraph'` and NOT retroactively corrected. Consumers relying on exact `kind` values for headers/footers should use the non-streaming mode.

**BufWriter:** Wrap `io::Stdout` in `BufWriter<io::Stdout>` with 128 KB buffer; flush after each frame.

**Critical tests:**
- 100-page document in streaming mode: output contains exactly 102 newline-delimited JSON objects: 1 header object (first), 100 page objects (in page_index=0 to page_index=99 order), 1 footer object (last). Each object is complete and valid JSON.
- Out-of-order page completion: pages buffered and emitted in correct index order
- Consumer reads frame-by-frame with `newline` delimiter: each frame is valid JSON

### 6.3 PyO3 Python Bindings

Build a Python extension module exposing the extraction API.

**Module:** `pdftract` (import as `import pdftract`)

**API surface:**
```python
# Synchronous extraction
result: dict = pdftract.extract(path: str, **options) -> dict
text: str = pdftract.extract_text(path: str, **options) -> str

# Streaming (returns an iterator of page dicts)
pages: Iterator[dict] = pdftract.extract_stream(path: str, **options)
# Yields only page dicts (frame: 'page' equivalent). Metadata and errors are not yielded — call extract() for the full document result including metadata.

# Options (keyword arguments mapped to ExtractionOptions):
# ocr=False, ocr_language=["eng"], include_invisible=False,
# extract_forms=False, extract_attachments=False, readability_threshold=0.5,
# password=None, max_decompress_gb=2,
# full_render=False  # no-op if binary compiled without full-render feature

# Exceptions
class PdftractError(Exception): ...       # extraction failed
class EncryptionError(PdftractError): ... # encrypted, no password
```

**Python GIL handling:** Release the GIL during extraction (`py.allow_threads(|| ...)`) so Python threads can continue while a page is being processed.

**Build:** `maturin build --features python` produces a `.whl` for the current platform. CI cross-compiles for all five target triples (see `docs/notes/sdk-architecture.md`).

**CI note:** PyO3 wheel cross-compilation for macOS and Windows from a Linux runner is handled using `maturin build --target <triple>` with the `cross` tool (Docker-based cross-compilation). The Argo WorkflowTemplate `pdftract-py-ci` (to be created in `jedarden/declarative-config → k8s/iad-ci/argo-workflows/`) will use a `ghcr.io/rust-cross/manylinux` base image for Linux wheel builds and `osxcross` toolchain for macOS targets. Windows `.whl` is built using `cross` with `x86_64-pc-windows-gnu`. All five triples ship to PyPI on milestone tags via the same workflow.

**Crates:** `pyo3` (feature `extension-module`), `maturin` (build tool)

**Critical tests:**
- `pdftract.extract("test.pdf")` returns a dict with correct `metadata.page_count`
- `pdftract.extract_text("test.pdf")` returns a plain-text string
- `pdftract.extract("nonexistent.pdf")` raises `PdftractError`
- `pdftract.extract("encrypted.pdf")` raises `EncryptionError`
- Python threading: 4 threads each extracting different PDFs simultaneously; no deadlock

### 6.4 HTTP Serve Mode

Implement `pdftract serve --port PORT`. Requires `--features serve` at compile time (`axum` + `tokio` are not in the default build — they add ~2 MB to the binary). The pre-built release binaries for the `serve` Docker image are compiled with `--features ocr,serve`.

**Endpoints:**

| Method | Path | Request | Response |
|---|---|---|---|
| POST | `/extract` | multipart/form-data `file=<pdf>` + optional form fields for options | JSON extraction result |
| POST | `/extract/text` | same | `text/plain` body |
| POST | `/extract/stream` | same | NDJSON stream (Content-Type: application/x-ndjson) |
| GET | `/health` | none | `{"status":"ok","version":"x.y.z"}` |

**Optional form fields (all endpoints):**

| Field | Type | Default | Maps to |
|---|---|---|---|
| `ocr` | boolean | `false` | `ExtractionOptions.ocr` |
| `ocr_language` | string (comma-separated) | `eng` | `ExtractionOptions.ocr_language` |
| `readability_threshold` | float | `0.5` | `ExtractionOptions.readability_threshold` |
| `include_invisible` | boolean | `false` | `ExtractionOptions.include_invisible` |
| `extract_forms` | boolean | `false` | `ExtractionOptions.extract_forms` |
| `extract_attachments` | boolean | `false` | `ExtractionOptions.extract_attachments` |
| `password` | string | `""` | `ExtractionOptions.password` |
| `full_render` | boolean | `false` | `ExtractionOptions.full_render` (no-op if binary compiled without `full-render` feature) |

**Error responses:**

| Status | Condition |
|---|---|
| 400 | Bad request (no file field, unsupported content type) |
| 413 | Request exceeds `--max-upload-mb` limit |
| 422 | Extraction error (encrypted file, corrupt file) |
| 500 | Internal error |

Response body for all error statuses is `{"error":"code","message":"..."}`. A custom `RequestBodyLimit` rejection handler is implemented to convert tower-http's default plain-text 413 response to the standard JSON error body `{"error":"REQUEST_TOO_LARGE","message":"Request body exceeds the configured limit"}`.

**Concurrency:** axum handles concurrent requests; rayon thread pool is shared across all requests. No per-request thread spawning. Each POST handler bridges async and sync via `tokio::task::spawn_blocking(|| extraction_call())`, which runs the synchronous rayon work on tokio's blocking thread pool (separate from the async executor). Rayon provides within-document page-level parallelism; tokio's blocking pool handles per-request concurrency. Rayon's default pool sizing (equivalent to the logical CPU count) is used; no explicit pool configuration is required.

**Request size limit:** Default 256 MB; configurable via `--max-upload-mb`.

**Security constraints:**
- **Decompression limit:** Configured via `ExtractionOptions.max_decompress_bytes`; exposed in serve mode as the `max_decompress_gb` form field. Also accessible via `--max-decompress-gb` CLI flag and `max_decompress_gb=2` Python keyword arg.
- **Authentication:** No auth is built in. Deploy behind a reverse proxy (nginx, Traefik) with authentication. The serve mode is not safe to expose directly on a public port without a proxy.
- **Path parameters:** No file-path parameters are accepted in serve mode — the PDF is always received as a multipart upload. This eliminates path traversal risk.

**Crates:** `axum`, `tokio`, `tower-http` (for `RequestBodyLimit`, `TraceLayer`), `multer` (multipart parsing)

**Critical tests:**
- `curl -F file=@test.pdf http://localhost:8080/extract`: valid JSON response
- File exceeding size limit: HTTP 413 response with JSON body `{"error":"REQUEST_TOO_LARGE","message":"Request body exceeds the configured limit"}` (not tower-http's default plain-text response)
- Concurrent requests with 8 simultaneous PDFs: all complete correctly
- `/health` endpoint: 200 OK, even while extractions are in progress

### 6.5 Markdown Output Mode

Emit structure-preserving CommonMark Markdown with optional positional anchors. Markdown is one of several output formats; the user may request any combination simultaneously via Phase 6.6's multi-output architecture.

**Block kind → Markdown emission:**

| Block kind | Markdown emission |
|---|---|
| `heading` (level N) | `#` × N + space + text + `\n\n` (level taken from Phase 7.1 StructTree when available, otherwise inferred from font-size hierarchy in Phase 4.4) |
| `paragraph` | text + `\n\n`; soft line breaks within a paragraph encoded as trailing `  \n` |
| `list` (bulleted) | `- item\n` per line item, terminated by blank line |
| `list` (numbered) | `1. item\n` per line item; numbering inherits the source numbering |
| `code` (Phase 4.4 / Phase 7) | Fenced block ```` ```lang ```` ... ```` ``` ```` with `lang` set from monospace-font heuristic + optional shebang/keyword sniffing |
| `formula` (Phase 7) | `$inline$` or `$$display$$` — LaTeX from OpenType Math; raw glyph fallback otherwise |
| `table` | GitHub-flavored pipe table (`\| col \| col \|`); falls back to inline HTML `<table>` for merged cells, colspan/rowspan, or nested content |
| `caption` | Italic line directly under the preceding figure: `*caption text*` |
| `figure` | `![alt-from-/Alt](#)` placeholder; alt text from StructTree `/Alt` (Phase 7.1) when present |
| `header` / `footer` | Excluded by default (same as plain text mode); included with `--include-headers-footers` |
| `watermark` | Excluded by default; included with `--include-watermarks` |
| `quote` | `> ` prefixed lines |

**Inline span styling (Phase 4.1 flags):**
- Bold (bit 0) → `**text**`
- Italic (bit 1) → `*text*`
- Bold + italic → `***text***`
- Subscript (bit 3) → `<sub>text</sub>`
- Superscript (bit 4) → `<sup>text</sup>`
- Smallcaps (bit 2) → `<span style="font-variant: small-caps">text</span>` (CommonMark has no smallcaps; HTML is the standard fallback)
- Color-only differences: no styling (color is not semantically meaningful in Markdown)

**Inline links (Phase 7.6 hyperlinks):** `[anchor text](https://target)` — anchor text is the union span text under the link annotation's rect.

**Footnotes:** Reference style `[^1]` in body; definitions at end of each section: `[^1]: footnote text`. When Phase 7 footnote-anchor resolution is unavailable, footnotes are inlined parenthetically.

**Positional anchors (opt-in via `--md-anchors`):**

Each block emits a single-line HTML comment immediately before its content:

```markdown
<!-- pdftract: page=3 block=12 bbox=[72.0,640.5,540.0,672.0] kind=heading -->
## Chapter 3
```

Comment format is a stable schema parseable with one regex:
```
<!-- pdftract: page=(\d+) block=(\d+) bbox=\[([\d.,]+)\] kind=(\w+) -->
```

HTML comments are passthrough in every major Markdown renderer (GitHub, GitLab, Obsidian, Notion import, `pulldown-cmark`, `marked`, `markdown-it`), so anchored output is still human-readable.

**Per-page break:** Horizontal rule `\n\n---\n\n` between consecutive pages by default. Suppressed with `--md-no-page-breaks` for downstream LLM ingestion where page breaks are noise.

**Acceptance criteria:**
- Output passes CommonMark validation (`pulldown-cmark` round-trip)
- All headings, paragraphs, tables, lists, code blocks appear in the same reading order as the JSON output
- Anchors round-trip: parsing anchored Markdown back yields the original block list (modulo inline styling, which is the format's normal lossy boundary)
- Reproducibility: same input → byte-identical Markdown across runs

**Crates:** None new — pure string formatting on top of Phase 4 blocks.

**Critical tests:**
- LaTeX-produced paper: headings at correct levels, equations wrapped in `$...$`
- Markdown table with merged-cell input: falls back to `<table>` HTML
- Bullet list with nested sublist: correctly indented `- item` lines
- `--md-anchors`: comment precedes every block
- Bold + italic span: emitted as `***text***`
- Reproducibility: same PDF extracted twice yields byte-identical Markdown

### 6.6 Multi-Output Emission Architecture

Support emitting multiple output formats from a single extraction pass. Users routinely want JSON for programmatic consumers AND Markdown for human readers AND plain text for downstream tooling — running extraction three times is wasteful. The architecture below lets one extraction populate any subset of `{json, markdown, text, ndjson}` concurrently.

**CLI design:**

```bash
# Single output to stdout (default)
pdftract extract file.pdf

# Single output to a file
pdftract extract file.pdf --json out.json
pdftract extract file.pdf --md out.md
pdftract extract file.pdf --text out.txt

# Multiple outputs from one extraction pass
pdftract extract file.pdf --json out.json --md out.md --text out.txt

# Use `-` for stdout in any output
pdftract extract file.pdf --md - --json out.json     # md to stdout, JSON to file

# Auto-named outputs by base path
pdftract extract file.pdf --format json,markdown,text -o out
# → produces out.json, out.md, out.txt
```

**Validation rules:**
- At most one format may use `-` (stdout)
- Repeating the same format flag is an error (`--json a.json --json b.json` rejected)
- `--ndjson` is mutually exclusive with all other formats (NDJSON streams page-by-page; cannot be combined with whole-document emission)
- All output files are opened upfront and committed atomically (write to a temp file, rename on success) so an interrupted extraction never leaves partial output files behind

**Architecture:**

```rust
trait OutputSink: Send {
    fn open(&mut self, header: &DocumentHeader) -> io::Result<()>;
    /// Called as pages complete; sinks may buffer for whole-document emission.
    fn page(&mut self, page: &Page) -> io::Result<()>;
    fn close(&mut self, footer: &DocumentFooter) -> io::Result<()>;
}
```

Concrete sinks: `JsonSink`, `MarkdownSink`, `TextSink`, `NdjsonSink`, `ReceiptSink` (Phase 6.8). The extraction pipeline pushes the document model through each registered sink. Whole-document sinks (JSON, Markdown) buffer the page list and emit on `close`. Streaming sinks (NDJSON, page-by-page text) emit on each `page` call.

**Memory ceiling:** When multiple non-streaming sinks are active, the in-memory document model is held until the slowest sink completes. The model is dominated by the span list (~200 bytes per span); a 500-page document with 200 spans/page holds ~20 MB peak — well within target.

**HTTP serve mode (Phase 6.4) update:**

- New `format` form field accepting a comma-separated list of `json|markdown|text` (NDJSON requested via the existing `/extract/stream` endpoint, never combined)
- Single-format requests return the body directly with the appropriate `Content-Type`
- Multi-format requests return `multipart/mixed`, one part per format, each with the appropriate `Content-Type`

**MCP server (Phase 6.7) update:**

Tool calls accept a `formats: ["json", "markdown", "text"]` parameter. Response is an object keyed by format name.

**Acceptance criteria:**
- Single extraction → 3 simultaneous outputs (JSON + MD + text) completes within 1.1× the time of single-format extraction
- Cross-format consistency: all sinks observe the same `document_fingerprint` (Phase 1.7) in their headers
- Atomicity: a panic mid-extraction leaves NO partial output files on disk (verified by injecting a panic in a fixture test)

**Critical tests:**
- `--json a.json --md b.md` → both files produced, both valid
- `--md - --json out.json` → Markdown to stdout, JSON to file
- Crash mid-extraction → no partial output files (only temp files, which are removed on drop)
- Same extraction with `--json` only vs. `--json --md` → JSON byte-identical (Markdown does not perturb the JSON sink)
- `--ndjson --md b.md` → rejected at CLI parse time with a clear error

### 6.7 MCP Server Mode

Expose pdftract as a Model Context Protocol (MCP) server so LLM agents (Claude Desktop, Claude Code, Cursor, Continue, custom agents using the Anthropic or OpenAI SDKs) can invoke extraction as a tool. Two transports are supported, **mutually exclusive per process**: stdio (for local agent host-process integration) and HTTP+SSE (for remote service deployment).

**Subcommand:** `pdftract mcp [--stdio | --bind ADDR]`. Exactly one transport flag must be specified; if neither is given, `--stdio` is the default. The two modes are runtime-exclusive — a single `pdftract mcp` invocation listens on exactly one transport. Operators deploying both modes run two separate processes.

**Stdio mode (local):**
- JSON-RPC 2.0 framed per MCP spec (Content-Length-headered messages over stdin/stdout)
- stdin = client requests, stdout = server responses, stderr = server logs (never JSON-RPC)
- Single-client; one process per agent attachment
- Process exits cleanly when stdin closes (EOF)

**Remote mode (HTTP+SSE):**
- `pdftract mcp --bind 0.0.0.0:8080` (or `127.0.0.1:8080` if loopback)
- HTTP+SSE transport per MCP spec: `POST /` for client→server, `GET /sse` for server→client streaming
- Multiple concurrent clients; reuses the Phase 6.4 rayon thread pool and tokio runtime
- Authentication: bearer token via `--auth-token VALUE` (env var `PDFTRACT_MCP_TOKEN` also accepted). **Required** when binding to a non-loopback address — startup aborts with a clear error if `--bind 0.0.0.0:...` is given without a token

**MCP capabilities advertised:**
- `tools/list` → returns the tool catalog below
- `resources/list` → empty (pdftract has no static resources)
- `prompts/list` → empty
- `logging/setLevel` → respected (mapped to `env_logger` levels)

**Tool catalog:**

| Tool | Description | Required args | Optional args |
|---|---|---|---|
| `extract` | Full extraction returning the document JSON | `path` (string) | `pages` (string e.g. "1-5,7"), `ocr` (bool), `formats` (string array; multi-output), `auto_profile` (bool), `password` (string), `receipts` (`"off"\|"lite"\|"svg"`) |
| `extract_text` | Plain-text extraction | `path` | `pages`, `ocr`, `password` |
| `extract_markdown` | Markdown extraction | `path` | `pages`, `ocr`, `anchors` (bool, default false), `password` |
| `search` | Regex search across the file returning matches with page+bbox | `path`, `pattern` | `case_insensitive`, `max_matches`, `password` |
| `get_metadata` | Metadata + outline + fingerprint only (cheap; no full extraction) | `path` | `password` |
| `get_table` | Single table by page index and table index (Phase 7.2) | `path`, `page`, `table_index` | `password` |
| `get_form_fields` | AcroForm/XFA field values (Phase 7.4) | `path` | `password` |
| `get_attachments` | Embedded files (Phase 7.5) | `path` | `include_data` (bool — when true, file bytes are base64-encoded into the response) |
| `hash` | Compute structural fingerprint only (Phase 1.7) | `path` | `password` |
| `classify` | Run Phase 5.6 classifier only (no extraction) | `path` | — |

The `path` argument accepts local filesystem paths (relative to the working directory) and `https://` URLs (uses Phase 1.8 remote source adapter when the `remote` feature is enabled).

**Path-traversal protection:** When `--root DIR` is set at startup, all local paths are resolved relative to `DIR` and any resolved path that escapes `DIR` is rejected with JSON-RPC error code -32602 ("Invalid params"). Without `--root`, the working directory is the implicit root. HTTPS URLs are unaffected by `--root`.

**Logging and observability:** Every tool invocation emits a structured log line to stderr: ISO-8601 timestamp, tool name, path (or its hash if `--no-log-paths`), duration in milliseconds, response size in bytes, error code if any. Log level controlled by `RUST_LOG` and the MCP `logging/setLevel` request (whichever is more verbose).

**Mode-exclusivity rationale:** Running both stdio and HTTP simultaneously would require dual ownership of stdout — stdio mode treats stdout as the JSON-RPC sink, while HTTP mode treats it as a log channel. Forbidding the combination at the CLI layer makes the contract unambiguous.

**Acceptance criteria:**
- Stdio mode responds to `tools/list` within 50 ms of receiving the request on stdin
- Remote mode handles 50 concurrent clients each running `extract` on different PDFs without errors
- Switching between transports requires only a flag change; no other configuration touched
- Bearer token required when binding to a non-loopback address: startup aborts with a clear error if missing

**Feature flag:** `mcp` (depends on `serve`). When `mcp` is enabled, the binary gains the `mcp` subcommand and shares the `axum`/`tokio` dependency footprint with `serve`. JSON-RPC framing is hand-written; no separate crate.

**Crates:** Reuses `axum`, `tokio`, `tower-http` from Phase 6.4. No new direct dependencies.

**Critical tests:**
- Stdio mode: piping `{"jsonrpc":"2.0","id":1,"method":"tools/list"}\n` to stdin produces the expected tool list on stdout
- HTTP+SSE mode: tools/list and extract calls succeed via curl
- Path-traversal attempt with `--root /var/data`: `path="../../etc/passwd"` rejected with -32602
- Bearer token required: `--bind 0.0.0.0:8080` without token aborts startup; with token, valid requests succeed and missing tokens get 401
- Tool error on encrypted PDF: JSON-RPC error response with code -32000 and human-readable message
- Two simultaneous `pdftract mcp` invocations: each listens on its own transport without conflict; one stdio, one HTTP

### 6.8 Visual Citation Receipts

For every span and block, optionally emit a portable *receipt* object that downstream consumers can use as verifiable proof of provenance. Each receipt binds a piece of extracted text to a specific region in a specific PDF in a way that can be independently re-verified by re-running pdftract on the original file (or by visual inspection of the embedded SVG clip).

**Enabled with:** `--receipts=lite` or `--receipts=svg` (CLI), `ExtractionOptions.receipts = "lite" | "svg" | "off"` (default `"off"`).

**Receipt object (added to spans and blocks when receipts are enabled):**

```json
{
  "text": "Net Income: $2.4M",
  "bbox": [220.0, 412.0, 412.0, 432.0],
  "receipt": {
    "pdf_fingerprint": "pdftract-v1:a7f3...",
    "page_index": 14,
    "bbox": [220.0, 412.0, 412.0, 432.0],
    "content_hash": "sha256:9b21...",
    "extraction_version": "1.0.0",
    "svg_clip": "<svg ...>...</svg>"          // present only when --receipts=svg
  }
}
```

Field definitions:
- `pdf_fingerprint`: Phase 1.7 fingerprint of the source PDF
- `page_index`: 0-based page index (matches Phase 6.1 schema)
- `bbox`: same coordinates as the parent span's bbox, included so the receipt is self-contained
- `content_hash`: SHA-256 of the span's `text` after NFC normalization
- `extraction_version`: the pdftract version that produced this receipt (semver)
- `svg_clip`: a self-contained SVG element rendering only the glyphs whose bboxes fall within the receipt bbox. Glyph paths are extracted via `ttf-parser`'s outline API and embedded inline (no font-file dependency); the SVG coordinate system is normalized to the bbox itself so the SVG renders standalone in any browser

**Lite vs. SVG modes:**
- `lite` (small): adds ~120 bytes per receipt — fingerprint + page_index + bbox + content_hash + extraction_version. No rendering work. Best for agent citations where the verifier has access to the original PDF.
- `svg` (portable): adds ~1–5 KB per receipt depending on glyph count. Best for standalone display in dashboards, audit reports, or compliance trails where the verifier does not have the source PDF.

**Verifier protocol:** A receipt is verified by:
1. Recomputing the source PDF's fingerprint with `pdftract hash` — must equal `pdf_fingerprint`
2. Re-extracting the page at `page_index` — at least one span on the page must have a bbox overlapping the receipt bbox by ≥ 90% (IoU) and a `text` whose NFC-normalized SHA-256 equals `content_hash`

A reference verifier ships as `pdftract verify-receipt FILE.pdf RECEIPT.json`. Exit code 0 if the receipt verifies; non-zero with a diagnostic line on failure (codes: 10 = fingerprint mismatch, 11 = bbox mismatch, 12 = content mismatch).

**SVG-clip generation:**
1. Identify all glyphs whose bbox center falls within the receipt bbox (uses Phase 3 glyph list)
2. For each glyph, query its font's outline via `ttf-parser`'s glyph-outline API (already in default deps)
3. Concatenate outline paths in a single SVG with `<path>` elements positioned per glyph bbox
4. Fill color taken from each glyph's `fill_color`
5. ViewBox normalized to `[0 0 width height]` of the receipt bbox

For glyphs whose Unicode came from OCR (no font outlines available), embed a base64-encoded 150-DPI raster PNG crop of the bbox region instead, with `data-source="ocr"` attribute on the SVG root. The verifier protocol still works (the receipt's `content_hash` is computed from the resolved Unicode, regardless of source).

**Acceptance criteria:**
- 100% of receipts from a clean extraction verify successfully when re-run on the same PDF
- Receipts survive a producer-tool re-save with no content edit (fingerprint preserved → receipts still verify)
- Receipts FAIL to verify when the source PDF's content changes (a single edited paragraph invalidates receipts in that region but not elsewhere — granular verification, not all-or-nothing)
- SVG receipts render correctly in `<img src="data:image/svg+xml,...">` in current Chrome, Firefox, and Safari (verified via headless-browser pixel diff against expected PNG, < 1% difference)
- Receipt generation adds ≤ 10% to extraction time for `lite`, ≤ 25% for `svg`

**Crates:** Reuses `sha2` and `ttf-parser` from default deps; no new dependencies. SVG output is hand-written XML.

**Feature flag:** `receipts` — opt-in. The output schema retains `receipt: null` placeholders when the feature is compiled out and receipts were not requested, so downstream JSON consumers see a stable shape.

**Critical tests:**
- Round-trip: extract with `--receipts=lite` → verify-receipt against same PDF → success
- Tamper detection: edit one glyph in the PDF → receipts in that region fail verification; others still pass
- SVG clip: render in headless browser; pixel diff vs. expected image < 1%
- OCR-sourced receipt: SVG contains base64 PNG; `data-source="ocr"` attribute present
- 100 receipts on a 100-page document: aggregate JSON size increase ≤ 15 KB with `lite`, ≤ 500 KB with `svg`

### 6.9 Content-Addressed Cache Layer

Cache extraction results keyed by PDF fingerprint (Phase 1.7) + extraction-options hash. Resubmitting the same logical PDF with the same options returns the cached result without re-running extraction. Cache hits are O(1) filesystem reads; misses run extraction and populate the cache for next time.

**Storage layout (filesystem-backed; no external database):**

```
<cache_dir>/
  index.json                          # cache version + LRU metadata
  <fp[0:2]>/<fp[2:4]>/<full_fp>/
    <opts_hash_1>.json.zst           # cached extraction result, zstd-compressed
    <opts_hash_2>.json.zst
```

Each entry's filename encodes its zstd-compressed size for fast LRU computation without re-stat (e.g. `e7a1f3-12387.json.zst`). The two-byte prefix directories keep any single dir under 65 K entries.

**Cache key:**
1. PDF fingerprint (Phase 1.7) — 32 bytes hex
2. SHA-256 of the canonical JSON serialization of the extraction options (sorted keys, normalized booleans, defaulted unspecified fields)

**Eviction policy:** LRU with configurable size limit (default 1 GiB). On cache write, if total compressed size exceeds the limit, evict the least-recently-touched entries until under budget. Touched-time updated on every cache hit via the index's append-only audit log (no per-entry stat churn).

**CLI:**
- `pdftract extract --cache-dir DIR file.pdf` — enable cache for a one-off extraction
- `pdftract serve --cache-dir DIR --cache-size 4GiB` — enable cache for the HTTP server (and MCP server in remote mode)
- `pdftract cache stats DIR` — print hit ratio, total size, entry count, age histogram
- `pdftract cache clear DIR` — delete all entries
- `pdftract cache purge DIR --older-than 30d` — TTL-based cleanup
- `pdftract --no-cache` — disable the cache at the call site even if `--cache-dir` is set globally

**Concurrency:** Multiple processes can share the same cache directory safely. Cache writes are atomic (write to a temp file, rename). Multiple readers can read the same entry simultaneously. LRU touched-times use O_APPEND writes to a sentinel file to avoid contention. When two processes both miss the same key, both run extraction (no exclusive lock); the second write wins. Duplicated work is rare and tolerated to avoid the complexity and risk of a distributed lock.

**Cache validity:** Entries are tagged with `extraction_version` (the pdftract semver). On binary upgrade, entries from older versions are invalidated by virtue of being looked up under the new version key (cache miss). Stale entries are purged opportunistically during normal LRU eviction; an explicit `pdftract cache purge DIR --version "<1.0.0"` is provided for forced invalidation.

**Streaming consideration:** NDJSON streaming mode (Phase 6.2) does NOT serve responses from cache (caching defeats streaming's whole point). However, the cache IS populated as the streaming extraction runs to completion, so subsequent non-streaming calls for the same PDF hit the cache.

**Output integration:**
- JSON output adds `metadata.cache_status: "hit" | "miss" | "skipped"` and `metadata.cache_age_seconds: N` (omitted on miss/skipped)
- HTTP serve mode adds an `X-Pdftract-Cache: hit | miss | skipped` response header

**Acceptance criteria:**
- Cache hit on 100-page PDF: result returned in < 20 ms p99
- 1000 concurrent cache hits: throughput > 10,000 req/s (filesystem-bound; commodity SSD)
- Cache survives process restart (filesystem-only state)
- Disabling the cache (`--no-cache`) reverts to baseline extraction with zero overhead

**Crates:** `zstd` (~50 KB; the only new direct crate for this phase). No external database; filesystem-only storage.

**Feature flag:** `cache` — implicitly enabled by `serve`. Adds `zstd` only when active.

**Critical tests:**
- Hit-then-modify: extract; edit PDF content; re-extract → cache miss
- Hit-then-touch-metadata: extract; modify `/Producer` (no content change) → cache hit (same fingerprint)
- Concurrent extractors on same fingerprint: both succeed; no deadlock; second write atomic
- Cache exceeds size limit: LRU evicts oldest; new writes succeed; no orphaned files
- `pdftract cache stats` on an empty dir: reports zero entries cleanly
- Corrupt entry on disk (truncated file): treated as a miss; entry deleted; extraction re-runs

### 6.10 `pdftract doctor` — Environment Health Check

The `doctor` subcommand validates the runtime environment without performing an extraction. It exists so an operator (or a CI smoke test) can confirm in one command that the pdftract binary and its OS-level dependencies are in a usable state. The command is REQUIRED to run on every fresh deployment and is the recommended first action when an extraction fails for non-PDF-content reasons.

**Subcommand surface:**

```
pdftract doctor [--features] [--json] [--exit-on-fail] [--profile-dir DIR] [--cache-dir DIR]
```

| Flag | Effect |
|---|---|
| `--features` | Print which features were compiled into this binary and exit. No diagnostic checks run. |
| `--json` | Emit results as a single JSON document (machine-consumable). Default is a colored human-readable table. |
| `--exit-on-fail` | Exit code 1 if ANY check reports `FAIL`; otherwise exit code 0 even if `WARN`s are present. Default exit policy: 0 unless any check is `FAIL`. |
| `--profile-dir DIR` | Verify the profile search path includes `DIR` and that every YAML in `DIR` parses cleanly. |
| `--cache-dir DIR` | Verify `DIR` is writable, free space ≥ 1 GiB, and the layout is the current cache schema version. |

**Checks performed.** Each check produces one row in the output table with three columns: `Check`, `Result` (one of `OK` / `WARN` / `FAIL`), `Detail` (short human-readable reason).

| Check | OK | WARN | FAIL |
|---|---|---|---|
| `pdftract binary` | Version + git-sha + features compiled in listed | — | — |
| `tesseract install` (when `ocr` feature compiled) | `tesseract --version` parses; major ≥ 5 | major == 4 | binary missing or major ≤ 3 |
| `tesseract languages` (when `ocr` feature compiled) | required langs (`eng` by default; configurable via `--lang`) all present | optional langs missing | `eng` missing |
| `leptonica install` (transitive Tesseract dep) | `pkg-config --modversion lept` ≥ 1.79 | older | not found |
| `libtiff` (when `ocr` feature compiled) | found via `pkg-config` | — | not found |
| `libopenjp2` (when `ocr` feature compiled, JPEG 2000 fixtures) | found | — | not found |
| `pdfium native lib` (when `full-render` compiled) | runtime detection succeeds, version ≥ 6555 | older | not found |
| `network reachability` (when `remote` compiled) | HEAD https://example.com returns 2xx in ≤ 5 s | 3xx / slow | failure |
| `cache directory` (when `--cache-dir` passed or `cache` feature default-on) | writable, free space ≥ 1 GiB, layout version current | free space < 1 GiB or layout migration available | not writable or layout incompatible |
| `profile search path` (when `profiles` compiled) | every YAML parses; no `PROFILE_SECRETS_FORBIDDEN` | dir empty | parse errors or secret-keys present |
| `ulimit -n` (Linux/macOS) | ≥ 1024 | 512 ≤ n < 1024 | < 512 |
| `available RAM` (from `/proc/meminfo` or sysctl) | ≥ 256 MiB free | 128 MiB ≤ n < 256 MiB | < 128 MiB |
| `system locale` | UTF-8 locale active | non-UTF-8 with C fallback | unset |
| `temp dir writable` (`$TMPDIR` / `/tmp`) | writable + free space ≥ 100 MiB | free space < 100 MiB | not writable |

**Output formats.**
- Default (TTY): colored table with check name, status badge, and detail; summary line `N OK, M WARN, K FAIL` at the bottom.
- `--json`: a single JSON object `{"summary":{"ok":N,"warn":M,"fail":K},"checks":[{"name":"…","status":"OK|WARN|FAIL","detail":"…"},…]}`.
- Non-TTY default: same content as TTY, plain text, no color escapes.

**Exit codes.**
- 0: all checks pass (no `FAIL`)
- 1: at least one `FAIL` and `--exit-on-fail` set, OR any `FAIL` regardless of `--exit-on-fail` per default policy

**Crates:** No new direct crates. Reuses `directories` for path discovery, `which` (already in dev-deps; promoted to runtime here gated behind the `cli` feature), `os_info` / `sysinfo` is NOT pulled in — RAM and ulimit checks use direct `/proc` reads or `libc::getrlimit` to avoid binary bloat.

**Feature flag:** None; `doctor` ships in the default-feature binary. Checks for features the binary was not built with are skipped (and reported as `N/A` in `--json`).

**Critical tests:**
- A fresh Alpine container with `pdftract` binary copied in but no Tesseract / Leptonica / libtiff: `pdftract doctor` exits 1 (no `--exit-on-fail` flag needed — default policy fails on any `FAIL`); table shows three `FAIL` rows; `--json` output deserializes and includes the three.
- A fully-provisioned container: `pdftract doctor` exits 0, all rows `OK`.
- Network unreachable (offline CI runner): the `network reachability` row reports `WARN` (slow) or `FAIL` (DNS failure); does not crash.
- `--exit-on-fail` flag: exits 1 on any FAIL across all rows; exits 0 if only WARNs are present.
- `--profile-dir` pointed at a directory containing a profile with `password:` key: the profile-search-path row reports `FAIL` with reference to `PROFILE_SECRETS_FORBIDDEN`.

---

## Phase 7: Advanced Features

**Goal:** StructTree exploitation, table detection, AcroForm/XFA, attachments, signatures.  
**Complexity:** Medium–Complex per feature  
**Estimate:** 4–5 weeks (features developed independently; can be parallelized across developers)  
**Depends on:** Phase 6 complete

### 7.1 StructTree Exploitation (Tagged PDF)

Use the PDF structure tree as the authoritative reading order for tagged documents.

**Implementation:**
1. From document catalog `/StructTreeRoot`, load the root `StructElem`
2. Walk the structure tree depth-first; at each `StructElem`, record the element type (mapped via `/RoleMap` if non-standard), the `/ActualText` attribute (overrides extracted text if present), the `/Alt` attribute (alternative text for figures), and the `/Lang` attribute (BCP-47 language tag)
3. For each `StructElem`, collect its MCID references: each marked content sequence (identified by its MCID from Phase 3.4) is assigned to its owning `StructElem` via the `ParentTree`
4. Build the block list by traversing the structure tree in document order; each `StructElem` maps to one block; its constituent MCIDs provide the spans in reading order
5. Map structure element types to block kinds: `P` → paragraph, `H`/`H1`–`H6` → heading with level, `Table` → table, `L`/`LI` → list, `Figure` → figure, `Artifact` → suppressed (not emitted in output)

**Validation:** If `MarkInfo /Suspects true`, fall back to XY-cut for any page where the structure tree coverage is less than 80% of extracted glyphs.

**`reading_order_algorithm`:** Set to `"struct_tree"` when used.

**Crates:** None beyond Phase 1 parser

**Critical tests:**
- Word-generated tagged PDF: heading levels correctly extracted (H1/H2 map to level 1/2)
- Tagged PDF with `/ActualText` on a ligature: ActualText value used, not glyph-decoded text
- Tagged PDF with `/Artifact` marked content: artifact glyphs excluded from output
- PDF with `Suspects true`: falls back to XY-cut, `reading_order_algorithm = "xy_cut"`

### 7.2 Table Detection and Structure Reconstruction

Detect tables and reconstruct cell structure.

**Detection pipeline:**
1. **Line-based detection:** Collect all horizontal and vertical path segments from the content stream (operators `m`/`l`/`S`, `re`/`S`, `re`/`f`). Cluster collinear segments. Find intersection points. Build grid from intersections. See `docs/research/table-structure-reconstruction.md` for the full grid reconstruction algorithm.
2. **Borderless table detection:** If no ruling lines found, examine span alignment: if 3+ lines share identical x0 positions for multiple groups, treat as candidate columns. Require 3+ rows to confirm.
3. **Cell content assignment:** For each cell bbox, collect all spans whose centroid falls within the bbox. Assign to the cell.
4. **Header row detection:** First row is header if all cells have bold font or if StructTree marks the row as `TH` type.
5. **Merged cell detection:** Missing interior edge between two cells → colspan or rowspan; infer from geometry.

**Output:** Block with `kind: "table"` and a parallel `table` object in the page output with rows/cells as per the schema.

**Crates:** None (geometry is pure arithmetic)

**Critical tests:**
- 5×3 bordered table: all 15 cells extracted with correct text
- Merged header cell spanning 3 columns: colspan=3 in output
- Borderless two-column table: detected via alignment heuristic
- Table spanning two pages: detected and flagged (full reconstruction deferred to non-streaming mode)

### 7.3 Digital Signature Metadata

Extract digital signature field metadata.

**Implementation:** Walk AcroForm `/Fields` array looking for Sig-type fields (`/FT /Sig`). For each signature field, extract: `/T` (field name), `/V` (signature dict) → `/Name` (signer name), `/M` (signing date, ISO 8601), `/Reason`, `/Location`, `/ByteRange` (byte ranges signed, for coverage analysis), `/SubFilter` (signature format: `adbe.pkcs7.detached`, `adbe.x509.rsa.sha1`, etc.).

**Validation:** pdftract does NOT perform cryptographic validation (that requires the full certificate chain and OCSP/CRL infrastructure). Instead, report `validation_status: "not_checked"`. A future version may integrate `ring` or `openssl` for validation.

**Output:** `signatures` array at document level per the output schema.

**Crates:** None beyond Phase 1 parser

**Critical tests:**
- PDF with two signature fields: both extracted with correct signer names and dates
- Signature field with no `/V` (unsigned): extracted with `value: null`
- `/ByteRange` coverage: correctly computed as fraction of file bytes signed

### 7.4 AcroForm and XFA Field Extraction

Extract interactive form field definitions and current values.

**AcroForm:**
- Walk `/Fields` recursively (fields may be nested in `/Kids`)
- For each field: `/T` (partial name), `/FT` (type: Tx/Btn/Ch/Sig), `/V` (current value), `/DV` (default value), `/Ff` (flags: required, read-only, multi-line), `/Rect` (bbox)
- Tx fields: `/V` is a string
- Btn fields: `/V` is a name (the selected appearance state); compute is_checked
- Ch fields: `/V` is selected option; `/Opt` array lists all options
- Construct full field names by joining partial names with `.`

**XFA:**
- If `/AcroForm /XFA` is present, parse the XFA XML stream(s) (either single stream or array of named streams concatenated as XML)
- Walk the XFA data model to extract field values from `<field>` elements; use the XFA field name as the key
- If both AcroForm and XFA are present, prefer XFA values for overlapping fields

**Crates:** `quick-xml` (XFA parsing)

**Critical tests:**
- PDF with text field, checkbox, and dropdown: all three types extracted with correct values
- Nested field hierarchy: full dot-separated name constructed correctly
- XFA-only form: all field values extracted from XFA XML
- Hybrid XFA+AcroForm: XFA values preferred

### 7.5 Portfolio and Attachment Extraction

Extract embedded files from PDF portfolios and `/EmbeddedFiles` name trees.

**Implementation:**
- Locate the `/EmbeddedFiles` name tree in the catalog `/Names` dictionary
- Walk the name tree leaves, each yielding a `Filespec` dictionary
- From each `Filespec`: `/F` or `/UF` (filename), `/Desc` (description), `/Type /Filespec`, `/EF` dict → `/F` stream (the embedded file data)
- From the EF stream dictionary: `/Subtype` (MIME type hint), `/Params` dict → `/Size`, `/CreationDate`, `/ModDate`, `/CheckSum`
- Decode the stream (applying its filters)

**Size limit:** If attachment stream decoded size > 50 MB, include metadata only and set `data: null` with a `truncated: true` flag. When non-null, `data` is the base64-encoded content of the decoded attachment stream (standard alphabet, no line breaks, no padding omitted). The JSON Schema at `docs/schema/v1.0/pdftract.schema.json` must reflect `{"type": "string", "contentEncoding": "base64"}` for this field. In the Python API, `data` is returned as a Python `bytes` object (PyO3 converts from base64 automatically). In the CLI `--text` mode, attachments are not included.

**Portfolio navigator:** Check for `/Collection` entry in catalog; if present, extract portfolio schema and sort fields for richer metadata.

**Output:** `attachments` array at document level.

**Crates:** None beyond Phase 1 parser and stream decoder

**Critical tests:**
- PDF with 3 embedded files of different MIME types: all three extracted with correct filenames and sizes
- Attachment with no `/Desc`: description is null (not empty string)
- Attachment exceeding size limit: metadata present, `data: null`, `truncated: true`

### 7.6 Hyperlink and Annotation Extraction

Extract URI hyperlinks and page annotation objects.

**Implementation:**
- For each page, walk the `/Annots` array in the page dictionary
- Collect Link annotations (`/Subtype /Link`):
  - Extract `/A` action dict: if `/S /URI`, read the `/URI` string as the target URL
  - Extract `/Dest`: if present (named or explicit destination), record as an internal link
  - Both URI and internal links are appended to the document-level `links` array with `page_index`, `rect` (the annotation bbox), and `uri` or `dest` as appropriate
- Collect other annotation subtypes (Highlight, Stamp, FreeText, Note, Squiggly, StrikeOut, Underline):
  - Extract `/Subtype`, `/Rect`, `/Contents` (comment text), `/T` (author), `/M` (modification date), `/C` (color array)
  - Append to the page-level `annotations` array

**Output:** Document-level `links` array (URI and internal destination links from all pages); page-level `annotations` array (all non-link annotations on each page).

**Crates:** None beyond Phase 1 parser

**Critical tests:**
- PDF with 5 URI hyperlinks: all 5 appear in document-level `links` with correct URLs
- Link annotation with named destination (`/Dest /SectionTwo`): emitted as internal link with `dest: "SectionTwo"`
- Page with Highlight and Note annotations: both appear in page-level `annotations` with correct subtypes
- Annotation with no `/Contents`: `contents` field is null (not empty string)

### 7.7 Article Thread Chains

Reconstruct PDF article thread chains for multi-column and multi-page reading flows.

**Implementation:**
- Read the `/Threads` array from the document catalog; each entry is an article thread dict
- Each thread dict has `/F` (first bead object reference) and `/I` (thread info dict with `/Title`, `/Author`, `/Subject`, `/Keywords`)
- Walk the bead chain by following `/N` (next bead) links from the first bead; detect the chain end when `/N` loops back to the first bead (circular list)
- Each bead dict has `/R` (page object reference, resolves to the page containing the bead) and `/V` (bbox rect of the bead region on the page)
- Reconstruct the ordered list of beads for each thread: `[{ page_index, rect }, ...]`

**Output:** Document-level `threads` array; each entry has `title` (from thread info `/Title`, or null), `author`, `subject`, and `beads` (ordered list of `{ page_index, rect }` objects).

**Crates:** None beyond Phase 1 parser

**Critical tests:**
- PDF with two article threads: both reconstructed with correct bead order and page references
- Thread with no `/I` info dict: `title`, `author`, `subject` all null; bead chain still reconstructed
- Bead `/V` rect correctly converted to PDF user-space coordinates for the referenced page
- Circular bead chain termination: chain walk stops after visiting all beads without infinite loop

### 7.8 `pdftract grep` — Folder Search with Bounding-Box Results and Progress Observability

ripgrep-style regex search across one or more PDFs that returns matches with their page index and bbox in PDF user-space coordinates. Single-pass parsing — no intermediate "extract to disk then grep" detour. Designed to be fast over folders of hundreds-to-thousands of PDFs without ever appearing hung.

**Subcommand:**

```
pdftract grep [OPTIONS] PATTERN [PATH...]
```

If no path is given, search the current directory (recursive by default when no path is given). Paths may be files, directories, or `https://` URLs (when the `remote` feature is enabled).

**Options:**

| Flag | Default | Effect |
|---|---|---|
| `-r`, `--recursive` | implied when paths are dirs | Recurse into directories looking for `*.pdf` |
| `-i`, `--ignore-case` | off | Case-insensitive search |
| `-E`, `--extended-regexp` | off | Treat PATTERN as full regex (default is literal) |
| `-F`, `--fixed-strings` | on | Literal string match (default) |
| `-w`, `--word-regexp` | off | Match on word boundaries |
| `-v`, `--invert-match` | off | Print non-matching spans instead |
| `-l`, `--files-with-matches` | off | Print only filenames with ≥ 1 match |
| `-c`, `--count` | off | Print match counts per file |
| `-j N`, `--threads N` | CPU count | Worker thread count for parallel file processing |
| `--ocr` | off | Run OCR on scanned pages too (slower; usually narrow PSM_SPARSE_TEXT mode) |
| `--json` | off | JSON-Lines output (one match per line) |
| `--highlight DIR` | — | Write annotated PDFs to `DIR/<name>-highlighted.pdf` |
| `--max-results N` | unlimited | Stop after N total matches |
| `--progress` | auto | Show progress bar (default: on if TTY, off otherwise) |
| `--no-progress` | — | Force-disable the progress bar |
| `--progress-json` | off | Emit machine-readable progress events to stderr |
| `--quiet` | off | Suppress all output except exit code |

**Default output format (human-readable):**

```
docs/contract.pdf:p4:[120.5,400.0,380.0,418.0]:  Termination clause and notice period of 30 days
                  └─ page (1-based), span bbox in PDF user space
```

**JSON-Lines output (`--json`), one match per line:**

```json
{"path":"contract.pdf","page_index":3,"bbox":[120.5,400.0,380.0,418.0],"match_text":"Termination clause","span_text":"Termination clause and notice period of 30 days","span_confidence":0.98,"pdf_fingerprint":"pdftract-v1:..."}
```

**Match granularity:** Matches are reported at the *span* level — a span is the smallest text unit with a single bbox. If a single match crosses spans (rare; can happen after Phase 4.7 readability correction joins spans), the union bbox of the constituent spans is reported and `crosses_spans: true` is added to the JSON line.

**`--highlight DIR` output:**

For each input PDF `<name>.pdf`, write `DIR/<name>-highlighted.pdf` with:
- A new `/Annots` layer per page containing yellow `/Highlight` annotations (`/Subtype /Highlight`, `/QuadPoints` derived from each match bbox)
- The original content stream is **not modified** — only the `/Annots` array is amended, so the output is a valid PDF that opens correctly in Acrobat, Preview, browser PDF viewers, and other readers

**Progress observability — the core requirement that grep must never appear hung:**

Two mechanisms, both designed to update at least once every 500 ms even on slow files:

1. **Progress bar (TTY default), via `indicatif`:**

   ```
   Searching: [▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇         ] 387/512 files (76%)  78 MB/s  ETA 00:00:08
   Current: docs/contract-2024-q3.pdf  (page 24/120)
   ```

   The "Current" line ticks every 100 ms with the page-within-file progress, even when the overall file count is unchanged.

2. **JSON progress events (`--progress-json`), emitted to stderr, one event per line:**

   ```json
   {"event":"start","files_total":512,"bytes_total":104857600,"started_at":"2026-05-16T14:22:01Z"}
   {"event":"file_start","path":"docs/a.pdf","size":12345}
   {"event":"file_progress","path":"docs/a.pdf","pages_done":24,"pages_total":120}
   {"event":"file_done","path":"docs/a.pdf","matches":3,"duration_ms":140}
   {"event":"done","files_processed":512,"matches_total":1287,"duration_ms":18420}
   ```

**Slow-file warning:** If any single file takes > 30 seconds, emit a warning line to stderr including the file path and elapsed time. Processing continues — never abort.

**Benchmarking — folder-scale throughput is a CI-gated acceptance criterion:**

Phase 0 CI gains a new benchmark target `pdftract-grep-1000` that runs the search across the fixture set `tests/fixtures/grep-corpus/` (1000 PDFs, ~100 MB total). Wall-clock time and throughput are recorded in `benches/results/<commit-sha>.json` and compared against:
- `pdfgrep` (existing tool): pdftract must be ≥ 2× faster on the same corpus
- `pdftotext + ripgrep` (sequential pipeline): pdftract must be ≥ 3× faster
- pdftract's own historical results: a > 10% regression blocks PR

Per-PR observability is the same as the user-facing observability above: the CI runner uses `--progress-json` so the Argo Workflow logs show a steady stream of `file_done` events, never a multi-minute silence that looks like a hang.

**Acceptance criteria (CI-gated):**
- Throughput: search "the" across the 1000-PDF corpus at ≥ 50 MB/s on a 4-core CI machine
- First-match latency: first match printed to stdout within 100 ms of process start on the 1000-PDF corpus
- Memory: peak RSS during the 1000-file search < 200 MB
- Annotated output: every match in the JSON output is present as a `/Highlight` annotation in the corresponding `--highlight DIR/<name>-highlighted.pdf`
- Progress: bar updates at least once every 500 ms even when individual files are slow
- Non-PDF files in a folder (`.txt`, `.docx`, `.zip`): silently skipped (no diagnostic noise)
- Encrypted PDF without password: skipped with a single per-file diagnostic; not counted as a match

**Feature flag:** `grep` — adds `regex` (~500 KB), `walkdir` (~30 KB), `indicatif` (~50 KB). The CLI's default-feature binary size budget rises from 4 MB to 4.6 MB to accommodate this; documented as an approved allocation in the Weight Targets table (see Primary Objectives). `grep` is recommended for the `pdftract:full` Docker image and the `pdftract-full` GitHub Release binaries.

**Crates:** `regex` (1.10), `walkdir` (2), `indicatif` (0.17) — all gated behind `grep`

**Critical tests:**
- Literal match across 100 PDFs: all matches reported in the expected order
- Regex match (`\$\d+\.\d{2}`): all dollar-amount patterns found
- `--highlight DIR`: annotated PDFs pass `pdftract extract` round-trip; annotations render correctly in Chrome's built-in PDF viewer (verified via headless-browser screenshot)
- `--progress-json`: all expected event types emitted in order
- 5 GB PDF in the middle of a folder: slow-file warning emitted at the 30s mark; processing continues; other files complete
- 1000-PDF benchmark: throughput meets the 50 MB/s gate

### 7.9 Inspector Mode — Web Debug Viewer

Renders each page of a PDF in a browser with extracted spans, blocks, columns, and reading-order arrows overlaid. The single most useful tool for understanding *why* an extraction produced a given result — critical for user trust and for diagnosing edge cases in real-world PDFs. Implemented as a separate subcommand of the binary (not a feature of the daemon-style `serve` mode) so that the backend-process API surface remains unaffected.

**Subcommand:** `pdftract inspect FILE.pdf [--port PORT] [--bind ADDR] [--no-open]`

Defaults: `--port 7676`, `--bind 127.0.0.1`. The inspector binds to loopback only by default; binding to a non-loopback address requires `--auth-token` for the same reason as Phase 6.7 MCP HTTP mode.

**Behavior on launch:**
1. Run the full Phase 1–6 extraction pipeline on the given file
2. Cache the extraction result in memory (no on-disk artifacts)
3. Start an `axum` HTTP server on the chosen port
4. Open the user's default browser to `http://<bind>:<port>/` (suppressed with `--no-open` for CI/headless environments)

**Web interface:**

The frontend is a single self-contained static HTML/CSS/JS bundle compiled into the binary via `include_bytes!`. No CDN, no JavaScript frameworks (vanilla DOM + minimal CSS). Total bundle size: < 80 KB stripped+gzipped.

**Page display:**

Each page renders as a single inline SVG. The page background is reconstructed from the PDF's own content stream (glyph outlines via `ttf-parser`, vector paths from content stream `m`/`l`/`re` operators) — *not* a rasterization. This means:
- Sharp at any zoom level
- Text selection works against the actual extracted spans (invisible `<text>` elements overlay the glyph paths)
- Tiny bundle (no rasterization library, no pdfium)
- For Scanned pages where vector outlines aren't available, the source raster image is embedded as a base64 PNG

**Overlay layers (toggleable independently; state persists in `localStorage`):**

| Layer | Visualization |
|---|---|
| Spans | Thin outline rectangles around each span; color encodes confidence (red < 0.5, yellow 0.5–0.8, green > 0.8) |
| Blocks | Translucent rectangles around each block; fill color encodes block kind (heading=blue, paragraph=gray, table=teal, list=purple, code=orange, header/footer=light gray, figure=brown, caption=pink) |
| Columns | Dashed vertical lines at column boundaries; column index labels at the page top |
| Reading order | Curved arrows connecting blocks in the extracted reading order (numbered 1, 2, 3, ...) |
| Confidence heatmap | Per-glyph color grade: red < 0.5 → green > 0.9 |
| OCR regions | Cyan diagonal-stripe overlay on regions whose text came from Tesseract (Phase 5) |
| MCID | Numeric MCID labels in the corner of each marked-content block (Phase 3.4) |
| Anchors | Block-ID labels at the top-left corner of each block (matches Phase 6.5 Markdown anchor IDs) |

**Hover details:**

Hovering any span shows a tooltip:
```
Text:        "Net Income"
Font:        ABCDEF+Helvetica-Bold (size 11pt)
Confidence:  0.98 (source: to_unicode)
Bbox:        [220.5, 412.0, 312.0, 423.0]
Block:       paragraph #14 (column 2)
MCID:        47
Reading idx: 28
```

Clicking a span jumps the right-hand JSON-tree panel to the corresponding node and highlights it.

**Search:** A top-bar input filters spans whose text matches the query; matched spans get a bright outline. Enter cycles through matches.

**Navigation:**
- Left sidebar: clickable page list with thumbnails
- Toolbar: Prev/Next page buttons
- Keyboard: `←` / `→` for prev/next; `/` to focus search; `1`–`8` to toggle the eight overlay layers
- URL fragment carries page number for shareable links: `http://localhost:7676/#page=14`

**Acceptance criteria:**
- A 100-page PDF opens in the inspector and renders the first page within 2 seconds
- All eight overlay layers toggle individually without a re-render (CSS-only visibility)
- Hover tooltip appears within 50 ms of mouse enter
- Frontend bundle < 80 KB compressed
- Works in current Chrome, Firefox, and Safari (verified via headless-browser smoke tests in CI)
- `--no-open` flag prevents `xdg-open`/`open`/`cmd /c start` invocation

**Feature flag:** `inspect` (depends on `serve`). The frontend bundle adds ~80 KB. Total `--features ocr,serve,inspect` budget: 12.5 MB; documented as an approved allocation in the Weight Targets table.

**Crates:** Reuses `axum`, `tokio` from `serve`. Static bundle assets via `include_bytes!`. No new external crates.

**Critical tests:**
- Launch inspector on a sample PDF; HTTP `GET /` returns 200 with a valid HTML document
- All eight layer toggles produce the expected DOM changes (verified via headless-browser test)
- Keyboard shortcuts trigger their bound actions
- Search filter narrows visible spans correctly
- `--no-open` prevents the OS browser launcher; useful for CI/headless tests
- Inspector launched on a Scanned PDF: raster background embedded as base64 PNG; OCR confidence overlays render

### 7.10 Document Profiles — Configurable Extraction Templates

User-editable YAML profiles drive the Phase 5.6 document classifier and apply type-specific extraction tuning. Built-in profiles cover the common document types (invoice, receipt, contract, scientific paper, slide deck, form, bank statement, legal filing, book chapter); end users can copy, edit, or author new profiles without recompiling pdftract.

**Profile file format (YAML):**

```yaml
# ~/.config/pdftract/profiles/invoice.yaml
name: invoice
description: Invoices and bills with line items and totals
priority: 10                          # higher = preferred when multiple profiles match

# Matching predicates (any/all/none combinator tree)
match:
  all:
    - any:
        - text_contains: ["INVOICE", "Invoice #", "Bill To", "Tax Invoice"]
        - heading_matches: '^Invoice\b'
    - any:
        - has_currency_pattern: true
        - structural: {has_table: true}
    - structural:
        page_count: {min: 1, max: 5}
  none:
    - text_contains: ["abstract", "bibliography", "scientific paper"]

# Extraction tuning (overrides ExtractionOptions defaults when this profile matches)
extraction:
  reading_order: line_dominant         # invoices flow left-to-right line-by-line
  table_detection: strict_borders       # invoice tables typically have borders
  readability_threshold: 0.4            # tolerate lower readability for numeric-heavy data
  include_invisible: false

# Per-profile structured-field extraction (emitted in metadata.profile_fields)
fields:
  invoice_number:
    regex: 'Invoice\s*#?\s*([\w-]+)'
    near: ["Invoice", "Invoice Number", "Invoice #"]
    max_distance_pt: 200
  total:
    regex: '([\d,]+\.\d{2})'
    near: ["Total", "Amount Due", "Balance Due", "Grand Total"]
    parse: decimal
    max_distance_pt: 80
  vendor:
    region: top_quarter                 # top 25% of first page
    pick: largest_font
  invoice_date:
    near: ["Date", "Invoice Date"]
    parse: date
  customer:
    near: ["Bill To", "Customer", "Sold To"]
    pick: nearest_below
```

**Match DSL primitives:**

| Predicate | Value type | Effect |
|---|---|---|
| `text_contains` | string or `[string, ...]` | Any of the strings appears in any page text |
| `text_matches` | regex string | Any page text matches the regex |
| `heading_matches` | regex string | Any heading-block text matches the regex |
| `has_currency_pattern` | bool | `\$\d` / `€\d` / `£\d` / `¥\d` etc. appears |
| `has_signature_field` | bool | AcroForm sig field present (requires Phase 7.3) |
| `structural` | object | Sub-predicates against extracted structure |
| `structural.page_count` | `{min,max}` | Page count range |
| `structural.has_table` | bool | At least one block of `kind: table` |
| `structural.has_form_field` | bool | At least one AcroForm field |
| `structural.has_math` | bool | OpenType Math operators present |
| `structural.heading_depth` | `{min,max}` | Heading hierarchy depth range |
| `structural.font_diversity` | `{min,max}` | Number of distinct font names |

Combinators: `all`, `any`, `none`. Nested arbitrarily.

**Extraction tuning keys (override `ExtractionOptions` defaults when a profile is active):**

| Key | Values | Default |
|---|---|---|
| `reading_order` | `xy_cut`, `docstrum`, `line_dominant`, `struct_tree` | (auto-selected per Phase 4.5) |
| `table_detection` | `default`, `strict_borders`, `borderless_only`, `off` | `default` |
| `readability_threshold` | float 0.0–1.0 | 0.5 |
| `include_invisible` | bool | false |
| `include_headers_footers` | bool | false |
| `force_ocr` | bool | false |
| `min_block_chars` | int | 0 |

**Field-extraction DSL:**

Each field has zero or more localization hints (`near`, `region`, `pick`) and an extractor (`regex`, `parse`).

Localizers:
- `near: ["str", ...]` — find anchor spans containing any of the strings, then restrict candidates to spans within `max_distance_pt` (default 100) of those anchors
- `region: top_quarter | bottom_quarter | left_half | right_half | top:N | bottom:N | bbox:[x0,y0,x1,y1]` — restrict to a page fraction or explicit rectangle
- `pick: largest_font | smallest_font | nearest_below | nearest_right | first | last` — disambiguate when multiple candidates match

Extractors:
- `regex: "..."` — apply to candidate span text; capture group 1 (or 0 if no captures) is the value
- `parse: decimal | date | int | bool` — parse into a typed result; format detection is heuristic

**Output (added to JSON when a profile matches and the user passed `--auto` or `--profile`):**

```json
"metadata": {
  "document_type": "invoice",
  "document_type_confidence": 0.87,
  "document_type_reasons": ["text_contains matched 'Invoice #'", "structural.has_table = true"],
  "profile_name": "invoice",
  "profile_version": "1.0.0",
  "profile_fields": {
    "invoice_number": "INV-2025-00123",
    "total": 1247.50,
    "vendor": "Acme Widgets LLC",
    "invoice_date": "2025-09-14",
    "customer": "Jane Smith"
  }
}
```

**CLI:**

```bash
pdftract extract --auto file.pdf                # classify and apply best-matching profile
pdftract extract --profile invoice file.pdf     # force a named built-in profile
pdftract extract --profile path/to/profile.yaml file.pdf   # load from disk

pdftract profiles list                          # show all available profiles (built-in + user)
pdftract profiles show invoice                  # dump a profile YAML to stdout
pdftract profiles export invoice > my.yaml      # copy a built-in for editing
pdftract profiles install my.yaml               # install into ~/.config/pdftract/profiles/
pdftract profiles validate my.yaml              # syntax + schema check, no extraction
```

**Profile resolution order:**
1. Explicit `--profile NAME` or `--profile PATH` — exact match required
2. With `--auto`: evaluate all loaded profiles against the document, pick the highest-priority profile with confidence ≥ 0.6
3. Without either flag: no profile is applied; default `ExtractionOptions` used

**Profile search path (lowest priority first; later wins on name collision):**
1. Built-in profiles compiled into the binary
2. `/etc/pdftract/profiles/*.yaml` (system-wide)
3. `$XDG_CONFIG_HOME/pdftract/profiles/*.yaml` (defaults to `~/.config/pdftract/profiles/`)
4. `--profile-dir DIR` (CLI flag, repeatable)

This ordering lets system administrators ship a default in `/etc/pdftract/profiles/`, lets a user override per-user under `~/.config/`, and lets a single invocation override per-run via `--profile-dir`. A user who wants to slightly tweak a built-in profile runs `pdftract profiles export invoice > ~/.config/pdftract/profiles/invoice.yaml`, edits the file, and the next `--profile invoice` invocation picks up the modified copy.

**Built-in profiles shipped in v1.0.0:**

| Profile | Key extracted fields |
|---|---|
| `invoice` | invoice_number, vendor, customer, invoice_date, due_date, total, subtotal, tax, line_items |
| `receipt` | merchant, date, total, tax, items, payment_method |
| `contract` | parties, effective_date, term, governing_law, signatures |
| `scientific_paper` | title, authors, abstract, doi, journal, publication_date, references |
| `slide_deck` | title, presenter, date, slide_titles |
| `form` | (no field extractor; reading_order = line_dominant; surfaces all form_fields from Phase 7.4) |
| `bank_statement` | account_number, statement_period, opening_balance, closing_balance, transactions |
| `legal_filing` | case_number, court, parties, filing_date, docket_entries |
| `book_chapter` | title, chapter_number, author, sections |

Each built-in profile ships with at least 5 fixture documents and a regression test in `tests/fixtures/profiles/<name>/`.

**Hot-reload:** `pdftract serve --profile-dir DIR` re-reads the profile directory on every request when `--profile-hot-reload` is set, so operators can drop a new YAML in and the next request picks it up without a restart. Disabled by default (file I/O on every request is wasteful for stable deployments).

**Acceptance criteria:**
- Built-in `invoice` profile correctly identifies and extracts fields from a labelled fixture corpus of 50 invoices with ≥ 90% per-field accuracy
- User-authored profile loaded from disk overrides a built-in profile of the same name
- A profile YAML with malformed match expression fails `pdftract profiles validate` with a clear error including line number and a pointer to the bad token
- Profile field extraction adds < 5% to total per-document time
- Hot-reload picks up profile changes within one request when enabled

**Feature flag:** `profiles` — adds `serde_yaml` (~200 KB). Auto-pulls in `regex` from `grep` (or enables it standalone if `grep` is off). Built-in profiles compile into the binary via `include_str!`; user profiles load at runtime.

**Crates:** `serde_yaml` (0.9), `regex` (already added by `grep` feature; auto-enabled if needed)

**Critical tests:**
- Acrobat sample invoice: classified as `invoice` with confidence > 0.8; fields extracted with ≥ 90% accuracy across the 50-invoice fixture corpus
- Custom profile with priority 100 that matches every document: overrides all built-ins
- Profile with malformed regex: rejected by `profiles validate` with clear, line-numbered error
- Profile field `total` not found on the page: `profile_fields.total: null`, no error
- Hot-reload: `pdftract serve --profile-dir DIR --profile-hot-reload`; dropping a new YAML into `DIR` and the next request picks it up
- User profile shadowing a built-in: `pdftract profiles list` shows the user version with a `(overrides built-in)` annotation

---

## Cross-Cutting: Test Infrastructure

Tests are organized into three tiers:

### Tier 1: Unit Tests (in-crate `#[test]`)

Each module has unit tests covering the critical test cases listed per phase above. These run with `cargo test` and have no external dependencies.

**Target:** 100% of public function surfaces; all error paths exercised.

### Tier 2: Integration Tests (`tests/` directory)

Integration tests use a corpus of reference PDFs stored in `tests/fixtures/`. Each fixture has a corresponding expected-output JSON file. Tests verify:
- Exact text content match (for clean vector PDFs)
- Schema validity (all output against JSON Schema)
- Performance: extraction of a 100-page vector PDF completes in **< 3 seconds** on a 4-core CI machine (failure = CI block)

**Fixture categories:**
- `tests/fixtures/vector/`: clean LaTeX, Word, InDesign outputs
- `tests/fixtures/scanned/`: physical scans at various DPIs and skew angles
- `tests/fixtures/cjk/`: Chinese, Japanese, Korean documents
- `tests/fixtures/malformed/`: truncated, corrupt xref, circular references
- `tests/fixtures/encrypted/`: AES-128, AES-256, RC4 encrypted
- `tests/fixtures/forms/`: AcroForm and XFA documents
- `tests/fixtures/tagged/`: PDF/UA and PDF/A-a tagged documents
- `tests/fixtures/encoding/`: fonts with no ToUnicode CMap; verifies Levels 2–4 Unicode recovery; matched against known-good Unicode output
- `tests/fixtures/perf/`: one or more large (≥100 page) vector PDFs for speed benchmarking; output is validated for correctness but the primary metric is wall-clock time

`tests/fixtures/bench/` (Tier 4) uses the same PDFs as `tests/fixtures/perf/` plus competitor-run results; no separate corpus needed.

### Tier 3: Regression Corpus (CI only)

A private corpus of 500 real-world PDFs from diverse sources runs on every PR. Output is compared against a golden snapshot using a character-level diff. Any regression > 0.5% character error rate blocks the PR.

### Tier 4: Competitive Benchmarks (CI, tracked over time)

Benchmark suite runs `pdftract`, `pdfminer.six`, `pypdf`, and `pdfplumber` against identical fixture PDFs on the same CI machine. Results are stored as a JSON artifact per commit so regressions are detectable.

**Benchmark runner infrastructure:** A dedicated step in the `pdftract-ci` WorkflowTemplate uses a `python:3.11-slim` container. A `benches/competitors/requirements.txt` file (checked into repo) pins: `pdfminer.six==20231228`, `pypdf==4.2.0`, `pdfplumber==0.11.0`. A `benches/competitors/run_all.py` script drives competitor runs and emits results as `benches/results/<commit-sha>.json`. Results are stored as Argo Workflow artifacts. The pdftract binary time is measured with `hyperfine --warmup 2 --runs 5`.

**Metrics tracked per tool per fixture:**
- Wall-clock extraction time (mean of 5 runs)
- Peak RSS (resident set size)
- Character error rate vs. ground truth
- Reading order correctness score

**Minimum passing bar (blocks PR if missed):**
- pdftract must be ≥ 10× faster than `pdfminer.six` on vector PDFs
- pdftract CER must be ≤ `pdfminer.six` CER on all fixture categories
- pdftract binary (default features) must be ≤ 4 MB stripped

**Benchmark fixtures** (`tests/fixtures/bench/`):
- `vector-10.pdf`, `vector-100.pdf`: clean LaTeX output
- `cjk-20.pdf`: mixed CJK
- `two-column-academic.pdf`: multi-column reading order
- `scanned-5.pdf`: physical scan (OCR path only in pdftract)

### Tier 5: Property and Fuzz Tests

Tier 5 establishes the lower bound on parser robustness: every public parser surface MUST tolerate adversarial input without panic, and where applicable MUST satisfy a stated algebraic property. Tier 5 runs on every PR for a bounded budget; a nightly job runs for a larger budget.

**Crates.** `proptest` (dev-dependency only; not in the published crate's runtime dependency closure). `cargo-fuzz` (developer tooling; not a Cargo dependency).

**Targets and properties.**

| Target | Property |
|---|---|
| Phase 1.1 lexer | For any byte sequence of length ≤ 64 KiB, the lexer MUST NOT panic. It MUST either produce a valid token stream or terminate with a `LEXER_ERROR` diagnostic. |
| Phase 1.2 object parser | For any random valid token stream, parsing → object → string → re-parsing produces a structurally equal object (round-trip). |
| Phase 1.3 xref resolver | For any random xref-byte layout (including injected `/Prev` chains and corrupted offsets), the resolver MUST either produce a valid xref table or fall through to the forward-scan fallback with `XREF_REPAIRED`. No panic, no infinite loop (cycle detection enforces termination per Anti-Patterns). |
| Phase 1.5 stream decoder | For any input ≤ 1 MiB through any decoder, the output MUST be ≤ `max_decompress_bytes` (TH-01). A decoder that exceeds the cap MUST emit `STREAM_BOMB` and abort that stream. |
| Phase 2.2 font ToUnicode CMap parser | For any random CMap program ≤ 16 KiB, the parser MUST NOT panic. Invalid programs produce a `TOUNICODE_PARSE_ERROR` diagnostic; extraction continues with Level-3 / Level-4 fallback. |
| Phase 3.1 content stream interpreter | For any random sequence of well-typed PDF operators (drawn from a strategy that respects BT/ET pairing and the graphics-state stack), interpretation MUST NOT panic. Mismatched BT/ET pairs MUST emit `CONTENT_STREAM_MISMATCH` and continue. |
| Phase 7.10 profile YAML loader | For any random valid YAML ≤ 4 KiB, the loader MUST NOT panic. Invalid profile schemas produce a `PROFILE_INVALID` diagnostic with a line number. Profiles containing secret-keyword keys MUST trigger `PROFILE_SECRETS_FORBIDDEN` (per Secrets Handling). |

**Fuzz harnesses.** Each parser target has a `cargo-fuzz` harness under `fuzz/` whose corpus is seeded from `tests/fixtures/malformed/`. Harnesses:
- `fuzz/lexer/`
- `fuzz/objects/`
- `fuzz/xref/`
- `fuzz/streams/`
- `fuzz/cmap/`
- `fuzz/content/`
- `fuzz/profile_yaml/`

**Corpus minimization.** Any new crash discovered by fuzzing is minimized via `cargo fuzz cmin`, archived under `tests/fixtures/fuzz-corpus/<target>/<crash-id>.bin`, and exercised in Tier 2 as a regression test. The fix for a fuzz-discovered crash MUST land in the same PR as the corpus addition; merging the fix without the regression test is rejected at code review.

**Runtime budget.**
- Per-PR: each `fuzz/*` target runs for **1 CPU-hour** in the `pdftract-ci` workflow. Discovered crashes block the PR.
- Nightly: each `fuzz/*` target runs for **24 CPU-hours** in a dedicated `pdftract-fuzz` workflow. Discovered crashes file an automatic issue and tag the corpus.
- Quarterly: full corpus replayed against the latest `main` with `cargo fuzz run --release`; any new crash is treated as a P1 bug.

**Acceptance.** Any new fuzz-discovered crash MUST be added to the regression corpus and exercised as a Tier 2 test **before** the CVE-class fix is merged. The fix commit and the corpus commit MAY be the same PR; they MUST NOT be merged separately.

---

## Phase Completion Criteria

Each phase's `Delivers:` line names the artifacts the phase produces. This section converts every phase into a testable exit gate: a phase MUST NOT be marked complete unless **every** check in its list passes on the same commit. A check failure blocks the phase's milestone tag. The exit-gate list complements (does not replace) the per-section "Critical tests:" bullets already in each phase.

### Phase 0 — CI Infrastructure

Phase 0 is complete when ALL of the following pass on the same commit:
- [ ] `pdftract-ci` WorkflowTemplate is deployed to `iad-ci` via ArgoCD and shows `Synced` + `Healthy`
- [ ] `pdftract-py-ci` WorkflowTemplate stub is deployed and exits with status 0 on a manual submit
- [ ] A test commit triggers `pdftract-ci`; all five target-triple build jobs complete with status `Succeeded`
- [ ] `cargo audit` and `cargo deny check` run as CI steps and emit zero advisories of severity ≥ medium
- [ ] `cargo bloat --release --features default --crates` records the per-crate size baseline into `benches/results/<commit-sha>.json`
- [ ] `cargo clippy --features default -- -D warnings` exits clean
- [ ] A milestone-tag test (`vNN.NN.NN-test`) triggers binary upload to GitHub Releases (artifact verifiable by `gh release view`)
- [ ] Phase 0 critical tests in `tests/integration/ci/` pass

### Phase 1 — Core PDF Parser

Phase 1 is complete when ALL of the following pass on the same commit:
- [ ] `cargo test --features default,decrypt -p pdftract-core` — 100% pass, 0 flaky on 10 consecutive runs
- [ ] Integration tests `tests/integration/parser/{lexer,objects,xref,document,streams,recovery}.rs` all pass
- [ ] Phase 1.7 critical tests: 10 invocations of `pdftract hash` on the same input produce byte-identical fingerprints (INV-3); fingerprint regex `^pdftract-v1:[0-9a-f]{64}$` matches (INV-13)
- [ ] Phase 1.8 critical tests: `pdftract extract --range 1-1` over a 500-page remote PDF downloads < 5 MB (Weight Targets row)
- [ ] `cargo clippy --features default,decrypt,remote -- -D warnings` clean
- [ ] No `unwrap()` / `expect()` / `panic!()` in `pdftract-core` library code (clippy lint enforced; INV-8)
- [ ] Parser fuzz target (`fuzz/lexer/`, `fuzz/objects/`, `fuzz/xref/`) runs for ≥ 1 CPU-hour with zero crashes
- [ ] Tier 2 fixture `tests/fixtures/malformed/` extracts without panic; every fixture either produces output or returns a documented `errors[]` entry

### Phase 2 — Font and Encoding Pipeline

Phase 2 is complete when ALL of the following pass on the same commit:
- [ ] `cargo test --features default,decrypt -p pdftract-core --test fonts` — 100% pass
- [ ] Integration tests `tests/integration/fonts/{type_detection,encoding,cjk,type3,glyph_shape}.rs` all pass
- [ ] Phase 2.2 acceptance: ≥ 90% Level-4 Unicode recovery rate on `tests/fixtures/encoding/` (Primary Objectives Accuracy row; proof obligation in the Ledger)
- [ ] Phase 2.5 acceptance: glyph-shape DB matches every Latin/Greek/Cyrillic test glyph at confidence ≥ 0.7
- [ ] `cargo bloat --features default --crates` shows font-fingerprint data file contributes ≤ 600 KB to the binary
- [ ] `build/CHECKSUMS.sha256` verifies on every build (Supply Chain Considerations)
- [ ] `cargo clippy --features default,decrypt -- -D warnings` clean

### Phase 3 — Content Stream Processing

Phase 3 is complete when ALL of the following pass on the same commit:
- [ ] `cargo test --features default,decrypt -p pdftract-core --test content_streams` — 100% pass
- [ ] Integration tests `tests/integration/content/{graphics_state,text_operators,xobjects,marked_content,inline_images}.rs` all pass
- [ ] Phase 3.1–3.4 critical tests (each section's bullet list) all pass
- [ ] Form XObject recursion depth limit (default 8) is enforced; exceeding it emits a `FORM_XOBJECT_RECURSION` diagnostic without panic
- [ ] Marked-content MCID tracking produces a deterministic MCID→span map; round-trip property test passes
- [ ] `cargo clippy --features default,decrypt -- -D warnings` clean

### Phase 4 — Text Assembly and Layout

Phase 4 is complete when ALL of the following pass on the same commit:
- [ ] `cargo test --features default,decrypt,markdown -p pdftract-core --test assembly` — 100% pass
- [ ] Integration tests `tests/integration/assembly/{spans,lines,columns,blocks,reading_order,serialization,readability}.rs` all pass
- [ ] Phase 4.5 reading-order accuracy ≥ 95% on multi-column fixtures (Primary Objectives Accuracy row)
- [ ] Phase 4.6 plain-text + Markdown output validates byte-for-byte against `tests/fixtures/expected/`
- [ ] Phase 4.7 readability composite score ≥ 0.85 on `tests/fixtures/vector/` (Primary Objectives Accuracy row)
- [ ] Benchmark: 100-page vector PDF extracts in < 3 s on 4-core CI (Primary Objectives Speed row); hyperfine mean of 5 runs reported in `benches/results/<commit-sha>.json`
- [ ] Tier 4 competitive benchmark: ratio ≥ 10× vs `pdfminer.six==20231228` (Proof Obligation row 1)
- [ ] Tier 4 competitive benchmark: ratio ≥ 5× vs `pypdf==4.2.0` (Proof Obligation row 2)
- [ ] CER vs golden on regression corpus: regression Δ < 0.5% (Tier 3 gate)
- [ ] JSON output validates against `docs/schema/v1.0/pdftract.schema.json` for every fixture (INV-11)
- [ ] `cargo clippy --features default,decrypt,markdown -- -D warnings` clean

### Phase 5 — OCR Integration

Phase 5 is complete when ALL of the following pass on the same commit:
- [ ] `cargo test --features default,decrypt,ocr -p pdftract-core --test ocr` — 100% pass, glibc CI only (musl excluded per Phase 0 Step 2)
- [ ] Integration tests `tests/integration/ocr/{classification,extraction,preprocessing,tesseract,assisted_ocr,doc_type}.rs` all pass
- [ ] Phase 5.1 page classifier produces deterministic class labels for every fixture in `tests/fixtures/scanned/` and `tests/fixtures/vector/`
- [ ] Phase 5.4 acceptance: WER < 3% on `tests/fixtures/scanned/` 300-DPI corpus (Primary Objectives Accuracy row; Proof Obligation row 6)
- [ ] Phase 5.6 acceptance: ≥ 90% classification accuracy on 200-doc corpus (Proof Obligation row 5)
- [ ] OCR speed: 10-page scanned PDF extracts in < 30 s on 4-core CI (Primary Objectives Speed row)
- [ ] `pdftract classify` subcommand prints the correct label for every fixture in `tests/fixtures/classification/`
- [ ] `cargo clippy --features default,decrypt,ocr -- -D warnings` clean

### Phase 6 — Output and API

Phase 6 is complete when ALL of the following pass on the same commit:
- [ ] `cargo test --features full -p pdftract-core -p pdftract-cli -p pdftract-py` — 100% pass; the Python test suite (`pytest crates/pdftract-py/tests/`) also green
- [ ] Integration tests `tests/integration/output/{json,ndjson,markdown,multi_output}.rs` all pass
- [ ] JSON output validates against `docs/schema/v1.0/pdftract.schema.json` for every fixture (INV-11)
- [ ] Phase 6.4 acceptance: serve mode reports single-page extraction p99 < 150 ms under `wrk -t4 -c32 -d30s` (Primary Objectives Speed row)
- [ ] Phase 6.6 multi-output overhead ≤ 1.1× single-format time (Primary Objectives Weight row; Proof Obligation row 8)
- [ ] Phase 6.6 byte-identical per-format output regardless of concurrent activation (INV-7)
- [ ] Phase 6.7 MCP critical tests: stdio mode produces only JSON-RPC frames on stdout (INV-9); HTTP mode requires bearer token on non-loopback bind (TH-03 test)
- [ ] Phase 6.8 receipt round-trip: `extract --receipts=lite` followed by `pdftract verify-receipt` succeeds for every fixture (INV-5)
- [ ] Phase 6.9 cache-hit latency < 20 ms p99 for 100-page PDF (Primary Objectives Weight row; Proof Obligation row 9)
- [ ] Phase 6.9 byte-identical JSON across cache hit and fresh extraction (INV-6)
- [ ] Phase 6.10 `pdftract doctor` exits 0 in a fully-provisioned container and surfaces every defect in a container with all system libs missing
- [ ] PyO3 wheel builds for all five target triples via `pdftract-py-ci`; `pip install` smoke test passes on each
- [ ] `cargo clippy --features full -- -D warnings` clean

### Phase 7 — Advanced Features

Phase 7 is complete when ALL of the following pass on the same commit:
- [ ] `cargo test --features full -p pdftract-core -p pdftract-cli` — 100% pass
- [ ] Per-subsection integration tests: `tests/integration/advanced/{structtree,tables,signatures,acroform,attachments,annotations,article_threads,grep,inspect,profiles}.rs` all pass
- [ ] Phase 7.8 `grep` benchmark: ≥ 50 MB/s aggregate throughput on `tests/fixtures/grep-corpus/` (1000 PDFs; Primary Objectives Weight row; Proof Obligation row 10)
- [ ] Phase 7.8 `grep --highlight` produces annotated PDFs validating against `docs/schema/v1.0/pdftract.schema.json` highlights subschema
- [ ] Phase 7.9 inspector mode launches on `127.0.0.1:0` by default; binds to public address only with explicit `--bind` and a printed token
- [ ] Phase 7.9 inspector frontend bundle ≤ 80 KB minified (R12 risk register check)
- [ ] Phase 7.10 profiles: `pdftract profiles validate` rejects every fixture in `tests/fixtures/profiles/invalid/` with line-numbered diagnostics; accepts every fixture in `tests/fixtures/profiles/valid/`
- [ ] Phase 7.10 profile-resolution order matches the Phase 7.10 spec on every fixture in `tests/fixtures/profiles/resolution/`
- [ ] Default-feature binary still < 4 MB stripped (no Phase 7 feature contaminates default)
- [ ] `cargo clippy --features full -- -D warnings` clean

---

## Phase Dependencies and Sequencing

```
Phase 0 (CI Infrastructure) ← must complete before Phase 1 code review
  └─► Phase 1 (Core Parser)
        │   ├─ 1.7 PDF Structural Fingerprint ← feeds Phase 6.8 receipts and Phase 6.9 cache
        │   └─ 1.8 Remote Source Adapter (HTTP Range Reads) ← `remote` feature
        └─► Phase 2 (Font Pipeline)
              └─► Phase 3 (Content Stream)
                    └─► Phase 4 (Text Assembly)
                          ├─ 4.7 Readability Validation ← feeds back into 5.1 page classification
                          └─► Phase 5 (OCR)       ← Scanned PDFs work here; 4.7 escalates broken-vector pages here
                                ├─ 5.6 Document Type Classification ← feeds Phase 7.10 profile selection
                                └─► Phase 6 (Output and API)
                                      ├─ 6.1 JSON / 6.2 NDJSON / 6.3 PyO3 / 6.4 HTTP serve (existing)
                                      ├─ 6.5 Markdown Output (cross-cuts 6.6)
                                      ├─ 6.6 Multi-Output Emission Architecture
                                      ├─ 6.7 MCP Server Mode (stdio | HTTP, mutually exclusive)
                                      ├─ 6.8 Visual Citation Receipts ← depends on 1.7
                                      └─ 6.9 Content-Addressed Cache Layer ← depends on 1.7
                                            └─► Phase 7 (Advanced)
                                                  ├─ 7.1 StructTree (independent)
                                                  ├─ 7.2 Tables (independent)
                                                  ├─ 7.3 Signatures (independent)
                                                  ├─ 7.4 Forms (independent)
                                                  ├─ 7.5 Attachments (independent)
                                                  ├─ 7.6 Hyperlinks & Annotations (independent)
                                                  ├─ 7.7 Article Threads (independent)
                                                  ├─ 7.8 `pdftract grep` (depends on Phases 1–4)
                                                  ├─ 7.9 Inspector Mode (depends on Phase 6; uses 6.4 serve infra)
                                                  └─ 7.10 Document Profiles ← consumes 5.6 classification
```

Phase 0 is a prerequisite for all subsequent phases — no milestone release can ship without active CI. Phase 7 sub-tasks are independent of each other and can be assigned to separate developers once Phase 6 is complete.

**Cross-phase dependencies introduced by the new features:**
- 6.8 Receipts and 6.9 Cache depend on Phase 1.7's PDF Structural Fingerprint
- 7.10 Profiles depends on Phase 5.6's Document Type Classification
- 6.5 Markdown and 6.6 Multi-Output are tightly coupled — Markdown lands behind the multi-output architecture
- 6.7 MCP Server reuses 6.4 HTTP Serve infrastructure; both modes share the same handlers
- 7.8 grep and 7.10 profiles share the `regex` crate; either feature pulls it in

---

## Release Milestones

| Milestone | Phases Complete | Capability |
|---|---|---|
| v0.1.0 (Alpha) | 0, 1 (incl. 1.7 fingerprint, 1.8 remote source), 2–4 (incl. 4.7) | CI infrastructure active; vector PDF extraction with readability validation; plain text, JSON, **and Markdown** output via the multi-output architecture (Phase 6.5 + 6.6 ship in 0.1 because they are pure code on top of Phase 4); PDF structural fingerprint via `pdftract hash`; HTTP range-read remote source via `--features remote`; CLI only; all applicable primary objective targets must pass (OCR speed target excluded until v0.2.0) |
| v0.2.0 (Beta) | 0, 1–5 (incl. 5.6 classification) | + Scanned PDF OCR; all page classes handled; document type classifier (`pdftract classify`); competitive benchmark suite green |
| v0.3.0 (RC) | 0, 1–6 (incl. 6.7 MCP, 6.8 Receipts, 6.9 Cache) | + PyO3 bindings; HTTP serve; MCP server (stdio + HTTP modes, mutually exclusive); visual citation receipts (`--receipts=lite\|svg` with `pdftract verify-receipt`); content-addressed extraction cache (`pdftract cache stats\|clear\|purge`); full JSON schema; NDJSON streaming |
| v1.0.0 (Stable) | 0, 1–7 (incl. 7.8 grep, 7.9 inspector, 7.10 profiles) | + StructTree; tables; forms; signatures; attachments; hyperlinks; article threads; `pdftract grep` folder search with progress observability and `--highlight` annotated-PDF output; `pdftract inspect` web debug viewer; configurable document profiles (built-in + user YAML; `pdftract profiles` subcommand family) |

Binary releases for all five target triples are published to GitHub Releases on every milestone tag in two variants:
- `pdftract-<triple>` — `--features default` (~4 MB stripped)
- `pdftract-full-<triple>` — `--features full` (~14 MB stripped; includes mcp, inspect, grep, profiles, cache, receipts, remote, serve, ocr, markdown)

The PyO3 wheel is published to PyPI on every milestone tag. The full release pipeline — artifact taxonomy, distribution channels, signing, provenance, Argo WorkflowTemplates — is specified in the Release Engineering and Distribution section below. The multi-language SDK roster that consumes these artifacts is specified in SDK Architecture and Language Coverage.

---

## Release Engineering and Distribution

This section consolidates the artifact taxonomy, distribution channels, signing, and provenance policies that drive every milestone release. All publishing is automated by Argo WorkflowTemplates on the `iad-ci` cluster per ADR-009; secrets live in OpenBao and reach workflows via ESO-synced Kubernetes Secrets (see Secrets Handling in the Threat Model section).

### Artifact Taxonomy

Every milestone tag (`vX.Y.Z`) produces the same fixed set of artifacts. The set is identical across milestones — only the version and content differ. All artifacts MUST be reproducible from the tagged commit; Cargo.lock is checked in for the binary crates and `--locked --frozen` is enforced in every Argo build step.

| Artifact | Count | Channel | Contents |
|---|---|---|---|
| Binary archive (default features) | 5 (one per triple) | GitHub Release | `pdftract-vX.Y.Z-<triple>.tar.gz` (Unix) or `.zip` (Windows). Each contains: stripped binary, `LICENSE-MIT`, `LICENSE-APACHE`, `README.md`, `CHANGELOG.md` excerpt for this version |
| Binary archive (full features) | 5 (one per triple) | GitHub Release | `pdftract-full-vX.Y.Z-<triple>.tar.gz`. Same layout; built with `--features full` |
| `SHA256SUMS` | 1 | GitHub Release | Aggregate checksums for all binary archives AND the PyPI wheels AND the SBOM |
| `SHA256SUMS.sig` | 1 | GitHub Release | Sigstore-keyless signature (`cosign sign-blob`) of `SHA256SUMS`. Verifies every artifact in one shot via `cosign verify-blob --signature SHA256SUMS.sig SHA256SUMS` |
| `multiple.intoto.jsonl` | 1 | GitHub Release | SLSA Level 3 build provenance attestation naming the source commit, builder identity, exact command line, and materials consumed |
| `pdftract-vX.Y.Z.cdx.json` | 1 | GitHub Release | CycloneDX SBOM generated by `cargo cyclonedx` for both binary crates and the Python wheel |
| Python wheel | 5 (one per triple) | PyPI | `pdftract-X.Y.Z-cp311-cp311-<platform_tag>.whl`; abi3-tagged for forward compatibility across Python minor versions |
| Python sdist | 1 | PyPI | `pdftract-X.Y.Z.tar.gz` (source distribution for platforms with no prebuilt wheel) |
| Rust crates | 2 (or 3 with `pdftract-libpdftract`) | crates.io | `pdftract-core@X.Y.Z`, `pdftract-cli@X.Y.Z`; published in order by `pdftract-crates-publish` |
| Docker images | 3 base tags × 2 architectures = 6 image manifests under 3 multi-arch manifest lists | GHCR (`ghcr.io/jedarden/pdftract`) | `:X.Y.Z` (default features), `:ocr-X.Y.Z`, `:full-X.Y.Z`; also tagged `:latest`, `:ocr`, `:full` (floating); each manifest list signed via `cosign sign --yes` |

The 5 target triples: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-gnu`.

GitHub auto-generates source tarball and zip from the tag — no separate artifact.

**NOT in any release:**
- Build intermediates, dependency vendor archives, fuzz corpora, test fixtures (consumers retrieve them via `git archive` from the tag if needed)
- Pre-release artefacts (`vX.Y.Z-rc.N`) follow the same artifact set but publish to PyPI's pre-release channel (`pip install pdftract==X.Y.Z-rc.N` only — never installed by default `pip install pdftract`) and GHCR's pre-release tags; the GitHub Release is marked "pre-release"

### Distribution Channels

| Channel | What ships | Credential source |
|---|---|---|
| **GitHub Releases** | Binary archives, checksums, signatures, SLSA attestation, SBOM, release notes | GitHub PAT (OpenBao `github-pat-pdftract` → ESO → workflow) |
| **PyPI** | Python wheels + sdist | PyPI API token (OpenBao `pypi-token-pdftract` → ESO → workflow). NOT OIDC-trusted-publisher: that's GitHub-Actions-only, see ADR-009 |
| **crates.io** | `pdftract-core`, `pdftract-cli` (and `pdftract-libpdftract` if shipped) | crates.io API token (OpenBao `crates-io-token-pdftract` → ESO) |
| **GHCR** (`ghcr.io/jedarden/pdftract`) | Multi-arch Docker images (amd64 + arm64) for `:latest`, `:ocr`, `:full` plus version tags | GitHub PAT with `write:packages` (same source as the GitHub Releases credential) |
| **docs.rs** | Auto-generated Rust API docs for `pdftract-core` | Automatic on crates.io publish |
| **`pdftract.com`** (Cloudflare Pages) | User documentation (mdBook), live demo links | Cloudflare API token (OpenBao `cloudflare-pages-token` → ESO); built by `pdftract-docs-build` Argo template — same pattern as the existing `website-build` template |
| **Cargo binstall index** | Metadata referencing GitHub Release binaries so `cargo binstall pdftract` downloads pre-built binaries instead of compiling | Crates.io metadata field; no extra channel |

Homebrew formula, Nix flake, AUR, .deb/.rpm packaging are deferred to v1.1+ (see Non-Goals: "Native package-manager distribution beyond cargo/PyPI/Docker is deferred until v1.1+; users on Homebrew/Nix/Arch install via `cargo install` or the GHCR Docker image in the meantime").

### Argo WorkflowTemplates

The release pipeline is split into independent WorkflowTemplates so each can be re-run idempotently if any single channel fails. All templates live in `jedarden/declarative-config → k8s/iad-ci/argo-workflows/`.

| Template | Trigger | Output | Failure mode |
|---|---|---|---|
| `pdftract-ci` | Every push, every PR | Test + lint + bench + audit + bloat results | Blocks PR merge |
| `pdftract-build-binaries` | Milestone tag (`vX.Y.Z`) | 10 binary archives uploaded as Argo artifacts | Tag retried via `argo retry`; partial output discarded |
| `pdftract-py-ci` | Milestone tag | 5 wheels + sdist | Re-runnable; PyPI rejects duplicate uploads (manual `pip yank` required to retry the same version) |
| `pdftract-crates-publish` | Milestone tag, after `pdftract-build-binaries` green | `pdftract-core` published, wait for crates.io index propagation (max 5 min poll), then `pdftract-cli` | Re-runnable; crates.io rejects duplicate publishes; partial publish leaves a half-published version recoverable via `cargo yank` |
| `pdftract-docker-build` | Milestone tag | 3 multi-arch manifest lists pushed to GHCR with cosign signatures | Re-runnable; tag-overwrite policy in GHCR permits idempotent retry |
| `pdftract-github-release` | After all above complete | One GitHub Release populated with binary archives, `SHA256SUMS`, `SHA256SUMS.sig`, `multiple.intoto.jsonl`, SBOM, release notes generated by `git-cliff` from Conventional Commits since the previous tag | Re-runnable; existing release replaced via `gh release create --clobber` |
| `pdftract-docs-build` | Milestone tag, after `pdftract-crates-publish` (so docs.rs links resolve) | mdBook user docs deployed to Cloudflare Pages | Re-runnable |
| `pdftract-sdk-<lang>-publish` | Milestone tag, after `pdftract-build-binaries` | One per non-native SDK (see SDK Architecture and Language Coverage); publishes to npm / NuGet / RubyGems / etc. | Re-runnable; rate-limit-aware |

### Signing and Provenance

Three layers of supply-chain assurance, all generated by Argo on `iad-ci`:

1. **`SHA256SUMS.sig`** — Sigstore keyless signature of `SHA256SUMS`, generated by `cosign sign-blob` with the Argo runner's OIDC identity from the `iad-ci` cluster's OIDC issuer. Verifiable in seconds with `cosign verify-blob`.
2. **`multiple.intoto.jsonl`** — SLSA Level 3 build provenance attestation. Names the source commit, the builder identity, the tools used, the exact command line, and the materials consumed. Generated via `slsa-github-generator` adapted for Argo Workflows.
3. **Docker image signing** — Each multi-arch manifest signed via `cosign sign --yes ghcr.io/jedarden/pdftract:X.Y.Z@sha256:...`. Discoverable via `cosign tree ghcr.io/jedarden/pdftract:X.Y.Z`.

### License Files

The pdftract project is dual-licensed under **MIT OR Apache-2.0** (standard Rust convention). Each binary archive ships both `LICENSE-MIT` and `LICENSE-APACHE`. Each crate's `Cargo.toml` declares `license = "MIT OR Apache-2.0"`. The Python wheel ships both license files in its `dist-info`. Each Docker image carries both in `/usr/share/doc/pdftract/`. The `cargo deny` license-check policy is configured to permit the project's own licenses plus MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib — and reject GPL/AGPL/LGPL in default-feature dependencies.

### Minimum Supported Rust Version (MSRV)

`pdftract-core` and `pdftract-cli` SHALL build on Rust 1.78 or newer. MSRV is pinned via `rust-version = "1.78"` in both `Cargo.toml` files and tested on every PR by a matrix step in `pdftract-ci` that runs `cargo build --features default` against `rust:1.78-slim`. Bumping MSRV is a MINOR-version event with at least one release of warning in `CHANGELOG.md`; never a PATCH bump. New direct dependencies whose MSRV exceeds the project's MSRV are rejected at code-review time.

### Cross-Platform Test Limitation (KU-12)

Per ADR-009, `iad-ci` is Linux-only. macOS and Windows binaries are **built** via `cross` but never **executed** in CI. This is acknowledged as Known Unknown KU-12 with the following mitigation:

- A manual smoke-test runbook in `docs/operations/manual-platform-smoke.md` is executed by the release lead before each milestone tag against at least one physical macOS machine and one Windows VM
- User bug reports for platform-specific issues acknowledged within 48 hours and addressed in the next patch release
- README and marketing copy state: "Linux is fully CI-tested; macOS and Windows are build-tested and manually smoke-tested per release"
- No claim of "tested on macOS/Windows" appears in CI status badges

Adding GitHub-Actions-driven macOS/Windows runtime testing is OUT OF SCOPE per ADR-009. Re-evaluated at v1.0.0 sign-off based on actual platform-bug volume.

### Contributor Workflow

Because CI runs on the private `iad-ci` cluster, external contributors cannot trigger CI from their fork. `CONTRIBUTING.md` SHALL state:

1. Fork and open a pull request against `jedarden/pdftract:main`
2. A maintainer will trigger the `pdftract-ci` Argo workflow against your branch (results posted as a PR comment)
3. **Local validation expected before opening the PR:** `cargo test --features default`, `cargo clippy --all-targets -- -D warnings`, `cargo bloat --release --features default` (binary size within budget), `cargo audit` (no medium+ advisories)
4. PR template requires: linked issue or RFC, scope statement (which Phase / which Acceptance Scenario), test plan, manual-test evidence, performance impact (if hot path touched)

`SECURITY.md` accompanies the Threat Model with the responsible-disclosure contact (`security@jedarden.com`) and a 90-day disclosure window aligned with industry norms. Reported vulnerabilities are tracked privately; CVEs are filed via GitHub's private vulnerability reporting; advisories are coordinated with downstream package maintainers (Homebrew, distro packagers if any exist at the time).

`CODE_OF_CONDUCT.md` adopts the Contributor Covenant v2.1.

`.github/ISSUE_TEMPLATE/` directory contains templates for: bug reports (must include `pdftract doctor` output), feature requests, performance regressions, and security advisories (which redirect to `SECURITY.md`).

### Release Engineering Acceptance Criteria

- A milestone tag triggers ALL release workflows automatically; no manual step beyond the tag push
- All artifacts verifiable from a single `cosign verify-blob --signature SHA256SUMS.sig SHA256SUMS`
- `cosign verify ghcr.io/jedarden/pdftract:X.Y.Z` succeeds against the keyless Sigstore identity
- `cargo binstall pdftract` on a clean machine downloads the binary archive matching the host triple and verifies its checksum
- `pip install pdftract` on a clean machine installs the appropriate platform wheel
- A failed channel publish (e.g. PyPI 5xx) does NOT block other channels — partial release is acceptable and rerunnable
- Release rollback is `git revert` + new patch release; no published artifact is ever DELETED (yank only — preserves historical reachability)
- Release readiness gated by the Pre-Release Go/No-Go checklist (see Rollout and Rollback)

---

## SDK Architecture and Language Coverage

The CLI binary's JSON output schema (`schema_version: 1.0`) IS the API. Every SDK in every language exposes the same method surface — `extract`, `extract_text`, `extract_markdown`, `extract_stream`, `search`, `get_metadata`, `hash`, `classify`, `verify_receipt` — and chooses the transport that fits the language ecosystem.

### Repository Layout (monorepo)

All SDK source is vendored in **this monorepo** at root-level `pdftract-<lang>/` directories (`pdftract-go/`, `pdftract-dotnet/`, `pdftract-java/`, `pdftract-node/`, …) — a single source of truth, versioned and CI-tested alongside the CLI/core they wrap. SDKs are NOT maintained as separate repositories. The `pdftract sdk codegen --lang <L>` generator emits/refreshes the in-repo `pdftract-<L>/` directory (its `--out` defaults to the monorepo path, not a sibling). Each SDK is still **published** to its language registry (PyPI, npm, crates.io, Maven Central, NuGet, pkg.go.dev, …) from the monorepo by the release pipeline; the registry/package names in "The Ten SDKs" below are publish targets, not separate source repos. (Go note: the module path is served from the `pdftract-go/` subdirectory; the legacy standalone `github.com/jedarden/pdftract-<lang>` repos are retired/archived in favor of the monorepo.)

### Integration Patterns

| Pattern | When to use | Pros | Cons |
|---|---|---|---|
| **Subprocess** (default for non-native SDKs) | All non-native SDKs | Zero FFI, single binary distribution, the JSON contract IS the wire format, easy versioning | 10–50 ms spawn cost per call |
| **HTTP client** (to `pdftract serve`) | Long-lived servers, web apps, scripts hitting the same files often | No spawn cost; multi-tenant friendly; any language with an HTTP library | Server MUST be running |
| **Native FFI** | Only when the ecosystem strongly demands it (Python, C/C++) | Native types; zero IPC overhead | Per-language build matrix; ABI versioning hell |
| **MCP** | LLM agent integration (covered in Phase 6.7) | Standard protocol; agent-native | Limited to MCP-compatible clients |

WASM is explicitly NOT a transport — see Non-Goals.

### The Ten SDKs

| # | Language | Primary Transport | Package | Phase |
|---|---|---|---|---|
| 1 | **Python** | PyO3 native binding; subprocess fallback if the native module fails to load (musl-only environments, exotic platforms) | PyPI: `pdftract` | v0.3.0 (Phase 6.3 — already in plan) |
| 2 | **Rust** | Direct crate import (no IPC) | crates.io: `pdftract-core`, `pdftract-cli` | v0.3.0 (Phase 6; crates.io publish per Release Engineering) |
| 3 | **JavaScript / TypeScript** (Node.js) | Subprocess via `child_process.spawn` + JSON stream parse; async API via `Readable` streams; native ESM + CJS dual-package | npm: `@pdftract/sdk` | v1.0.0 |
| 4 | **Go** | Subprocess via `os/exec` + `encoding/json` Decoder; context.Context-aware for cancellation | go module: `github.com/jedarden/pdftract-go` (git-tag-based; no central registry); `pkg.go.dev` auto-indexed | v1.0.0 |
| 5 | **Java / Kotlin** | Subprocess via `ProcessBuilder` + Jackson; AutoCloseable `Pdftract` client; Kotlin extension functions in the same artifact | Maven Central: `com.jedarden:pdftract` (via OSSRH staging) | v1.0.0 |
| 6 | **C# / .NET** | Subprocess via `System.Diagnostics.Process` + `System.Text.Json`; async-first (`Task<Document> ExtractAsync(...)`) | NuGet: `Pdftract` | v1.0.0 |
| 7 | **C / C++** | Native FFI via `libpdftract` shared library (`cdylib` Cargo target); cbindgen-generated `pdftract.h`; `extern "C"` API returns owned JSON strings the caller frees with `pdftract_free()`; reentrant; thread-safe | GitHub Release (`.so` / `.dylib` / `.dll` + `.h` + `.pc` pkg-config file) + Homebrew formula + vcpkg port | v1.0.0 |
| 8 | **Ruby** | Subprocess via `Open3` + `JSON.parse` | RubyGems: `pdftract` | v1.1+ |
| 9 | **PHP** | Subprocess via `proc_open` + `json_decode`; PSR-3 logger integration | Packagist: `jedarden/pdftract` (Composer auto-discovers from git tag) | v1.1+ |
| 10 | **Swift** | Subprocess via `Process` + `JSONDecoder`; Linux + macOS (server-side use; not iOS) | Swift Package Manager: `pdftract-swift` (git-tag-based) | v1.1+ |

Drop-in alternatives if a v1.1+ language slot is reassigned based on user demand: Kotlin (separate from Java for Android-first), Dart (Flutter), Elixir (BEAM document pipelines), R (data science). Re-evaluated at v1.0.0 sign-off.

### The SDK Contract

Every SDK SHALL implement the same surface. The full spec lives in `docs/notes/sdk-contract.md`; this section summarizes it.

**Method surface (mirrors the CLI subcommands and MCP tool catalog):**

| Method | Maps to CLI | Maps to MCP tool |
|---|---|---|
| `extract(path_or_url, options) -> Document` | `pdftract extract --json` | `extract` |
| `extract_text(path_or_url, options) -> string` | `pdftract extract --text` | `extract_text` |
| `extract_markdown(path_or_url, options) -> string` | `pdftract extract --md` | `extract_markdown` |
| `extract_stream(path_or_url, options) -> Iterator<Page>` | `pdftract extract --ndjson` | (streaming via MCP not exposed) |
| `search(path_or_url, pattern, options) -> Iterator<Match>` | `pdftract grep` | `search` |
| `get_metadata(path_or_url, options) -> Metadata` | `pdftract extract --metadata-only` | `get_metadata` |
| `hash(path_or_url, options) -> Fingerprint` | `pdftract hash` | `hash` |
| `classify(path_or_url) -> Classification` | `pdftract classify` | `classify` |
| `verify_receipt(path, receipt) -> bool` | `pdftract verify-receipt` | (not exposed via MCP) |

**Error mapping (CLI exit code → native exception class):**

| Exit | Meaning | Native exception |
|---|---|---|
| 0 | Success | (no exception) |
| 2 | Corrupt PDF | `CorruptPdfError` |
| 3 | Encrypted, password missing or wrong | `EncryptionError` |
| 4 | Source unreadable (file or URL) | `SourceUnreachableError` |
| 5 | Network interrupted | `RemoteFetchInterruptedError` |
| 6 | TLS or certificate failure | `TlsError` |
| 10 | Receipt verification failed | `ReceiptVerifyError` |
| any other non-zero | Internal | `PdftractError` (base class) |

Every language-specific exception inherits from a single `PdftractError` base type per the language's conventions: Python `class PdftractError(Exception)`, Java `class PdftractException extends Exception`, C# `class PdftractException : Exception`, Go (single error type with `errors.As`-compatible kind), etc.

**Versioning compatibility:**

- SDK semver is pinned to binary semver
- SDK MAJOR matches binary MAJOR exactly (`@pdftract/sdk@1.x.y` works with `pdftract@1.0.0` through `pdftract@1.x.x`)
- SDK MINOR may add wrappers for new binary features behind feature flags; calling a method whose underlying CLI subcommand the binary doesn't recognise raises `UnsupportedOperationError`
- SDK rejects a binary whose MAJOR differs from its own with a clear startup error
- SDK constructor accepts an explicit binary path; otherwise probes PATH; otherwise downloads the matching binary version into a per-user cache (opt-in via `auto_install=true`)

### The Conformance Suite

`tests/sdk-conformance/cases.json` is the shared, language-neutral test specification. Each case has:

```json
{
  "id": "extract-vector-academic-paper",
  "fixture": "fixtures/vector/academic-paper-2col.pdf",
  "method": "extract",
  "options": {"ocr": false},
  "expected": {
    "metadata.page_count": 12,
    "metadata.document_type": "scientific_paper",
    "pages[0].blocks[0].kind": "heading",
    "errors.length": 0
  },
  "tolerances": {
    "pages[*].blocks[*].bbox": {"abs": 0.5}
  }
}
```

Every SDK has a `pdftract-sdk-conformance` test runner that executes the suite against its native client + the bundled binary. CI gate: 100% pass for v1.0.0 SDK release.

The suite is the SDK API contract — adding or modifying a case requires updating every SDK before the corresponding milestone tag.

### Code Generation and Maintenance Leverage

The C/`libpdftract` binding is hand-maintained (cbindgen output + a `cdylib` Cargo target).

The 8 subprocess SDKs share:
- A single Tera template (`templates/sdk-skeleton/<lang>/`)
- A generator subcommand: `pdftract sdk codegen --lang go --out pdftract-go`
- The shared conformance suite

The generator emits the package skeleton, method stubs, the conformance-test runner, and the language-native error hierarchy. Hand-written content is limited to: idiomatic ergonomics on top of the stubs, async wrappers where the language prefers async, the language's package metadata file (`package.json`, `go.mod`, `pom.xml`, etc.). Typical SDK after generation: ~300 LOC, ~150 LOC hand-written.

### Per-SDK Release Channels

Each SDK has its own Argo WorkflowTemplate that runs on milestone tags, after `pdftract-build-binaries` completes:

| SDK | Argo template | Channel | Credential source (OpenBao key) |
|---|---|---|---|
| `pdftract-py` | `pdftract-py-ci` (already in plan) | PyPI | `pypi-token-pdftract` |
| `pdftract-rust` | `pdftract-crates-publish` (Release Engineering) | crates.io | `crates-io-token-pdftract` |
| `pdftract-node` | `pdftract-node-publish` | npm | `npm-token-pdftract` |
| `pdftract-go` | `pdftract-go-publish` | git tag on `github.com/jedarden/pdftract-go`; `pkg.go.dev` auto-indexes | `github-pat-pdftract` |
| `pdftract-java` | `pdftract-java-publish` | Maven Central via OSSRH | `ossrh-creds-pdftract` + `ossrh-gpg-key` |
| `pdftract-dotnet` | `pdftract-dotnet-publish` | NuGet.org | `nuget-api-key-pdftract` |
| `pdftract-libpdftract` | `pdftract-libpdftract-build` | GitHub Release (binary), Homebrew formula PR (auto-opened), vcpkg port PR (manual reviewer involvement) | `github-pat-pdftract` for the formula PR |
| `pdftract-ruby` | `pdftract-ruby-publish` | RubyGems | `rubygems-api-key-pdftract` |
| `pdftract-php` | `pdftract-php-publish` | Packagist (auto-discovers from git tag — no token needed) | n/a |
| `pdftract-swift` | `pdftract-swift-publish` | git tag on `github.com/jedarden/pdftract-swift` (SPM is git-tag-based) | `github-pat-pdftract` |

Each SDK lives in its own git repository to keep release cadence and issue tracking independent.

### SDK Acceptance Criteria

- **Hard gate (containerized conformance execution)**: All subprocess-generated SDKs (Ruby, PHP, .NET, Swift, Node, Go, Java) MUST pass containerized conformance execution before bead closure and before publish workflows proceed. Conformance must run inside the official language Docker image on iad-ci (e.g., `ruby:3.2-slim`, `node:22-slim`, `golang:1.22`) via the `sdk-conformance-verify` WorkflowTemplate. **A link to the passing conformance run MUST be attached to the bead before closure** — structural completeness alone is insufficient; the code must actually execute and pass. **Closing a generated-SDK bead without a conformance-run link is a defect and the bead MUST be reopened.**
- 100% of the shared conformance suite passes on every SDK before publishing
- SDK ships within 24 hours of binary release (Argo cascade is automatic)
- SDK README documents: install command, three usage examples (basic extract, OCR, search), binary version compatibility matrix, troubleshooting (binary not found, version mismatch, network failure)
- SDK exposes language-native types for `Document`, `Page`, `Span`, `Block`, `Match`, `Fingerprint`, `Classification` — NOT raw JSON dicts
- SDK respects the language's async conventions where applicable (Node.js: Promises; Python: optional async via `asyncio.to_thread`; C#: `Task<T>`; Java: `CompletableFuture<T>` optional; Go: context.Context for cancellation)
- SDK option names mirror the CLI flags after language-native casing conversion: `--ocr-language` → Node `ocrLanguage` / Python `ocr_language` / Go `OCRLanguage` / Java `ocrLanguage` / C# `OcrLanguage`
- Conformance suite results published as an Argo artifact and linked from each SDK's README

### Maintenance Reality Check

10 SDKs is real ongoing work. Honest budget:

- 1 maintainer can cover all 10 if and only if: the contract is rigid (changes require an ADR), conformance is comprehensive, subprocess SDKs are kept thin (no business logic above the binary), and native FFI is limited to Python + C
- Initial implementation: ~3 weeks for the first 5 non-Python SDKs (Node, Go, Java, C#, C-FFI) post-Phase 6
- Steady-state for a binary release that doesn't change the JSON schema → all SDKs auto-pass conformance and ship via Argo cascade with zero per-SDK code change; only the version field updates
- Schema changes (rare; gated by `schema_version` bump) → one PR per SDK to add wrappers for new fields; all 10 PRs can be opened in a single afternoon if the generator template is current

Re-evaluate the SDK roster at v1.0.0 sign-off based on actual user demand signals (download counts, GitHub stars, issues filed per SDK).

---

## Migration Plan

pdftract is greenfield: there is no prior pdftract release to migrate from. The Migration Plan exists nonetheless because the project commits to a multi-axis versioning contract from v0.1.0 onward. Every artifact pdftract produces (binary, JSON output, fingerprint, profile YAML, cache entry) carries a version label, and every cross-version transition has a defined keep/drop/reinterpret policy. The plan exists so that the first user who upgrades from v0.X to v1.0 — or from v1.0 to v2.0 — can do so deterministically.

### Versioned Axes

| Axis | Field name | Bumped by | Consumer impact |
|---|---|---|---|
| Binary semver | `pdftract --version` | Source code changes (per Backward Compatibility rules below) | CLI users, embedders of `pdftract-core` |
| JSON output schema | `schema_version` in JSON output (e.g. `"1.0"`) | Additive: minor. Breaking: major. | Downstream consumers parsing pdftract JSON |
| Fingerprint algorithm | Prefix on every fingerprint string (`pdftract-v1:…`) | Always a major-version bump on the binary; the version prefix changes | Any user relying on stable fingerprints across releases (cache, receipts) |
| Profile YAML | `profile_version` field (e.g. `"1.0.0"`) inside every profile YAML | Profile-spec changes; the loader emits `PROFILE_VERSION_MISMATCH` if unsupported | Users authoring custom profiles |
| Cache entry | `extraction_version` field in every cache entry (matches the binary semver of the producer) | Bumps with the binary | Cache-hit logic; mismatched entries are cache misses, NOT errors |

### Keep / Drop / Reinterpret Matrix

The table below documents the upgrade policy per axis. "Keep" means the new release accepts the old field unchanged; "Drop" means the field is removed (only allowed at major); "Reinterpret" means the semantic meaning changes (only allowed at major, with a documented migration step).

| Axis | Patch (X.Y.Z+1) | Minor (X.Y+1.0) | Major (X+1.0.0) |
|---|---|---|---|
| CLI flag name | Keep | Keep + ADD new (old also keeps working) | Keep with deprecation warning OR Drop with `--FLAG no longer supported` |
| CLI exit code | Keep | Keep (new codes only) | May reassign (with Revision History entry) |
| JSON `schema_version` | Keep (same) | Increment minor (additive only) | Increment major; old reader sees unknown root, refuses |
| JSON field within current schema_version | Keep | Add (consumers SHOULD tolerate unknown fields per ADR-008 family) | Drop / Reinterpret with `schema_version` major bump |
| Fingerprint prefix | Keep (`pdftract-v1:`) | Keep | Bump (`pdftract-v2:`) |
| Profile YAML `profile_version` | Keep | Increment minor (additive); old profiles still load | Increment major; old profiles trigger `PROFILE_VERSION_MISMATCH`, surface a clear migration message |
| Profile field name | Keep | Add new fields; deprecated fields log a warning | Remove deprecated field; emit clear error |
| Cache `extraction_version` | Keep | Treat mismatch as miss, opportunistic LRU eviction | Treat mismatch as miss; `pdftract cache purge` recommended |

### Sample Upgrade Scenarios

**Scenario M-01: A consumer parses `schema_version: "1.0"` output today; upgrades to a pdftract that emits `"1.1"`.**
The consumer's parser SHOULD ignore unknown fields. The new fields in 1.1 are documented as `OPTIONAL` in the schema; missing them never breaks 1.0-era code. Per the policy above, 1.1 is a strict superset of 1.0.

**Scenario M-02: A user has a custom profile `invoice-v3.yaml` with `profile_version: "1.0.0"`. They upgrade to a pdftract built against profile spec `2.0.0`.**
The loader emits `PROFILE_VERSION_MISMATCH` with a clear error: "Profile invoice-v3.yaml declares profile_version 1.0.0; this binary supports 2.x. See `docs/migrations/profiles-v2.md` for the migration guide." pdftract exits 78 (configuration error) for that profile; other profiles still load.

**Scenario M-03: A receipt issued by `pdftract-v1:` fingerprints is verified by a binary at fingerprint algorithm v2.**
The receipt verification step inspects the prefix. If the binary's algorithm version differs, the verification fails with `RECEIPT_FINGERPRINT_VERSION_MISMATCH` and points to the `pdftract migrate-fingerprints` tool (introduced if and only if v2 ever ships).

**Scenario M-04: A cache populated by pdftract 1.0.0 is read by pdftract 1.1.0.**
The cache reader compares `extraction_version` in the entry against its own. Different patch / minor: cache miss (per LRU policy in Phase 6.9); old entry is evicted opportunistically on the next write. Different major: cache miss; `pdftract cache purge` is recommended to free disk immediately.

### Migration Tooling

The following tools ship if and only if the corresponding migration ever becomes required:

| Tool | Ships when | What it does |
|---|---|---|
| `pdftract migrate-fingerprints --from v1 --to v2 OLD_DIR NEW_DIR` | A fingerprint algorithm bump ever happens | Re-hashes every PDF in `OLD_DIR` under the new algorithm; writes the mapping to `NEW_DIR/fingerprint-map.json` |
| `pdftract migrate-profile FILE` | Profile-spec major bump | Rewrites `FILE` in place (with `.bak` backup) under the new spec; reports any field that requires manual review |
| `pdftract cache migrate` | Cache layout schema change | Re-encodes every cache entry into the new layout in-place |

### Schema Migration Policy

The JSON output schema (`docs/schema/v1.0/pdftract.schema.json`) follows JSON-Schema-style additive-evolution rules:

- `schema_version: "1.1"` SHALL be a **strict superset** of `"1.0"`: every `"1.0"`-valid document SHALL also be `"1.1"`-valid. New fields are optional; no field is removed; no field's semantic meaning changes within a major version.
- Downstream consumers reading `"1.1"` output with a `"1.0"`-aware parser MUST tolerate unknown fields. The schema explicitly sets `additionalProperties: true` for the v1.x line to make this enforceable.
- Semantic changes to an existing field require a major-version bump and a corresponding `schema_version` major bump (`"2.0"`). The Revision History MUST flag the change with a migration note pointing to a per-axis migration guide under `docs/migrations/`.

### Profile-Version Deprecation Window

When a profile field is deprecated in a minor release:
1. The field continues to work for at least **two minor releases** after the deprecation announcement (e.g. deprecated in 1.4.0 → removed at the earliest in 2.0.0, but in practice never removed before 1.6.0 even if a major bump happens earlier).
2. The loader emits a `PROFILE_FIELD_DEPRECATED` warning each time the field is read; the warning includes the line number in the YAML.
3. The CHANGELOG entry for the deprecation release names the field, the deprecation reason, and the recommended replacement.

### Cache Invalidation Policy

An `extraction_version` mismatch in a cache entry is **always a cache miss, never an error**. The cache is opportunistic by design. Mismatched entries are evicted lazily by the LRU policy; operators who want to reclaim space immediately run `pdftract cache purge` (Phase 6.9). This policy ensures that upgrading the binary never breaks a `pdftract serve` deployment.

### Backward Compatibility

This subsection is normative; the Versioned Axes table above governs the contract.

**Semver semantics.** The project follows semantic versioning (`MAJOR.MINOR.PATCH`):

- **MAJOR bump** (e.g. 1.x.x → 2.0.0) is required for any of:
  - Renaming or removing a CLI flag (e.g. `--out FILE` → something else)
  - Changing an exit code's meaning
  - Bumping `schema_version` past minor
  - Bumping the fingerprint algorithm version
  - Changing an MCP tool's signature (parameter names or types)
  - Changing a PyO3 API signature (function or method)
  - Changing the cache layout in a way that requires `cache migrate`
- **MINOR bump** (e.g. 1.4.0 → 1.5.0) for:
  - New CLI flag (MUST be optional; default behavior unchanged)
  - New schema fields (MUST be optional)
  - New MCP tool
  - New profile type or new profile field
  - New subcommand
  - New feature flag
- **PATCH bump** (e.g. 1.4.0 → 1.4.1) for:
  - Bug fixes that preserve all observable behavior on conforming inputs
  - Internal refactors with zero API surface change
  - Documentation fixes

**Deprecation window.** Any breaking change in a MAJOR bump MUST be preceded by at least one MINOR release that emits a `DEPRECATED` warning. The CHANGELOG.md entry for the deprecation release names the breaking change planned for the next major, with a migration guide URL.

**`ExtractionOptions` field deprecation.** Deprecated `ExtractionOptions` fields log a warning when set but continue to work for the duration of the deprecation window. The Python `ExtractionOptions` class issues a `DeprecationWarning` per `warnings.warn(…, DeprecationWarning)`; the CLI emits a stderr `WARN: --FLAG is deprecated; use --NEW-FLAG`. Removed fields trigger an immediate error (exit 64; `RuntimeError` in Python).

**CLI flag removal.** Removing or renaming a flag in a MINOR is FORBIDDEN. Removal happens only in MAJOR. After removal, the flag emits `--FLAG is no longer supported; use --NEW-FLAG` (if a replacement exists) or `--FLAG is no longer supported; this functionality was removed in vX.0.0` (if not) and exits 64.

**Library `pdftract-core` semver.** The library crate follows the same semver semantics. Adding a new public function or struct field marked with `#[non_exhaustive]` is a MINOR change. Removing or changing a public signature is a MAJOR change. The crate is published with `rust-version = "1.74"` (or the current MSRV); raising the MSRV is a MINOR-level event, lowering it is PATCH.

---

## Rollout and Rollback

This section codifies the release gate, the canary policy, and the rollback signal taxonomy. The release-gate checklist below MUST run on every milestone tag (`v0.1.0`, `v0.2.0`, …, `v1.0.0`) before the tag is created. Any failed item blocks the tag.

### Pre-Release Go/No-Go Checklist

For every milestone tag, ALL of the following items MUST be green on the same commit:

- [ ] All Phase Completion Criteria for the phases included in this milestone are green (per the Phase Completion Criteria section)
- [ ] All Tier 1 (unit) tests pass with zero flakes across 10 consecutive runs
- [ ] All Tier 2 (integration) tests pass on every supported triple
- [ ] All Tier 3 (regression corpus) tests pass with CER regression Δ < 0.5% vs the previous tag
- [ ] All Tier 4 (competitive benchmarks) pass minimum bars: ≥ 10× pdfminer.six, ≥ 5× pypdf, binary ≤ 4 MB stripped (default features)
- [ ] All Tier 5 (property + fuzz) tests pass with zero new corpus additions in the same PR
- [ ] Binary size is within budget for every triple in both `--features default` and `--features full` variants (Weight Targets)
- [ ] Adoption baseline metrics recorded into `benches/results/<tag>.json` for the quarterly review
- [ ] CHANGELOG.md updated with a new top-level entry naming all user-visible changes, deprecations, and breaking changes
- [ ] SemVer impact reviewed: no surprise breaking change in a MINOR or PATCH (Backward Compatibility)
- [ ] Threat Model entries unchanged, OR each change reviewed and recorded with a test fixture
- [ ] Proof Obligations Ledger: no claim is currently invalidated; every claim has a passing CI signal
- [ ] `pdftract doctor` exits 0 in a representative Docker container for each variant
- [ ] CI status for the tagged commit is green across `pdftract-ci`, `pdftract-py-ci`, and `pdftract-fuzz` (latest nightly run)
- [ ] Security advisories: `cargo audit` clean of severity ≥ medium

### Canary Policy

Pre-release versions are tagged as `vX.Y.Z-rc.N` (e.g. `v1.0.0-rc.1`). Per the canary policy:
- **PyPI:** Pre-release wheels are uploaded with the pre-release marker; `pip install pdftract` SHALL NOT install them by default (a user installs an RC with `pip install pdftract --pre`).
- **GitHub Releases:** Pre-release tags are marked "pre-release" in the GitHub UI; binaries are present but not advertised on the project's homepage.
- **Docker Hub:** Pre-releases get an explicit `:1.0.0-rc.1` tag; the `:latest` tag never points to a pre-release. The `:next` floating tag (introduced for canary use) follows the most recent pre-release.
- **MCP integrations:** RC builds connect to RC-tagged Claude Desktop / Cursor / Continue test instances first; production MCP configs are not updated until the RC has soaked for ≥ 1 week with no signal.

### Production Rollback

Every binary release is retained on GitHub Releases forever; no release is ever deleted. Users downgrade by:
- **Cargo:** `cargo install pdftract --version X.Y.Z` (locks to a specific version)
- **PyPI:** `pip install pdftract==X.Y.Z`
- **Docker:** `docker pull ronaldraygun/pdftract:X.Y.Z` (the floating `:latest` is never used in production per Rollback and binary downgrade in Cross-Cutting Concerns)
- **GitHub Releases:** download the prior `pdftract-<triple>` or `pdftract-full-<triple>` binary

The rollback path is documented in `docs/operations/rollback.md` with one runbook per install method.

### Rollback Signals

A rollback is triggered when **any** of the following signals fires within 14 days of a release. The signal is recorded in the project's incident log; the rollback decision is made by the release lead.

| Signal | Detection | Threshold |
|---|---|---|
| Accuracy regression on the regression corpus | Tier 3 metric tracked per release | CER > 0.5% above the previous tag's baseline |
| Latency regression | Tier 4 hyperfine median (or `pdftract serve` p99 latency in adopter telemetry) | p99 > 20% above the previous tag's baseline |
| User-reported correctness bugs | Issues tagged `bug` and `correctness` filed against the new tag | > 5 within 48 hours of release |
| Security advisory | `cargo audit` advisory or external CVE filed against pdftract or a direct dep | CVSS ≥ 7 |
| Critical OS / packaging regression | Smoke tests in `pdftract-ci` post-release | any failure on a supported triple |
| Adoption signal | PyPI weekly downloads drop > 30% week-over-week after a release | only counts if the cause is clearly the release |

### Rollback Action

The release lead executes the rollback by:
1. Filing an incident issue with the signal, the affected version, and the planned action
2. Reverting the offending commit(s) via `git revert` (NEVER `git reset --hard`; never amend a tagged release commit)
3. Tagging an immediate patch release (`X.Y.Z+1`) containing only the revert
4. Updating CHANGELOG.md with the rollback note and the original release's status changed to "withdrawn"
5. Opening a GitHub Discussions thread under "Announcements" naming the issue, the rollback, and the recommended downgrade target
6. If a security signal triggered the rollback, filing a GitHub Security Advisory with the affected versions

The patch release MUST go through the same Pre-Release Go/No-Go Checklist as a normal release. A rollback is NOT an excuse to skip gates.

---

## Monitoring and Alerting

`pdftract serve --metrics PORT` (and `pdftract mcp --bind ... --metrics PORT`) exposes a Prometheus-compatible `/metrics` endpoint on the given port. This subsection specifies the metric surface and the operator-tunable alert thresholds.

**Feature flag.** `metrics` (implicitly enabled by `serve`). No new direct crates beyond `axum` (already pulled in by `serve`); metrics are formatted as plain text per OpenMetrics v1.0.

**Endpoint policy.**
- `/metrics` MUST bind only on the `--metrics PORT` listener, NOT on the main `serve` or `mcp` port. This permits a different network reachability for metrics scraping vs production traffic.
- `/metrics` is unauthenticated by default; operators are RECOMMENDED to restrict scraping at the network layer (firewall, K8s `NetworkPolicy`).
- `/metrics` content-type is `application/openmetrics-text; version=1.0.0; charset=utf-8`.

**Metric surface.** All metric names are prefixed `pdftract_`. Counters end in `_total`; histograms in `_seconds` or `_bytes`; gauges have no suffix.

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `pdftract_extractions_total` | counter | `result="success|error"`, `ocr="true|false"` | Extractions started, partitioned by outcome and OCR path |
| `pdftract_extraction_duration_seconds` | histogram | — | Wall-clock extraction time per request; buckets: `[0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10, 30, 60]` |
| `pdftract_pages_extracted_total` | counter | — | Pages emitted (sum across requests) |
| `pdftract_cache_hits_total` | counter | — | Cache hits (Phase 6.9) |
| `pdftract_cache_misses_total` | counter | — | Cache misses |
| `pdftract_cache_size_bytes` | gauge | — | Current on-disk cache size |
| `pdftract_mcp_requests_total` | counter | `tool="extract|search|hash|classify|verify_receipt|…"` | MCP tool invocations |
| `pdftract_http_requests_total` | counter | `endpoint`, `status` | HTTP requests by endpoint and status code |
| `pdftract_remote_bytes_downloaded_total` | counter | — | HTTP range-read traffic from `remote` adapter (Phase 1.8) |
| `pdftract_diagnostic_emitted_total` | counter | `code`, `severity="error|warn|info"` | Diagnostics emitted, partitioned by code |
| `pdftract_inflight_extractions` | gauge | — | Extractions currently in progress |
| `pdftract_rayon_pool_utilization` | gauge | — | Fraction of rayon worker threads currently busy (0..1) |
| `pdftract_build_info` | gauge (constant 1) | `version`, `git_sha`, `features` | Build identification for the `info` join |

**Suggested alert thresholds** (operator-tunable; pdftract ships sample Prometheus rules in `docs/operations/prometheus-rules.yaml`):

| Alert | Rule | Severity |
|---|---|---|
| Slow extractions | `histogram_quantile(0.99, pdftract_extraction_duration_seconds) > 5` for 5m | warn |
| Cache underperforming | `pdftract_cache_hits_total / (pdftract_cache_hits_total + pdftract_cache_misses_total) < 0.30` for 1h | info |
| Diagnostic flood | `sum(rate(pdftract_diagnostic_emitted_total{severity="error"}[5m])) > 10` | warn |
| HTTP 5xx rate | `sum(rate(pdftract_http_requests_total{status=~"5.."}[5m])) / sum(rate(pdftract_http_requests_total[5m])) > 0.01` for 5m | page |
| Worker pool saturated | `pdftract_rayon_pool_utilization > 0.95` for 5m | warn |
| Cache size growing unchecked | `deriv(pdftract_cache_size_bytes[1h]) > 1e9` (1 GB/h) for 6h | warn |

**Health and readiness endpoints.**
- `GET /health` returns `200 OK` with `{"status":"ok","version":"X.Y.Z"}`. Always returns 200 as long as the process is up; intended for liveness probes.
- `GET /ready` returns `200 OK` only when the rayon pool utilization is below 90% AND the cache (if enabled) is writable. Returns 503 otherwise. Intended for readiness probes; routing layers SHOULD pull a node out of rotation when `/ready` reports 503.

**Cardinality.** Operators are warned not to use unbounded labels (e.g. per-request paths); the `endpoint` label on `pdftract_http_requests_total` is restricted to the registered route templates, never the raw path.
