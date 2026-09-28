#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
CHECKER="$SCRIPT_DIR/check-argo-workflow-drift.sh"
SOURCE="$REPO_ROOT/.ci/argo-workflows/pdftract-ci.yaml"
WORK_DIR=$(mktemp -d "${TMPDIR:-/tmp}/pdftract-argo-drift-test.XXXXXX")
trap 'rm -rf "$WORK_DIR"' EXIT HUP INT TERM

DEPLOYED_REPO="$WORK_DIR/declarative-config"
DEPLOYED_FILE="$DEPLOYED_REPO/k8s/iad-ci/argo-workflows/pdftract-ci.yaml"
mkdir -p "$(dirname -- "$DEPLOYED_FILE")"
cp "$SOURCE" "$DEPLOYED_FILE"

echo "Test: identical source and deployment pass"
PDFTRACT_CI_SOURCE="$SOURCE" DECLARATIVE_CONFIG_DIR="$DEPLOYED_REPO" \
    "$CHECKER" >/dev/null

echo "Test: resource-only deployment override passes"
sed -i '0,/memory: 4Gi/s//memory: 2Gi/' "$DEPLOYED_FILE"
PDFTRACT_CI_SOURCE="$SOURCE" DECLARATIVE_CONFIG_DIR="$DEPLOYED_REPO" \
    "$CHECKER" >/dev/null

echo "Test: quality-gate drift fails"
sed -i '0,/name: schema-validation$/s//name: schema-validation-drifted/' "$DEPLOYED_FILE"
if PDFTRACT_CI_SOURCE="$SOURCE" DECLARATIVE_CONFIG_DIR="$DEPLOYED_REPO" \
    "$CHECKER" >/dev/null 2>&1; then
    echo "ERROR: quality-gate drift was not detected" >&2
    exit 1
fi

echo "Argo workflow drift tests passed"
