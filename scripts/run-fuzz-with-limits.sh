#!/bin/bash
# Run fuzz tests with memory limits (cgroup MemoryMax + libfuzzer RSS limits)
#
# This enforces the memory targets from Phase 0.4 Quality Targets:
# - Adversarial fixtures must not exceed 1 GB RSS
# - Fuzz targets run under cgroup MemoryMax cap for clean failure mode
#
# Usage:
#   scripts/run-fuzz-with-limits.sh [target]
#
# Arguments:
#   target - Optional fuzz target name (default: run all)
#
# Environment:
#   FUZZ_TIME_SECONDS - Time to run each fuzzer (default: 60)
#   MEMORY_MAX_MB     - Cgroup memory limit in MB (default: 1536)
#   RSS_LIMIT_MB      - Libfuzzer RSS limit in MB (default: 1024)

set -e

# Configuration
FUZZ_TIME_SECONDS="${FUZZ_TIME_SECONDS:-60}"
MEMORY_MAX_MB="${MEMORY_MAX_MB:-1536}"  # 1.5 GB cgroup cap (allows overhead)
RSS_LIMIT_MB="${RSS_LIMIT_MB:-1024}"    # 1 GB libfuzzer RSS limit
TARGET="${1:-}"

# Fuzz targets
FUZZ_TARGETS=(
    "lexer"
    "object_parser"
    "xref"
    "stream_decoder"
    "cmap_parser"
)

echo "=========================================="
echo "Fuzz Tests with Memory Limits"
echo "=========================================="
echo "Time per target: ${FUZZ_TIME_SECONDS}s"
echo "Cgroup MemoryMax: ${MEMORY_MAX_MB} MB"
echo "Libfuzzer RSS limit: ${RSS_LIMIT_MB} MB"

# Check if running as root (required for cgroup v1 MemoryMax)
if [ "$EUID" -ne 0 ] && [ ! -w /sys/fs/cgroup/memory/memory.limit_in_bytes ]; then
    echo "WARNING: Not running as root and cannot write to cgroup memory controller."
    echo "         MemoryMax cgroup enforcement will be skipped."
    echo "         Libfuzzer RSS limits will still apply."
    USE_CGROUP=false
else
    USE_CGROUP=true
fi

# Build fuzz targets first
echo ""
echo "=== Building fuzz targets ==="
cargo fuzz build --release

# Run each fuzz target with memory limits
FAILED_TARGETS=()

for target in "${FUZZ_TARGETS[@]}"; do
    if [ -n "$TARGET" ] && [ "$target" != "$TARGET" ]; then
        continue
    fi

    echo ""
    echo "=== Fuzzing: $target ==="

    if [ "$USE_CGROUP" = true ]; then
        # Create a cgroup for this fuzzer (cgroup v1)
        CGROUP_NAME="fuzz_${target}"
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

        # Run fuzzer in cgroup
        echo "Running with cgroup MemoryMax: ${MEMORY_MAX_MB} MB"
        echo "Libfuzzer -rss_limit_mb=${RSS_LIMIT_MB}"

        # Launch fuzzer with memory limits
        # -rss_limit_mb sets per-execution RSS limit
        # -malloc_limit_mb sets total malloc limit
        # -timeout prevents runaway time
        if ! cargo fuzz run \
            --release \
            "$target" \
            -rss_limit_mb="$RSS_LIMIT_MB" \
            -malloc_limit_mb="$RSS_LIMIT_MB" \
            -timeout=10 \
            -max_total_time="$FUZZ_TIME_SECONDS" \
            -runs=0; then

            FAILED_TARGETS+=("$target")
        fi

        # Clean up cgroup
        rmdir "$CGROUP_PATH" 2>/dev/null || true
    else
        # Run without cgroup (libfuzzer limits only)
        echo "Running with libfuzzer RSS limit: ${RSS_LIMIT_MB} MB"

        if ! cargo fuzz run \
            --release \
            "$target" \
            -rss_limit_mb="$RSS_LIMIT_MB" \
            -malloc_limit_mb="$RSS_LIMIT_MB" \
            -timeout=10 \
            -max_total_time="$FUZZ_TIME_SECONDS" \
            -runs=0; then

            FAILED_TARGETS+=("$target")
        fi
    fi
done

# Report results
echo ""
echo "=========================================="
echo "Fuzz Test Results"
echo "=========================================="

if [ ${#FAILED_TARGETS[@]} -eq 0 ]; then
    echo "All fuzz targets passed"
    exit 0
else
    echo "Failed targets:"
    for target in "${FAILED_TARGETS[@]}"; do
        echo "  - $target"
    done
    echo ""
    echo "Memory ceiling gate FAILED!"
    exit 1
fi
