# ADR-003: RUSTSEC-2020-0144 Advisory Exception for lzw Crate

## Status
Accepted

## Context
The lzw crate (v0.10.0) is subject to RUSTSEC-2020-0144, which marks the crate as
unmaintained. pdftract uses the lzw crate to implement the LZWDecode filter for PDF
streams, as specified in the PDF 1.7 specification (section 7.4.4).

## Decision
RUSTSEC-2020-0144 is explicitly ignored for the lzw crate until a viable alternative
becomes available.

## Rationale
- LZW is a **mandatory PDF filter** - the PDF spec requires LZWDecode support for full compliance
- The lzw crate is the only Rust LZW implementation compatible with PDF LZW encoding
- Alternative crate (weezl) is **incompatible** with PDF LZW:
  - PDF LZW uses "early code change" variant (code tables reset at 256 vs 257)
  - weezl only supports standard LZW (GIF/TIFF variants)
  - PDF test fixtures fail to decode correctly with weezl
- The lzw crate is simple (~400 LOC) and has been stable for years
- No security vulnerabilities have been reported in the lzw algorithm implementation
- The "unmaintained" status reflects lack of new features, not security issues

## Alternatives Considered
- **weezl crate**: Incompatible with PDF LZW encoding (early code change variant)
- **Pure Rust implementation**: Would require re-implementing and testing ~400 LOC of complex bit manipulation
- **C binding (libtiff)**: Violates pdftract's zero-dependency-beyond-libc goal

## Risk Assessment
- **Low risk**: The lzw crate is small, stable, and handles a well-defined algorithm
- **No known CVEs**: RUSTSEC-2020-0144 is about maintenance status, not a specific vulnerability
- **Contained scope**: LZW decoding is a single, well-tested code path
- **Fuzzing**: The LZW decode path is exercised by two cargo-fuzz targets:
  - `stream_decoder` — every decompression filter including LZWDecode with
    default parameters, plus the decompression-bomb limit (INV-8 / EC-10)
  - `lzw_decode` — the PDF `/DecodeParms` space around LZWDecode specifically:
    `/EarlyChange` 0 (GIF variant) and 1 (Adobe/TIFF variant), TIFF predictor 2
    and PNG predictors 10-15 via `apply_predictor`, and `/Columns`, `/Colors`,
    `/BitsPerComponent` including the `MAX_ROW_BYTES` clamping path; every
    input runs under both the 512 MiB document budget and a 100-byte bomb
    budget. Both targets run nightly in the `pdftract-nightly-fuzz`
    CronWorkflow under the memory ceiling (1.5 GiB cgroup cap with libFuzzer
    `-rss_limit_mb`/`-malloc_limit_mb` at 1024 MB in the in-tree manifest;
    bounded `-rss_limit_mb` and pod memory limits in the deployed
    declarative-config manifest), seeded with real LZW streams from
    `tests/fixtures/` (see `fuzz/seeds/lzw_decode/`). Coverage evidence:
    `docs/notes/lzw-fuzz-coverage.md` (measured 2026-09-15: 1,339,822
    executions in 151 s, 1,273 new coverage units, peak RSS 461 MB under
    the 1024 MB cap, zero crashes)

## Consequences
- pdftract can continue using the lzw crate for LZWDecode filter support
- This exception will be re-evaluated if:
  - A security vulnerability is discovered in lzw
  - A compatible Rust LZW library becomes available
  - PDF spec changes remove the LZW requirement

## Future Work
- Monitor the weezl crate for PDF-compatible LZW support
- Consider contributing PDF LZW variant to weezl
- Re-evaluate this ADR annually or upon security reports

## References
- RUSTSEC-2020-0144: https://rustsec.org/advisories/RUSTSEC-2020-0144
- lzw crate: https://crates.io/crates/lzw
- PDF 1.7 spec, section 7.4.4: LZWDecode filter
- Fuzz targets: `fuzz/fuzz_targets/lzw_decode.rs`, `fuzz/fuzz_targets/stream_decoder.rs`
- Nightly fuzz job: `.ci/argo-workflows/pdftract-nightly-fuzz.yaml` (in-tree
  source), `pdftract-nightly-fuzz` CronWorkflow in `declarative-config`
  (`k8s/iad-ci/argo-workflows/`) for the deployed manifest
- LZW fuzz seeds: `fuzz/seeds/lzw_decode/`, regenerated with
  `scripts/gen_lzw_fuzz_seeds.py`
- Coverage evidence: `docs/notes/lzw-fuzz-coverage.md` (measured 2026-09-15,
  bead pdftract-fc90d8fa)
