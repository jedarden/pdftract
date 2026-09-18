# Distribution verification

Status checked 2026-09-17 from a clean extraction of commit `162e30ba`.

## Locally supported images

The Dockerfile accepts the selectors `default` and `full`. `full` expands to
the workspace CLI features that compile cleanly in this checkout:
`serve,mcp,inspect,grep,cache,receipts,markdown`. OCR, remote sources,
profiles, and PDFium remain planned tiers because their feature-gated code does
not currently compile from a clean checkout; they are not advertised as image
contents.

Both variants use the pinned `rust:1.90.0-bookworm` builder
(`sha256:3914072ca0c3b8aad871db9169a651ccfce30cf58303e5d6f2db16d1d8a7e58f`)
and `gcr.io/distroless/cc-debian12:nonroot`
(`sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f`)
runtime. The image sizes below are Docker's uncompressed `.Size` value on the
verification host:

| Variant | Compiled CLI features | Core default features | Size |
|---|---|---|---:|
| `default` | none | `serde,decrypt,quick-xml,cache` | 16,640,200 bytes |
| `full` | `serve,mcp,inspect,grep,cache,receipts,markdown` | `serde,decrypt,quick-xml,cache` | 16,917,321 bytes |

## Smoke and clean-consumer verification

The automated contract is `scripts/verify-docker.sh`. For each image it runs
`pdftract --version`, prints `doctor --features`, bind-mounts
`tests/fixtures/test-minimal.pdf`, extracts JSON, and requires the canonical
text `Dummy PDF file`.

The same clean extraction passed:

```text
docker build --check --build-arg FEATURES=default <clean-extraction>       # exit 0
docker build --check --build-arg FEATURES=full <clean-extraction>          # exit 0
PDFTRACT_DOCKER_VARIANTS="default full" sh scripts/verify-docker.sh       # exit 0
```

No crates.io, PyPI, GitHub Release, GHCR, Homebrew formula, or hosted website
is claimed as currently installable. The README and user guide list those as
planned channels only; a real release must add clean-consumer checks before
adding pull/install commands.

The Argo Docker workflow runs the same default/full smoke script before its
release-time build/sign/SBOM stages and records the JSON size/feature report as
an artifact.
