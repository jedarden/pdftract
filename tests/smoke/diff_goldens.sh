#!/bin/sh
# Diff a pdftract binary's smoke-corpus output against the recorded goldens.
#
# This is the comparison step of the per-release platform smoke test
# (docs/operations/manual-platform-smoke.md). Run it on the platform under
# test (macOS: Terminal; Windows: Git Bash or WSL) against the binary that was
# installed from the release archive:
#
#     tests/smoke/diff_goldens.sh /path/to/installed/pdftract
#
# Per fixture it extracts corpus/<name>.pdf to a temp dir and byte-compares
# the JSON with goldens/<name>.json. Extraction output is deterministic given
# identical PDF bytes, so any difference is a real finding, not noise: a
# platform divergence in the archive under test, or an unintended extraction
# regression since the goldens were recorded.
#
# Exit codes: 0 all fixtures match · 1 at least one fixture differs or failed
# to extract · 2 usage/corpus problem · 3 goldens not recorded for this corpus
# revision (bootstrap needed — see tests/smoke/README.md).
set -u

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
[ -d "$GOLDENS" ] && ls "$GOLDENS"/*.json >/dev/null 2>&1 || {
    echo "error: no goldens recorded for this corpus revision" >&2
    echo "bootstrap: run record_goldens.sh from a reference binary that" >&2
    echo "extracts the corpus (see tests/smoke/README.md)" >&2
    exit 3
}

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    else
        shasum -a 256 "$1" | cut -d' ' -f1
    fi
}

# Same corpus, same comparison — verify the corpus bytes first.
while read -r expected name; do
    [ -n "$name" ] || continue
    actual=$(sha256_file "$CORPUS/$name")
    if [ "$actual" != "$expected" ]; then
        echo "error: corpus/$name does not match corpus.manifest" >&2
        exit 2
    fi
done <"$MANIFEST"

TMPDIR_SMOKE=$(mktemp -d "${TMPDIR:-/tmp}/pdftract-smoke.XXXXXX")
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT INT TERM

failures=0
total=0
for pdf in "$CORPUS"/*.pdf; do
    name=$(basename "$pdf" .pdf)
    total=$((total + 1))
    actual_json="$TMPDIR_SMOKE/$name.json"
    golden="$GOLDENS/$name.json"
    if ! "$BIN" extract "$pdf" --json "$actual_json" >/dev/null 2>&1; then
        echo "FAIL $name: extraction exited non-zero"
        failures=$((failures + 1))
        continue
    fi
    if diff -u "$golden" "$actual_json" >"$TMPDIR_SMOKE/$name.diff" 2>&1; then
        echo "PASS $name"
    else
        echo "FAIL $name: output differs from goldens/$name.json"
        sed -n '1,20p' "$TMPDIR_SMOKE/$name.diff"
        failures=$((failures + 1))
    fi
done

echo "----------------------------------------"
echo "smoke golden diff: $((total - failures))/$total fixtures match"
if [ "$failures" -gt 0 ]; then
    echo "RESULT: FAIL — see docs/operations/manual-platform-smoke.md failure policy"
    exit 1
fi
echo "RESULT: PASS"
exit 0
