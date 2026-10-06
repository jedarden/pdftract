#!/bin/sh
set -eu

# The Argo manifests are maintained in pdftract and promoted to
# declarative-config. The deployment repository may change only Kubernetes
# resource requests/limits for the iad-ci cluster; each workflow contract must
# remain byte-for-byte identical after those blocks are normalized away.

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
SOURCE_FILE=${PDFTRACT_CI_SOURCE:-$REPO_ROOT/.ci/argo-workflows/pdftract-ci.yaml}
FUZZ_SOURCE_FILE=${PDFTRACT_FUZZ_SOURCE:-$REPO_ROOT/.ci/argo-workflows/pdftract-nightly-fuzz.yaml}
SUPPLY_CHAIN_SOURCE_FILE=${PDFTRACT_SUPPLY_CHAIN_SOURCE:-$REPO_ROOT/.ci/argo-workflows/pdftract-nightly-supply-chain.yaml}
EVIDENCE_SOURCE_FILE=${PDFTRACT_EVIDENCE_SOURCE:-$REPO_ROOT/.ci/argo-workflows/pdftract-nightly-evidence-configmap.yaml}
CONFIG_URL=${DECLARATIVE_CONFIG_REPO_URL:-https://git.ardenone.com/jedarden/declarative-config.git}
CONFIG_REF=${DECLARATIVE_CONFIG_REF:-main}
CONFIG_PATH=${DECLARATIVE_CONFIG_WORKFLOW_PATH:-k8s/iad-ci/argo-workflows/pdftract-ci.yaml}
FUZZ_CONFIG_PATH=${DECLARATIVE_CONFIG_FUZZ_WORKFLOW_PATH:-k8s/iad-ci/argo-workflows/pdftract-nightly-fuzz.yaml}
SUPPLY_CHAIN_CONFIG_PATH=${DECLARATIVE_CONFIG_SUPPLY_CHAIN_WORKFLOW_PATH:-k8s/iad-ci/argo-workflows/pdftract-nightly-supply-chain.yaml}
EVIDENCE_CONFIG_PATH=${DECLARATIVE_CONFIG_EVIDENCE_PATH:-k8s/iad-ci/argo-workflows/pdftract-nightly-evidence-configmap.yaml}
CONFIG_DIR=${DECLARATIVE_CONFIG_DIR:-}
LIVE_CHECK=0
case ${1:-} in
    '') ;;
    --live) LIVE_CHECK=1 ;;
    *) echo "usage: $0 [--live]" >&2; exit 2 ;;
esac
if [ "$#" -gt 1 ]; then
    echo "usage: $0 [--live]" >&2
    exit 2
fi

for source_file in "$SOURCE_FILE" "$FUZZ_SOURCE_FILE" "$SUPPLY_CHAIN_SOURCE_FILE" "$EVIDENCE_SOURCE_FILE"; do
    if [ ! -f "$source_file" ]; then
        echo "ERROR: source Argo manifest not found: $source_file" >&2
        exit 2
    fi
done

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
    git -C "$CONFIG_DIR" sparse-checkout set --no-cone "/$CONFIG_PATH" "/$FUZZ_CONFIG_PATH" "/$SUPPLY_CHAIN_CONFIG_PATH" "/$EVIDENCE_CONFIG_PATH"
else
    CONFIG_DIR=$(CDPATH= cd -- "$CONFIG_DIR" && pwd)
fi

CI_DEPLOYED_FILE="$CONFIG_DIR/$CONFIG_PATH"
FUZZ_DEPLOYED_FILE="$CONFIG_DIR/$FUZZ_CONFIG_PATH"
SUPPLY_CHAIN_DEPLOYED_FILE="$CONFIG_DIR/$SUPPLY_CHAIN_CONFIG_PATH"
EVIDENCE_DEPLOYED_FILE="$CONFIG_DIR/$EVIDENCE_CONFIG_PATH"
for deployed_file in "$CI_DEPLOYED_FILE" "$FUZZ_DEPLOYED_FILE" "$SUPPLY_CHAIN_DEPLOYED_FILE" "$EVIDENCE_DEPLOYED_FILE"; do
    if [ ! -f "$deployed_file" ]; then
        echo "ERROR: deployed Argo manifest not found: $deployed_file" >&2
        exit 2
    fi
done

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

compare_workflow() {
    workflow_name=$1
    source_file=$2
    deployed_file=$3
    source_normalized="$WORK_DIR/$workflow_name.source.normalized.yaml"
    deployed_normalized="$WORK_DIR/$workflow_name.deployed.normalized.yaml"

    normalize_workflow "$source_file" "$source_normalized"
    normalize_workflow "$deployed_file" "$deployed_normalized"

    source_sha=$(sha256sum "$source_file" | awk '{print $1}')
    deployed_sha=$(sha256sum "$deployed_file" | awk '{print $1}')
    echo "Source ($workflow_name):   $source_file (sha256 $source_sha)"
    echo "Deployed ($workflow_name): $deployed_file (sha256 $deployed_sha)"

    if cmp -s "$source_normalized" "$deployed_normalized"; then
        echo "Argo workflow contract is synchronized: $workflow_name"
        return 0
    fi

    echo "ERROR: Argo workflow contract drift detected: $workflow_name" >&2
    echo "Only resources maps may differ between source and deployed copies." >&2
    diff -u "$deployed_normalized" "$source_normalized" >&2 || true
    return 1
}

failed=0
if ! compare_workflow "pdftract-ci" "$SOURCE_FILE" "$CI_DEPLOYED_FILE"; then
    failed=1
fi
if ! compare_workflow "pdftract-nightly-fuzz" "$FUZZ_SOURCE_FILE" "$FUZZ_DEPLOYED_FILE"; then
    failed=1
fi
if ! compare_workflow "pdftract-nightly-supply-chain" "$SUPPLY_CHAIN_SOURCE_FILE" "$SUPPLY_CHAIN_DEPLOYED_FILE"; then
    failed=1
fi
if ! compare_workflow "pdftract-nightly-evidence" "$EVIDENCE_SOURCE_FILE" "$EVIDENCE_DEPLOYED_FILE"; then
    failed=1
fi

if [ "$failed" -ne 0 ]; then
    exit 1
fi

echo "Argo workflow contracts are synchronized"
echo "Allowed deployment-only differences: resources maps"

if [ "$LIVE_CHECK" -eq 1 ]; then
    # This in-cluster proxy uses the devpod-observer read-only ServiceAccount.
    # The CI workflow's own ServiceAccount cannot read CronWorkflows.
    LIVE_API_BASE=${ARGO_LIVE_API_BASE:-http://kubectl-proxy.devpod-observer.svc.cluster.local:8001}
    LIVE_DIR=${ARGO_LIVE_CRONWORKFLOW_DIR:-}
    for workflow_name in pdftract-nightly-supply-chain pdftract-nightly-fuzz; do
        live_file="$WORK_DIR/$workflow_name.live.json"
        if [ -n "$LIVE_DIR" ]; then
            if [ ! -f "$LIVE_DIR/$workflow_name.json" ]; then
                echo "ERROR: live CronWorkflow missing: $workflow_name" >&2
                exit 1
            fi
            cp "$LIVE_DIR/$workflow_name.json" "$live_file"
        else
            live_url="$LIVE_API_BASE/apis/argoproj.io/v1alpha1/namespaces/argo-workflows/cronworkflows/$workflow_name"
            if ! wget -q -O "$live_file" "$live_url"; then
                echo "ERROR: cannot read live CronWorkflow $workflow_name from iad-ci/argo-workflows" >&2
                exit 1
            fi
        fi
        case "$workflow_name" in
            pdftract-nightly-supply-chain)
                deployed_file=$SUPPLY_CHAIN_DEPLOYED_FILE
                schedule='0 3 * * *'
                ;;
            pdftract-nightly-fuzz)
                deployed_file=$FUZZ_DEPLOYED_FILE
                schedule='0 4 * * *'
                ;;
        esac
        if ! python3 "$SCRIPT_DIR/check-live-cronworkflow-drift.py" \
            "$deployed_file" "$live_file" "$workflow_name" "$schedule"; then
            failed=1
        fi
    done
    [ "$failed" -eq 0 ] || exit 1
    echo "Live iad-ci CronWorkflow specs match declarative-config"
fi
