#!/usr/bin/env bash
# Measure Word Error Rate (WER) for OCR testing
# Usage: ./scripts/measure-wer.sh <fixture-pdf-path>
#
# This script:
# 1. Extracts text from the PDF using pdftract OCR
# 2. Compares against ground truth using the WER calculation script
# 3. Returns exit code 0 if WER ≤ 3%, 1 otherwise
#
# Environment variables:
# - PDFTRACT_PATH: Path to pdftract binary (default: cargo build output)
# - VERBOSE: Set to 1 for verbose output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Default pdftract path - try to find it in common locations
find_pdftract() {
    # Try cargo build output first
    if [[ -f "$PROJECT_ROOT/target/debug/pdftract" ]]; then
        echo "$PROJECT_ROOT/target/debug/pdftract"
        return 0
    fi

    # Try release build
    if [[ -f "$PROJECT_ROOT/target/release/pdftract" ]]; then
        echo "$PROJECT_ROOT/target/release/pdftract"
        return 0
    fi

    # Try CLI crate specifically
    if [[ -f "$PROJECT_ROOT/target/debug/pdftract-cli" ]]; then
        echo "$PROJECT_ROOT/target/debug/pdftract-cli"
        return 0
    fi

    if [[ -f "$PROJECT_ROOT/target/release/pdftract-cli" ]]; then
        echo "$PROJECT_ROOT/target/release/pdftract-cli"
        return 0
    fi

    # Fallback to just 'pdftract' and hope it's in PATH
    echo "pdftract"
}

PDFTRACT="${PDFTRACT_PATH:-$(find_pdftract)}"
FIXTURE_PDF="$1"

if [[ ! -f "$FIXTURE_PDF" ]]; then
    echo "Error: Fixture PDF not found: $FIXTURE_PDF" >&2
    exit 1
fi

# Determine ground truth path
FIXTURE_DIR="$(dirname "$FIXTURE_PDF")"
FIXTURE_NAME="$(basename "$FIXTURE_PDF" .pdf)"
GROUND_TRUTH="$FIXTURE_DIR/$FIXTURE_NAME.txt"

if [[ ! -f "$GROUND_TRUTH" ]]; then
    echo "Error: Ground truth file not found: $GROUND_TRUTH" >&2
    exit 1
fi

# Create temp directory for OCR output
TEMP_DIR="$(mktemp -d)"
trap "rm -rf '$TEMP_DIR'" EXIT

OCR_OUTPUT="$TEMP_DIR/ocr_output.txt"

echo "Measuring WER for: $FIXTURE_PDF"
echo "Ground truth: $GROUND_TRUTH"
echo ""

# Extract text using pdftract with OCR
if [[ "${VERBOSE:-0}" == "1" ]]; then
    echo "Running: $PDFTRACT extract '$FIXTURE_PDF' --ocr --text - > '$OCR_OUTPUT'"
fi

if ! "$PDFTRACT" extract "$FIXTURE_PDF" --ocr --text - > "$OCR_OUTPUT" 2>&1; then
    echo "Error: pdftract extract failed" >&2
    cat "$OCR_OUTPUT" >&2
    exit 1
fi

# Calculate WER
WER_SCRIPT="$PROJECT_ROOT/tests/fixtures/scanned/calculate_wer.py"

if [[ ! -f "$WER_SCRIPT" ]]; then
    echo "Error: WER calculation script not found: $WER_SCRIPT" >&2
    exit 1
fi

if [[ "${VERBOSE:-0}" == "1" ]]; then
    echo "Running WER calculation..."
    python3 "$WER_SCRIPT" "$GROUND_TRUTH" "$OCR_OUTPUT" --verbose --cer
else
    python3 "$WER_SCRIPT" "$GROUND_TRUTH" "$OCR_OUTPUT"
fi

WER_EXIT_CODE=$?

if [[ $WER_EXIT_CODE -eq 0 ]]; then
    echo ""
    echo "✓ WER ≤ 3% - PASSED"
else
    echo ""
    echo "✗ WER > 3% - FAILED"
fi

exit $WER_EXIT_CODE
