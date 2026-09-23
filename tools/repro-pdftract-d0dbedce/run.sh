#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
PDF="$ROOT/tools/repro-pdftract-d0dbedce/input.pdf"

if [[ -n "${PDFTRACT_BIN:-}" ]]; then
    command=("$PDFTRACT_BIN")
else
    command=(cargo run --quiet -p pdftract-cli --)
fi

set +e
output=$("${command[@]}" extract "$PDF" --json - 2>&1)
status=$?
set -e

printf '%s\n' "$output"

if [[ "$status" -ne 1 ]]; then
    printf 'expected exit 1, got %s\n' "$status" >&2
    exit 1
fi
if ! grep -Fq 'No trailer in xref section' <<<"$output"; then
    printf 'expected xref failure was not present\n' >&2
    exit 1
fi

printf 'reproduced: startxref-inside-xref-keyword\n'
