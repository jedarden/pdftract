#!/usr/bin/env bash
# Validate the public diagnostic registry against its documentation and wire
# contracts. Keep this list explicit: adding or removing a registry contract
# should be a deliberate change to this gate.

set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
cd "$REPO_ROOT"

readonly REGISTRY_TESTS=(
  diagnostics_catalog_drift
  diagnostics_serialization_format
  diagnostics_roundtrip_contract
  diagnostics_severity_serialization
)

run_tests() {
  local feature_label=$1
  shift

  echo "=== Diagnostic registry validation (${feature_label}) ==="
  for test_target in "${REGISTRY_TESTS[@]}"; do
    echo "--- cargo test --locked $* -p pdftract-core --test ${test_target}"
    cargo test --locked "$@" -p pdftract-core --test "$test_target"
  done
}

run_tests "default features"
run_tests "all features" --all-features

echo "=== Diagnostic registry validation passed ==="
