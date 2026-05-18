#!/bin/bash
# Wrapper for pdftract text extraction
# Usage: run-pdftract.sh <pdf-file>
set -euo pipefail

PDF_FILE="$1"

if [ ! -f "$PDF_FILE" ]; then
    echo "ERROR: File not found: $PDF_FILE" >&2
    exit 1
fi

# Run pdftract text extraction
# Assumes pdftract binary is in PATH
pdftract extract "$PDF_FILE" --output text > /dev/null
