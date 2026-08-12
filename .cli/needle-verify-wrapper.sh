#!/usr/bin/env bash
# NEEDLE per-bead verify wrapper
# Pushes worker worktree to wip/<worker>/<bead> branch, submits rust-verify workflow,
# polls to completion, and returns exit code + logs to the agent.
#
# Usage: needle-verify-wrapper.sh <bead-id> <worker-name> <repo-path> [test-args]
#
# Environment variables:
# - KUBECONFIG: Path to kubectl config (defaults to ~/.kube/iad-ci.kubeconfig)
# - DRY_RUN: Set to "true" to skip workflow submission (for testing)

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to log messages
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $*"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $*${NC}" >&2
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARN: $*${NC}" >&2
}

# Check required arguments
if [ $# -lt 3 ]; then
    error "Usage: $0 <bead-id> <worker-name> <repo-path> [test-args]"
    exit 1
fi

BEAD_ID="$1"
WORKER_NAME="$2"
REPO_PATH="$3"
TEST_ARGS="${4:-}"

# Validate inputs
if [ -z "$BEAD_ID" ]; then
    error "bead-id cannot be empty"
    exit 1
fi

if [ -z "$WORKER_NAME" ]; then
    error "worker-name cannot be empty"
    exit 1
fi

if [ ! -d "$REPO_PATH" ]; then
    error "repo-path does not exist: $REPO_PATH"
    exit 1
fi

# Set kubeconfig
KUBECONFIG="${KUBECONFIG:-$HOME/.kube/iad-ci.kubeconfig}"
if [ ! -f "$KUBECONFIG" ]; then
    error "kubeconfig not found: $KUBECONFIG"
    exit 1
fi

# Branch naming: wip/<worker>/<bead>
BRANCH_NAME="wip/$WORKER_NAME/$BEAD_ID"
WORKFLOW_NAME="rust-verify-$BEAD_ID-${WORKER_NAME}-$(date +%s)"

log "Starting verification for bead $BEAD_ID (worker: $WORKER_NAME)"
log "Repo: $REPO_PATH"
log "Branch: $BRANCH_NAME"

# Change to repo directory
cd "$REPO_PATH"

# Check if we're in a git repo
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    error "Not a git repository: $REPO_PATH"
    exit 1
fi

# Get current commit
CURRENT_COMMIT=$(git rev-parse HEAD)
log "Current commit: $CURRENT_COMMIT"

# Check if there are any uncommitted changes
if ! git diff-index --quiet HEAD --; then
    log "Uncommitted changes detected, creating commit"

    # Check if git config has user.email and user.name
    if ! git config user.email > /dev/null 2>&1; then
        warn "git user.email not set, configuring temporary identity"
        git config user.email "needle@worker.local"
        git config user.name "NEEDLE Worker"
    fi

    # Create commit with bead ID in message
    git add -A
    git commit -m "NEEDLE verify: $BEAD_ID

Worker: $WORKER_NAME
Bead: $BEAD_ID
Test args: $TEST_ARGS

Auto-generated commit for rust-verify workflow.
"
    COMMIT_CREATED=true
else
    log "No uncommitted changes, using existing commit"
    COMMIT_CREATED=false
fi

# Create or checkout the wip branch
if git show-ref --verify --quiet "refs/heads/$BRANCH_NAME"; then
    log "Branch $BRANCH_NAME already exists, updating it"
    git checkout "$BRANCH_NAME"
    git reset --hard "$CURRENT_COMMIT"
else
    log "Creating new branch $BRANCH_NAME"
    git checkout -b "$BRANCH_NAME"
fi

# Force-push to forgejo origin (not github mirror) to update branch on each run
log "Force-pushing branch to forgejo origin"
if git push --force origin "$BRANCH_NAME" 2>&1; then
    log "Branch force-pushed successfully"
else
    error "Failed to push branch to origin"
    # Clean up on failure
    git checkout main > /dev/null 2>&1 || git checkout master > /dev/null 2>&1 || true
    exit 1
fi

# Get repo URL for workflow
REPO_URL=$(git config --get remote.origin.url)
log "Repo URL: $REPO_URL"

# Prepare workflow manifest
WORKFLOW_MANIFEST="/tmp/rust-verify-$BEAD_ID.yaml"

cat > "$WORKFLOW_MANIFEST" <<EOF
apiVersion: argoproj.io/v1alpha1
kind: Workflow
metadata:
  generateName: rust-verify-${BEAD_ID}-${WORKER_NAME}-
  namespace: argo-workflows
  labels:
    bead-id: "${BEAD_ID}"
    worker-name: "${WORKER_NAME}"
spec:
  workflowTemplateRef:
    name: rust-verify
  arguments:
    parameters:
      - name: repo
        value: "${REPO_URL}"
      - name: revision
        value: "refs/heads/${BRANCH_NAME}"
      - name: test-args
        value: "${TEST_ARGS}"
EOF

# Dry run mode
if [ "${DRY_RUN:-false}" = "true" ]; then
    warn "DRY_RUN mode: skipping workflow submission"
    log "Workflow manifest:"
    cat "$WORKFLOW_MANIFEST"

    # Clean up branch
    git checkout main > /dev/null 2>&1 || git checkout master > /dev/null 2>&1 || true
    git branch -D "$BRANCH_NAME" > /dev/null 2>&1 || true

    exit 0
fi

# Submit workflow
log "Submitting rust-verify workflow"
if kubectl --kubeconfig="$KUBECONFIG" create -f "$WORKFLOW_MANIFEST" 2>&1 | tee /tmp/workflow-submit.log; then
    WORKFLOW_NAME=$(grep "workflow.argoproj.io/" /tmp/workflow-submit.log | head -1 | awk '{print $1}' || echo "")
    log "Workflow submitted: ${WORKFLOW_NAME:-rust-verify-$BEAD_ID}"
else
    error "Failed to submit workflow"

    # Clean up branch on failure
    git checkout main > /dev/null 2>&1 || git checkout master > /dev/null 2>&1 || true
    git branch -D "$BRANCH_NAME" > /dev/null 2>&1 || true

    exit 1
fi

# Extract actual workflow name from submit output
if [ -z "$WORKFLOW_NAME" ]; then
    # Fallback: get most recent workflow with our labels
    WORKFLOW_NAME=$(kubectl --kubeconfig="$KUBECONFIG" get workflows \
        -n argo-workflows \
        -l "bead-id=${BEAD_ID},worker-name=${WORKER_NAME}" \
        -o jsonpath='{.items[-1].metadata.name}' 2>/dev/null || echo "")
fi

if [ -z "$WORKFLOW_NAME" ]; then
    error "Could not determine workflow name"
    exit 1
fi

log "Monitoring workflow: $WORKFLOW_NAME"

# Poll for workflow completion
POLL_INTERVAL=10
MAX_WAIT=1800  # 30 minutes
ELAPSED=0

while [ $ELAPSED -lt $MAX_WAIT ]; do
    # Get workflow status
    WORKFLOW_STATUS=$(kubectl --kubeconfig="$KUBECONFIG" get workflow "$WORKFLOW_NAME" \
        -n argo-workflows \
        -o jsonpath='{.status.phase}' 2>/dev/null || echo "Unknown")

    WORKFLOW_MESSAGE=$(kubectl --kubeconfig="$KUBECONFIG" get workflow "$WORKFLOW_NAME" \
        -n argo-workflows \
        -o jsonpath='{.status.message}' 2>/dev/null || echo "")

    log "Workflow status: $WORKFLOW_STATUS ${WORKFLOW_MESSAGE:+($WORKFLOW_MESSAGE)}"

    case "$WORKFLOW_STATUS" in
        Succeeded)
            log "Workflow succeeded!"

            # Get output parameters
            RESULT=$(kubectl --kubeconfig="$KUBECONFIG" get workflow "$WORKFLOW_NAME" \
                -n argo-workflows \
                -o jsonpath='{.status.outputs.parameters[?(@.name=="result")].value}' 2>/dev/null || echo "")

            OUTPUT=$(kubectl --kubeconfig="$KUBECONFIG" get workflow "$WORKFLOW_NAME" \
                -n argo-workflows \
                -o jsonpath='{.status.outputs.parameters[?(@.name=="output")].value}' 2>/dev/null || echo "")

            log "Result: $RESULT"

            # Clean up branch
            git checkout main > /dev/null 2>&1 || git checkout master > /dev/null 2>&1 || true
            git branch -D "$BRANCH_NAME" > /dev/null 2>&1 || true

            # Return exit code based on result
            if [ "$RESULT" = "pass" ]; then
                log "Verification PASSED"
                [ -n "$OUTPUT" ] && echo "$OUTPUT"
                exit 0
            else
                error "Verification FAILED"
                [ -n "$OUTPUT" ] && echo "$OUTPUT" >&2
                exit 1
            fi
            ;;
        Failed|Error)
            error "Workflow failed: $WORKFLOW_STATUS"

            # Get output if available
            OUTPUT=$(kubectl --kubeconfig="$KUBECONFIG" get workflow "$WORKFLOW_NAME" \
                -n argo-workflows \
                -o jsonpath='{.status.outputs.parameters[?(@.name=="output")].value}' 2>/dev/null || echo "")

            [ -n "$OUTPUT" ] && echo "$OUTPUT" >&2

            # Clean up branch
            git checkout main > /dev/null 2>&1 || git checkout master > /dev/null 2>&1 || true
            git branch -D "$BRANCH_NAME" > /dev/null 2>&1 || true

            exit 1
            ;;
        Pending|Running)
            # Continue polling
            sleep "$POLL_INTERVAL"
            ELAPSED=$((ELAPSED + POLL_INTERVAL))
            ;;
        *)
            warn "Unknown workflow status: $WORKFLOW_STATUS"
            sleep "$POLL_INTERVAL"
            ELAPSED=$((ELAPSED + POLL_INTERVAL))
            ;;
    esac
done

# Timeout reached
error "Workflow did not complete within ${MAX_WAIT}s"

# Clean up branch
git checkout main > /dev/null 2>&1 || git checkout master > /dev/null 2>&1 || true
git branch -D "$BRANCH_NAME" > /dev/null 2>&1 || true

exit 1
