#!/bin/sh
set -eu

# The WorkflowTemplate is maintained in pdftract and promoted to
# declarative-config. The deployment repository may change only Kubernetes
# resource requests/limits for the iad-ci cluster; the workflow contract itself
# must remain byte-for-byte identical after those blocks are normalized away.

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
SOURCE_FILE=${PDFTRACT_CI_SOURCE:-$REPO_ROOT/.ci/argo-workflows/pdftract-ci.yaml}
CONFIG_URL=${DECLARATIVE_CONFIG_REPO_URL:-https://git.ardenone.com/jedarden/declarative-config.git}
CONFIG_REF=${DECLARATIVE_CONFIG_REF:-main}
CONFIG_PATH=${DECLARATIVE_CONFIG_WORKFLOW_PATH:-k8s/iad-ci/argo-workflows/pdftract-ci.yaml}
CONFIG_DIR=${DECLARATIVE_CONFIG_DIR:-}

if [ ! -f "$SOURCE_FILE" ]; then
    echo "ERROR: source WorkflowTemplate not found: $SOURCE_FILE" >&2
    exit 2
fi

WORK_DIR=$(mktemp -d "${TMPDIR:-/tmp}/pdftract-argo-drift.XXXXXX")
cleanup() {
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT HUP INT TERM

if [ -z "$CONFIG_DIR" ]; then
    CONFIG_DIR="$WORK_DIR/declarative-config"
    echo "Fetching declarative-config@$CONFIG_REF for drift validation"
    # The deployment repository contains considerably more than this one
    # WorkflowTemplate. Use a blobless, sparse checkout so every CI run fetches
    # only the commit metadata and the file being validated.
    git clone --quiet --filter=blob:none --sparse --depth 1 \
        --branch "$CONFIG_REF" "$CONFIG_URL" "$CONFIG_DIR"
    git -C "$CONFIG_DIR" sparse-checkout set --no-cone "/$CONFIG_PATH"
else
    CONFIG_DIR=$(CDPATH= cd -- "$CONFIG_DIR" && pwd)
fi

DEPLOYED_FILE="$CONFIG_DIR/$CONFIG_PATH"
if [ ! -f "$DEPLOYED_FILE" ]; then
    echo "ERROR: deployed WorkflowTemplate not found: $DEPLOYED_FILE" >&2
    exit 2
fi

# Remove each complete resources map and replace it with one marker. This is
# deliberately structural rather than a list of line numbers, so adding a new
# template cannot silently move an exception to the wrong step. A missing or
# extra resources map still changes the normalized files and fails the check.
normalize_workflow() {
    input=$1
    output=$2
    awk '
        function leading_spaces(line) {
            match(line, /^[[:space:]]*/)
            return RLENGTH
        }
        BEGIN { skipped_indent = -1 }
        skipped_indent >= 0 {
            if ($0 ~ /^[[:space:]]*$/) next
            current_indent = leading_spaces($0)
            if (current_indent > skipped_indent) next
            skipped_indent = -1
        }
        $0 ~ /^[[:space:]]*resources:[[:space:]]*$/ {
            resources_indent = leading_spaces($0)
            printf "%*sresources: <deployment-only-resource-override>\n", resources_indent, ""
            skipped_indent = resources_indent
            next
        }
        { print }
    ' "$input" > "$output"
}

normalize_workflow "$SOURCE_FILE" "$WORK_DIR/source.normalized.yaml"
normalize_workflow "$DEPLOYED_FILE" "$WORK_DIR/deployed.normalized.yaml"

source_sha=$(sha256sum "$SOURCE_FILE" | awk '{print $1}')
deployed_sha=$(sha256sum "$DEPLOYED_FILE" | awk '{print $1}')
echo "Source:   $SOURCE_FILE (sha256 $source_sha)"
echo "Deployed: $DEPLOYED_FILE (sha256 $deployed_sha)"

if cmp -s "$WORK_DIR/source.normalized.yaml" "$WORK_DIR/deployed.normalized.yaml"; then
    echo "Argo workflow contract is synchronized"
    echo "Allowed deployment-only differences: resources maps"
    exit 0
fi

echo "ERROR: Argo workflow contract drift detected" >&2
echo "Only resources maps may differ between source and deployed copies." >&2
diff -u "$WORK_DIR/deployed.normalized.yaml" "$WORK_DIR/source.normalized.yaml" >&2 || true
exit 1
