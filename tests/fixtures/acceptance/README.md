# CLI acceptance fixtures

The manifest runs the real `pdftract extract` CLI against tracked PDFs. Each
case pins its input by SHA-256 and compares stdout and any output files byte for
byte with `expected/`. The vector cases use the W3C `test-minimal.pdf` for
text, JSON, Markdown (`--md`), NDJSON, and simultaneous output. Cases run in
manifest order with the C locale and UTC timezone.

The `font-recovery` case uses `encoding/no-mapping.pdf`, which has no ToUnicode
map. Character codes 0–6 have unmapped custom glyph names and extract as seven
U+FFFD characters; codes 7–8 use `/A` and `/B` in the font's `/Differences`
array and recover as Unicode `AB` through glyph-name mapping. The exact golden
also checks the trailing space. The `layout-reading-order` case uses
`hybrid/hybrid-003-mixed-column-layout.pdf`: the left vector heading and article
blocks must precede the right-column caption in the extracted text. The PDF's
right-column body is an image, so this case checks vector text order only.

The two `ocr` cases exercise the opt-in CLI OCR path on tracked CC0 PDFs from
the scanned corpus. `ocr-scanned-noisy` compares OCR-sourced JSON text with its
reviewed ground truth at a strict WER < 5% and requires the `scanned` page
route. `ocr-mixed-vector-scanned` uses the raster body ground truth at WER < 5%,
checks the vector header is still present, compares the CLI text output with
`expected/ocr-mixed-vector-scanned.txt`, and requires the `mixed` page route.
Both cases run together under `--category ocr`; the runner rejects a manifest
that omits either kind. It fails before extraction when `pdfimages`, Tesseract,
or English traineddata is missing, and reports a binary built without the
`ocr` feature explicitly.

Build a binary, then run vector cases from the repository root:

```sh
cargo build --locked -p pdftract-cli --bin pdftract
PDFTRACT_BINARY=/build/pdftract/debug/pdftract \
  PDFTRACT_FIXTURE_ARTIFACT_DIR="$PWD/fixture-artifacts" \
  scripts/run-fixture-acceptance.sh --category vector
```

Run just the font and layout cases with the repeatable category selector:

```sh
PDFTRACT_BINARY=/build/pdftract/debug/pdftract \
  PDFTRACT_FIXTURE_ARTIFACT_DIR="$PWD/fixture-artifacts" \
  scripts/run-fixture-acceptance.sh --category font --category layout
```

The runner retains each case's `stdout`, `stderr`, `status.json`, any output
files, OCR WER metrics, and a `summary.json` in the specified artifact directory, including on
failure. A missing fixture, wrong fixture hash, missing golden, changed golden,
or missing CLI output fails the run.

For OCR, use this reproducible Nix environment from the repository root.
The `LIBCLANG_PATH` and `BINDGEN_EXTRA_CLANG_ARGS` values let the Rust
Tesseract bindings find Nix's libclang and C headers. The shell supplies
Tesseract 5.5.2, Leptonica 1.87.0, and Poppler's `pdfimages` on this host.

```sh
nix-shell -p tesseract leptonica pkg-config clang poppler-utils --run '
  export LIBCLANG_PATH=$(dirname "$(clang -print-file-name=libclang.so)")
  libc_include=$(clang -E -v -x c /dev/null 2>&1 |
    grep -oE "/nix/store/[^ ]*-glibc-[^ ]*-dev/include" | head -1)
  export BINDGEN_EXTRA_CLANG_ARGS="-isystem $libc_include"
  cargo build --locked -p pdftract-cli --bin pdftract --features ocr
  mkdir -p "$PWD/.verify-ocr-tmp"
  TMPDIR="$PWD/.verify-ocr-tmp" \
    PDFTRACT_BINARY=/build/pdftract/debug/pdftract \
    PDFTRACT_FIXTURE_ARTIFACT_DIR="$PWD/.verify-ocr-artifacts" \
    scripts/run-fixture-acceptance.sh --category ocr
'
```

The vector and encoding fixtures' provenance is in
`tests/fixtures/PROVENANCE.md`; the mixed-column fixture is documented in
`tests/fixtures/hybrid/README.md`. OCR fixture provenance and ground truth
hashes are in `tests/fixtures/scanned/` sidecars.
