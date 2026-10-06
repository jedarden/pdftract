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
- The historical weezl incompatibility assessment below is superseded by the
  2026-10-06 review addendum; migration still requires integration validation.
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
    budget. Both targets are configured to run nightly in the
    `pdftract-nightly-fuzz` CronWorkflow under the memory ceiling (1.5 GiB
    cgroup cap with libFuzzer
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

## Operational verification status

The 2026-09-15 measurements in `docs/notes/lzw-fuzz-coverage.md` are local
coverage evidence, not evidence from an in-cluster nightly run. A read-only
check of `iad-ci` on 2026-09-28 found the deployed `pdftract-nightly-fuzz`
CronWorkflow and its controller annotation `last-used-schedule: 0 4 * * *`,
with `lastScheduledTime: 2026-09-28T04:00:00Z`, but its status was
`failed: 11`, `succeeded: 0`; no retained Workflow with terminal phase, exit
code, or logs was available to export. The live object also used the older
sequential DAG with `podGC: OnWorkflowCompletion` and
`ttlStrategy.secondsAfterCompletion: 259200` (three days), rather than the
current in-tree manifest's retention fields.

The companion `pdftract-nightly-supply-chain` CronWorkflow is now present in
the in-tree manifest and the `iad-ci` declarative-config manifest. A GitOps
verification created Workflow `pdftract-nightly-supply-chain-1790623980`; the
cluster reported its `setup` node as `Succeeded`, and the setup pod remained
available for log inspection. This is cluster deployment and partial execution
evidence, not a passing nightly result: the namespace quota kept `cargo-audit`
pending while stale one-minute verification attempts drained, and the
application-wide Argo sync also encountered unrelated API-discovery failures.
Until the Workflow reaches a terminal phase with recorded exit statuses and
task logs, nightly execution of the compensating control remains
**unverified**. The local 2026-09-15 coverage measurements must continue to be
described as local evidence only.

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
- Nightly supply-chain job: `.ci/argo-workflows/pdftract-nightly-supply-chain.yaml`
  (in-tree source), `pdftract-nightly-supply-chain` CronWorkflow in
  `declarative-config` (`k8s/iad-ci/argo-workflows/`) for the deployed manifest
- LZW fuzz seeds: `fuzz/seeds/lzw_decode/`, regenerated with
  `scripts/gen_lzw_fuzz_seeds.py`
- Coverage evidence: `docs/notes/lzw-fuzz-coverage.md` (measured 2026-09-15,
  bead pdftract-fc90d8fa)

## Review addendum — 2026-10-06 (pdftract-26b61246)

- **Installed versions and tree:** `cargo tree -i lzw --locked` reports
  `lzw 0.10.0` directly in `pdftract-cli` and `pdftract-core` (and transitively
  in consumers of core). `cargo tree -i weezl --locked` reports `weezl 0.1.12`
  through `gif`, `lopdf`, and `tiff`; it is not the PDF decoder dependency.
- **Maintenance and advisory sources:** [crates.io](https://crates.io/api/v1/crates/lzw)
  still lists `lzw 0.10.0` as the newest release (2016-02-28). The
  [RustSec advisory record](https://github.com/RustSec/advisory-db/blob/main/crates/lzw/RUSTSEC-2020-0144.md)
  still marks RUSTSEC-2020-0144 *informational/unmaintained*, with no patched
  release; the [lzw advisory directory](https://github.com/RustSec/advisory-db/tree/main/crates/lzw)
  contained no separate vulnerability advisory on the review date.
  [weezl 0.2.1](https://crates.io/api/v1/crates/weezl) is the newest published
  release (2026-05-15). Its [changelog](https://github.com/image-rs/weezl/blob/master/Changes.md)
  and [open issues](https://github.com/image-rs/weezl/issues) do not announce a
  PDF-specific mode or port.
- **PDF compatibility assessment:** The published weezl decoder already has
  MSB bit order plus TIFF code-size switching, matching this repository's
  `lzw::DecoderEarlyChange` path for PDF `/EarlyChange 1`; its standard switch
  matches `/EarlyChange 0`. Temporary, dependency-neutral probes on 2026-10-06
  decoded all eight tracked early/late LZW fixtures identically with both
  `weezl 0.1.12` and `0.2.1`. On deterministic 1,024- and 8,192-byte varied
  inputs and an 8,192-byte repeated input, weezl streams decoded identically
  with the corresponding `lzw` decoder across code-width changes; the `lzw`
  late-change encoder also round-tripped through weezl. A truncated fixture
  yielded 11 partial output bytes through weezl's `into_vec` API before
  `InvalidCode`, so partial recovery appears implementable. These probes do
  not establish parity for every malformed PDF, predictor, or decompression
  budget path. Porting an early-change variant upstream is unnecessary on the
  observed code-size behavior.
- **Risk and action:** The exception remains provisionally acceptable because
  the advisory is about maintenance, no distinct lzw vulnerability was found,
  and the existing local fuzz evidence covers the current decoder. Nightly
  compensating-control execution remains unverified as described above. The
  historical claim that no compatible weezl path exists is no longer a sound
  reason to keep `lzw`. The already locked `weezl 0.1.12` compiled with Rust
  1.75 in the probe, below this workspace's 1.78 minimum; `weezl 0.2.1`
  declares Rust 1.88 and cannot be adopted under the current MSRV. Proposed
  implementation bead `pdftract-e2dc29e2` is held for human admission to
  validate bounded streaming, truncation, predictors, fixtures, fuzzing, and
  the MSRV before changing the dependency or decoder.
- **Next due:** Review the exception when that implementation proposal has a
  verified outcome, on any new lzw security advisory or viable replacement
  development, and in all cases by 2027-09-15 (annual September review).
  `pdftract-26b61246` remains the single monitoring owner for the advisory,
  weezl/PDF-LZW, and ADR-002 option-ext commitments.
