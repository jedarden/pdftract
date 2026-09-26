#!/usr/bin/env bash
# Backwards-compatible entry point for the Phase 5 scanned-corpus WER gate.
#
# The canonical implementation is scripts/measure-wer.sh. Keep this path as a
# thin wrapper because older local instructions and workflow references use
# ci/wer-gate.sh; unlike the former implementation, this cannot silently skip
# missing optional fixtures.

set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec "$ROOT/scripts/measure-wer.sh" "$@"
