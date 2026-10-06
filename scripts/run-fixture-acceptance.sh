#!/usr/bin/env bash
set -Eeuo pipefail
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
artifact_dir=${PDFTRACT_FIXTURE_ARTIFACT_DIR:-"$repo_root/fixture-artifacts"}
if [[ -z "${PDFTRACT_BINARY:-}" ]]; then
  echo "Set PDFTRACT_BINARY to a built pdftract executable" >&2
  exit 2
fi
exec python3 "$repo_root/scripts/run-fixture-acceptance.py" \
  --binary "$PDFTRACT_BINARY" --artifact-dir "$artifact_dir" "$@"
