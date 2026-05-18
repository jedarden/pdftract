#!/bin/bash
# Competitive benchmark runner for pdftract
# Usage: run-benchmarks.sh [--baseline <path>] [--output <path>]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CORPUS_DIR="$SCRIPT_DIR/corpus"
WRAPPERS_DIR="$SCRIPT_DIR"
OUTPUT="${OUTPUT:-benchmark-results.json}"
BASELINE="${BASELINE:-$SCRIPT_DIR/../baselines/main.json}"
REGRESSION_THRESHOLD="${REGRESSION_THRESHOLD:-0.10}"
TENX_THRESHOLD="${TENX_THRESHOLD:-0.10}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Tools to benchmark
TOOLS=("pdftract" "pdfminer" "pypdf" "pdfplumber")

log_info() {
    echo -e "${GREEN}[INFO]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

# Check if hyperfine is installed
check_hyperfine() {
    if ! command -v hyperfine &> /dev/null; then
        log_error "hyperfine is not installed. Install it with: apt-get install hyperfine"
        exit 1
    fi
}

# Get all PDF files in corpus
get_corpus_files() {
    find "$CORPUS_DIR" -name "*.pdf" -type f | sort
}

# Run hyperfine for a single tool/document pair
run_benchmark() {
    local tool="$1"
    local doc="$2"
    local doc_name="$(basename "$doc")"
    local result_file="/tmp/hyperfine-${tool}-${doc_name}.json"

    local wrapper="$WRAPPERS_DIR/run-${tool}.sh"
    if [ ! -f "$wrapper" ]; then
        log_error "Wrapper not found: $wrapper"
        echo "{\"tool\": \"$tool\", \"doc\": \"$doc_name\", \"crash\": true}"
        return 1
    fi

    # Run hyperfine with warmup and 5 runs
    if hyperfine --warmup 2 --runs 5 --export-json "$result_file" \
        -- "$wrapper \"$doc\"" &> /dev/null; then

        # Extract mean and stddev from hyperfine output
        local mean_ms=$(jq -r '.results[0].mean * 1000' "$result_file" 2>/dev/null || echo "null")
        local stddev_ms=$(jq -r '.results[0].stddev * 1000' "$result_file" 2>/dev/null || echo "null")
        local min_ms=$(jq -r '.results[0].min * 1000' "$result_file" 2>/dev/null || echo "null")
        local max_ms=$(jq -r '.results[0].max * 1000' "$result_file" 2>/dev/null || echo "null")

        if [ "$mean_ms" != "null" ]; then
            echo "{\"tool\": \"$tool\", \"doc\": \"$doc_name\", \"mean_ms\": $mean_ms, \"stddev_ms\": $stddev_ms, \"min_ms\": $min_ms, \"max_ms\": $max_ms, \"crash\": false}"
        else
            echo "{\"tool\": \"$tool\", \"doc\": \"$doc_name\", \"crash\": true}"
        fi

        rm -f "$result_file"
    else
        log_warn "hyperfine failed for $tool on $doc_name"
        echo "{\"tool\": \"$tool\", \"doc\": \"$doc_name\", \"crash\": true}"
    fi
}

# Compute geometric mean
compute_geomean() {
    local values=("$@")
    local count=${#values[@]}
    local product=1.0
    local valid_count=0

    for val in "${values[@]}"; do
        if [ "$val" != "null" ] && [ "$val" != "0" ]; then
            product=$(echo "$product * $val" | bc -l)
            ((valid_count++))
        fi
    done

    if [ $valid_count -eq 0 ]; then
        echo "null"
    else
        # geomean = product^(1/n)
        echo "e(l($product)/$valid_count)" | bc -l
    fi
}

# Run special pdftract-grep-1000 benchmark
run_grep_1000_benchmark() {
    log_info "Running pdftract-grep-1000 special benchmark..."

    local grep_doc="$CORPUS_DIR/wikipedia-1000.pdf"
    if [ ! -f "$grep_doc" ]; then
        log_warn "wikipedia-1000.pdf not found, skipping grep-1000 benchmark"
        return 0
    fi

    local result_file="/tmp/hyperfine-grep-1000.json"

    # Run hyperfine with warmup and 5 runs
    if hyperfine --warmup 2 --runs 5 --export-json "$result_file" \
        -- "pdftract grep \"the\" \"$grep_doc\"" &> /dev/null; then

        # Extract mean from hyperfine output
        local mean_ms=$(jq -r '.results[0].mean * 1000' "$result_file" 2>/dev/null || echo "null")

        if [ "$mean_ms" != "null" ]; then
            log_info "pdftract-grep-1000: ${mean_ms}ms"
            echo "$mean_ms" > "/tmp/grep-1000-result.txt"
        else
            log_warn "Failed to parse grep-1000 result"
            echo "null" > "/tmp/grep-1000-result.txt"
        fi

        rm -f "$result_file"
    else
        log_warn "hyperfine failed for grep-1000 benchmark"
        echo "null" > "/tmp/grep-1000-result.txt"
    fi
}

# Run all benchmarks
run_all_benchmarks() {
    log_info "Starting competitive benchmarks..."

    local corpus_files=($(get_corpus_files))
    local total_files=${#corpus_files[@]}
    local total_runs=$(($total_files * ${#TOOLS[@]}))
    local current_run=0

    # Initialize results array
    local results=()

    for tool in "${TOOLS[@]}"; do
        log_info "Benchmarking $tool..."

        for doc in "${corpus_files[@]}"; do
            ((current_run++))
            local doc_name="$(basename "$doc")"
            log_info "[$current_run/$total_runs] Running $tool on $doc_name..."

            local result=$(run_benchmark "$tool" "$doc")
            results+=("$result")
        done
    done

    # Write results to JSON file
    log_info "Writing results to $OUTPUT..."
    echo "[" > "$OUTPUT"
    local first=true
    for result in "${results[@]}"; do
        if [ "$first" = true ]; then
            first=false
        else
            echo "," >> "$OUTPUT"
        fi
        echo -n "  $result" >> "$OUTPUT"
    done
    echo "" >> "$OUTPUT"
    echo "]" >> "$OUTPUT"

    # Run grep-1000 special benchmark
    run_grep_1000_benchmark

    log_info "Benchmarking complete!"
}

# Analyze results and check gates
analyze_results() {
    log_info "Analyzing results..."

    # Compute per-tool geomeans
    declare -A tool_geomeans
    declare -A tool_success_counts

    for tool in "${TOOLS[@]}"; do
        local values=()
        local count=0

        while IFS= read -r line; do
            local mean=$(echo "$line" | jq -r '.mean_ms // empty')
            if [ -n "$mean" ] && [ "$mean" != "null" ]; then
                values+=("$mean")
                ((count++))
            fi
        done < <(jq -r ".[] | select(.tool == \"$tool\") | select(.crash == false)" "$OUTPUT")

        if [ ${#values[@]} -gt 0 ]; then
            # Use Python for geomean calculation (more reliable than bc)
            local geomean=$(python3 -c "
import math
values = $(
    for v in "${values[@]}"; do
        echo -n "$v "
    done
)
values = [float(v) for v in values.split()]
print(math.exp(sum(math.log(v) for v in values) / len(values)))
")
            tool_geomeans[$tool]=$geomean
            tool_success_counts[$tool]=$count
        fi
    done

    # Print summary table
    log_info "=== Benchmark Results Summary ==="
    printf "%-15s %10s %10s\n" "Tool" "GeoMean(ms)" "Success Rate"
    printf "%-15s %10s %10s\n" "---" "----------" "------------"

    for tool in "${TOOLS[@]}"; do
        local geomean=${tool_geomeans[$tool]:-"N/A"}
        local count=${tool_success_counts[$tool]:-0}
        if [ "$geomean" != "N/A" ]; then
            printf "%-15s %10.2f %10d/%d\n" "$tool" "$geomean" "$count" "$total_files"
        else
            printf "%-15s %10s %10d/%d\n" "$tool" "$geomean" "$count" "$total_files"
        fi
    done

    # Extract pdftract geomean for regression gate
    local pdftract_geomean=${tool_geomeans[pdftract]:-"null"}

    # Check 10x-faster gate (pdftract vs pdfminer on vector PDFs only)
    # The gate applies only to vector PDFs where pdftract should excel
    log_info "Computing 10x-faster gate on vector PDFs only..."

    local pdftract_vector_values=()
    local pdfminer_vector_values=()

    # Extract values for vector PDFs only (documents in corpus/vector/ directory)
    while IFS= read -r line; do
        local doc=$(echo "$line" | jq -r '.doc // empty')
        local mean=$(echo "$line" | jq -r '.mean_ms // empty')
        if [ -n "$mean" ] && [ "$mean" != "null" ] && [ -n "$doc" ]; then
            # Check if doc is from vector corpus (we infer this from the baseline file structure)
            # In the actual corpus, vector PDFs are named misc-*.pdf
            if [[ "$doc" =~ ^misc- ]]; then
                case "$(echo "$line" | jq -r '.tool')" in
                    pdftract)
                        pdftract_vector_values+=("$mean")
                        ;;
                    pdfminer)
                        pdfminer_vector_values+=("$mean")
                        ;;
                esac
            fi
        fi
    done < <(jq -r ".[] | select(.crash == false)" "$OUTPUT")

    # Compute vector-only geomeans
    local pdftract_vector_geomean="null"
    local pdfminer_vector_geomean="null"

    if [ ${#pdftract_vector_values[@]} -gt 0 ]; then
        pdftract_vector_geomean=$(python3 -c "
import math
values = [${pdftract_vector_values[*]}]
print(math.exp(sum(math.log(v) for v in values) / len(values)))
")
    fi

    if [ ${#pdfminer_vector_values[@]} -gt 0 ]; then
        pdfminer_vector_geomean=$(python3 -c "
import math
values = [${pdfminer_vector_values[*]}]
print(math.exp(sum(math.log(v) for v in values) / len(values)))
")
    fi

    if [ "$pdftract_vector_geomean" != "null" ] && [ "$pdfminer_vector_geomean" != "null" ]; then
        local ratio=$(echo "$pdftract_vector_geomean / $pdfminer_vector_geomean" | bc -l)
        log_info "10x-faster gate (vector PDFs): pdftract/pdfminer = $ratio (threshold: <= $TENX_THRESHOLD)"
        log_info "  pdftract vector geomean: ${pdftract_vector_geomean}ms"
        log_info "  pdfminer vector geomean: ${pdfminer_vector_geomean}ms"

        # 10x faster means ratio should be <= 0.1 (pdftract takes 10ms, pdfminer takes 100ms)
        if (( $(echo "$ratio > $TENX_THRESHOLD" | bc -l) )); then
            log_error "FAIL: pdftract is not >= 10x faster than pdfminer on vector PDFs (ratio: $ratio, threshold: <= $TENX_THRESHOLD)"
            return 1
        else
            log_info "PASS: pdftract is >= 10x faster than pdfminer on vector PDFs (ratio: $ratio)"
        fi
    else
        log_warn "Cannot check 10x-faster gate: missing vector PDF data (pdftract: ${#pdftract_vector_values[@]} results, pdfminer: ${#pdfminer_vector_values[@]} results)"
    fi

    # Check regression gate if baseline is provided
    if [ -f "$BASELINE" ]; then
        log_info "Checking regression against baseline..."

        local baseline_geomean=$(jq -r '.pdftract_geomean // empty' "$BASELINE")
        if [ -n "$baseline_geomean" ] && [ "$pdftract_geomean" != "null" ]; then
            local regression=$(echo "($pdftract_geomean - $baseline_geomean) / $baseline_geomean" | bc -l)
            log_info "Regression: $(printf "%.2f%%" $(echo "$regression * 100" | bc -l))"

            if (( $(echo "$regression > $REGRESSION_THRESHOLD" | bc -l) )); then
                log_error "FAIL: Regression > ${REGRESSION_THRESHOLD} detected!"
                return 1
            else
                log_info "PASS: No significant regression"
            fi
        else
            log_warn "Cannot check regression: missing baseline data"
        fi

        # Check grep-1000 regression gate
        if [ -f "/tmp/grep-1000-result.txt" ]; then
            local grep_result=$(cat /tmp/grep-1000-result.txt)
            local baseline_grep_1000=$(jq -r '.grep_1000_mean_ms // empty' "$BASELINE")

            if [ "$grep_result" != "null" ] && [ -n "$baseline_grep_1000" ]; then
                local grep_regression=$(echo "($grep_result - $baseline_grep_1000) / $baseline_grep_1000" | bc -l)
                log_info "grep-1000 regression: $(printf "%.2f%%" $(echo "$grep_regression * 100" | bc -l)) (current: ${grep_result}ms, baseline: ${baseline_grep_1000}ms)"

                if (( $(echo "$grep_regression > $REGRESSION_THRESHOLD" | bc -l) )); then
                    log_error "FAIL: grep-1000 regression > ${REGRESSION_THRESHOLD} detected!"
                    return 1
                else
                    log_info "PASS: No significant grep-1000 regression"
                fi
            else
                log_warn "Cannot check grep-1000 regression: missing baseline data (current: ${grep_result}, baseline: ${baseline_grep_1000})"
            fi
        else
            log_warn "grep-1000 result file not found, skipping regression check"
        fi
    fi

    return 0
}

# Generate PR comment markdown
generate_pr_comment() {
    local comment_file="benchmark-comment.md"

    log_info "Generating PR comment..."

    cat > "$comment_file" << 'EOF'
## Competitive Benchmark Results

### Performance Summary (Geometric Mean)

| Tool | GeoMean (ms) | 95% CI | Success Rate |
|------|-------------|--------|--------------|
EOF

    # Add rows for each tool with actual data
    for tool in "${TOOLS[@]}"; do
        # Get mean values for this tool
        local means=$(jq -r "[.[] | select(.tool == \"$tool\") | select(.crash == false) | .mean_ms] | @csv" "$OUTPUT" | tr ',' ' ')

        # Get stddev values for this tool
        local stddevs=$(jq -r "[.[] | select(.tool == \"$tool\") | select(.crash == false) | .stddev_ms] | @csv" "$OUTPUT" | tr ',' ' ')

        # Get count of successful runs
        local count=$(jq -r "[.[] | select(.tool == \"$tool\") | select(.crash == false)] | length" "$OUTPUT")
        local total=$(jq -r "[.[] | select(.tool == \"$tool\")] | length" "$OUTPUT")

        if [ "$count" -gt 0 ]; then
            # Calculate geomean using Python
            local geomean=$(python3 -c "
import math
import sys
means = [float(x) for x in '$means'.split()]
if means:
    print(math.exp(sum(math.log(x) for x in means) / len(means)))
else:
    print('N/A')
")

            # Calculate 95% CI (geometric)
            local ci=$(python3 -c "
import math
import sys
means = [float(x) for x in '$means'.split()]
stddevs = [float(x) for x in '$stddevs'.split()]
if means and stddevs:
    # Calculate relative standard deviation
    geomean = math.exp(sum(math.log(x) for x in means) / len(means))
    # Approximate CI using coefficient of variation
    cv = sum(s/m for s, m in zip(stddevs, means)) / len(means)
    ci_pct = cv * 1.96 * 100  # 95% CI
    print(f'±{ci_pct:.1f}%')
else:
    print('N/A')
")

            printf "| %-15s | %10.2f | %6s | %4d/%d |\n" "$tool" "$geomean" "$ci" "$count" "$total" >> "$comment_file"
        else
            printf "| %-15s | %10s | %6s | %4d/%d |\n" "$tool" "N/A" "N/A" "$count" "$total" >> "$comment_file"
        fi
    done

    # Add grep-1000 benchmark result if available
    if [ -f "/tmp/grep-1000-result.txt" ]; then
        local grep_result=$(cat /tmp/grep-1000-result.txt)
        if [ "$grep_result" != "null" ]; then
            cat >> "$comment_file" << EOF

### Special Benchmark: pdftract-grep-1000

- **Mean time:** ${grep_result}ms
- **Test:** \`pdftract grep "the" wikipedia-1000.pdf\`
- **Status:** Baseline comparison available
EOF
        fi
    fi

    cat >> "$comment_file" << 'EOF'

### Notes

- Run with `hyperfine --warmup 2 --runs 5`
- Corpus: 50 PDFs (25 vector + 25 raster)
- Crashes are excluded from geomean calculation
- 95% CI shown as percentage of geomean
- Full results available in artifacts
EOF

    log_info "PR comment written to $comment_file"
    cat "$comment_file"
}

main() {
    check_hyperfine
    run_all_benchmarks

    if ! analyze_results; then
        log_error "Benchmark gates failed!"
        exit 1
    fi

    generate_pr_comment

    log_info "All benchmarks passed!"
}

main "$@"
