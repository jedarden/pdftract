#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
CHECKER="$SCRIPT_DIR/check-argo-workflow-drift.sh"
SOURCE="$REPO_ROOT/.ci/argo-workflows/pdftract-ci.yaml"
FUZZ_SOURCE="$REPO_ROOT/.ci/argo-workflows/pdftract-nightly-fuzz.yaml"
WORK_DIR=$(mktemp -d "${TMPDIR:-/tmp}/pdftract-argo-drift-test.XXXXXX")
trap 'rm -rf "$WORK_DIR"' EXIT HUP INT TERM

DEPLOYED_REPO="$WORK_DIR/declarative-config"
DEPLOYED_FILE="$DEPLOYED_REPO/k8s/iad-ci/argo-workflows/pdftract-ci.yaml"
FUZZ_DEPLOYED_FILE="$DEPLOYED_REPO/k8s/iad-ci/argo-workflows/pdftract-nightly-fuzz.yaml"
mkdir -p "$(dirname -- "$DEPLOYED_FILE")"
cp "$SOURCE" "$DEPLOYED_FILE"
cp "$FUZZ_SOURCE" "$FUZZ_DEPLOYED_FILE"

check() {
    PDFTRACT_CI_SOURCE="$SOURCE" \
    PDFTRACT_FUZZ_SOURCE="$FUZZ_SOURCE" \
    DECLARATIVE_CONFIG_DIR="$DEPLOYED_REPO" \
        "$CHECKER" "$@"
}

echo "Test: identical source and deployment pass"
check >/dev/null

echo "Test: resource-only deployment override passes"
sed -i '0,/memory: 4Gi/s//memory: 2Gi/' "$DEPLOYED_FILE"
sed -i '0,/memory: 4Gi/s//memory: 2Gi/' "$FUZZ_DEPLOYED_FILE"
check >/dev/null

echo "Test: quality-gate drift fails"
sed -i '0,/name: schema-validation$/s//name: schema-validation-drifted/' "$DEPLOYED_FILE"
if check >/dev/null 2>&1; then
    echo "ERROR: quality-gate drift was not detected" >&2
    exit 1
fi

echo "Test: fuzz-target drift fails"
cp "$SOURCE" "$DEPLOYED_FILE"
sed -i '0,/name: fuzz-lzw-decode$/s//name: fuzz-lzw-decode-drifted/' "$FUZZ_DEPLOYED_FILE"
if check >/dev/null 2>&1; then
    echo "ERROR: fuzz-target drift was not detected" >&2
    exit 1
fi

echo "Argo workflow drift tests passed"
