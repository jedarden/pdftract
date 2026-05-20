# pdftract

[![MSRV](https://img.shields.io/badge/MSRV-1.78-orange)](https://github.com/rust-lang/rust/releases/tag/1.78.0)

A PDF text extraction library that gets the hard parts right.

## What it does

- **Correct reading order** — layout regions are segmented and sequenced before text is emitted, handling multi-column pages, sidebars, footnotes, and mixed-layout documents without relying on PDF operator order
- **Font encoding recovery** — when `ToUnicode` CMaps are absent, wrong, or incomplete, pdftract works through a layered recovery pipeline: glyph name lookup via the Adobe Glyph List, font fingerprinting against known metrics and embedded checksums, and glyph outline shape matching
- **Structure tree extraction** — PDF/UA and PDF/A documents encode their logical structure (headings, paragraphs, lists, tables, reading order) in a `StructTree`; pdftract reads this directly when present, producing accurate semantic output at no extra cost
- **Per-page hybrid routing** — each page is independently classified and routed to the appropriate pipeline: vector text extraction, full OCR, or assisted OCR where vector hints improve raster accuracy
- **Structured output with provenance** — the primary output is JSON carrying per-span bounding boxes, font name, size, and confidence score alongside the extracted text, not a flat string dump

## Output

```json
{
  "pages": [
    {
      "page": 1,
      "blocks": [
        { "kind": "heading", "text": "Introduction", "bbox": [72, 680, 400, 700] },
        { "kind": "paragraph", "text": "...", "bbox": [72, 640, 540, 670] }
      ],
      "spans": [
        { "text": "Introduction", "bbox": [72, 680, 400, 700], "font": "Times-Bold", "size": 14.0, "confidence": 0.99 }
      ]
    }
  ],
  "metadata": { "title": "...", "author": "...", "page_count": 10 }
}
```

## Usage

```
pdftract extract invoice.pdf            # structured JSON to stdout
pdftract extract invoice.pdf --text     # plain text to stdout
pdftract extract invoice.pdf --output out.json
pdftract serve --port 8080              # HTTP service: POST /extract
```

## Architecture

Rust core with PyO3 Python bindings and a CLI binary. The same binary runs as a command-line tool or as an HTTP microservice — the container deployment is just `pdftract serve`.

See `docs/research/` for technical deep-dives into the PDF specification, font encoding, glyph Unicode recovery, and tagged PDF structure. See `docs/notes/` for SDK invocation examples in Python, Node.js, Go, Ruby, Java, Rust, and Bash.

## Verifying Releases

All releases are signed using [Sigstore](https://sigstore.dev/) keyless signing with OIDC from the iad-ci cluster. This provides cryptographic proof that artifacts were produced by the official CI/CD pipeline and haven't been tampered with.

### Verify Binary Archives

To verify downloaded binary archives:

```bash
# Download release artifacts
gh release download vX.Y.Z --dir /tmp/pdftract-release

# Verify the SHA256SUMS signature
cosign verify-blob \
  --certificate-identity-regexp 'https://iad-ci-oidc.ardenone.com.*' \
  --certificate-oidc-issuer 'https://iad-ci-oidc.ardenone.com' \
  --signature SHA256SUMS.sig \
  --certificate SHA256SUMS.pem \
  SHA256SUMS

# Verify individual artifacts against checksums
sha256sum -c SHA256SUMS
```

### Verify Docker Images

To verify Docker images before running them:

```bash
# Verify the main image
cosign verify \
  --certificate-identity-regexp 'https://iad-ci-oidc.ardenone.com.*' \
  --certificate-oidc-issuer 'https://iad-ci-oidc.ardenone.com' \
  ghcr.io/jedarden/pdftract:X.Y.Z

# Verify the OCR variant
cosign verify \
  --certificate-identity-regexp 'https://iad-ci-oidc.ardenone.com.*' \
  --certificate-oidc-issuer 'https://iad-ci-oidc.ardenone.com' \
  ghcr.io/jedarden/pdftract:ocr-X.Y.Z

# Verify the full variant
cosign verify \
  --certificate-identity-regexp 'https://iad-ci-oidc.ardenone.com.*' \
  --certificate-oidc-issuer 'https://iad-ci-oidc.ardenone.com' \
  ghcr.io/jedarden/pdftract:full-X.Y.Z
```

### View SLSA Provenance

Each Docker image includes SLSA provenance attestation:

```bash
cosign verify-attestation \
  --certificate-identity-regexp 'https://iad-ci-oidc.ardenone.com.*' \
  --certificate-oidc-issuer 'https://iad-ci-oidc.ardenone.com' \
  --type slsaprovenance \
  ghcr.io/jedarden/pdftract:X.Y.Z
```

The provenance includes the build configuration, source commit, and builder identity.

## Security

For responsible disclosure of security vulnerabilities, please email [security@jedarden.com](mailto:security@jedarden.com). See [`SECURITY.md`](SECURITY.md) for our disclosure policy, supported versions, and PGP key for encrypted reports.

**PGP Key:** The public key for `security@jedarden.com` is available at [`docs/security/pgp-public-key.asc`](docs/security/pgp-public-key.asc).

> **NOTE:** The PGP key is currently a placeholder. The security contact must generate and publish a 4096-bit RSA key for `security@jedarden.com`. See `docs/security/pgp-public-key.asc` for generation instructions.

## Status

Early development. See `docs/plan/` for the implementation roadmap.
