#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
CHECKER="$SCRIPT_DIR/check-argo-workflow-drift.sh"
SOURCE="$REPO_ROOT/.ci/argo-workflows/pdftract-ci.yaml"
FUZZ_SOURCE="$REPO_ROOT/.ci/argo-workflows/pdftract-nightly-fuzz.yaml"
SUPPLY_CHAIN_SOURCE="$REPO_ROOT/.ci/argo-workflows/pdftract-nightly-supply-chain.yaml"
EVIDENCE_SOURCE="$REPO_ROOT/.ci/argo-workflows/pdftract-nightly-evidence-configmap.yaml"
WORK_DIR=$(mktemp -d "${TMPDIR:-/tmp}/pdftract-argo-drift-test.XXXXXX")
trap 'rm -rf "$WORK_DIR"' EXIT HUP INT TERM

DEPLOYED_REPO="$WORK_DIR/declarative-config"
DEPLOYED_FILE="$DEPLOYED_REPO/k8s/iad-ci/argo-workflows/pdftract-ci.yaml"
FUZZ_DEPLOYED_FILE="$DEPLOYED_REPO/k8s/iad-ci/argo-workflows/pdftract-nightly-fuzz.yaml"
SUPPLY_CHAIN_DEPLOYED_FILE="$DEPLOYED_REPO/k8s/iad-ci/argo-workflows/pdftract-nightly-supply-chain.yaml"
EVIDENCE_DEPLOYED_FILE="$DEPLOYED_REPO/k8s/iad-ci/argo-workflows/pdftract-nightly-evidence-configmap.yaml"
mkdir -p "$(dirname -- "$DEPLOYED_FILE")"
cp "$SOURCE" "$DEPLOYED_FILE"
cp "$FUZZ_SOURCE" "$FUZZ_DEPLOYED_FILE"
cp "$SUPPLY_CHAIN_SOURCE" "$SUPPLY_CHAIN_DEPLOYED_FILE"
cp "$EVIDENCE_SOURCE" "$EVIDENCE_DEPLOYED_FILE"

check() {
    PDFTRACT_CI_SOURCE="$SOURCE" \
    PDFTRACT_FUZZ_SOURCE="$FUZZ_SOURCE" \
    PDFTRACT_SUPPLY_CHAIN_SOURCE="$SUPPLY_CHAIN_SOURCE" \
    PDFTRACT_EVIDENCE_SOURCE="$EVIDENCE_SOURCE" \
    DECLARATIVE_CONFIG_DIR="$DEPLOYED_REPO" \
        "$CHECKER" "$@"
}

echo "Test: identical source and deployment pass"
check >/dev/null

echo "Test: resource-only deployment override passes"
sed -i '0,/memory: 4Gi/s//memory: 2Gi/' "$DEPLOYED_FILE"
sed -i '0,/memory: 4Gi/s//memory: 2Gi/' "$FUZZ_DEPLOYED_FILE"
sed -i '0,/memory: 2Gi/s//memory: 1Gi/' "$SUPPLY_CHAIN_DEPLOYED_FILE"
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

echo "Test: supply-chain policy drift fails"
cp "$FUZZ_SOURCE" "$FUZZ_DEPLOYED_FILE"
cp "$SUPPLY_CHAIN_SOURCE" "$SUPPLY_CHAIN_DEPLOYED_FILE"
sed -i '0,/name: cargo-deny$/s//name: cargo-deny-drifted/' "$SUPPLY_CHAIN_DEPLOYED_FILE"
if check >/dev/null 2>&1; then
    echo "ERROR: supply-chain policy drift was not detected" >&2
    exit 1
fi

echo "Test: evidence collector drift fails"
cp "$SUPPLY_CHAIN_SOURCE" "$SUPPLY_CHAIN_DEPLOYED_FILE"
sed -i 's/"missing_logs": \[\]/"missing_logs": [] # drift/' "$EVIDENCE_DEPLOYED_FILE"
if check >/dev/null 2>&1; then
    echo "ERROR: evidence collector drift was not detected" >&2
    exit 1
fi

cp "$EVIDENCE_SOURCE" "$EVIDENCE_DEPLOYED_FILE"
LIVE_DIR="$WORK_DIR/live"
mkdir -p "$LIVE_DIR"
python3 - "$FUZZ_DEPLOYED_FILE" "$SUPPLY_CHAIN_DEPLOYED_FILE" "$LIVE_DIR" <<'PY'
import json
import sys
from pathlib import Path

import yaml

for manifest in map(Path, sys.argv[1:3]):
    document = yaml.safe_load(manifest.read_text())
    (Path(sys.argv[3]) / f"{document['metadata']['name']}.json").write_text(
        json.dumps(document)
    )
PY

check_live() {
    ARGO_LIVE_CRONWORKFLOW_DIR="$LIVE_DIR" check --live
}

echo "Test: matching live CronWorkflows pass"
check_live >/dev/null

echo "Test: missing live CronWorkflow fails"
mv "$LIVE_DIR/pdftract-nightly-fuzz.json" "$LIVE_DIR/fuzz.saved.json"
if check_live >/dev/null 2>&1; then
    echo "ERROR: missing live CronWorkflow was not detected" >&2
    exit 1
fi
mv "$LIVE_DIR/fuzz.saved.json" "$LIVE_DIR/pdftract-nightly-fuzz.json"

echo "Test: changed live 03:00 schedule fails"
cp "$LIVE_DIR/pdftract-nightly-supply-chain.json" "$LIVE_DIR/supply.saved.json"
python3 - "$LIVE_DIR/pdftract-nightly-supply-chain.json" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
document = json.loads(path.read_text())
document["spec"]["schedules"] = ["0 5 * * *"]
path.write_text(json.dumps(document))
PY
if check_live >/dev/null 2>&1; then
    echo "ERROR: changed live 03:00 schedule was not detected" >&2
    exit 1
fi
mv "$LIVE_DIR/supply.saved.json" "$LIVE_DIR/pdftract-nightly-supply-chain.json"

echo "Test: changed live 04:00 schedule fails"
cp "$LIVE_DIR/pdftract-nightly-fuzz.json" "$LIVE_DIR/fuzz.saved.json"
python3 - "$LIVE_DIR/pdftract-nightly-fuzz.json" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
document = json.loads(path.read_text())
document["spec"]["schedules"] = ["0 5 * * *"]
path.write_text(json.dumps(document))
PY
if check_live >/dev/null 2>&1; then
    echo "ERROR: changed live 04:00 schedule was not detected" >&2
    exit 1
fi
mv "$LIVE_DIR/fuzz.saved.json" "$LIVE_DIR/pdftract-nightly-fuzz.json"

echo "Test: material live spec drift fails"
cp "$LIVE_DIR/pdftract-nightly-fuzz.json" "$LIVE_DIR/fuzz.saved.json"
python3 - "$LIVE_DIR/pdftract-nightly-fuzz.json" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
document = json.loads(path.read_text())
document["spec"]["workflowSpec"]["entrypoint"] = "wrong-entrypoint"
path.write_text(json.dumps(document))
PY
if check_live >/dev/null 2>&1; then
    echo "ERROR: material live spec drift was not detected" >&2
    exit 1
fi
mv "$LIVE_DIR/fuzz.saved.json" "$LIVE_DIR/pdftract-nightly-fuzz.json"

echo "Test: live resource drift from GitOps fails"
python3 - "$LIVE_DIR/pdftract-nightly-fuzz.json" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
document = json.loads(path.read_text())
document["spec"]["workflowSpec"]["templates"][0]["container"]["resources"]["requests"]["memory"] = "128Mi"
path.write_text(json.dumps(document))
PY
if check_live >/dev/null 2>&1; then
    echo "ERROR: live resource drift was not detected" >&2
    exit 1
fi

echo "Argo workflow drift tests passed"
