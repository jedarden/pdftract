# Vector CLI acceptance fixture

The manifest runs the real `pdftract extract` CLI against the tracked W3C
`tests/fixtures/test-minimal.pdf` vector PDF. Its SHA-256 pins the input, and
`expected/` contains the exact bytes for text, JSON, Markdown (`--md`), and
NDJSON. The final case produces text on stdout while writing Markdown and JSON
files in the same invocation. Cases run in manifest order with the C locale
and UTC timezone.

Build a binary, then run all cases from the repository root:

```sh
cargo build --locked -p pdftract-cli --bin pdftract
PDFTRACT_BINARY=/build/pdftract/debug/pdftract \
  PDFTRACT_FIXTURE_ARTIFACT_DIR="$PWD/fixture-artifacts" \
  scripts/run-fixture-acceptance.sh
```

The runner retains each case's `stdout`, `stderr`, `status.json`, any output
files, and a `summary.json` in the specified artifact directory, including on
failure. A missing fixture, golden, or CLI output fails the run. To check the
failure path, copy the manifest to a temporary location, change one expected
path to a different existing golden, and pass `--manifest` with that copy.

The fixture's provenance and license are recorded in
`tests/fixtures/PROVENANCE.md`.
