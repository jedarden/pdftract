#!/bin/bash
# Run proptests with memory limits (cgroup MemoryMax wrapper)
#
# This enforces memory ceilings on property tests so pathological cases
# fail fast with allocation errors instead of OOMing the host.
#
# Usage:
#   scripts/run-proptest-with-limits.sh [test_name]
#
# Arguments:
#   test_name - Optional proptest name (default: run all)
#
# Environment:
#   PROPTEST_CASES - Number of test cases per module (default: 1000)
#   MEMORY_MAX_MB  - Cgroup memory limit in MB (default: 2048)
#   PROPTEST_SEED  - Proptest seed for reproducibility (default: random)

set -e

# Configuration
PROPTEST_CASES="${PROPTEST_CASES:-1000}"
MEMORY_MAX_MB="${MEMORY_MAX_MB:-2048}"  # 2 GB cgroup cap
TEST_NAME="${1:-}"

# Proptest modules (test binary names)
PROPTEST_MODULES=(
    "lexer"
    "object_parser"
    "xref"
    "stream"
    "cmap_parser"
)

echo "=========================================="
echo "Property Tests with Memory Limits"
echo "=========================================="
echo "Cases per module: ${PROPTEST_CASES}"
echo "Cgroup MemoryMax: ${MEMORY_MAX_MB} MB"

# Check if running as root (required for cgroup v1 MemoryMax)
if [ "$EUID" -ne 0 ] && [ ! -w /sys/fs/cgroup/memory/memory.limit_in_bytes ]; then
    echo "WARNING: Not running as root and cannot write to cgroup memory controller."
    echo "         MemoryMax cgroup enforcement will be skipped."
    echo "         Tests will run without memory ceiling protection."
    USE_CGROUP=false
else
    USE_CGROUP=true
fi

# Set proptest environment variables
export PROPTEST_CASES
if [ -z "$PROPTEST_SEED" ]; then
    PROPTEST_SEED=$(date +%s%N | sha256sum | head -c 16)
    echo "Generated proptest seed: $PROPTEST_SEED"
fi
export PROPTEST_SEED
echo "Seed: $PROPTEST_SEED"

# Build proptest harness first
echo ""
echo "=== Building proptest harness ==="
cargo build --features proptest --tests

# Run proptests with memory limits
FAILED_MODULES=()

if [ "$USE_CGROUP" = true ]; then
    # Create a cgroup for this test run
    CGROUP_NAME="proptest"
    CGROUP_PATH="/sys/fs/cgroup/memory/${CGROUP_NAME}"

    # Clean up any existing cgroup
    if [ -d "$CGROUP_PATH" ]; then
        rmdir "$CGROUP_PATH" 2>/dev/null || true
    fi

    # Create cgroup
    mkdir -p "$CGROUP_PATH"

    # Set memory limit (convert MB to bytes)
    MEMORY_MAX_BYTES=$((MEMORY_MAX_MB * 1024 * 1024))
    echo "$MEMORY_MAX_BYTES" > "$CGROUP_PATH/memory.limit_in_bytes"

    # Disable OOM killer (let it fail cleanly)
    echo 0 > "$CGROUP_PATH/memory.oom_control" 2>/dev/null || true

    echo ""
    echo "=== Running proptests with cgroup MemoryMax ==="

    # Run cargo nextest proptest in the cgroup
    (
        # Add current process to the cgroup
        echo $$ > "$CGROUP_PATH/tasks"

        if [ -n "$TEST_NAME" ]; then
            echo "Running single test: $TEST_NAME"
            cargo nextest run --features proptest --proptest --profile=ci-proptest "$TEST_NAME" || {
                EXIT_CODE=$?
                if [ $EXIT_CODE -ne 0 ]; then
                    FAILED_MODULES+=("$TEST_NAME")
                fi
            }
        else
            echo "Running all proptest modules..."
            for module in "${PROPTEST_MODULES[@]}"; do
                echo ""
                echo "=== Testing: $module ==="
                if ! cargo nextest run --features proptest --proptest --profile=ci-proptest "$module"; then
                    FAILED_MODULES+=("$module")
                fi
            done
        fi
    ) || {
        EXIT_CODE=$?
        # Clean up cgroup
        rmdir "$CGROUP_PATH" 2>/dev/null || true
        echo "Proptest run failed with exit code: $EXIT_CODE"
    }

    # Clean up cgroup
    rmdir "$CGROUP_PATH" 2>/dev/null || true

else
    echo ""
    echo "=== Running proptests without cgroup enforcement ==="

    if [ -n "$TEST_NAME" ]; then
        echo "Running single test: $TEST_NAME"
        cargo nextest run --features proptest --proptest --profile=ci-proptest "$TEST_NAME" || {
            EXIT_CODE=$?
            if [ $EXIT_CODE -ne 0 ]; then
                FAILED_MODULES+=("$TEST_NAME")
            fi
        }
    else
        echo "Running all proptest modules..."
        for module in "${PROPTEST_MODULES[@]}"; do
            echo ""
            echo "=== Testing: $module ==="
            if ! cargo nextest run --features proptest --proptest --profile=ci-proptest "$module"; then
                FAILED_MODULES+=("$module")
            fi
        done
    fi
fi

# Report results
echo ""
echo "=========================================="
echo "Proptest Results"
echo "=========================================="

if [ ${#FAILED_MODULES[@]} -eq 0 ]; then
    echo "All proptest modules passed"
    exit 0
else
    echo "Failed modules:"
    for module in "${FAILED_MODULES[@]}"; do
        echo "  - $module"
    done
    echo ""
    echo "Memory ceiling gate FAILED!"
    exit 1
fi
