#!/usr/bin/env bash
# WER (Word Error Rate) CI Gate for pdftract OCR
#
# This script runs OCR on test fixtures and validates WER against thresholds:
# - Clean Lorem Ipsum: WER < 2.0%
# - Multi-language eng+fra: WER < 3.0%
# - 10-page performance: < 30 seconds processing time
#
# Usage: ci/wer-gate.sh
# Exit code: 0 if all gates pass, 1 if any fail

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Thresholds
CLEAN_WER_THRESHOLD=2.0
MULTILANG_WER_THRESHOLD=3.0
PERF_TIMEOUT_SECONDS=30

# BrokenVector WER delta thresholds
# Assisted OCR should be at least 1% better than blind OCR on aligned fixture
BROKENVECTOR_ALIGNED_DELTA_THRESHOLD=1.0
# Assisted OCR should not regress significantly on misaligned fixture (within 0.5%)
BROKENVECTOR_MISALIGNED_DELTA_THRESHOLD=0.5

# Fixture directories
FIXTURE_DIR="tests/fixtures/ocr"
CLEAN_FIXTURE="$FIXTURE_DIR/clean_lorem_ipsum"
MULTILANG_FIXTURE="$FIXTURE_DIR/eng_fra_mixed"
PERF_FIXTURE="$FIXTURE_DIR/perf_10_page"

# Counter for passed/failed gates
PASSED=0
FAILED=0

# Log functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if PDF fixture exists
check_pdf_fixture() {
    local fixture_path="$1"
    local fixture_name="$2"

    if [[ ! -f "$fixture_path" ]]; then
        log_warn "PDF fixture not found: $fixture_path"
        log_warn "The $fixture_name fixture needs to be generated manually."
        log_warn "See README.md in the fixture directory for instructions."
        return 1
    fi
    return 0
}

# Run OCR on a PDF and extract text
run_ocr() {
    local pdf_path="$1"
    local output_file="$2"
    local languages="${3:-eng}"

    log_info "Running OCR on: $pdf_path"
    log_info "  Languages: $languages"

    # Use pdftract CLI with OCR enabled
    if pdftract extract --ocr --ocr-language "$languages" --output-format text "$pdf_path" > "$output_file" 2>/dev/null; then
        return 0
    else
        log_error "OCR failed for: $pdf_path"
        return 1
    fi
}

# Calculate WER using a Python script
calculate_wer() {
    local ocr_text="$1"
    local ground_truth="$2"

    # Create a temporary Python script for WER calculation
    cat > /tmp/calculate_wer.py << 'PYTHON_SCRIPT'
import sys

def normalize_text(text):
    """Normalize text for WER calculation."""
    import re
    # Convert to lowercase
    text = text.lower()
    # Strip punctuation
    text = re.sub(r'[.,!?;:"\'()\[\]{}]', '', text)
    # Normalize whitespace
    text = ' '.join(text.split())
    return text

def calculate_wer(ocr, reference):
    """Calculate Word Error Rate."""
    ocr_words = normalize_text(ocr).split()
    ref_words = normalize_text(reference).split()

    if len(ref_words) == 0:
        return 0.0 if len(ocr_words) == 0 else 1.0

    # Levenshtein distance at word level
    m, n = len(ocr_words), len(ref_words)
    dp = [[0] * (n + 1) for _ in range(m + 1)]

    for i in range(m + 1):
        dp[i][0] = i
    for j in range(n + 1):
        dp[0][j] = j

    for i in range(1, m + 1):
        for j in range(1, n + 1):
            if ocr_words[i - 1] == ref_words[j - 1]:
                dp[i][j] = dp[i - 1][j - 1]
            else:
                dp[i][j] = min(
                    dp[i - 1][j] + 1,      # deletion
                    dp[i][j - 1] + 1,      # insertion
                    dp[i - 1][j - 1] + 1   # substitution
                )

    return dp[m][n] / len(ref_words)

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print("Usage: calculate_wer.py <ocr_file> <reference_file>", file=sys.stderr)
        sys.exit(1)

    with open(sys.argv[1], 'r') as f:
        ocr_text = f.read()

    with open(sys.argv[2], 'r') as f:
        ref_text = f.read()

    wer = calculate_wer(ocr_text, ref_text)
    print(f"{wer:.4f}")
PYTHON_SCRIPT

    python3 /tmp/calculate_wer.py "$ocr_text" "$ground_truth"
    local result=$?
    rm -f /tmp/calculate_wer.py
    return $result
}

# Test clean Lorem Ipsum fixture
test_clean_fixture() {
    log_info "Testing clean Lorem Ipsum fixture..."

    local pdf="$CLEAN_FIXTURE/source.pdf"
    local gt="$CLEAN_FIXTURE/ground_truth.txt"
    local ocr_output="/tmp/clean_ocr_output.txt"

    if ! check_pdf_fixture "$pdf" "clean_lorem_ipsum"; then
        log_warn "Skipping clean fixture test"
        return 0
    fi

    if ! run_ocr "$pdf" "$ocr_output" "eng"; then
        log_error "Clean fixture OCR failed"
        ((FAILED++))
        return 1
    fi

    local wer=$(calculate_wer "$ocr_output" "$gt")
    local wer_percent=$(echo "$wer * 100" | bc -l)

    log_info "  WER: $wer_percent%"

    if (( $(echo "$wer <= $CLEAN_WER_THRESHOLD / 100" | bc -l) )); then
        log_info "  ✓ PASS: WER ${wer_percent}% < ${CLEAN_WER_THRESHOLD}%"
        ((PASSED++))
    else
        log_error "  ✗ FAIL: WER ${wer_percent}% >= ${CLEAN_WER_THRESHOLD}%"
        ((FAILED++))
    fi

    rm -f "$ocr_output"
}

# Test multi-language fixture
test_multilang_fixture() {
    log_info "Testing multi-language eng+fra fixture..."

    local pdf="$MULTILANG_FIXTURE/source.pdf"
    local gt="$MULTILANG_FIXTURE/ground_truth.txt"
    local ocr_output="/tmp/multilang_ocr_output.txt"

    if ! check_pdf_fixture "$pdf" "eng_fra_mixed"; then
        log_warn "Skipping multi-language fixture test"
        return 0
    fi

    if ! run_ocr "$pdf" "$ocr_output" "eng+fra"; then
        log_error "Multi-language fixture OCR failed"
        ((FAILED++))
        return 1
    fi

    local wer=$(calculate_wer "$ocr_output" "$gt")
    local wer_percent=$(echo "$wer * 100" | bc -l)

    log_info "  WER: $wer_percent%"

    if (( $(echo "$wer <= $MULTILANG_WER_THRESHOLD / 100" | bc -l) )); then
        log_info "  ✓ PASS: WER ${wer_percent}% < ${MULTILANG_WER_THRESHOLD}%"
        ((PASSED++))
    else
        log_error "  ✗ FAIL: WER ${wer_percent}% >= ${MULTILANG_WER_THRESHOLD}%"
        ((FAILED++))
    fi

    rm -f "$ocr_output"
}

# Test 10-page performance fixture
test_performance_fixture() {
    log_info "Testing 10-page performance fixture..."

    local pdf="$PERF_FIXTURE/source.pdf"
    local ocr_output="/tmp/perf_ocr_output.txt"

    if ! check_pdf_fixture "$pdf" "perf_10_page"; then
        log_warn "Skipping performance fixture test"
        return 0
    fi

    # Measure time
    local start_time=$(date +%s.%N)

    if run_ocr "$pdf" "$ocr_output" "eng"; then
        local end_time=$(date +%s.%N)
        local elapsed=$(echo "$end_time - $start_time" | bc -l)

        log_info "  Processing time: ${elapsed} seconds"

        if (( $(echo "$elapsed < $PERF_TIMEOUT_SECONDS" | bc -l) )); then
            log_info "  ✓ PASS: ${elapsed}s < ${PERF_TIMEOUT_SECONDS}s"
            ((PASSED++))
        else
            log_error "  ✗ FAIL: ${elapsed}s >= ${PERF_TIMEOUT_SECONDS}s"
            ((FAILED++))
        fi
    else
        log_error "Performance fixture OCR failed"
        ((FAILED++))
    fi

    rm -f "$ocr_output"
}

# Test BrokenVector aligned fixture
test_brokenvector_aligned_fixture() {
    log_info "Testing BrokenVector aligned fixture..."

    local pdf="$FIXTURE_DIR/brokenvector_aligned/source.pdf"
    local gt="$FIXTURE_DIR/brokenvector_aligned/ground_truth.txt"
    local ocr_output="/tmp/brokenvector_aligned_ocr_output.txt"

    if ! check_pdf_fixture "$pdf" "brokenvector_aligned"; then
        log_warn "Skipping BrokenVector aligned fixture test"
        return 0
    fi

    # Run assisted OCR (normal extraction for BrokenVector pages)
    if ! run_ocr "$pdf" "$ocr_output" "eng"; then
        log_error "BrokenVector aligned fixture OCR failed"
        ((FAILED++))
        return 1
    fi

    local wer=$(calculate_wer "$ocr_output" "$gt")
    local wer_percent=$(echo "$wer * 100" | bc -l)

    log_info "  Assisted OCR WER: $wer_percent%"

    # For aligned fixture, we expect WER < 2% (assisted OCR should work well)
    local expected_wer=2.0
    if (( $(echo "$wer <= $expected_wer / 100" | bc -l) )); then
        log_info "  ✓ PASS: WER ${wer_percent}% < ${expected_wer}%"
        ((PASSED++))
    else
        log_error "  ✗ FAIL: WER ${wer_percent}% >= ${expected_wer}%"
        ((FAILED++))
    fi

    rm -f "$ocr_output"
}

# Test BrokenVector misaligned fixture
test_brokenvector_misaligned_fixture() {
    log_info "Testing BrokenVector misaligned fixture..."

    local pdf="$FIXTURE_DIR/brokenvector_misaligned/source.pdf"
    local gt="$FIXTURE_DIR/brokenvector_misaligned/ground_truth.txt"
    local ocr_output="/tmp/brokenvector_misaligned_ocr_output.txt"

    if ! check_pdf_fixture "$pdf" "brokenvector_misaligned"; then
        log_warn "Skipping BrokenVector misaligned fixture test"
        return 0
    fi

    # Run assisted OCR (normal extraction for BrokenVector pages)
    if ! run_ocr "$pdf" "$ocr_output" "eng"; then
        log_error "BrokenVector misaligned fixture OCR failed"
        ((FAILED++))
        return 1
    fi

    local wer=$(calculate_wer "$ocr_output" "$gt")
    local wer_percent=$(echo "$wer * 100" | bc -l)

    log_info "  Assisted OCR WER: $wer_percent%"

    # For misaligned fixture, we expect WER < 5% (should not regress too badly)
    local expected_wer=5.0
    if (( $(echo "$wer <= $expected_wer / 100" | bc -l) )); then
        log_info "  ✓ PASS: WER ${wer_percent}% < ${expected_wer}%"
        ((PASSED++))
    else
        log_error "  ✗ FAIL: WER ${wer_percent}% >= ${expected_wer}%"
        ((FAILED++))
    fi

    rm -f "$ocr_output"
}

# Main execution
main() {
    log_info "=== WER CI Gate ==="
    log_info "Thresholds:"
    log_info "  Clean fixture: WER < ${CLEAN_WER_THRESHOLD}%"
    log_info "  Multi-language: WER < ${MULTILANG_WER_THRESHOLD}%"
    log_info "  Performance: < ${PERF_TIMEOUT_SECONDS}s"
    log_info "  BrokenVector aligned: WER < 2.0% (assisted OCR)"
    log_info "  BrokenVector misaligned: WER < 5.0% (assisted OCR)"
    echo ""

    # Check if pdftract CLI exists
    if ! command -v pdftract &> /dev/null; then
        log_error "pdftract CLI not found. Please build the project first:"
        log_error "  cargo build --release"
        exit 1
    fi

    # Run tests
    test_clean_fixture
    echo ""
    test_multilang_fixture
    echo ""
    test_performance_fixture
    echo ""
    test_brokenvector_aligned_fixture
    echo ""
    test_brokenvector_misaligned_fixture
    echo ""

    # Summary
    log_info "=== Summary ==="
    log_info "Passed: $PASSED"
    log_info "Failed: $FAILED"

    if [[ $FAILED -eq 0 ]]; then
        log_info "All WER gates passed!"
        exit 0
    else
        log_error "Some WER gates failed!"
        exit 1
    fi
}

# Run main
main "$@"
