#!/usr/bin/env bash
# Measure Word Error Rate (WER) between OCR output and ground truth
# Usage: ./scripts/measure-wer.sh <ocr_output_file> <ground_truth_file>
#
# Computes WER = (substitutions + insertions + deletions) / total_words
# Exit code 0 if WER ≤ 3%, otherwise exit code 1
#
# Environment variables:
# - VERBOSE: Set to 1 for verbose output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Show usage
show_usage() {
    cat <<EOF
Usage: $0 <ocr_output_file> <ground_truth_file>

Measure Word Error Rate (WER) between OCR output and ground truth text.

Arguments:
  ocr_output_file    Path to file containing OCR output text
  ground_truth_file  Path to file containing ground truth text

Exit codes:
  0  WER ≤ 3% (passes quality gate)
  1  WER > 3% (fails quality gate)
  2  Error (missing files, etc.)

Example:
  $0 output.txt ground-truth.txt

Environment:
  VERBOSE=1  Enable verbose output
EOF
}

# Parse arguments
if [[ "${1:-}" == "--help" ]] || [[ "${1:-}" == "-h" ]]; then
    show_usage
    exit 0
fi

if [[ $# -ne 2 ]]; then
    echo "Error: Invalid arguments" >&2
    echo "" >&2
    show_usage >&2
    exit 2
fi

OCR_OUTPUT="$1"
GROUND_TRUTH="$2"

# Validate input files
if [[ ! -f "$OCR_OUTPUT" ]]; then
    echo "Error: OCR output file not found: $OCR_OUTPUT" >&2
    exit 2
fi

if [[ ! -f "$GROUND_TRUTH" ]]; then
    echo "Error: Ground truth file not found: $GROUND_TRUTH" >&2
    exit 2
fi

# Find WER calculation script
WER_SCRIPT="$PROJECT_ROOT/tools/calculate_wer.py"

if [[ ! -f "$WER_SCRIPT" ]]; then
    echo "Error: WER calculation script not found: $WER_SCRIPT" >&2
    exit 2
fi

# Calculate WER
if [[ "${VERBOSE:-0}" == "1" ]]; then
    python3 "$WER_SCRIPT" "$GROUND_TRUTH" "$OCR_OUTPUT" --verbose
else
    python3 "$WER_SCRIPT" "$GROUND_TRUTH" "$OCR_OUTPUT"
fi

WER_EXIT_CODE=$?

# Exit with the WER script's exit code (0 for WER ≤ 3%, 1 for WER > 3%)
exit $WER_EXIT_CODE
