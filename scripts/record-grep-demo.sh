#!/usr/bin/env bash
# Regenerate docs/assets/grep-demo.gif from the committed grep fixture.
set -euo pipefail

repo_root=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
output=${1:-"$repo_root/docs/assets/grep-demo.gif"}

if ! command -v vhs >/dev/null 2>&1; then
    printf '%s\n' 'error: vhs is required (v0.11 or newer)' >&2
    exit 1
fi
if ! command -v ttyd >/dev/null 2>&1; then
    printf '%s\n' 'error: ttyd is required by vhs' >&2
    exit 1
fi

if [[ -n "${PDFTRACT_BIN:-}" ]]; then
    pdftract_bin=$PDFTRACT_BIN
else
    build_root=${CARGO_TARGET_DIR:-"${TMPDIR:-/tmp}/pdftract-grep-demo-target"}
    CARGO_TARGET_DIR=$build_root cargo build -p pdftract-cli --features grep --release
    pdftract_bin="$build_root/release/pdftract"
fi

if [[ ! -x "$pdftract_bin" ]]; then
    printf 'error: pdftract binary is not executable: %s\n' "$pdftract_bin" >&2
    exit 1
fi

mkdir -p "$(dirname -- "$output")"
output_abs=$(CDPATH= cd -- "$(dirname -- "$output")" && pwd)/$(basename -- "$output")
bin_dir=$(CDPATH= cd -- "$(dirname -- "$pdftract_bin")" && pwd)

# VHS accepts a relative tape output, while -o lets this wrapper place the
# generated GIF at the requested path. The PATH entry keeps the tape portable.
(cd "$repo_root" && PATH="$bin_dir:$PATH" vhs scripts/grep-demo.tape -o "$output_abs")

printf 'wrote %s (%s bytes)\n' "$output_abs" "$(wc -c < "$output_abs")"
