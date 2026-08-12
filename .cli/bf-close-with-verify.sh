#!/usr/bin/env bash
# NEEDLE bead close wrapper with rust-verify validation
#
# This wrapper integrates rust-verify validation into the bead close lifecycle.
# Beads can only close if the rust-verify workflow passes (exit code 0).
#
# Usage: bf-close-with-verify.sh <bead-id> [close-reason] [options]
#
# Options:
#   --skip-verify    Bypass validation and close immediately (for infra beads)
#   --test-args      Arguments to pass to cargo test (e.g., "-p pdftract-core --lib")
#   --timeout        Maximum time to wait for workflow completion (default: 1800s)
#
# Environment variables:
#   KUBECONFIG: Path to kubectl config (defaults to ~/.kube/iad-ci.kubeconfig)
#   NEEDLE_WORKER: Worker name (defaults to "needle-worker")
#   NEEDLE_REPO: Repository path (defaults to current directory)

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $*${NC}"
}

# Default values
SKIP_VERIFY=false
TEST_ARGS=""
TIMEOUT=1800
NEEDLE_WORKER="${NEEDLE_WORKER:-claude-code-glm-4.7-lab-test-fix}"
NEEDLE_REPO="${NEEDLE_REPO:-$(pwd)}"
BEAD_ID=""
CLOSE_REASON=""

# Parse arguments
while [ $# -gt 0 ]; do
    case $1 in
        --skip-verify)
            SKIP_VERIFY=true
            shift
            ;;
        --test-args)
            TEST_ARGS="$2"
            shift 2
            ;;
        --timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        -*)
            error "Unknown option: $1"
            exit 1
            ;;
        *)
            if [ -z "$BEAD_ID" ]; then
                BEAD_ID="$1"
            elif [ -z "$CLOSE_REASON" ]; then
                CLOSE_REASON="$1"
            else
                error "Unexpected argument: $1"
                exit 1
            fi
            shift
            ;;
    esac
done

# Validate required arguments
if [ -z "$BEAD_ID" ]; then
    error "Usage: $0 <bead-id> [close-reason] [--skip-verify] [--test-args <args>] [--timeout <seconds>]"
    exit 1
fi

# Default close reason if not provided
if [ -z "$CLOSE_REASON" ]; then
    CLOSE_REASON="Completed"
fi

# Validate bead ID format (basic check)
if [[ ! "$BEAD_ID" =~ ^(bf|bd|nd|needle)-[a-zA-Z0-9]+$ ]]; then
    error "Invalid bead ID format: $BEAD_ID"
    exit 1
fi

# Check if we're in a git repo
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    error "Not a git repository: $NEEDLE_REPO"
    exit 1
fi

log "Starting bead close process for $BEAD_ID"
log "Worker: $NEEDLE_WORKER"
log "Repository: $NEEDLE_REPO"

# Skip verification if requested
if [ "$SKIP_VERIFY" = true ]; then
    warn "Skipping rust-verify validation (--skip-verify flag)"
    log "Closing bead $BEAD_ID immediately"

    # Call bf close directly
    if bf close "$BEAD_ID" --reason "$CLOSE_REASON"; then
        log "✓ Bead $BEAD_ID closed successfully (validation skipped)"
        exit 0
    else
        error "✗ Failed to close bead $BEAD_ID"
        exit 1
    fi
fi

# Run rust-verify validation
log "Running rust-verify validation before close"
info "Test args: ${TEST_ARGS:-<default>}"
info "Timeout: ${TIMEOUT}s"

# Get the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Call the needle-verify-wrapper.sh
if "$SCRIPT_DIR/needle-verify-wrapper.sh" "$BEAD_ID" "$NEEDLE_WORKER" "$NEEDLE_REPO" "$TEST_ARGS"; then
    log "✓ Rust-verify validation passed for bead $BEAD_ID"

    # Validation passed, proceed with close
    log "Closing bead $BEAD_ID"

    if bf close "$BEAD_ID" --reason "$CLOSE_REASON"; then
        log "✓ Bead $BEAD_ID closed successfully with verification"
        exit 0
    else
        error "✗ Validation passed but failed to close bead $BEAD_ID"
        exit 1
    fi
else
    EXIT_CODE=$?
    error "✗ Rust-verify validation failed for bead $BEAD_ID (exit code: $EXIT_CODE)"
    error "Bead close blocked - fix tests and retry"

    # Display the last few lines of workflow output for debugging
    info "To see full workflow logs, run:"
    info "  kubectl --kubeconfig=\${KUBECONFIG:-~/.kube/iad-ci.kubeconfig} logs -n argo-workflows <pod-name> -c main"

    exit $EXIT_CODE
fi