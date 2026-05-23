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
- ** fuzzing**: The LZW decoder is covered by the project's fuzzing harness

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
