#!/bin/sh
# Record platform-smoke goldens from a reference pdftract binary.
#
# Goldens are the canonical expected `pdftract extract --json` outputs for
# tests/smoke/corpus/. They are recorded ONCE per behavioral baseline (normally
# from the Linux release build at a release tag — extraction output is pure
# computation over the PDF bytes, so the Linux reference defines what every
# platform must produce) and committed. Later releases and the macOS/Windows
# smoke runs diff against them via diff_goldens.sh.
#
# Usage:  tests/smoke/record_goldens.sh /path/to/pdftract
#
# The corpus is verified against corpus.manifest before anything is recorded;
# refresh the manifest with generate_corpus.py if the corpus changed
# deliberately, and commit the new goldens together with the new manifest.
#
# Exit codes: 0 recorded all goldens · 1 extraction failed (nothing recorded
# for the failing fixtures) · 2 usage/corpus/manifest problem.
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CORPUS="$SCRIPT_DIR/corpus"
GOLDENS="$SCRIPT_DIR/goldens"
MANIFEST="$SCRIPT_DIR/corpus.manifest"

[ $# -eq 1 ] || {
    echo "usage: $0 /path/to/pdftract" >&2
    exit 2
}
BIN=$1
[ -x "$BIN" ] || {
    echo "error: not an executable: $BIN" >&2
    exit 2
}
[ -f "$MANIFEST" ] || {
    echo "error: corpus.manifest missing — run generate_corpus.py first" >&2
    exit 2
}

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    else
        # macOS ships shasum instead of sha256sum
        shasum -a 256 "$1" | cut -d' ' -f1
    fi
}

# Corpus bytes must match the committed manifest, otherwise the goldens would
# silently describe a different fixture set than the one smoke runs use.
while read -r expected name; do
    [ -n "$name" ] || continue
    actual=$(sha256_file "$CORPUS/$name")
    if [ "$actual" != "$expected" ]; then
        echo "error: corpus/$name does not match corpus.manifest" >&2
        echo "  expected $expected" >&2
        echo "  actual   $actual" >&2
        echo "regenerate with generate_corpus.py if the change is deliberate" >&2
        exit 2
    fi
done <"$MANIFEST"

mkdir -p "$GOLDENS"
status=0
for pdf in "$CORPUS"/*.pdf; do
    name=$(basename "$pdf" .pdf)
    out="$GOLDENS/$name.json"
    if "$BIN" extract "$pdf" --json "$out" >/dev/null 2>&1 && [ -s "$out" ]; then
        echo "recorded goldens/$name.json"
    else
        rm -f "$out"
        echo "FAILED: extraction of corpus/$name.pdf — golden not recorded" >&2
        status=1
    fi
done

if [ "$status" -eq 0 ]; then
    # Provenance of this recording, for the completed smoke checklist. The
    # goldens themselves stay pure outputs; this note records who made them.
    bin_sha=$(sha256_file "$BIN")
    {
        echo "# Golden recording provenance"
        echo
        echo "- binary: \`$($BIN --version 2>&1 | head -1)\`"
        echo "- binary sha256: \`$bin_sha\`"
        echo "- corpus.manifest sha256: \`$(sha256_file "$MANIFEST")\`"
        echo "- platform: \`$(uname -s -m 2>/dev/null || echo unknown)\`"
    } >"$GOLDENS/RECORDING.md"
    echo "wrote goldens/RECORDING.md"
fi

exit $status
