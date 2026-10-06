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

Build a binary, then run all cases from the repository root:

```sh
cargo build --locked -p pdftract-cli --bin pdftract
PDFTRACT_BINARY=/build/pdftract/debug/pdftract \
  PDFTRACT_FIXTURE_ARTIFACT_DIR="$PWD/fixture-artifacts" \
  scripts/run-fixture-acceptance.sh
```

Run just the font and layout cases with the repeatable category selector:

```sh
PDFTRACT_BINARY=/build/pdftract/debug/pdftract \
  PDFTRACT_FIXTURE_ARTIFACT_DIR="$PWD/fixture-artifacts" \
  scripts/run-fixture-acceptance.sh --category font --category layout
```

The runner retains each case's `stdout`, `stderr`, `status.json`, any output
files, and a `summary.json` in the specified artifact directory, including on
failure. A missing fixture, wrong fixture hash, missing golden, changed golden,
or missing CLI output fails the run.

The vector and encoding fixtures' provenance is in
`tests/fixtures/PROVENANCE.md`; the mixed-column fixture is documented in
`tests/fixtures/hybrid/README.md`.
